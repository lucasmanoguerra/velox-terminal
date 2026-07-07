//! Tick → Candle aggregation for multiple timeframes.

use std::collections::HashMap;
use velox_core::{Candle, Tick};

/// Manages candle aggregation for a symbol across multiple timeframes.
pub struct CandleAggregator {
    timeframes: Vec<i64>,         // seconds
    current_candles: HashMap<(String, i64), CandleBuilder>,
}

impl CandleAggregator {
    pub fn new(timeframes: &[i64]) -> Self {
        Self {
            timeframes: timeframes.to_vec(),
            current_candles: HashMap::new(),
        }
    }

    /// Process a tick and return any completed candles.
    pub fn process_tick(&mut self, tick: &Tick) -> Vec<Candle> {
        let symbol_str = symbol_to_string(&tick.symbol);
        let mut completed = Vec::new();

        for &tf in &self.timeframes {
            let key = (symbol_str.clone(), tf);
            let builder = self.current_candles.entry(key).or_insert_with(|| {
                CandleBuilder::new(tick.symbol, tick.timestamp, tf)
            });

            if let Some(candle) = builder.add_tick(tick) {
                completed.push(candle);
                // Start next candle
                let next_ts = align_timestamp(tick.timestamp, tf) + chrono::Duration::seconds(tf);
                self.current_candles.insert(
                    (symbol_str.clone(), tf),
                    CandleBuilder::new(tick.symbol, next_ts, tf),
                );
            }
        }

        completed
    }
}

/// Builds a single candle from ticks.
struct CandleBuilder {
    symbol: [u8; 8],
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    timestamp: chrono::DateTime<chrono::Utc>,
    timeframe_secs: i64,
    trade_count: u64,
    vwap_sum: f64,
    start_ts: chrono::DateTime<chrono::Utc>,
}

impl CandleBuilder {
    fn new(symbol: [u8; 8], timestamp: chrono::DateTime<chrono::Utc>, timeframe_secs: i64) -> Self {
        let aligned = align_timestamp(timestamp, timeframe_secs);
        Self {
            symbol,
            open: 0.0,
            high: f64::MIN,
            low: f64::MAX,
            close: 0.0,
            volume: 0.0,
            timestamp: timestamp,
            timeframe_secs,
            trade_count: 0,
            vwap_sum: 0.0,
            start_ts: aligned,
        }
    }

    /// Add a tick. Returns the completed candle if the tick exceeds this candle's timeframe.
    fn add_tick(&mut self, tick: &Tick) -> Option<Candle> {
        if tick.timestamp < self.start_ts {
            return None; // tick belongs to previous candle, ignore
        }

        let next_start = self.start_ts + chrono::Duration::seconds(self.timeframe_secs);
        if tick.timestamp >= next_start {
            // Finalize current candle
            let candle = self.build();
            return Some(candle);
        }

        // Accumulate
        if self.trade_count == 0 {
            self.open = tick.price;
        }
        self.high = self.high.max(tick.price);
        self.low = self.low.min(tick.price);
        self.close = tick.price;
        self.volume += tick.volume;
        self.trade_count += 1;
        self.vwap_sum += tick.price * tick.volume;

        None
    }

    fn build(&self) -> Candle {
        let vwap = if self.volume > 0.0 {
            Some(self.vwap_sum / self.volume)
        } else {
            None
        };

        Candle {
            symbol: self.symbol,
            open: self.open,
            high: if self.high == f64::MIN { 0.0 } else { self.high },
            low: if self.low == f64::MAX { 0.0 } else { self.low },
            close: self.close,
            volume: self.volume,
            timestamp: self.start_ts,
            timeframe_secs: self.timeframe_secs,
            trade_count: Some(self.trade_count),
            vwap,
        }
    }
}

fn align_timestamp(ts: chrono::DateTime<chrono::Utc>, timeframe_secs: i64) -> chrono::DateTime<chrono::Utc> {
    let unix = ts.timestamp();
    let aligned = unix - (unix % timeframe_secs);
    chrono::DateTime::from_timestamp(aligned, 0).unwrap_or(ts)
}

fn symbol_to_string(sym: &[u8; 8]) -> String {
    String::from_utf8_lossy(sym).trim().to_string()
}
