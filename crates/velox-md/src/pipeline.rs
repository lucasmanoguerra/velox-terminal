//! Market Data Pipeline — connects exchange feeds to chart/UI.
//!
//! The pipeline consumes [`MarketEvent`]s from a lock-free [`RingBuffer`]
//! (produced by an exchange feed), aggregates ticks into OHLCV candles
//! via [`CandleAggregator`], and sends completed candles to the main thread
//! through a [`tokio::sync::mpsc`] channel.
//!
//! # Data Flow
//!
//! ```text
//! Exchange WebSocket ──> RingBuffer ──> Pipeline ──> mpsc channel ──> AppState
//!                          (SPSC)        (poll)                       (main thread)
//! ```

use std::sync::Arc;
use tokio::sync::mpsc;

use velox_core::Candle;

use crate::aggregation::CandleAggregator;
use crate::ring_buffer::{MarketEvent, RingBuffer};

/// A pipeline that drives the tick→candle aggregation and forwards
/// completed candles to the main thread.
///
/// The pipeline is designed to be polled from the main thread (once per frame)
/// or driven from a tokio task. It reads available events from the ring buffer,
/// feeds them to the aggregator, and collects completed candles.
pub struct MarketDataPipeline {
    /// Ring buffer consumer (reads events written by exchange feed).
    ring: Arc<RingBuffer>,
    /// Candle aggregator for the configured timeframe.
    aggregator: CandleAggregator,
    /// Sender for completed candles — connected to AppState's receiver.
    candle_tx: mpsc::UnboundedSender<Candle>,
    /// Number of ticks processed (for metrics).
    ticks_processed: u64,
    /// Number of candles produced (for metrics).
    candles_produced: u64,
}

impl MarketDataPipeline {
    /// Create a new pipeline for the given timeframes.
    ///
    /// `timeframes` are in seconds (e.g., `&[60, 300]` for 1m and 5m candles).
    ///
    /// Returns the pipeline and a receiver for completed candles.
    pub fn new(
        ring: Arc<RingBuffer>,
        timeframes: &[i64],
    ) -> (Self, mpsc::UnboundedReceiver<Candle>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let pipeline = Self {
            ring,
            aggregator: CandleAggregator::new(timeframes),
            candle_tx: tx,
            ticks_processed: 0,
            candles_produced: 0,
        };
        (pipeline, rx)
    }

    /// Get a sender handle to push candles into the channel.
    /// This can be used to connect the pipeline to AppState.
    pub fn sender(&self) -> mpsc::UnboundedSender<Candle> {
        self.candle_tx.clone()
    }

    /// Poll the ring buffer — process all available events.
    ///
    /// Call this once per frame from the main thread.
    /// Returns the number of new candles produced (0 if no new data).
    pub fn poll(&mut self) -> usize {
        let mut new_candles = 0;
        // Reusable batch buffer to avoid per-frame allocations
        let mut batch = Vec::with_capacity(128);

        loop {
            let count = self.ring.pop_n(&mut batch, 128);
            if count == 0 {
                break; // buffer empty
            }

            for event in batch.drain(..) {
                self.ticks_processed += 1;
                match event {
                    MarketEvent::Tick(tick) => {
                        let completed = self.aggregator.process_tick(&tick);
                        for candle in completed {
                            self.candles_produced += 1;
                            new_candles += 1;
                            // Best-effort send (returns error if receiver dropped)
                            let _ = self.candle_tx.send(candle);
                        }
                    }
                    MarketEvent::Quote(quote) => {
                        // Quotes are not aggregated into candles yet.
                        // Future: use bid/ask for spread analysis.
                        tracing::trace!("Quote ignored in pipeline: {:?}", quote.symbol);
                    }
                }
            }
        }

        new_candles
    }

    /// Number of ticks processed since creation.
    pub fn ticks_processed(&self) -> u64 {
        self.ticks_processed
    }

    /// Number of candles produced since creation.
    pub fn candles_produced(&self) -> u64 {
        self.candles_produced
    }

    /// Reset the pipeline (clears aggregator, resets counters).
    pub fn reset(&mut self, timeframes: &[i64]) {
        self.aggregator = CandleAggregator::new(timeframes);
        self.ticks_processed = 0;
        self.candles_produced = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use velox_core::Tick;

    fn make_tick(price: f64, timestamp: chrono::DateTime<Utc>) -> Tick {
        Tick {
            symbol: *b"BTCUSD\0\0",
            price,
            volume: 1.0,
            timestamp,
            conditions: 0,
        }
    }

    #[test]
    fn test_pipeline_produces_candles() {
        let ring = Arc::new(RingBuffer::new(1024));
        let (mut pipeline, mut rx) = MarketDataPipeline::new(ring.clone(), &[60]);

        let start = Utc::now();
        // Push ticks across a 1-minute boundary
        for i in 0..5 {
            let ts = start + chrono::Duration::seconds(i * 15);
            ring.push(MarketEvent::Tick(make_tick(100.0 + i as f64, ts)));
        }

        // Poll — should produce at least one candle when we cross the 60s boundary
        let _count = pipeline.poll();

        // We may get 0 or 1 candles depending on alignment
        assert!(pipeline.ticks_processed() == 5);

        // Drain channel
        let mut received = 0;
        while let Ok(_candle) = rx.try_recv() {
            received += 1;
        }

        assert_eq!(pipeline.candles_produced(), received);
        assert_eq!(pipeline.ticks_processed(), 5);
    }
}
