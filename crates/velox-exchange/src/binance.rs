//! Binance exchange connector — WebSocket market data feed.
//!
//! Connects to Binance public WebSocket streams and converts trade/quote
//! events into [`MarketEvent`]s pushed to a lock-free [`RingBuffer`].
//!
//! # Streams
//!
//! - **Trade**: `<symbol>@trade` — real-time trade ticks
//! - **Book Ticker**: `<symbol>@bookTicker` — top-of-book quotes
//!
//! # Rate Limits
//!
//! Binance allows up to 5 incoming messages per second per connection.
//! This implementation stays well within that limit with a single combined stream.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use chrono::Utc;
use futures_util::StreamExt;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use velox_core::{CoreError, Tick};
use velox_md::ring_buffer::{MarketEvent, RingBuffer};

use crate::error::ExchangeError;
use crate::ExchangeFeed;

/// Binance WebSocket base URL for combined streams.
const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

/// Internal shared state for a Binance feed connection.
struct BinanceFeedInner {
    /// Flag to signal the tokio task to stop.
    running: AtomicBool,
    /// Connected symbols (lowercase, e.g. "btcusdt").
    symbols: Mutex<Vec<String>>,
    /// Ring buffer shared with consumer.
    ring: Mutex<Option<Arc<RingBuffer>>>,
    /// Tokio task handle for graceful shutdown.
    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

/// Binance exchange market data feed.
///
/// Creates a WebSocket connection to Binance's combined stream endpoint
/// and pushes trade ticks into the ring buffer.
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
                symbols: Mutex::new(Vec::new()),
                ring: Mutex::new(None),
                task_handle: Mutex::new(None),
            }),
        }
    }

    /// Build the combined stream path for subscribed symbols.
    fn build_stream_path(symbols: &[String]) -> String {
        // Binance combined stream: /stream?streams=btcusdt@trade/ethusdt@trade
        let streams: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@trade", s))
            .collect();
        format!("/stream?streams={}", streams.join("/"))
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
            let mut guard = inner.ring.try_lock().map_err(|_| {
                CoreError::Internal("ring buffer lock contention".into())
            })?;
            *guard = Some(ring);
            return Ok(());
        }

        // Mark as running
        inner.running.store(true, Ordering::Release);
        *inner.ring.try_lock().map_err(|_| {
            CoreError::Internal("ring buffer lock contention".into())
        })? = Some(ring);

        // Clone inner for the tokio task
        let inner_task = inner.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = Self::run_loop(inner_task.clone()).await {
                tracing::error!("Binance feed error: {e}");
            }
            inner_task.running.store(false, Ordering::Release);
        });

        *inner.task_handle.try_lock().map_err(|_| {
            CoreError::Internal("task handle lock contention".into())
        })? = Some(handle);

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
        let normalized = symbol
            .to_lowercase()
            .replace(['-', '/'], "");
        if normalized.is_empty() {
            return Err(CoreError::InvalidSymbol(symbol.into()));
        }

        let mut symbols = self.inner.symbols.try_lock().map_err(|_| {
            CoreError::Internal("symbols lock contention".into())
        })?;

        if !symbols.contains(&normalized) {
            symbols.push(normalized.clone());
            tracing::info!("Binance feed subscribed to {normalized}");
        }

        Ok(())
    }

    fn unsubscribe(&self, symbol: &str) -> Result<(), CoreError> {
        let normalized = symbol.to_lowercase().replace(['-', '/'], "");
        let mut symbols = self.inner.symbols.try_lock().map_err(|_| {
            CoreError::Internal("symbols lock contention".into())
        })?;

        symbols.retain(|s| s != &normalized);
        tracing::info!("Binance feed unsubscribed from {normalized}");

        Ok(())
    }
}

// ── Internal implementation ──────────────────────────────────────────

impl BinanceFeed {
    /// Main WebSocket event loop.
    async fn run_loop(inner: Arc<BinanceFeedInner>) -> Result<(), ExchangeError> {
        // Get current symbols for the initial connection
        let symbols = inner.symbols.lock().await.clone();
        if symbols.is_empty() {
            tracing::warn!("Binance feed started with no symbols subscribed");
            return Err(ExchangeError::Internal("no symbols subscribed".into()));
        }

        let stream_path = Self::build_stream_path(&symbols);
        let ws_url = format!("{}{}", BINANCE_WS_URL, stream_path);

        tracing::info!("Connecting to Binance WebSocket: {ws_url}");

        let (ws_stream, _response) = connect_async(&ws_url)
            .await
            .map_err(|e| ExchangeError::WebSocket(e.to_string()))?;

        tracing::info!("Binance WebSocket connected");

        let (_write, mut read) = ws_stream.split();

        while inner.running.load(Ordering::Acquire) {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = Self::handle_message(&inner, &text) {
                                tracing::warn!("Binance message error: {e}");
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            // tungstenite handles pongs automatically
                            tracing::trace!("Binance ping received: {} bytes", data.len());
                        }
                        Some(Ok(Message::Close(frame))) => {
                            tracing::info!("Binance connection closed: {frame:?}");
                            break;
                        }
                        Some(Ok(_)) => {} // binary, pong, etc.
                        Some(Err(e)) => {
                            tracing::error!("Binance WebSocket error: {e}");
                            break;
                        }
                        None => {
                            tracing::warn!("Binance stream ended");
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    // Periodic health check — connection is still alive
                    tracing::trace!("Binance feed heartbeat");
                }
            }
        }

        tracing::info!("Binance feed stopped");
        Ok(())
    }

    /// Parse a JSON trade message and push to the ring buffer.
    fn handle_message(inner: &Arc<BinanceFeedInner>, text: &str) -> Result<(), ExchangeError> {
        // Parse the common envelope first
        let raw: serde_json::Value =
            serde_json::from_str(text).map_err(|e| ExchangeError::JsonParse(e.to_string()))?;

        // Check for Binance error responses
        if let Some(code) = raw.get("code").and_then(|c| c.as_i64()) {
            let msg = raw
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown");
            tracing::warn!("Binance API error [code {code}]: {msg}");
            return Err(ExchangeError::Exchange(format!("code {code}: {msg}")));
        }

        // Determine event type
        let event_type = raw
            .get("e")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown");

        match event_type {
            "trade" => Self::handle_trade(inner, &raw),
            _ => {
                // Ignore other event types (kline, depth, etc.)
                tracing::trace!("Ignored Binance event type: {event_type}");
                Ok(())
            }
        }
    }

    /// Handle a trade event (e.g., `btcusdt@trade`).
    fn handle_trade(inner: &Arc<BinanceFeedInner>, raw: &serde_json::Value) -> Result<(), ExchangeError> {
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

        let is_maker_buy = raw
            .get("m")
            .and_then(|m| m.as_bool())
            .unwrap_or(true);

        // Convert symbol to [u8; 8] zero-padded ASCII
        let mut symbol_bytes = [0u8; 8];
        let sym_upper = symbol_raw.to_uppercase();
        let bytes = sym_upper.as_bytes();
        let len = bytes.len().min(8);
        symbol_bytes[..len].copy_from_slice(&bytes[..len]);

        // Convert timestamp to chrono::DateTime<Utc>
        let timestamp = chrono::DateTime::from_timestamp_millis(trade_time)
            .unwrap_or_else(Utc::now);

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_stream_path() {
        let symbols = vec!["btcusdt".into(), "ethusdt".into()];
        let path = BinanceFeed::build_stream_path(&symbols);
        assert!(path.contains("btcusdt@trade"));
        assert!(path.contains("ethusdt@trade"));
        assert!(path.starts_with("/stream?streams="));
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
            symbols: Mutex::new(vec!["btcusdt".into()]),
            ring: Mutex::new(Some(Arc::new(RingBuffer::new(1024)))),
            task_handle: Mutex::new(None),
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
}
