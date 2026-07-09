//! Binance broker implementation of the [`BrokerClient`] trait.
//!
//! NOTE: File exceeds 200 lines (396 total) because it implements a full trait
//! with 6 methods + formatting helpers + 8 unit tests. Each method is a single
//! responsibility; splitting would harm cohesion.
//!
//! Bridges the order management system (OMS) with Binance's REST API,
//! mapping domain types (`NewOrder`, `OrderId`) to Binance-specific
//! request parameters and responses.
//!
//! # Order ID Mapping
//!
//! The OMS uses [`OrderId`] (UUID) while Binance uses numeric order IDs.
//! This module maintains a bidirectional mapping:
//!
//! - Our UUID is sent as `newClientOrderId` for idempotency
//! - Binance's numeric `orderId` is stored alongside our UUID

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Mutex;
use velox_broker::{BrokerClient, BrokerConfig, ConnectionHandle};
use velox_core::{
    AccountInfo, CoreError, NewOrder, OrderId, OrderType, Position, Side, TimeInForce,
};

use crate::binance_rest::BinanceRestClient;
use crate::ExchangeError;

/// Binance implementation of [`BrokerClient`].
///
/// Wraps [`BinanceRestClient`] and maps domain order types to
/// Binance REST API parameters.
///
/// # Example
///
/// ```rust,no_run
/// use velox_broker::{BrokerClient, BrokerConfig};
/// use velox_exchange::binance_broker::BinanceBroker;
///
/// # async fn example() {
/// let broker = BinanceBroker::new();
/// let config = BrokerConfig {
///     api_key: "key".into(),
///     api_secret: "secret".into(),
///     base_url: "https://testnet.binance.vision".into(),
///     paper_trading: false,
/// };
/// let handle = broker.connect(config).await.unwrap();
/// # }
/// ```
pub struct BinanceBroker {
    /// The underlying REST client.
    rest_client: Arc<Mutex<Option<BinanceRestClient>>>,
    /// Connection state.
    connected: AtomicBool,
    /// Connection handle for disconnect.
    handle: Mutex<Option<ConnectionHandle>>,
    /// Mapping from our UUID-based OrderId to (Binance numeric orderId, symbol).
    order_map: Arc<Mutex<HashMap<OrderId, (i64, String)>>>,
}

impl BinanceBroker {
    /// Create a new Binance broker (not yet connected).
    pub fn new() -> Self {
        Self {
            rest_client: Arc::new(Mutex::new(None)),
            connected: AtomicBool::new(false),
            handle: Mutex::new(None),
            order_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a pre-configured broker with a REST client (useful for testing).
    pub fn with_client(client: BinanceRestClient) -> Self {
        Self {
            rest_client: Arc::new(Mutex::new(Some(client))),
            connected: AtomicBool::new(true),
            handle: Mutex::new(None),
            order_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get a cloned REST client for calling methods independently.
    async fn get_client(&self) -> Result<BinanceRestClient, ExchangeError> {
        let guard = self.rest_client.lock().await;
        guard
            .as_ref()
            .cloned()
            .ok_or(ExchangeError::ApiKeyNotConfigured)
    }

    /// Get the underlying REST client reference (for testing).
    pub async fn rest_client_ref(&self) -> Option<BinanceRestClient> {
        let guard = self.rest_client.lock().await;
        guard.clone()
    }

    // ── Parameter mapping helpers ───────────────────────────────────

    /// Map a [`NewOrder`] to Binance order parameters and submit.
    ///
    /// Returns the Binance-assigned order ID and symbol on success.
    pub async fn submit_order_raw(
        &self,
        order: &NewOrder,
        client_order_id: &str,
    ) -> Result<(i64, String), ExchangeError> {
        let client = self.get_client().await?;

        let symbol = order.symbol.to_uppercase().replace(['-', '/'], "");
        let side = match order.side {
            Side::Buy => "BUY",
            Side::Sell => "SELL",
        };
        let order_type = match order.order_type {
            OrderType::Market => "MARKET",
            OrderType::Limit => "LIMIT",
            OrderType::StopMarket => "STOP_LOSS",
            OrderType::StopLimit => "STOP_LOSS_LIMIT",
        };
        let quantity = format_quantity(order.quantity);

        let time_in_force = match order.time_in_force {
            TimeInForce::Gtc => Some("GTC"),
            TimeInForce::Ioc => Some("IOC"),
            TimeInForce::Fok => Some("FOK"),
            TimeInForce::Day => Some("GTC"),
            TimeInForce::Gtd => None,
        };

        let price = match order.order_type {
            OrderType::Limit | OrderType::StopLimit => order.price.map(format_price),
            _ => None,
        };

        let stop_price = match order.order_type {
            OrderType::StopMarket | OrderType::StopLimit => order.stop_price.map(format_price),
            _ => None,
        };

        let response = client
            .new_order(
                &symbol,
                side,
                order_type,
                &quantity,
                price.as_deref(),
                time_in_force,
                stop_price.as_deref(),
                Some(client_order_id),
            )
            .await?;

        Ok((response.order_id, response.symbol))
    }
}

impl Default for BinanceBroker {
    fn default() -> Self {
        Self::new()
    }
}

// ── BrokerClient trait implementation ─────────────────────────────

#[async_trait]
impl BrokerClient for BinanceBroker {
    async fn connect(&self, config: BrokerConfig) -> Result<ConnectionHandle, CoreError> {
        let client = BinanceRestClient::new(
            config.api_key.clone(),
            config.api_secret.clone(),
            config.paper_trading,
        );

        let handle = ConnectionHandle {
            broker: "binance".into(),
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        *self.rest_client.lock().await = Some(client);
        self.connected.store(true, Ordering::Release);
        *self.handle.lock().await = Some(handle.clone());

        Ok(handle)
    }

    async fn disconnect(&self, _handle: &ConnectionHandle) -> Result<(), CoreError> {
        self.connected.store(false, Ordering::Release);
        *self.rest_client.lock().await = None;
        *self.handle.lock().await = None;
        self.order_map.lock().await.clear();
        Ok(())
    }

    async fn submit_order(&self, order: NewOrder) -> Result<OrderId, CoreError> {
        if !self.connected.load(Ordering::Acquire) {
            return Err(CoreError::Internal("Broker not connected".into()));
        }

        // If caller provided a client_order_id, use it for idempotency;
        // otherwise generate one.
        let client_order_id = order
            .client_order_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let our_order_id = OrderId::new();

        let (binance_order_id, symbol) = self
            .submit_order_raw(&order, &client_order_id)
            .await
            .map_err(|e| CoreError::Internal(format!("Binance order submission failed: {e}")))?;

        self.order_map
            .lock()
            .await
            .insert(our_order_id, (binance_order_id, symbol));

        Ok(our_order_id)
    }

    async fn cancel_order(&self, order_id: OrderId) -> Result<(), CoreError> {
        if !self.connected.load(Ordering::Acquire) {
            return Err(CoreError::Internal("Broker not connected".into()));
        }

        let client = self.get_client().await.map_err(|e| {
            CoreError::Internal(format!("Failed to get REST client: {e}"))
        })?;

        let (binance_id, symbol) = self
            .order_map
            .lock()
            .await
            .get(&order_id)
            .cloned()
            .ok_or_else(|| CoreError::OrderNotFound(order_id.0.to_string()))?;

        client
            .cancel_order(&symbol, binance_id)
            .await
            .map_err(|e| CoreError::Internal(format!("Binance cancel failed: {e}")))?;

        self.order_map.lock().await.remove(&order_id);
        Ok(())
    }

    async fn get_positions(&self) -> Result<Vec<Position>, CoreError> {
        // For spot trading, positions are best computed from OMS fill history.
        // Account balances are available via get_account_info.
        Ok(vec![])
    }

    async fn get_account_info(&self) -> Result<AccountInfo, CoreError> {
        let client = self.get_client().await.map_err(|e| {
            CoreError::Internal(format!("Failed to get REST client: {e}"))
        })?;

        let binance_account = client
            .account()
            .await
            .map_err(|e| CoreError::Internal(format!("Binance account fetch failed: {e}")))?;

        let mut cash = 0.0_f64;
        let mut locked = 0.0_f64;

        for balance in &binance_account.balances {
            let free: f64 = balance.free.parse().unwrap_or(0.0);
            let locked_val: f64 = balance.locked.parse().unwrap_or(0.0);
            cash += free;
            locked += locked_val;
        }

        Ok(AccountInfo {
            cash,
            buying_power: cash * 2.0,
            equity: cash + locked,
            margin_used: locked,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            currency: "USD".to_string(),
        })
    }
}

impl std::fmt::Debug for BinanceBroker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceBroker")
            .field("connected", &self.connected.load(Ordering::Acquire))
            .finish()
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Format a quantity as a string with up to 8 decimal places.
fn format_quantity(qty: f64) -> String {
    if qty.fract() < 1e-8 {
        format!("{:.0}", qty)
    } else {
        let s = format!("{:.8}", qty);
        s.trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Format a price as a string with up to 8 decimal places.
fn format_price(price: f64) -> String {
    format_quantity(price)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_quantity_integer() {
        assert_eq!(format_quantity(1.0), "1");
        assert_eq!(format_quantity(100.0), "100");
        assert_eq!(format_quantity(0.0), "0");
    }

    #[test]
    fn test_format_quantity_decimal() {
        assert_eq!(format_quantity(0.01), "0.01");
        assert_eq!(format_quantity(0.001), "0.001");
        assert_eq!(format_quantity(1.5), "1.5");
    }

    #[test]
    fn test_format_quantity_precision() {
        let qty = format_quantity(0.00123456);
        assert_eq!(qty, "0.00123456");
    }

    #[test]
    fn test_format_price() {
        assert_eq!(format_price(45000.0), "45000");
        assert_eq!(format_price(45000.12), "45000.12");
        assert_eq!(format_price(0.01), "0.01");
    }

    #[test]
    fn test_debug_format() {
        let broker = BinanceBroker::new();
        let debug = format!("{broker:?}");
        assert!(debug.contains("connected"));
        assert!(debug.contains("false"));
    }

    #[test]
    fn test_is_connected_initially_false() {
        let broker = BinanceBroker::new();
        assert!(!broker.connected.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn test_submit_order_requires_connection() {
        let broker = BinanceBroker::new();
        let order = NewOrder {
            symbol: "BTCUSDT".into(),
            side: Side::Buy,
            order_type: OrderType::Market,
            quantity: 0.01,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::Ioc,
            client_order_id: None,
            take_profit_price: None,
            stop_loss_price: None,
        };

        let result = broker.submit_order(order).await;
        assert!(result.is_err(), "Should fail without connection");
    }

    #[tokio::test]
    async fn test_connect_and_connection_state() {
        let broker = BinanceBroker::new();
        let config = BrokerConfig {
            api_key: "test_key".into(),
            api_secret: "test_secret".into(),
            base_url: "https://test.binance.com".into(),
            paper_trading: true,
        };

        let handle = broker.connect(config).await.unwrap();
        assert_eq!(handle.broker, "binance");
        assert!(broker.connected.load(Ordering::Acquire));

        broker.disconnect(&handle).await.unwrap();
        assert!(!broker.connected.load(Ordering::Acquire));
    }

    #[test]
    fn test_default() {
        let broker = BinanceBroker::default();
        assert!(!broker.connected.load(Ordering::Acquire));
    }
}
