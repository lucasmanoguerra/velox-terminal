//! Mock broker for testing and paper trading.

use async_trait::async_trait;
use velox_core::{NewOrder, Order, OrderId, Position, AccountInfo, Fill, CoreError};
use crate::client::{BrokerClient, BrokerConfig, ConnectionHandle};

/// A simulated broker for paper trading and testing.
pub struct MockBroker {
    name: String,
}

impl MockBroker {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

#[async_trait]
impl BrokerClient for MockBroker {
    async fn connect(&self, config: BrokerConfig) -> Result<ConnectionHandle, CoreError> {
        Ok(ConnectionHandle {
            broker: self.name.clone(),
            session_id: "mock-session-001".to_string(),
        })
    }

    async fn disconnect(&self, _handle: &ConnectionHandle) -> Result<(), CoreError> {
        Ok(())
    }

    async fn submit_order(&self, _order: NewOrder) -> Result<OrderId, CoreError> {
        Ok(OrderId::new())
    }

    async fn cancel_order(&self, _order_id: OrderId) -> Result<(), CoreError> {
        Ok(())
    }

    async fn get_positions(&self) -> Result<Vec<Position>, CoreError> {
        Ok(vec![])
    }

    async fn get_account_info(&self) -> Result<AccountInfo, CoreError> {
        Ok(AccountInfo {
            cash: 100_000.0,
            buying_power: 200_000.0,
            equity: 100_000.0,
            margin_used: 0.0,
            unrealized_pnl: 0.0,
            realized_pnl: 0.0,
            currency: "USD".to_string(),
        })
    }
}
