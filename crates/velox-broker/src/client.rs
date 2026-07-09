//! Broker client trait.

use async_trait::async_trait;
use velox_core::{AccountInfo, CoreError, NewOrder, OrderId, Position};

/// Configuration for connecting to a broker.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrokerConfig {
    pub api_key: String,
    pub api_secret: String,
    pub base_url: String,
    pub paper_trading: bool,
}

/// Connection handle (opaque).
#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    pub broker: String,
    pub session_id: String,
}

/// Trait that all broker implementations must satisfy.
#[async_trait]
pub trait BrokerClient: Send + Sync {
    /// Connect to the broker.
    async fn connect(&self, config: BrokerConfig) -> Result<ConnectionHandle, CoreError>;

    /// Disconnect gracefully.
    async fn disconnect(&self, handle: &ConnectionHandle) -> Result<(), CoreError>;

    /// Submit a new order.
    async fn submit_order(&self, order: NewOrder) -> Result<OrderId, CoreError>;

    /// Cancel an order.
    async fn cancel_order(&self, order_id: OrderId) -> Result<(), CoreError>;

    /// Get current positions.
    async fn get_positions(&self) -> Result<Vec<Position>, CoreError>;

    /// Get account info.
    async fn get_account_info(&self) -> Result<AccountInfo, CoreError>;
}
