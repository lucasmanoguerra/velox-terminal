//! Shared application state between UI panels and GPU renderer.
//!
//! `AppState` is the single source of truth for UI-visible state.
//! It is mutated on the main thread (winit event loop), so no locks are needed.
//!
//! # Live Data Flow
//!
//! 1. [`MarketDataPipeline`] polls the exchange feed's ring buffer and pushes
//!    completed candles into an `mpsc::UnboundedReceiver`.
//! 2. [`poll_candles`](AppState::poll_candles) drains the channel and stores
//!    candles in a `HashMap<i64, Vec<Candle>>` keyed by `timeframe_secs`.
//! 3. `self.candles` always reflects the **selected** timeframe's data.
//! 4. The chart renderer reads `self.candles` to upload GPU buffers.
//!
//! # Trading Modes
//!
//! - **Paper** (`TradingMode::Paper`): all orders go to [`PaperTrader`] with
//!   auto-fill at candle close. No broker connection required.
//! - **Live** (`TradingMode::Live`): orders are submitted to both [`PaperTrader`]
//!   (for local position tracking) and an optional [`BrokerClient`] (for real
//!   execution). Broker fills arrive via the User Data Stream and are applied
//!   to the local order manager.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use velox_broker::{BrokerClient, BrokerConfig};
use velox_chart::interaction::{ChartInteraction, ChartView};
use velox_chart::overlay::OverlayManager;
use velox_core::{Candle, NewOrder, Order, OrderBook, OrderId, OrderType, Position, Side, TimeInForce};
use velox_oms::PaperTrader;

/// Trading mode — determines how orders are routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradingMode {
    /// All orders are simulated locally (no broker connection needed).
    Paper,
    /// Orders are submitted to a real broker AND tracked locally.
    Live,
}

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

    // ── Order book depth ───────────────────────────────────────────────
    /// Latest order book snapshot (bids/asks) for the current symbol.
    pub depth: Option<OrderBook>,

    // ── Scrollbar / follow ─────────────────────────────────────────────
    /// Normalized scroll position of the chart view.
    pub scroll_pos: f64,

    /// Whether to auto-scroll to the newest data when new candles arrive.
    pub follow_mode: bool,

    // ── Broker / live trading ──────────────────────────────────────────
    /// Connected broker client (if any). Shared via Arc so async tasks can
    /// hold a reference.
    pub broker: Option<Arc<dyn BrokerClient>>,

    /// Current trading mode.
    pub trading_mode: TradingMode,

    /// Broker config (API keys, endpoint). Stored here so the UI can
    /// display/clear it.
    pub broker_config: Option<BrokerConfig>,

    /// Whether the broker is connected.
    pub broker_connected: bool,

    // ── Broker connection UI signals ────────────────────────────────
    /// API key input from the UI (pending).
    pub connect_api_key: String,
    /// API secret input from the UI (pending).
    pub connect_api_secret: String,
    /// API base URL input from the UI (pending).
    pub connect_base_url: String,
    /// Set to `true` by the UI to request a broker connection.
    /// Cleared by `app.rs` after processing.
    pub connect_requested: bool,
    /// Set to `true` by the UI to request a broker disconnect.
    pub disconnect_requested: bool,
    /// Last broker connection error.
    pub broker_error: Option<String>,
}

impl AppState {
    /// Create a new `AppState` with initial candles.
    pub fn new(candles: Vec<Candle>) -> Self {
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
            depth: None,
            scroll_pos: 0.0,
            follow_mode: true,
            broker: None,
            trading_mode: TradingMode::Paper,
            broker_config: None,
            broker_connected: false,
            connect_api_key: String::new(),
            connect_api_secret: String::new(),
            connect_base_url: String::new(),
            connect_requested: false,
            disconnect_requested: false,
            broker_error: None,
        }
    }

    /// Create an empty `AppState` (no initial candles).
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
            depth: None,
            scroll_pos: 0.0,
            follow_mode: true,
            broker: None,
            trading_mode: TradingMode::Paper,
            broker_config: None,
            broker_connected: false,
            connect_api_key: String::new(),
            connect_api_secret: String::new(),
            connect_base_url: String::new(),
            connect_requested: false,
            disconnect_requested: false,
            broker_error: None,
        }
    }

    /// Set the broker client and switch to Live mode.
    pub fn set_broker(&mut self, broker: Arc<dyn BrokerClient>, config: BrokerConfig) {
        self.broker = Some(broker);
        self.broker_config = Some(config);
        self.broker_connected = true;
        self.trading_mode = TradingMode::Live;
    }

    /// Remove the broker and revert to Paper mode.
    pub fn clear_broker(&mut self) {
        self.broker = None;
        self.broker_config = None;
        self.broker_connected = false;
        self.trading_mode = TradingMode::Paper;
        self.broker_error = None;
    }

    /// Set the receiver for live candle data from the market data pipeline.
    pub fn set_candle_receiver(&mut self, rx: mpsc::UnboundedReceiver<Candle>) {
        self.candle_rx = Some(rx);
    }

    /// Poll the candle channel — call once per frame.
    pub fn poll_candles(&mut self) -> usize {
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

            if bucket.len() > 1000 {
                bucket.drain(..bucket.len() - 500);
            }

            if tf == active_tf {
                if !did_reset && was_empty {
                    self.reset_view();
                    did_reset = true;
                }
                self.candles.push(candle);

                if self.follow_mode && !self.candles.is_empty() {
                    let (_, data_end) = ChartInteraction::data_range(&self.candles);
                    if !self.chart_interaction.is_at_right_edge(data_end) {
                        let range = self.chart_interaction.view.time_range();
                        self.chart_interaction.view.time_end = data_end;
                        self.chart_interaction.view.time_start = data_end - range;
                    }
                }
            }
        }

        if self.candles.len() > 1000 {
            self.candles.drain(..self.candles.len() - 500);
        }

        self.candles_produced = self.candles_by_tf.values().map(|v| v.len() as u64).sum();
        self.candles.len()
    }

    /// Switch the active timeframe.
    pub fn set_timeframe(&mut self, tf: i64) {
        if !self.timeframes.contains(&tf) {
            return;
        }
        self.selected_timeframe = tf;

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

    // ── Scrollbar ──────────────────────────────────────────────────────

    /// Sync `scroll_pos` from the current chart view and data range.
    pub fn sync_scroll_pos(&mut self) {
        if self.candles.is_empty() {
            self.scroll_pos = 0.0;
            return;
        }
        let (data_start, data_end) = ChartInteraction::data_range(&self.candles);
        self.scroll_pos = self.chart_interaction.scroll_pos(data_start, data_end);
    }

    /// Set the scroll position and update the chart view accordingly.
    pub fn set_scroll_pos(&mut self, fraction: f64) {
        if self.candles.is_empty() {
            return;
        }
        let (data_start, data_end) = ChartInteraction::data_range(&self.candles);
        self.chart_interaction
            .set_scroll_pos(fraction, data_start, data_end);
        self.scroll_pos = fraction;
        self.needs_redraw = true;
    }

    /// Toggle follow mode (auto-scroll to newest data).
    pub fn toggle_follow_mode(&mut self) {
        self.follow_mode = !self.follow_mode;
    }

    // ── Order methods ─────────────────────────────────────────────────

    /// Submit a buy market order with the current `order_entry_qty`.
    ///
    /// In Live mode the order is also forwarded to the broker asynchronously.
    pub fn buy_market(&mut self) {
        let sym = self.symbol.clone();
        let qty = self.order_entry_qty;

        // 1. Submit locally for position tracking
        let order_id = match self.paper_trader.submit_market_order(&sym, Side::Buy, qty) {
            Ok(id) => id,
            Err(e) => {
                self.order_error = Some(e);
                self.order_success = None;
                return;
            }
        };

        // 2. If live mode, also submit to broker
        if self.trading_mode == TradingMode::Live {
            self.submit_to_broker(sym.clone(), Side::Buy, qty, order_id);
        }

        self.order_success = Some(format!("Buy {} {} (ID: {:.8})", qty, sym, order_id.0));
        self.order_error = None;
    }

    /// Submit a sell market order with the current `order_entry_qty`.
    pub fn sell_market(&mut self) {
        let sym = self.symbol.clone();
        let qty = self.order_entry_qty;

        let order_id = match self.paper_trader.submit_market_order(&sym, Side::Sell, qty) {
            Ok(id) => id,
            Err(e) => {
                self.order_error = Some(e);
                self.order_success = None;
                return;
            }
        };

        if self.trading_mode == TradingMode::Live {
            self.submit_to_broker(sym.clone(), Side::Sell, qty, order_id);
        }

        self.order_success = Some(format!("Sell {} {} (ID: {:.8})", qty, sym, order_id.0));
        self.order_error = None;
    }

    /// Cancel an open order by ID.
    pub fn cancel_order(&mut self, order_id: OrderId) {
        // Cancel locally first
        match self.paper_trader.cancel_order(order_id) {
            Ok(()) => {
                self.order_success = Some(format!("Canceled order {:.8}", order_id.0));
                self.order_error = None;
            }
            Err(e) => {
                self.order_error = Some(e);
                return;
            }
        }

        // If live mode, also cancel at broker
        if self.trading_mode == TradingMode::Live && let Some(ref broker) = self.broker {
            let broker = broker.clone();
            tokio::spawn(async move {
                if let Err(e) = broker.cancel_order(order_id).await {
                    tracing::error!("Broker cancel failed: {e}");
                }
            });
        }
    }

    /// Shared helper: submit a market order to the broker in the background.
    fn submit_to_broker(&self, symbol: String, side: Side, quantity: f64, order_id: OrderId) {
        let broker = match &self.broker {
            Some(b) => b.clone(),
            None => return,
        };

        let order = NewOrder {
            symbol,
            side,
            order_type: OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Day,
            client_order_id: Some(order_id.0.to_string()),
        };

        tokio::spawn(async move {
            match broker.submit_order(order).await {
                Ok(id) => tracing::info!("Live order submitted: {:.8}", id.0),
                Err(e) => tracing::error!("Live order failed: {e}"),
            }
        });
    }

    /// Execute open market orders at the latest known price.
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
        self.order_error = None;
        self.order_success = None;
    }
}

/// Minimal mock broker for test use.
#[cfg(test)]
struct MockBroker;

#[cfg(test)]
#[async_trait::async_trait]
impl BrokerClient for MockBroker {
    async fn connect(
        &self,
        _config: BrokerConfig,
    ) -> Result<velox_broker::ConnectionHandle, velox_core::CoreError> {
        Ok(velox_broker::ConnectionHandle {
            broker: "mock".into(),
            session_id: "mock-session".into(),
        })
    }

    async fn disconnect(
        &self,
        _handle: &velox_broker::ConnectionHandle,
    ) -> Result<(), velox_core::CoreError> {
        Ok(())
    }

    async fn submit_order(
        &self,
        _order: NewOrder,
    ) -> Result<OrderId, velox_core::CoreError> {
        Ok(OrderId::new())
    }

    async fn cancel_order(
        &self,
        _order_id: OrderId,
    ) -> Result<(), velox_core::CoreError> {
        Ok(())
    }

    async fn get_positions(
        &self,
    ) -> Result<Vec<Position>, velox_core::CoreError> {
        Ok(vec![])
    }

    async fn get_account_info(
        &self,
    ) -> Result<velox_core::AccountInfo, velox_core::CoreError> {
        Ok(velox_core::AccountInfo {
            cash: 100000.0,
            buying_power: 200000.0,
            equity: 100000.0,
            margin_used: 0.0,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            currency: "USD".to_string(),
        })
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

        state.candles_by_tf.insert(
            300,
            vec![make_candle(300, 50000.0), make_candle(300, 50100.0)],
        );

        state.set_timeframe(300);
        assert_eq!(state.selected_timeframe, 300);
        assert_eq!(state.candles.len(), 2);
        assert_eq!(state.candles[0].timeframe_secs, 300);
    }

    #[test]
    fn test_default_trading_mode_is_paper() {
        let state = AppState::empty(&[60]);
        assert_eq!(state.trading_mode, TradingMode::Paper);
        assert!(state.broker.is_none());
        assert!(!state.broker_connected);
    }

    #[test]
    fn test_set_broker_switches_to_live() {
        let mut state = AppState::empty(&[60]);
        let (broker, config) = make_mock_broker();
        state.set_broker(broker, config);

        assert_eq!(state.trading_mode, TradingMode::Live);
        assert!(state.broker.is_some());
        assert!(state.broker_connected);
    }

    #[test]
    fn test_clear_broker_reverts_to_paper() {
        let mut state = AppState::empty(&[60]);
        let (broker, config) = make_mock_broker();
        state.set_broker(broker, config);
        state.clear_broker();

        assert_eq!(state.trading_mode, TradingMode::Paper);
        assert!(state.broker.is_none());
        assert!(!state.broker_connected);
    }

    /// Create a mock broker / config pair for testing.
    fn make_mock_broker() -> (Arc<dyn BrokerClient>, BrokerConfig) {
        let config = BrokerConfig {
            api_key: "test_key".into(),
            api_secret: "test_secret".into(),
            base_url: "https://test.binance.com".into(),
            paper_trading: false,
        };
        let broker = Arc::new(MockBroker);
        (broker, config)
    }
}
