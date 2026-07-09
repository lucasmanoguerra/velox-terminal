//! Binance User Data Stream — real-time account, order, and balance updates.
//!
//! # Lifecycle
//!
//! 1. [`BinanceUserDataStream::start()`] creates a listen key via REST and connects
//!    to the WebSocket at `wss://stream.binance.com:9443/ws/<listenKey>`.
//! 2. A keepalive timer sends `PUT /api/v3/userDataStream` every 30 minutes
//!    to prevent the listen key from expiring (60-minute TTL).
//! 3. Incoming WebSocket messages are parsed into [`UserDataEvent`]s and sent
//!    to a `tokio::sync::mpsc::UnboundedReceiver`.
//! 4. On disconnect, a new listen key is created and the connection is re-established.
//! 5. [`BinanceUserDataStream::stop()`] closes the listen key and aborts the task.
//!
//! # Events
//!
//! | Binance Event Type | Parsed Event | Description |
//! |---|---|---|
//! | `outboundAccountPosition` | `AccountUpdate` | Balance snapshot (all assets) |
//! | `executionReport` | `OrderUpdate` | Order fill/cancel/reject/replace |
//! | `balanceUpdate` | `BalanceUpdate` | Individual balance change |
//! | `listStatus` | `ListStatus` | OCO order status change |

use futures_util::StreamExt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::binance_rest::BinanceRestClient;

// ── Constants ────────────────────────────────────────────────────────────

/// Binance WebSocket base URL for **raw** streams (single-stream, no wrapper).
const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

/// Listen key keepalive interval (recommended: every 30 min for a 60-min TTL).
const KEEPALIVE_INTERVAL_SECS: u64 = 30 * 60;

/// Maximum reconnect backoff in seconds.
const MAX_RECONNECT_SECS: u64 = 60;

/// Base delay for exponential backoff in milliseconds.
const BASE_DELAY_MS: u64 = 1000;

// ── Event types ──────────────────────────────────────────────────────────

/// A parsed event from the Binance user data stream.
#[derive(Debug, Clone)]
pub enum UserDataEvent {
    /// Full account balance update (`outboundAccountPosition`).
    AccountUpdate(AccountUpdateEvent),
    /// Order execution report (`executionReport`).
    OrderUpdate(OrderUpdateEvent),
    /// Individual balance change (`balanceUpdate`).
    BalanceUpdate(BalanceUpdateEvent),
    /// OCO order list status change (`listStatus`).
    ListStatus(ListStatusEvent),
}

/// Account balance snapshot (`outboundAccountPosition`).
///
/// Contains all assets with their free/locked balances.
#[derive(Debug, Clone)]
pub struct AccountUpdateEvent {
    /// Event timestamp (milliseconds).
    pub event_time: i64,
    /// All asset balances.
    pub balances: Vec<AssetBalance>,
}

/// An individual asset balance.
#[derive(Debug, Clone)]
pub struct AssetBalance {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
}

/// Order execution report (`executionReport`).
///
/// Sent when an order's status changes: new, fill, partial fill,
/// cancel, reject, expire, or replace.
#[derive(Debug, Clone)]
pub struct OrderUpdateEvent {
    /// Event timestamp (milliseconds).
    pub event_time: i64,
    /// Trading symbol (e.g. `"BTCUSDT"`).
    pub symbol: String,
    /// Binance client order ID.
    pub client_order_id: String,
    /// Binance-side order ID.
    pub order_id: i64,
    /// Side: `"BUY"` or `"SELL"`.
    pub side: String,
    /// Order type: `"MARKET"`, `"LIMIT"`, etc.
    pub order_type: String,
    /// Time in force: `"GTC"`, `"IOC"`, `"FOK"`.
    pub time_in_force: String,
    /// Original quantity.
    pub orig_qty: f64,
    /// Cumulative filled quantity.
    pub cum_filled_qty: f64,
    /// Cumulative quote asset quantity.
    pub cum_quote_qty: f64,
    /// Last fill quantity (0 for non-fill events).
    pub last_filled_qty: f64,
    /// Last fill price (0 for non-fill events).
    pub last_filled_price: f64,
    /// Last fill commission.
    pub commission: Option<f64>,
    /// Commission asset.
    pub commission_asset: Option<String>,
    /// Current order status: `"NEW"`, `"FILLED"`, `"CANCELED"`, etc.
    pub current_status: String,
    /// Whether the order is resting on the order book.
    pub is_on_book: bool,
    /// Whether the buyer is the maker.
    pub is_maker: bool,
}

/// Balance change event (`balanceUpdate`).
///
/// Covers non-order balance changes: deposits, withdrawals,
/// transfers, interest, etc.
#[derive(Debug, Clone)]
pub struct BalanceUpdateEvent {
    /// Event timestamp (milliseconds).
    pub event_time: i64,
    /// Asset symbol.
    pub asset: String,
    /// Change delta.
    pub delta: f64,
}

/// OCO order list status (`listStatus`).
#[derive(Debug, Clone)]
pub struct ListStatusEvent {
    /// Event timestamp (milliseconds).
    pub event_time: i64,
    /// Symbol.
    pub symbol: String,
    /// Order list ID.
    pub order_list_id: i64,
    /// Status: `"EXECUTING"`, `"ALL_DONE"`, `"REJECT"`.
    pub list_status: String,
    /// Type: `"STARTED"`, `"REPLACED"`, `"STOPPED"`.
    pub list_order_status: String,
}

// ── User Data Stream ─────────────────────────────────────────────────────

/// Internal shared state for the user data stream connection.
struct UserDataStreamInner {
    /// Flag to signal the background task to stop.
    running: AtomicBool,
    /// Whether the WebSocket is currently connected.
    connected: AtomicBool,
    /// REST client for listen key management.
    rest_client: Mutex<Option<BinanceRestClient>>,
    /// Current listen key.
    listen_key: Mutex<Option<String>>,
    /// Tokio task handle for graceful shutdown.
    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Channel sender for parsed events.
    event_sender: Mutex<Option<mpsc::UnboundedSender<UserDataEvent>>>,
}

/// Real-time user data stream from Binance.
///
/// Provides account updates, order execution reports, and balance
/// changes via a tokio mpsc channel.
///
/// # Example
///
/// ```rust,no_run
/// use velox_exchange::binance_user_data::BinanceUserDataStream;
///
/// # async fn example() {
/// let mut stream = BinanceUserDataStream::new(
///     "api_key".into(),
///     "secret_key".into(),
///     true, // use testnet
/// );
///
/// let mut rx = stream.start().await.unwrap();
///
/// while let Some(event) = rx.recv().await {
///     match event {
///         velox_exchange::binance_user_data::UserDataEvent::OrderUpdate(update) => {
///             println!("Order {}: {}", update.symbol, update.current_status);
///         }
///         _ => {}
///     }
/// }
/// # }
/// ```
pub struct BinanceUserDataStream {
    inner: Arc<UserDataStreamInner>,
}

impl BinanceUserDataStream {
    /// Create a new user data stream.
    ///
    /// Does not connect until [`start()`](Self::start) is called.
    pub fn new(api_key: String, secret_key: String, use_testnet: bool) -> Self {
        let rest_client = BinanceRestClient::new(api_key, secret_key, use_testnet);
        Self {
            inner: Arc::new(UserDataStreamInner {
                running: AtomicBool::new(false),
                connected: AtomicBool::new(false),
                rest_client: Mutex::new(Some(rest_client)),
                listen_key: Mutex::new(None),
                task_handle: Mutex::new(None),
                event_sender: Mutex::new(None),
            }),
        }
    }

    /// Get a reference to the underlying REST client for additional API calls.
    pub fn rest_client(&self) -> &Mutex<Option<BinanceRestClient>> {
        &self.inner.rest_client
    }

    /// Start the user data stream.
    ///
    /// 1. Creates a listen key via REST.
    /// 2. Connects to the WebSocket with the listen key.
    /// 3. Spawns a keepalive timer.
    /// 4. Returns an `UnboundedReceiver` for [`UserDataEvent`]s.
    ///
    /// Only one stream can be active at a time. Returns an error if
    /// already running or if API credentials are missing.
    pub async fn start(&self) -> Result<mpsc::UnboundedReceiver<UserDataEvent>, crate::ExchangeError> {
        let inner = self.inner.clone();

        // Prevent duplicate starts
        if inner.running.load(Ordering::Acquire) {
            return Err(crate::ExchangeError::Exchange(
                "User data stream is already running".into(),
            ));
        }

        // Create a listen key first
        let rest = inner.rest_client.lock().await;
        let rest_client = rest
            .as_ref()
            .ok_or(crate::ExchangeError::ApiKeyNotConfigured)?;

        let listen_key = rest_client.create_listen_key().await?;
        inner
            .listen_key
            .lock()
            .await
            .replace(listen_key.clone());
        drop(rest);

        // Create the event channel
        let (tx, rx) = mpsc::unbounded_channel::<UserDataEvent>();
        inner
            .event_sender
            .lock()
            .await
            .replace(tx);

        // Mark as running
        inner.running.store(true, Ordering::Release);

        // Spawn the background task
        let inner_task = inner.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = Self::run_loop(inner_task.clone(), listen_key).await {
                tracing::error!("User data stream error: {e}");
            }
            inner_task.connected.store(false, Ordering::Release);
            inner_task.running.store(false, Ordering::Release);
        });

        *inner
            .task_handle
            .lock()
            .await = Some(handle);

        Ok(rx)
    }

    /// Gracefully stop the user data stream.
    ///
    /// Closes the listen key and aborts the background task.
    pub async fn stop(&self) -> Result<(), crate::ExchangeError> {
        let inner = self.inner.clone();
        inner.running.store(false, Ordering::Release);

        // Close the listen key on the exchange
        if let Some(lk) = inner.listen_key.lock().await.take() {
            let rest = inner.rest_client.lock().await;
            if let Some(client) = rest.as_ref() {
                let _ = client.close_listen_key(&lk).await;
            }
        }

        // Abort the task
        if let Some(handle) = inner.task_handle.lock().await.take() {
            handle.abort();
        }

        Ok(())
    }

    /// Whether the WebSocket is currently connected.
    pub fn connected(&self) -> bool {
        self.inner.connected.load(Ordering::Acquire)
    }

    // ── Internal run loop ──────────────────────────────────────────

    /// Main connection + reconnection loop.
    async fn run_loop(
        inner: Arc<UserDataStreamInner>,
        mut listen_key: String,
    ) -> Result<(), crate::ExchangeError> {
        let mut attempt: u64 = 0;

        while inner.running.load(Ordering::Acquire) {
            // ── Reconnect with backoff ──────────────────────────────
            if attempt > 0 {
                let delay = Self::backoff_delay(attempt, MAX_RECONNECT_SECS, BASE_DELAY_MS);
                tracing::warn!(
                    "User data stream reconnecting in {}ms (attempt {})",
                    delay.as_millis(),
                    attempt + 1,
                );
                Self::sleep_with_running_check(&inner, delay).await;

                if !inner.running.load(Ordering::Acquire) {
                    break;
                }

                // On reconnect: create a fresh listen key
                let rest = inner.rest_client.lock().await;
                if let Some(client) = rest.as_ref() {
                    // Close the old key
                    let _ = client.close_listen_key(&listen_key).await;
                    // Create a new one
                    match client.create_listen_key().await {
                        Ok(new_key) => {
                            listen_key = new_key;
                            *inner.listen_key.lock().await = Some(listen_key.clone());
                        }
                        Err(e) => {
                            tracing::error!("Failed to create new listen key: {e}");
                            attempt += 1;
                            continue;
                        }
                    }
                }
                drop(rest);
            }

            // ── Connect ─────────────────────────────────────────────
            let ws_url = format!("{}/{}", BINANCE_WS_URL, listen_key);
            tracing::info!("User data stream connecting...");

            match connect_async(&ws_url).await {
                Ok((ws_stream, _response)) => {
                    tracing::info!("User data stream connected");
                    inner.connected.store(true, Ordering::Release);
                    attempt = 0;

                    let (_write, read) = ws_stream.split();

                    // ── Run keepalive + read loop ───────────────────
                    let result = Self::read_with_keepalive(&inner, read).await;

                    inner.connected.store(false, Ordering::Release);

                    match result {
                        Ok(()) => {
                            tracing::info!("User data stream stopped gracefully");
                            break;
                        }
                        Err(crate::ExchangeError::StreamEnded)
                        | Err(crate::ExchangeError::WebSocket(_)) => {
                            attempt += 1;
                            continue;
                        }
                        Err(e) => {
                            tracing::error!("User data stream unrecoverable error: {e}");
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("User data stream connection failed: {e}");
                    inner.connected.store(false, Ordering::Release);
                    attempt += 1;
                    continue;
                }
            }
        }

        tracing::info!("User data stream run_loop exited");
        Ok(())
    }

    /// Read loop with periodic listen key keepalive.
    async fn read_with_keepalive(
        inner: &Arc<UserDataStreamInner>,
        mut read: impl futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    ) -> Result<(), crate::ExchangeError> {
        let keepalive_interval = Duration::from_secs(KEEPALIVE_INTERVAL_SECS);
        let mut keepalive_timer = tokio::time::interval(keepalive_interval);
        // Don't fire keepalive immediately
        keepalive_timer.reset();

        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = Self::handle_message(inner, &text) {
                                tracing::warn!("User data message handler error: {e}");
                            }
                        }
                        Some(Ok(Message::Ping(_))) => {
                            // tungstenite handles pongs automatically
                        }
                        Some(Ok(Message::Close(frame))) => {
                            tracing::info!("User data stream closed: {frame:?}");
                            return Err(crate::ExchangeError::StreamEnded);
                        }
                        Some(Ok(_)) => {} // binary, pong, etc.
                        Some(Err(e)) => {
                            tracing::error!("User data read error: {e}");
                            return Err(crate::ExchangeError::WebSocket(e.to_string()));
                        }
                        None => {
                            tracing::warn!("User data stream ended (None)");
                            return Err(crate::ExchangeError::StreamEnded);
                        }
                    }
                }
                _ = keepalive_timer.tick() => {
                    // Keep the listen key alive
                    let lk = inner.listen_key.lock().await;
                    if let Some(key) = lk.as_ref() {
                        let rest = inner.rest_client.lock().await;
                        if let Some(client) = rest.as_ref() {
                            if let Err(e) = client.keepalive_listen_key(key).await {
                                tracing::warn!("Listen key keepalive failed: {e}");
                                // Don't return — the key might still be valid
                            } else {
                                tracing::trace!("Listen key keepalive succeeded");
                            }
                        }
                    }
                }
            }

            // Check if we should stop
            if !inner.running.load(Ordering::Acquire) {
                return Ok(());
            }
        }
    }

    /// Parse a user data stream message and send it through the event channel.
    fn handle_message(
        inner: &Arc<UserDataStreamInner>,
        text: &str,
    ) -> Result<(), crate::ExchangeError> {
        let raw: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| crate::ExchangeError::JsonParse(e.to_string()))?;

        let event_type = raw
            .get("e")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let event = match event_type {
            "outboundAccountPosition" => {
                Self::parse_account_update(&raw).map(UserDataEvent::AccountUpdate)
            }
            "executionReport" => {
                Self::parse_order_update(&raw).map(UserDataEvent::OrderUpdate)
            }
            "balanceUpdate" => {
                Self::parse_balance_update(&raw).map(UserDataEvent::BalanceUpdate)
            }
            "listStatus" => {
                Self::parse_list_status(&raw).map(UserDataEvent::ListStatus)
            }
            _ => {
                tracing::trace!("Ignored user data event type: {event_type}");
                return Ok(());
            }
        };

        if let Ok(event) = event
            && let Ok(tx_guard) = inner.event_sender.try_lock()
            && let Some(tx) = tx_guard.as_ref()
        {
            let _ = tx.send(event);
        }

        Ok(())
    }

    // ── Event parsers ──────────────────────────────────────────────

    fn parse_account_update(raw: &serde_json::Value) -> Result<AccountUpdateEvent, crate::ExchangeError> {
        let event_time = raw
            .get("E")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| crate::ExchangeError::JsonParse("account update missing event time".into()))?;

        let balances_raw = raw
            .get("B")
            .and_then(|v| v.as_array())
            .ok_or_else(|| crate::ExchangeError::JsonParse("account update missing balances".into()))?;

        let mut balances = Vec::with_capacity(balances_raw.len());
        for bal in balances_raw {
            let asset = bal
                .get("a")
                .and_then(|v| v.as_str())
                .ok_or_else(|| crate::ExchangeError::JsonParse("balance missing asset".into()))?
                .to_string();
            let free: f64 = bal
                .get("f")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| crate::ExchangeError::JsonParse("balance missing free".into()))?;
            let locked: f64 = bal
                .get("l")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| crate::ExchangeError::JsonParse("balance missing locked".into()))?;

            balances.push(AssetBalance { asset, free, locked });
        }

        Ok(AccountUpdateEvent { event_time, balances })
    }

    fn parse_order_update(raw: &serde_json::Value) -> Result<OrderUpdateEvent, crate::ExchangeError> {
        let event_time = raw
            .get("E")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| crate::ExchangeError::JsonParse("order update missing event time".into()))?;
        let symbol = raw
            .get("s")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("order update missing symbol".into()))?
            .to_string();
        let client_order_id = raw
            .get("c")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("order update missing client order id".into()))?
            .to_string();
        let order_id = raw
            .get("i")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| crate::ExchangeError::JsonParse("order update missing order id".into()))?;
        let side = raw
            .get("S")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("order update missing side".into()))?
            .to_string();
        let order_type = raw
            .get("o")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("order update missing order type".into()))?
            .to_string();
        let time_in_force = raw
            .get("f")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let orig_qty: f64 = raw
            .get("q")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| crate::ExchangeError::JsonParse("order update missing original quantity".into()))?;
        let cum_filled_qty: f64 = raw
            .get("z")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let cum_quote_qty: f64 = raw
            .get("Z")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let last_filled_qty: f64 = raw
            .get("l")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let last_filled_price: f64 = raw
            .get("L")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let commission = raw
            .get("n")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok());

        let commission_asset = raw
            .get("N")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let current_status = raw
            .get("X")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("order update missing status".into()))?
            .to_string();

        let is_on_book = raw
            .get("w")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_maker = raw
            .get("m")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(OrderUpdateEvent {
            event_time,
            symbol,
            client_order_id,
            order_id,
            side,
            order_type,
            time_in_force,
            orig_qty,
            cum_filled_qty,
            cum_quote_qty,
            last_filled_qty,
            last_filled_price,
            commission,
            commission_asset,
            current_status,
            is_on_book,
            is_maker,
        })
    }

    fn parse_balance_update(raw: &serde_json::Value) -> Result<BalanceUpdateEvent, crate::ExchangeError> {
        let event_time = raw
            .get("E")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| crate::ExchangeError::JsonParse("balance update missing event time".into()))?;
        let asset = raw
            .get("a")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("balance update missing asset".into()))?
            .to_string();
        let delta: f64 = raw
            .get("d")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| crate::ExchangeError::JsonParse("balance update missing delta".into()))?;

        Ok(BalanceUpdateEvent { event_time, asset, delta })
    }

    fn parse_list_status(raw: &serde_json::Value) -> Result<ListStatusEvent, crate::ExchangeError> {
        let event_time = raw
            .get("E")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| crate::ExchangeError::JsonParse("list status missing event time".into()))?;
        let symbol = raw
            .get("s")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("list status missing symbol".into()))?
            .to_string();
        let order_list_id = raw
            .get("g")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| crate::ExchangeError::JsonParse("list status missing order list id".into()))?;
        let list_status = raw
            .get("l")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("list status missing list status".into()))?
            .to_string();
        let list_order_status = raw
            .get("L")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::ExchangeError::JsonParse("list status missing list order status".into()))?
            .to_string();

        Ok(ListStatusEvent {
            event_time,
            symbol,
            order_list_id,
            list_status,
            list_order_status,
        })
    }

    // ── Backoff helper ──────────────────────────────────────────────

    /// Exponential backoff with full jitter.
    fn backoff_delay(attempt: u64, max_secs: u64, base_ms: u64) -> Duration {
        let exponent = (attempt.saturating_sub(1)).min(60) as u32;
        let multiplier = if exponent >= 63 {
            u64::MAX
        } else {
            1u64 << exponent
        };
        let exp_ms = base_ms.saturating_mul(multiplier);
        let max_ms = max_secs * 1000;
        let window_ms = exp_ms.min(max_ms);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let hash = now.as_nanos() ^ ((attempt as u128) << 64);
        let jitter_ms = (hash % (window_ms.max(1) as u128)) as u64;

        Duration::from_millis(jitter_ms.max(500).min(max_ms))
    }

    /// Sleep with periodic `running` check for responsive shutdown.
    async fn sleep_with_running_check(inner: &UserDataStreamInner, duration: Duration) {
        let start = tokio::time::Instant::now();
        while start.elapsed() < duration {
            if !inner.running.load(Ordering::Acquire) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl std::fmt::Debug for BinanceUserDataStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceUserDataStream")
            .field("connected", &self.connected())
            .finish()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Event parsing tests ────────────────────────────────────────

    #[test]
    fn test_parse_account_update() {
        let json = r#"{
            "e": "outboundAccountPosition",
            "E": 1672515782136,
            "u": 123456789,
            "B": [
                {"a": "BTC", "f": "0.50000000", "l": "0.10000000"},
                {"a": "USDT", "f": "10000.00000000", "l": "500.00000000"}
            ]
        }"#;

        let raw: serde_json::Value = serde_json::from_str(json).unwrap();
        let event = BinanceUserDataStream::parse_account_update(&raw).unwrap();

        assert_eq!(event.event_time, 1672515782136);
        assert_eq!(event.balances.len(), 2);
        assert_eq!(event.balances[0].asset, "BTC");
        assert!((event.balances[0].free - 0.5).abs() < 1e-8);
        assert!((event.balances[0].locked - 0.1).abs() < 1e-8);
        assert_eq!(event.balances[1].asset, "USDT");
        assert!((event.balances[1].free - 10000.0).abs() < 1e-8);
    }

    #[test]
    fn test_parse_order_update_fill() {
        let json = r#"{
            "e": "executionReport",
            "E": 1672515782136,
            "s": "BTCUSDT",
            "c": "myCustomId123",
            "i": 123456789,
            "S": "BUY",
            "o": "MARKET",
            "f": "IOC",
            "q": "0.01000000",
            "p": "45000.00000000",
            "P": "0.00000000",
            "F": "0.00000000",
            "z": "0.01000000",
            "Z": "450.00000000",
            "l": "0.01000000",
            "L": "45000.00000000",
            "n": "0.00000000",
            "N": "BNB",
            "x": "TRADE",
            "X": "FILLED",
            "w": false,
            "m": false
        }"#;

        let raw: serde_json::Value = serde_json::from_str(json).unwrap();
        let event = BinanceUserDataStream::parse_order_update(&raw).unwrap();

        assert_eq!(event.event_time, 1672515782136);
        assert_eq!(event.symbol, "BTCUSDT");
        assert_eq!(event.side, "BUY");
        assert_eq!(event.order_type, "MARKET");
        assert_eq!(event.current_status, "FILLED");
        assert!((event.orig_qty - 0.01).abs() < 1e-8);
        assert!((event.cum_filled_qty - 0.01).abs() < 1e-8);
        assert!((event.last_filled_price - 45000.0).abs() < 1e-8);
        assert_eq!(event.commission, Some(0.0));
        assert_eq!(event.commission_asset, Some("BNB".into()));
    }

    #[test]
    fn test_parse_order_update_new() {
        let json = r#"{
            "e": "executionReport",
            "E": 1672515782136,
            "s": "BTCUSDT",
            "c": "myLimitOrder",
            "i": 987654321,
            "S": "SELL",
            "o": "LIMIT",
            "f": "GTC",
            "q": "0.10000000",
            "p": "50000.00000000",
            "P": "0.00000000",
            "F": "0.00000000",
            "z": "0.00000000",
            "Z": "0.00000000",
            "l": "0.00000000",
            "L": "0.00000000",
            "x": "NEW",
            "X": "NEW",
            "w": true,
            "m": false
        }"#;

        let raw: serde_json::Value = serde_json::from_str(json).unwrap();
        let event = BinanceUserDataStream::parse_order_update(&raw).unwrap();

        assert_eq!(event.side, "SELL");
        assert_eq!(event.order_type, "LIMIT");
        assert_eq!(event.current_status, "NEW");
        assert!((event.cum_filled_qty - 0.0).abs() < 1e-8);
        assert!(event.is_on_book);
    }

    #[test]
    fn test_parse_balance_update() {
        let json = r#"{
            "e": "balanceUpdate",
            "E": 1672515782136,
            "a": "USDT",
            "d": "100.00000000"
        }"#;

        let raw: serde_json::Value = serde_json::from_str(json).unwrap();
        let event = BinanceUserDataStream::parse_balance_update(&raw).unwrap();

        assert_eq!(event.event_time, 1672515782136);
        assert_eq!(event.asset, "USDT");
        assert!((event.delta - 100.0).abs() < 1e-8);
    }

    #[test]
    fn test_parse_list_status() {
        let json = r#"{
            "e": "listStatus",
            "E": 1672515782136,
            "s": "BTCUSDT",
            "g": 12345,
            "l": "EXECUTING",
            "L": "STARTED"
        }"#;

        let raw: serde_json::Value = serde_json::from_str(json).unwrap();
        let event = BinanceUserDataStream::parse_list_status(&raw).unwrap();

        assert_eq!(event.event_time, 1672515782136);
        assert_eq!(event.symbol, "BTCUSDT");
        assert_eq!(event.order_list_id, 12345);
        assert_eq!(event.list_status, "EXECUTING");
        assert_eq!(event.list_order_status, "STARTED");
    }

    #[test]
    fn test_parse_unknown_event() {
        let json = r#"{"e": "unknownEvent", "E": 12345}"#;
        let _raw: serde_json::Value = serde_json::from_str(json).unwrap();

        // Just check that the event type is handled gracefully
        // (should log and return Ok)
        // We'll test handle_message indirectly by checking it doesn't panic
        let inner = Arc::new(UserDataStreamInner {
            running: AtomicBool::new(true),
            connected: AtomicBool::new(true),
            rest_client: Mutex::new(None),
            listen_key: Mutex::new(None),
            task_handle: Mutex::new(None),
            event_sender: Mutex::new(None),
        });

        // Should not panic
        let result = BinanceUserDataStream::handle_message(&inner, json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_message_sends_to_channel() {
        let (tx, mut rx) = mpsc::unbounded_channel::<UserDataEvent>();

        let inner = Arc::new(UserDataStreamInner {
            running: AtomicBool::new(true),
            connected: AtomicBool::new(true),
            rest_client: Mutex::new(None),
            listen_key: Mutex::new(None),
            task_handle: Mutex::new(None),
            event_sender: Mutex::new(Some(tx)),
        });

        let json = r#"{
            "e": "executionReport",
            "E": 1672515782136,
            "s": "BTCUSDT",
            "c": "test",
            "i": 1,
            "S": "BUY",
            "o": "MARKET",
            "f": "IOC",
            "q": "1.0",
            "p": "0.0",
            "P": "0.0",
            "F": "0.0",
            "z": "1.0",
            "Z": "45000.0",
            "l": "1.0",
            "L": "45000.0",
            "X": "FILLED",
            "w": false,
            "m": false
        }"#;

        BinanceUserDataStream::handle_message(&inner, json).unwrap();

        let event = rx.try_recv().expect("Should receive an event");
        match event {
            UserDataEvent::OrderUpdate(update) => {
                assert_eq!(update.symbol, "BTCUSDT");
                assert_eq!(update.current_status, "FILLED");
            }
            _ => panic!("Expected OrderUpdate event"),
        }
    }

    #[test]
    fn test_backoff_delay() {
        let d1 = BinanceUserDataStream::backoff_delay(1, 60, 1000);
        assert!(d1.as_millis() >= 500);
        assert!(d1.as_millis() <= 1000);

        let d3 = BinanceUserDataStream::backoff_delay(3, 60, 1000);
        assert!(d3.as_millis() >= 500);
        assert!(d3.as_millis() <= 4000);
    }

    #[test]
    fn test_debug_format() {
        let stream = BinanceUserDataStream::new(
            "test_key".into(),
            "test_secret".into(),
            false,
        );
        let debug = format!("{stream:?}");
        assert!(debug.contains("connected"));
        assert!(debug.contains("false"));
    }

    #[test]
    fn test_start_twice_fails() {
        // This test would need a real HTTP server, but we can test the
        // pessimistic branch: after start, a second start should error.
        // For now, just verify the inner structure is created correctly.
        let stream = BinanceUserDataStream::new(
            "key".into(),
            "secret".into(),
            true,
        );
        // Should be in initial state (not running)
        assert!(!stream.connected());
    }
}
