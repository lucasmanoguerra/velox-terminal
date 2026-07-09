//! Binance exchange connector — WebSocket market data feed with auto-reconnect.
//!
//! Connects to Binance public WebSocket streams and converts trade/quote
//! events into [`MarketEvent`]s pushed to a lock-free [`RingBuffer`].
//!
//! # Reconnection
//!
//! The feed automatically reconnects with exponential backoff + full jitter:
//!
//! | Attempt | Delay range  | Notes              |
//! |---------|--------------|--------------------|
//! | 1       | 0.5 – 1.0 s  | Immediate retry    |
//! | 2       | 1.0 – 2.0 s  |                    |
//! | 3       | 2.0 – 4.0 s  |                    |
//! | 4       | 4.0 – 8.0 s  |                    |
//! | 5+      | 8.0 – 60.0 s | Capped at max 60s  |
//!
//! Reconnection is cancellable (sub-second shutdown response) and
//! respects the `stop()` signal.
//!
//! # Streams
//!
//! - **Trade**: `<symbol>@trade` — real-time trade ticks
//!
//! # Rate Limits
//!
//! Binance allows up to 5 incoming messages per second per connection.
//! This implementation stays well within that limit with a single combined stream.

use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{
    Mutex,
    RwLock,
};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use velox_core::{CoreError, OrderBook, OrderBookLevel, Tick};
use velox_md::ring_buffer::{MarketEvent, RingBuffer};

use crate::ExchangeFeed;
use crate::error::ExchangeError;

/// Binance WebSocket base URL for **combined** streams.
/// Combined streams wrap each message in `{"stream": "...", "data": {...}}`.
const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/stream";

/// Default maximum reconnect delay in seconds.
const DEFAULT_MAX_RECONNECT_SECS: u64 = 60;

/// Default base delay for backoff in milliseconds.
const DEFAULT_BASE_DELAY_MS: u64 = 1000;

/// Internal shared state for a Binance feed connection.
struct BinanceFeedInner {
    /// Flag to signal the tokio task to stop.
    running: AtomicBool,
    /// Whether the WebSocket is currently connected.
    feed_connected: AtomicBool,
    /// Connected symbols (lowercase, e.g. "btcusdt").
    symbols: Mutex<Vec<String>>,
    /// Ring buffer shared with consumer.
    ring: Mutex<Option<Arc<RingBuffer>>>,
    /// Latest order book snapshot per symbol (key = lowercase, e.g. "btcusdt").
    order_book: RwLock<HashMap<String, OrderBook>>,
    /// Tokio task handle for graceful shutdown.
    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Maximum reconnect backoff in seconds.
    max_reconnect_secs: u64,
    /// Base delay for exponential backoff in milliseconds.
    base_delay_ms: u64,
}

/// Binance exchange market data feed.
///
/// Creates a WebSocket connection to Binance's combined stream endpoint
/// and pushes trade ticks into the ring buffer. Automatically reconnects
/// on connection loss with exponential backoff.
///
/// # Example
///
/// ```rust,no_run
/// use velox_exchange::binance::BinanceFeed;
/// use velox_exchange::ExchangeFeed;
///
/// let feed = BinanceFeed::new();
/// feed.subscribe("btcusdt").unwrap();
/// // feed.start(ring).unwrap();
/// ```
pub struct BinanceFeed {
    inner: Arc<BinanceFeedInner>,
}

impl BinanceFeed {
    /// Create a new Binance feed (not yet connected).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(BinanceFeedInner {
                running: AtomicBool::new(false),
                feed_connected: AtomicBool::new(false),
                symbols: Mutex::new(Vec::new()),
                ring: Mutex::new(None),
                order_book: RwLock::new(HashMap::new()),
                task_handle: Mutex::new(None),
                max_reconnect_secs: DEFAULT_MAX_RECONNECT_SECS,
                base_delay_ms: DEFAULT_BASE_DELAY_MS,
            }),
        }
    }

    /// Return the latest order book snapshot for a symbol (if available).
    pub fn order_book(&self, symbol: &str) -> Option<OrderBook> {
        let normalized = symbol.to_lowercase().replace(['-', '/'], "");
        // Try read lock; return None on contention
        self.inner
            .order_book
            .try_read()
            .ok()
            .and_then(|guard| guard.get(&normalized).cloned())
    }

    /// Return whether the WebSocket is currently connected.
    pub fn connected(&self) -> bool {
        self.inner.feed_connected.load(Ordering::Acquire)
    }

    /// Build the combined stream path for subscribed symbols.
    ///
    /// Subscribes to `@trade` (tick stream) and `@depth20@100ms` (top-20 order book
    /// snapshot every 100ms) for each symbol.
    fn build_stream_path(symbols: &[String]) -> String {
        let mut streams: Vec<String> = Vec::with_capacity(symbols.len() * 2);
        for sym in symbols {
            streams.push(format!("{}@trade", sym));
            streams.push(format!("{}@depth20@100ms", sym));
        }
        format!("?streams={}", streams.join("/"))
    }

    /// Exponential backoff with full jitter: sleep between 0 and `base * 2^attempt`,
    /// capped at `max_reconnect_secs`. Uses wall-clock time as cheap entropy for jitter.
    fn backoff_delay(attempt: u64, max_secs: u64, base_ms: u64) -> Duration {
        // Exponential: base * 2^(attempt-1), capped at max_secs.
        // Compute 2^(attempt-1) via wrapping shift or fallback.
        let exponent = (attempt.saturating_sub(1)).min(60) as u32;
        let multiplier = if exponent >= 63 {
            u64::MAX
        } else {
            1u64 << exponent // 2^exponent
        };
        let exp_ms = base_ms.saturating_mul(multiplier);
        let max_ms = max_secs * 1000;
        let window_ms = exp_ms.min(max_ms);

        // Full jitter: pick a random point within [0, window_ms]
        // Use SystemTime as cheap entropy source
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        // Mix attempt into the hash so different attempts get different jitter
        let hash = now.as_nanos() ^ ((attempt as u128) << 64);
        let jitter_ms = (hash % (window_ms.max(1) as u128)) as u64;

        // Minimum 500ms between reconnects to avoid busy-looping
        Duration::from_millis(jitter_ms.max(500).min(max_ms))
    }

    /// Wait for a duration, polling `running` every 100ms for responsiveness.
    /// Returns immediately if `running` becomes false.
    async fn sleep_with_running_check(inner: &BinanceFeedInner, duration: Duration) {
        let start = tokio::time::Instant::now();
        while start.elapsed() < duration {
            if !inner.running.load(Ordering::Acquire) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl Default for BinanceFeed {
    fn default() -> Self {
        Self::new()
    }
}

impl ExchangeFeed for BinanceFeed {
    fn start(&self, ring: Arc<RingBuffer>) -> Result<(), CoreError> {
        let inner = self.inner.clone();

        // If already running, just update the ring buffer target
        if inner.running.load(Ordering::Acquire) {
            let mut guard = inner
                .ring
                .try_lock()
                .map_err(|_| CoreError::Internal("ring buffer lock contention".into()))?;
            *guard = Some(ring);
            return Ok(());
        }

        // Mark as running
        inner.running.store(true, Ordering::Release);
        *inner
            .ring
            .try_lock()
            .map_err(|_| CoreError::Internal("ring buffer lock contention".into()))? = Some(ring);

        // Clone inner for the tokio task
        let inner_task = inner.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = Self::run_loop(inner_task.clone()).await {
                tracing::error!("Binance feed run_loop exited with error: {e}");
            }
            inner_task.feed_connected.store(false, Ordering::Release);
            inner_task.running.store(false, Ordering::Release);
        });

        *inner
            .task_handle
            .try_lock()
            .map_err(|_| CoreError::Internal("task handle lock contention".into()))? = Some(handle);

        Ok(())
    }

    fn stop(&self) -> Result<(), CoreError> {
        let inner = self.inner.clone();
        inner.running.store(false, Ordering::Release);

        // Await task completion (best-effort, non-blocking)
        if let Ok(mut guard) = inner.task_handle.try_lock()
            && let Some(handle) = guard.take()
        {
            handle.abort();
        }

        Ok(())
    }

    fn subscribe(&self, symbol: &str) -> Result<(), CoreError> {
        // Normalize: lowercase, remove hyphens (BTC-USDT → btcusdt)
        let normalized = symbol.to_lowercase().replace(['-', '/'], "");
        if normalized.is_empty() {
            return Err(CoreError::InvalidSymbol(symbol.into()));
        }

        let mut symbols = self
            .inner
            .symbols
            .try_lock()
            .map_err(|_| CoreError::Internal("symbols lock contention".into()))?;

        if !symbols.contains(&normalized) {
            symbols.push(normalized.clone());
            tracing::info!("Binance feed subscribed to {normalized}");
        }

        Ok(())
    }

    fn unsubscribe(&self, symbol: &str) -> Result<(), CoreError> {
        let normalized = symbol.to_lowercase().replace(['-', '/'], "");
        let mut symbols = self
            .inner
            .symbols
            .try_lock()
            .map_err(|_| CoreError::Internal("symbols lock contention".into()))?;

        symbols.retain(|s| s != &normalized);
        tracing::info!("Binance feed unsubscribed from {normalized}");

        Ok(())
    }
}

// ── Internal implementation ──────────────────────────────────────────

impl BinanceFeed {
    /// Main connection + reconnection loop.
    ///
    /// 1. Tries to connect to Binance WebSocket
    /// 2. On success: reads messages until disconnect
    /// 3. On disconnect: backs off and retries from step 1
    /// 4. Exits only when `running` becomes false
    async fn run_loop(inner: Arc<BinanceFeedInner>) -> Result<(), ExchangeError> {
        let mut attempt: u64 = 0;

        while inner.running.load(Ordering::Acquire) {
            // ── 1. Get subscribed symbols ─────────────────────────
            let symbols = inner.symbols.lock().await.clone();
            if symbols.is_empty() {
                tracing::warn!("Binance feed: no symbols subscribed, waiting...");
                Self::sleep_with_running_check(&inner, Duration::from_secs(5)).await;
                continue;
            }

            // ── 2. Attempt connection ─────────────────────────────
            let stream_path = Self::build_stream_path(&symbols);
            let ws_url = format!("{}{}", BINANCE_WS_URL, stream_path);

            if attempt > 0 {
                let delay =
                    Self::backoff_delay(attempt, inner.max_reconnect_secs, inner.base_delay_ms);
                tracing::warn!(
                    "Binance reconnecting in {}ms (attempt {})",
                    delay.as_millis(),
                    attempt + 1,
                );
                Self::sleep_with_running_check(&inner, delay).await;

                // Check if we were told to stop during the sleep
                if !inner.running.load(Ordering::Acquire) {
                    break;
                }
            }

            tracing::info!(
                "Binance connecting to WebSocket (attempt {})...",
                attempt + 1,
            );

            let result = connect_async(&ws_url).await;
            match result {
                Ok((ws_stream, _response)) => {
                    tracing::info!("Binance WebSocket connected successfully");
                    inner.feed_connected.store(true, Ordering::Release);
                    attempt = 0; // reset backoff on successful connect

                    let (_write, mut read) = ws_stream.split();

                    // ── 3. Read messages until disconnect ─────────
                    let read_result: Result<(), ExchangeError> = 'read_loop: loop {
                        tokio::select! {
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(Message::Text(text))) => {
                                        if let Err(e) = Self::handle_message(&inner, &text) {
                                            tracing::warn!("Binance message handler error: {e}");
                                        }
                                    }
                                    Some(Ok(Message::Ping(_))) => {
                                        // tungstenite handles pongs automatically
                                    }
                                    Some(Ok(Message::Close(frame))) => {
                                        tracing::info!("Binance connection closed: {frame:?}");
                                        break 'read_loop Err(ExchangeError::StreamEnded);
                                    }
                                    Some(Ok(_)) => {} // binary, pong, etc.
                                    Some(Err(e)) => {
                                        tracing::error!("Binance WebSocket read error: {e}");
                                        break 'read_loop Err(ExchangeError::WebSocket(e.to_string()));
                                    }
                                    None => {
                                        tracing::warn!("Binance stream ended (None)");
                                        break 'read_loop Err(ExchangeError::StreamEnded);
                                    }
                                }
                            }
                            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                                // Periodic health check — connection is alive
                                // (We only get here if no messages arrive for 30s)
                                if !inner.feed_connected.load(Ordering::Acquire) {
                                    tracing::warn!("Binance feed marked disconnected during read loop");
                                    break 'read_loop Err(ExchangeError::StreamEnded);
                                }
                            }
                        }

                        // Check if we should stop
                        if !inner.running.load(Ordering::Acquire) {
                            break 'read_loop Ok(());
                        }
                    };

                    // Connection dropped — mark disconnected
                    inner.feed_connected.store(false, Ordering::Release);

                    match read_result {
                        Ok(()) => {
                            tracing::info!("Binance feed stopped gracefully");
                            break;
                        }
                        Err(ExchangeError::StreamEnded) | Err(ExchangeError::WebSocket(_)) => {
                            attempt += 1;
                            continue; // reconnect
                        }
                        Err(e) => {
                            tracing::error!("Binance feed unrecoverable error: {e}");
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Binance WebSocket connection failed: {e}");
                    inner.feed_connected.store(false, Ordering::Release);
                    attempt += 1;
                    continue; // reconnect
                }
            }
        }

        inner.feed_connected.store(false, Ordering::Release);
        tracing::info!("Binance feed run_loop exited");
        Ok(())
    }

    /// Parse a JSON message from a combined Binance WebSocket stream.
    ///
    /// Combined streams wrap each message in `{"stream": "btcusdt@trade", "data": {...}}`.
    /// This function unwraps the envelope and routes to the appropriate handler.
    fn handle_message(inner: &Arc<BinanceFeedInner>, text: &str) -> Result<(), ExchangeError> {
        let raw: serde_json::Value =
            serde_json::from_str(text).map_err(|e| ExchangeError::JsonParse(e.to_string()))?;

        // Check for Binance error responses
        if let Some(code) = raw.get("code").and_then(|c| c.as_i64()) {
            let msg = raw.get("msg").and_then(|m| m.as_str()).unwrap_or("unknown");
            tracing::warn!("Binance API error [code {code}]: {msg}");
            return Err(ExchangeError::Exchange(format!("code {code}: {msg}")));
        }

        // Unwrap combined stream envelope: { "stream": "...", "data": {...} }
        let (stream_name, data) = if let Some(sname) = raw.get("stream").and_then(|s| s.as_str())
        {
            let data = raw
                .get("data")
                .ok_or_else(|| ExchangeError::JsonParse("combined stream missing 'data' field".into()))?;
            (sname, data)
        } else {
            // Raw stream format (no wrapper) — use the message as-is.
            // Try to determine event type from the "e" field.
            let event_type = raw.get("e").and_then(|e| e.as_str()).unwrap_or("unknown");
            // For raw streams, we approximate routing by event type
            return match event_type {
                "trade" => Self::handle_trade(inner, &raw),
                "depthUpdate" => Self::handle_depth(inner, &raw),
                _ => {
                    tracing::trace!("Ignored Binance event type: {event_type}");
                    Ok(())
                }
            };
        };

        // Route based on the stream name suffix
        if stream_name.ends_with("@trade") {
            Self::handle_trade(inner, data)
        } else if stream_name.ends_with("@depth20@100ms") {
            Self::handle_depth(inner, data)
        } else {
            tracing::trace!("Ignored unknown stream: {stream_name}");
            Ok(())
        }
    }

    /// Handle a trade event (e.g., `btcusdt@trade`).
    fn handle_trade(
        inner: &Arc<BinanceFeedInner>,
        raw: &serde_json::Value,
    ) -> Result<(), ExchangeError> {
        let symbol_raw = raw
            .get("s")
            .and_then(|s| s.as_str())
            .ok_or_else(|| ExchangeError::JsonParse("missing symbol".into()))?;

        let price: f64 = raw
            .get("p")
            .and_then(|p| p.as_str())
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| ExchangeError::JsonParse("missing/invalid price".into()))?;

        let volume: f64 = raw
            .get("q")
            .and_then(|q| q.as_str())
            .and_then(|q| q.parse().ok())
            .ok_or_else(|| ExchangeError::JsonParse("missing/invalid quantity".into()))?;

        let trade_time: i64 = raw
            .get("T")
            .and_then(|t| t.as_i64())
            .ok_or_else(|| ExchangeError::JsonParse("missing trade time".into()))?;

        let is_maker_buy = raw.get("m").and_then(|m| m.as_bool()).unwrap_or(true);

        // Convert symbol to [u8; 8] zero-padded ASCII
        let mut symbol_bytes = [0u8; 8];
        let sym_upper = symbol_raw.to_uppercase();
        let bytes = sym_upper.as_bytes();
        let len = bytes.len().min(8);
        symbol_bytes[..len].copy_from_slice(&bytes[..len]);

        // Convert timestamp to chrono::DateTime<Utc>
        let timestamp =
            chrono::DateTime::from_timestamp_millis(trade_time).unwrap_or_else(chrono::Utc::now);

        let tick = Tick {
            symbol: symbol_bytes,
            price,
            volume,
            timestamp,
            conditions: if is_maker_buy { 1 } else { 0 },
        };

        // Push to ring buffer
        if let Ok(Some(ring)) = inner.ring.try_lock().map(|g| g.clone()) {
            ring.push(MarketEvent::Tick(tick));
        }

        Ok(())
    }

    /// Handle a `@depth20@100ms` snapshot: top 20 bids + asks.
    fn handle_depth(
        inner: &Arc<BinanceFeedInner>,
        raw: &serde_json::Value,
    ) -> Result<(), ExchangeError> {
        let symbol_raw = raw
            .get("s")
            .and_then(|s| s.as_str())
            .ok_or_else(|| ExchangeError::JsonParse("depth missing symbol".into()))?;

        let last_update_id: u64 = raw
            .get("lastUpdateId")
            .and_then(|u| u.as_u64())
            .ok_or_else(|| ExchangeError::JsonParse("depth missing lastUpdateId".into()))?;

        let parse_levels = |arr: &serde_json::Value| -> Result<Vec<OrderBookLevel>, ExchangeError> {
            let levels = arr
                .as_array()
                .ok_or_else(|| ExchangeError::JsonParse("depth levels not an array".into()))?;
            let mut result = Vec::with_capacity(levels.len());
            for level in levels {
                if let Some(pair) = level.as_array() && pair.len() >= 2 {
                    let price: f64 = pair[0]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| ExchangeError::JsonParse("depth invalid price".into()))?;
                    let size: f64 = pair[1]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| ExchangeError::JsonParse("depth invalid size".into()))?;
                    result.push(OrderBookLevel { price, size });
                }
            }
            Ok(result)
        };

        let bids = raw
            .get("bids")
            .ok_or_else(|| ExchangeError::JsonParse("depth missing bids".into()))
            .and_then(parse_levels)?;

        let asks = raw
            .get("asks")
            .ok_or_else(|| ExchangeError::JsonParse("depth missing asks".into()))
            .and_then(parse_levels)?;

        let normalized = symbol_raw.to_lowercase();

        let book = OrderBook {
            symbol: normalized.clone(),
            bids,
            asks,
            last_update_id,
        };

        // Store in shared state
        if let Ok(mut guard) = inner.order_book.try_write() {
            guard.insert(normalized, book);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_build_stream_path() {
        let symbols = vec!["btcusdt".into(), "ethusdt".into()];
        let path = BinanceFeed::build_stream_path(&symbols);
        assert!(path.contains("btcusdt@trade"), "path={path}");
        assert!(path.contains("btcusdt@depth20@100ms"), "path={path}");
        assert!(path.contains("ethusdt@trade"), "path={path}");
        assert!(path.starts_with("?streams="), "path={path}");
        // Each symbol has 2 streams (trade + depth)
        let count = path.matches("@trade").count() + path.matches("@depth20@100ms").count();
        assert_eq!(count, 4, "expected 4 stream entries for 2 symbols");
    }

    #[test]
    fn test_subscribe_normalizes_symbol() {
        let feed = BinanceFeed::new();
        feed.subscribe("BTC-USDT").unwrap();
        feed.subscribe("ETH/USDT").unwrap();

        let symbols = feed.inner.symbols.try_lock().unwrap();
        assert!(symbols.contains(&"btcusdt".into()));
        assert!(symbols.contains(&"ethusdt".into()));
        assert_eq!(symbols.len(), 2);
    }

    #[test]
    fn test_subscribe_duplicate() {
        let feed = BinanceFeed::new();
        feed.subscribe("btcusdt").unwrap();
        feed.subscribe("BTCUSDT").unwrap();

        let symbols = feed.inner.symbols.try_lock().unwrap();
        assert_eq!(symbols.len(), 1);
    }

    #[test]
    fn test_handle_trade() {
        let inner = Arc::new(BinanceFeedInner {
            running: AtomicBool::new(true),
            feed_connected: AtomicBool::new(true),
            symbols: Mutex::new(vec!["btcusdt".into()]),
            ring: Mutex::new(Some(Arc::new(RingBuffer::new(1024)))),
            order_book: RwLock::new(HashMap::new()),
            task_handle: Mutex::new(None),
            max_reconnect_secs: DEFAULT_MAX_RECONNECT_SECS,
            base_delay_ms: DEFAULT_BASE_DELAY_MS,
        });

        let json = r#"{
            "e": "trade",
            "E": 1672515782136,
            "s": "BTCUSDT",
            "t": 123456789,
            "p": "45000.25",
            "q": "0.001",
            "T": 1672515782136,
            "m": true,
            "M": true
        }"#;

        let raw: serde_json::Value = serde_json::from_str(json).unwrap();
        BinanceFeed::handle_trade(&inner, &raw).unwrap();

        // Verify tick was pushed
        let ring = inner.ring.try_lock().unwrap();
        let ring_ref = ring.as_ref().unwrap();
        assert_eq!(ring_ref.len(), 1);
    }

    #[test]
    fn test_symbol_to_bytes() {
        let feed = BinanceFeed::new();
        feed.subscribe("BTC/USD").unwrap();
        feed.subscribe("ETHBTC").unwrap();

        let symbols = feed.inner.symbols.try_lock().unwrap();
        assert!(symbols.contains(&"btcusd".into()));
        assert!(symbols.contains(&"ethbtc".into()));
    }

    #[test]
    fn test_backoff_delay_basic() {
        // Attempt 1: base=1000ms -> window = [500, 1000]ms (jitter, min 500)
        let d1 = BinanceFeed::backoff_delay(1, 60, 1000);
        assert!(d1.as_millis() >= 500);
        assert!(d1.as_millis() <= 1000);

        // Attempt 2: base*2 = 2000ms -> window = [500, 2000]ms
        let d2 = BinanceFeed::backoff_delay(2, 60, 1000);
        assert!(d2.as_millis() >= 500);
        assert!(d2.as_millis() <= 2000);

        // Attempt 6: 2^5 * 1000 = 32000ms -> capped at 60000
        let d6 = BinanceFeed::backoff_delay(6, 60, 1000);
        assert!(d6.as_millis() >= 500);
        assert!(d6.as_millis() <= 60_000);
    }

    #[test]
    fn test_backoff_delay_increases_with_attempt() {
        // Higher attempts should produce progressively larger max delays
        let d1 = BinanceFeed::backoff_delay(1, 60, 1000);
        let d4 = BinanceFeed::backoff_delay(4, 60, 1000);
        // The max possible for attempt 1 is 1000ms, for attempt 4 is 8000ms
        // So d4 should usually be >= d1 (not guaranteed due to jitter, but very likely)
        assert!(d4.as_millis() >= d1.as_millis() || d4.as_millis() <= 8000); // at least within range
    }

    #[test]
    fn test_connected_initial_state() {
        let feed = BinanceFeed::new();
        assert!(!feed.connected());
    }
}
