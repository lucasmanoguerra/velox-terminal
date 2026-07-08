//! Shared application state between UI panels and GPU renderer.
//!
//! `AppState` is the single source of truth for UI-visible state.
//! It is mutated on the main thread (winit event loop), so no locks are needed.
//!
//! # Live Data Flow
//!
//! 1. `MarketDataPipeline` (polled each frame via [`poll_candles`](AppState::poll_candles))
//!    reads from the exchange feed's ring buffer and pushes completed candles
//!    into an `mpsc::UnboundedReceiver`.
//! 2. [`poll_candles`](AppState::poll_candles) drains the channel and updates `self.candles`.
//! 3. The chart renderer reads `self.candles` to upload GPU buffers.

use tokio::sync::mpsc;
use velox_chart::interaction::{ChartInteraction, ChartView};
use velox_chart::overlay::OverlayManager;
use velox_core::Candle;

/// Shared application state, mutated sequentially on the main thread.
pub struct AppState {
    /// Current candles displayed on the chart.
    pub candles: Vec<Candle>,

    /// Zoom/pan state.
    pub chart_interaction: ChartInteraction,

    /// Indicator overlays.
    pub overlays: OverlayManager,

    /// Area of the chart panel in egui logical pixels.
    /// Set by `PanelManager::show()` every frame.
    pub chart_panel_rect: egui::Rect,

    /// Cursor position in physical (DPI-scaled) pixels.
    pub cursor_pos_physical: (f64, f64),

    /// Whether a redraw has been requested.
    pub needs_redraw: bool,

    /// Monotonic frame counter.
    pub frame_count: u64,

    /// Current symbol being displayed.
    pub symbol: String,

    /// Receiver for completed candles from the market data pipeline.
    /// Set by [`set_candle_receiver`](AppState::set_candle_receiver).
    candle_rx: Option<mpsc::UnboundedReceiver<Candle>>,

    /// Metrics for display
    pub ticks_processed: u64,
    pub candles_produced: u64,
    pub feed_connected: bool,
}

impl AppState {
    /// Create a new `AppState` with initial candles.
    pub fn new(candles: Vec<Candle>) -> Self {
        let view = ChartView::from_candles(&candles);
        Self {
            candles,
            chart_interaction: ChartInteraction::new(view),
            overlays: OverlayManager::new(),
            chart_panel_rect: egui::Rect::ZERO,
            cursor_pos_physical: (0.0, 0.0),
            needs_redraw: true,
            frame_count: 0,
            symbol: "BTC/USD".into(),
            candle_rx: None,
            ticks_processed: 0,
            candles_produced: 0,
            feed_connected: false,
        }
    }

    /// Create an empty `AppState` (no initial candles, no chart view).
    /// Used when the exchange feed will provide the first candles.
    pub fn empty() -> Self {
        Self {
            candles: Vec::new(),
            chart_interaction: ChartInteraction::new(ChartView::from_candles(&[])),
            overlays: OverlayManager::new(),
            chart_panel_rect: egui::Rect::ZERO,
            cursor_pos_physical: (0.0, 0.0),
            needs_redraw: true,
            frame_count: 0,
            symbol: "BTC/USD".into(),
            candle_rx: None,
            ticks_processed: 0,
            candles_produced: 0,
            feed_connected: false,
        }
    }

    /// Set the receiver for live candle data from the market data pipeline.
    pub fn set_candle_receiver(&mut self, rx: mpsc::UnboundedReceiver<Candle>) {
        self.candle_rx = Some(rx);
    }

    /// Poll the candle channel — call once per frame.
    ///
    /// Drains all available candles from the market data pipeline
    /// and appends them to `self.candles`. Resets the chart view
    /// when the first candle arrives.
    ///
    /// Returns the number of new candles received.
    pub fn poll_candles(&mut self) -> usize {
        let Some(rx) = &mut self.candle_rx else {
            return 0;
        };

        let mut count = 0;
        loop {
            match rx.try_recv() {
                Ok(candle) => {
                    let is_first = self.candles.is_empty();
                    self.candles.push(candle);

                    // Auto-scale view on first candle or when buffer is small
                    if is_first {
                        self.chart_interaction = ChartInteraction::new(
                            ChartView::from_candles(&self.candles),
                        );
                    }

                    count += 1;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.feed_connected = false;
                    break;
                }
            }
        }

        // Keep a reasonable window of candles (last 500)
        if self.candles.len() > 1000 {
            self.candles.drain(..self.candles.len() - 500);
        }

        count
    }

    /// Connect to a live feed (called when the pipeline provides a receiver).
    pub fn set_feed_connected(&mut self, connected: bool) {
        self.feed_connected = connected;
    }
}
