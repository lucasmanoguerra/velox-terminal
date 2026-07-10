//! Binance User Data Stream — WebSocket connector for real-time account/order/balance events.
//!
//! Connects to Binance WebSocket user data stream via listen key (created via REST API),
//! parses `outboundAccountPosition`, `executionReport`, `balanceUpdate`, and `listStatus`
//! events, and delivers them through an `mpsc::UnboundedReceiver` channel.
//!
//! Includes automatic reconnection with exponential backoff + full jitter,
//! listen key keepalive every 30 minutes, and responsive shutdown (<100ms).

pub mod types;
pub use types::*;

#[cfg(test)]
mod tests;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::binance_rest::BinanceRestClient;

/// Binance WebSocket base URL (production).
const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/ws";

/// Binance testnet WebSocket base URL.
const BINANCE_TESTNET_WS_URL: &str = "wss://testnet.binance.vision:9443/ws";

/// Listen key keepalive interval. Binance TTL is 60 min; refresh every 30.
const KEEPALIVE_INTERVAL_SECS: u64 = 30 * 60;

/// Maximum reconnect backoff in seconds.
const MAX_RECONNECT_SECS: u64 = 60;

/// Base delay for exponential backoff in milliseconds.
const BASE_DELAY_MS: u64 = 1000;

// ── Internal state ──────────────────────────────────────────────────────

/// Internal state shared between the handle and its async task.
pub(crate) struct UserDataStreamInner {
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
    /// Whether to use the Binance testnet WebSocket endpoint.
    use_testnet: bool,
}

// ── Public API ──────────────────────────────────────────────────────────

/// WebSocket connector for Binance User Data Stream.
///
/// Provides real-time account, order, and balance updates via a channel.
/// Automatically reconnects with exponential backoff on connection loss.
pub struct BinanceUserDataStream {
    inner: Arc<UserDataStreamInner>,
}

impl BinanceUserDataStream {
    /// Create a new user data stream connector.
    ///
    /// Does not connect until [`start`](Self::start) is called.
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
                use_testnet,
            }),
        }
    }

    /// Start the user data stream.
    ///
    /// 1. Creates a listen key via REST.
    /// 2. Connects to the WebSocket with the listen key.
    /// 3. Spawns a keepalive + read loop.
    /// 4. Returns an `UnboundedReceiver` for [`UserDataEvent`]s.
    ///
    /// Only one stream can be active at a time.
    pub async fn start(&self) -> Result<mpsc::UnboundedReceiver<UserDataEvent>, crate::ExchangeError> {
        let inner = self.inner.clone();

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
        inner.listen_key.lock().await.replace(listen_key.clone());
        drop(rest);

        // Create the event channel
        let (tx, rx) = mpsc::unbounded_channel();
        inner.event_sender.lock().await.replace(tx);

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

        inner.task_handle.lock().await.replace(handle);

        Ok(rx)
    }

    /// Gracefully stop the user data stream.
    pub async fn stop(&self) {
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
                    let _ = client.close_listen_key(&listen_key).await;
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
            let base_url = if inner.use_testnet {
                BINANCE_TESTNET_WS_URL
            } else {
                BINANCE_WS_URL
            };
            let ws_url = format!("{}/{}", base_url, listen_key);
            tracing::info!("User data stream connecting...");

            match connect_async(&ws_url).await {
                Ok((ws_stream, _response)) => {
                    tracing::info!("User data stream connected");
                    inner.connected.store(true, Ordering::Release);
                    attempt = 0;

                    let (_write, read) = ws_stream.split();

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
                        Some(Ok(Message::Ping(_))) => {}
                        Some(Ok(Message::Close(frame))) => {
                            tracing::info!("User data stream closed: {frame:?}");
                            return Err(crate::ExchangeError::StreamEnded);
                        }
                        Some(Ok(_)) => {}
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
                    let lk = inner.listen_key.lock().await;
                    if let Some(key) = lk.as_ref() {
                        let rest = inner.rest_client.lock().await;
                        if let Some(client) = rest.as_ref()
                            && let Err(e) = client.keepalive_listen_key(key).await
                        {
                            tracing::warn!("Listen key keepalive failed: {e}");
                        }
                    }
                }
            }

            if !inner.running.load(Ordering::Acquire) {
                return Ok(());
            }
        }
    }

    // ── Message handling ───────────────────────────────────────────

    /// Parse a user data stream message and send it through the event channel.
    pub(crate) fn handle_message(
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

    // ── Event parsing ──────────────────────────────────────────────

    /// Parse an `outboundAccountPosition` payload.
    pub(crate) fn parse_account_update(
        raw: &serde_json::Value,
    ) -> Result<AccountUpdateEvent, crate::ExchangeError> {
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

    /// Parse an `executionReport` payload.
    pub(crate) fn parse_order_update(
        raw: &serde_json::Value,
    ) -> Result<OrderUpdateEvent, crate::ExchangeError> {
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

    /// Parse a `balanceUpdate` payload.
    pub(crate) fn parse_balance_update(
        raw: &serde_json::Value,
    ) -> Result<BalanceUpdateEvent, crate::ExchangeError> {
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

    /// Parse a `listStatus` payload.
    pub(crate) fn parse_list_status(
        raw: &serde_json::Value,
    ) -> Result<ListStatusEvent, crate::ExchangeError> {
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
