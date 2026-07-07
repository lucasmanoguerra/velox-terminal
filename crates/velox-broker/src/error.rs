//! Broker error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrokerError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Order rejected: {reason} (code: {code})")]
    OrderRejected { reason: String, code: String },

    #[error("Timeout on operation: {0}")]
    Timeout(String),

    #[error("Disconnected")]
    Disconnected,
}
