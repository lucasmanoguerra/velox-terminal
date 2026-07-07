//! OMS error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OmsError {
    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Invalid state transition: {from:?} → {to:?}")]
    InvalidTransition { from: String, to: String },

    #[error("Order rejected: {0}")]
    Rejected(String),

    #[error("Risk check failed: {0}")]
    RiskCheckFailed(String),

    #[error("Broker error: {0}")]
    BrokerError(String),

    #[error("Duplicate order: {0}")]
    DuplicateOrder(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
