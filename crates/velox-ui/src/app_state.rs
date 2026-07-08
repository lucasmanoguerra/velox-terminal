//! Shared application state between UI panels and GPU renderer.
//!
//! `AppState` is the single source of truth for UI-visible state.
//! It is mutated on the main thread (winit event loop), so no locks are needed.
//!
//! # Live Data Flow
//!
//! 1. [`MarketDataPipeline`] polls the exchange feed's ring buffer and pushes
//!    completed candles (for all timeframes) into an `mpsc::UnboundedReceiver`.
//! 2. [`poll_candles`](AppState::poll_candles) drains the channel and stores
//!    candles in a `HashMap<i64, Vec<Candle>>` keyed by `timeframe_secs`.
//! 3. `self.candles` always reflects the **selected** timeframe's data.
//! 4. The chart renderer reads `self.candles` to upload GPU buffers.
//! 5. [`set_timeframe`](AppState::set_timeframe) swaps `self.candles` from
//!    the hash map and re-scales the view.

use std::collections::HashMap;
use tokio::sync::mpsc;
use velox_chart::interaction::{ChartInteraction, ChartView};
use velox_chart::overlay::OverlayManager;
use velox_core::{Candle, Order, OrderId, Position, Side};
use velox_oms::PaperTrader;

/// Shared application state, mutated sequentially on the main thread.
pub struct AppState {
    /// Candles for the **currently selected** timeframe — fed to the chart renderer.
    pub candles: Vec<Candle>,

    /// All candles indexed by timeframe_secs.
    candles_by_tf: HashMap<i64, Vec<Candle>>,

    /// Available timeframes in seconds (e.g. [60, 300, 3600]).
    pub timeframes: Vec<i64>,

    /// Currently selected timeframe in seconds.
    pub selected_timeframe: i64,

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
    candle_rx: Option<mpsc::UnboundedReceiver<Candle>>,

    /// Metrics for display
    pub ticks_processed: u64,
    pub candles_produced: u64,
    pub feed_connected: bool,

    // ── Order management ───────────────────────────────────────────────
    /// Paper trading engine (OrderManager + mock execution + positions).
    pub paper_trader: PaperTrader,

    /// Current quantity in the order entry slider.
    pub order_entry_qty: f64,

    /// Last order submission error (displayed briefly in UI).
    pub order_error: Option<String>,

    /// Last order success message (displayed briefly in UI).
    pub order_success: Option<String>,
}

impl AppState {
    /// Create a new `AppState` with initial candles.
    pub fn new(candles: Vec<Candle>) -> Self {
        // Detect timeframe from first candle, default to 60s
        let tf = candles.first().map(|c| c.timeframe_secs).unwrap_or(60);
        let view = ChartView::from_candles(&candles);
        let mut map = HashMap::new();
        map.insert(tf, candles.clone());
        Self {
            candles,
            candles_by_tf: map,
            timeframes: vec![tf],
            selected_timeframe: tf,
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
            paper_trader: PaperTrader::new(100_000.0),
            order_entry_qty: 0.01,
            order_error: None,
            order_success: None,
        }
    }

    /// Create an empty `AppState` (no initial candles, no chart view).
    /// Used when the exchange feed will provide the first candles.
    pub fn empty(timeframes: &[i64]) -> Self {
        let tf = timeframes.first().copied().unwrap_or(60);
        Self {
            candles: Vec::new(),
            candles_by_tf: HashMap::new(),
            timeframes: timeframes.to_vec(),
            selected_timeframe: tf,
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
            paper_trader: PaperTrader::new(100_000.0),
            order_entry_qty: 0.01,
            order_error: None,
            order_success: None,
        }
    }

    /// Set the receiver for live candle data from the market data pipeline.
    pub fn set_candle_receiver(&mut self, rx: mpsc::UnboundedReceiver<Candle>) {
        self.candle_rx = Some(rx);
    }

    /// Poll the candle channel — call once per frame.
    ///
    /// Drains all available candles (for ALL timeframes) and stores them
    /// in `candles_by_tf`. If a candle matches the currently selected
    /// timeframe, it is also appended to `self.candles` so the chart updates.
    ///
    /// Returns the number of new candles received.
    pub fn poll_candles(&mut self) -> usize {
        // Drain the channel into a local vec to avoid borrow conflicts
        let mut batch: Vec<Candle> = Vec::new();
        if let Some(rx) = &mut self.candle_rx {
            loop {
                match rx.try_recv() {
                    Ok(candle) => batch.push(candle),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        self.feed_connected = false;
                        break;
                    }
                }
            }
        }

        if batch.is_empty() {
            return 0;
        }

        let was_empty = self.candles.is_empty();
        let active_tf = self.selected_timeframe;
        let mut did_reset = false;

        for candle in batch {
            let tf = candle.timeframe_secs;
            let bucket = self.candles_by_tf.entry(tf).or_default();
            bucket.push(candle);

            // Keep a window of 500 per bucket
            if bucket.len() > 1000 {
                bucket.drain(..bucket.len() - 500);
            }

            // If this candle matches the active timeframe, update the view
            if tf == active_tf {
                if !did_reset && was_empty {
                    self.reset_view();
                    did_reset = true;
                }
                self.candles.push(candle);
            }
        }

        // Window for the active candles too
        if self.candles.len() > 1000 {
            self.candles.drain(..self.candles.len() - 500);
        }

        self.candles_produced = self.candles_by_tf.values().map(|v| v.len() as u64).sum();
        self.candles.len()
    }

    /// Switch the active timeframe.
    ///
    /// Swaps `self.candles` from the hash map and re-calculates the chart view.
    /// If the requested timeframe has no candles yet, the view is reset empty.
    pub fn set_timeframe(&mut self, tf: i64) {
        if !self.timeframes.contains(&tf) {
            return;
        }
        self.selected_timeframe = tf;

        // Swap the active candle view
        if let Some(bucket) = self.candles_by_tf.get(&tf) {
            let new_view = ChartView::from_candles(bucket);
            self.candles = bucket.clone();
            self.chart_interaction = ChartInteraction::new(new_view);
        } else {
            self.candles.clear();
            self.chart_interaction = ChartInteraction::new(ChartView::from_candles(&[]));
        }
        self.needs_redraw = true;
    }

    /// Reset the chart view to fit all candles for the current timeframe.
    pub fn reset_view(&mut self) {
        let view = if self.candles.is_empty() {
            ChartView::from_candles(&[])
        } else {
            ChartView::from_candles(&self.candles)
        };
        self.chart_interaction = ChartInteraction::new(view);
    }

    /// Human-readable label for the current timeframe.
    pub fn timeframe_label(&self) -> String {
        seconds_to_tf_label(self.selected_timeframe)
    }

    /// All timeframe labels for the selector UI.
    pub fn timeframe_labels(&self) -> Vec<(i64, String)> {
        self.timeframes
            .iter()
            .map(|&tf| (tf, seconds_to_tf_label(tf)))
            .collect()
    }

    /// Connect to a live feed.
    pub fn set_feed_connected(&mut self, connected: bool) {
        self.feed_connected = connected;
    }

    // ── Order methods ─────────────────────────────────────────────────

    /// Submit a buy market order with the current `order_entry_qty`.
    pub fn buy_market(&mut self) {
        let sym = self.symbol.clone();
        let qty = self.order_entry_qty;
        match self.paper_trader.submit_market_order(&sym, Side::Buy, qty) {
            Ok(id) => {
                self.order_success = Some(format!("Buy {} {} (ID: {:.8})", qty, sym, id.0));
                self.order_error = None;
            }
            Err(e) => {
                self.order_error = Some(e);
                self.order_success = None;
            }
        }
    }

    /// Submit a sell market order with the current `order_entry_qty`.
    pub fn sell_market(&mut self) {
        let sym = self.symbol.clone();
        let qty = self.order_entry_qty;
        match self.paper_trader.submit_market_order(&sym, Side::Sell, qty) {
            Ok(id) => {
                self.order_success = Some(format!("Sell {} {} (ID: {:.8})", qty, sym, id.0));
                self.order_error = None;
            }
            Err(e) => {
                self.order_error = Some(e);
                self.order_success = None;
            }
        }
    }

    /// Cancel an open order by ID.
    pub fn cancel_order(&mut self, order_id: OrderId) {
        match self.paper_trader.cancel_order(order_id) {
            Ok(()) => {
                self.order_success = Some(format!("Canceled order {:.8}", order_id.0));
                self.order_error = None;
            }
            Err(e) => {
                self.order_error = Some(e);
            }
        }
    }

    /// Execute open market orders at the latest known price.
    /// Called automatically during `poll_market_data()`.
    pub fn execute_open_orders(&mut self, price: f64) -> usize {
        let sym = self.symbol.clone();
        self.paper_trader.execute_open_orders(&sym, price)
    }

    /// Access computed positions.
    pub fn positions(&self) -> Vec<Position> {
        self.paper_trader.positions()
    }

    /// All orders for display.
    pub fn orders(&self) -> Vec<&Order> {
        self.paper_trader.orders()
    }

    /// Open (non-terminal) orders.
    pub fn open_orders(&self) -> Vec<&Order> {
        self.paper_trader.open_orders()
    }

    /// Update account equity/P&L from current positions.
    pub fn update_account(&mut self) {
        self.paper_trader.update_account();
    }

    /// Clear transient feedback messages.
    pub fn clear_feedback(&mut self) {
        // Keep messages around for a frame; clear them after display
        self.order_error = None;
        self.order_success = None;
    }
}

/// Convert seconds to a human-readable timeframe label.
fn seconds_to_tf_label(secs: i64) -> String {
    match secs {
        60 => "1m".into(),
        120 => "2m".into(),
        180 => "3m".into(),
        300 => "5m".into(),
        600 => "10m".into(),
        900 => "15m".into(),
        1800 => "30m".into(),
        3600 => "1h".into(),
        7200 => "2h".into(),
        14400 => "4h".into(),
        21600 => "6h".into(),
        43200 => "12h".into(),
        86400 => "1D".into(),
        604800 => "1W".into(),
        2_592_000 => "1M".into(),
        _ if secs < 3600 => format!("{}m", secs / 60),
        _ if secs < 86400 => format!("{}h", secs / 3600),
        _ => format!("{}s", secs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_candle(tf_secs: i64, close: f64) -> Candle {
        Candle {
            symbol: *b"BTCUSD\0\0",
            open: close - 100.0,
            high: close + 50.0,
            low: close - 150.0,
            close,
            volume: 100.0,
            timestamp: Utc::now(),
            timeframe_secs: tf_secs,
            trade_count: Some(10),
            vwap: Some(close),
        }
    }

    #[test]
    fn test_empty_with_timeframes() {
        let state = AppState::empty(&[60, 300, 3600]);
        assert_eq!(state.timeframes.len(), 3);
        assert_eq!(state.selected_timeframe, 60);
        assert!(state.candles.is_empty());
    }

    #[test]
    fn test_seconds_to_label() {
        assert_eq!(seconds_to_tf_label(60), "1m");
        assert_eq!(seconds_to_tf_label(300), "5m");
        assert_eq!(seconds_to_tf_label(3600), "1h");
        assert_eq!(seconds_to_tf_label(86400), "1D");
    }

    #[test]
    fn test_set_timeframe_switches_candles() {
        let mut state = AppState::empty(&[60, 300]);

        // Manually insert candles into the 5m bucket
        state.candles_by_tf.insert(
            300,
            vec![make_candle(300, 50000.0), make_candle(300, 50100.0)],
        );

        // Switch to 5m
        state.set_timeframe(300);
        assert_eq!(state.selected_timeframe, 300);
        assert_eq!(state.candles.len(), 2);
        assert_eq!(state.candles[0].timeframe_secs, 300);
    }
}
