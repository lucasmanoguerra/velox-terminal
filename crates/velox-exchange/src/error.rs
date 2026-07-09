//! Exchange connector error types.

use thiserror::Error;

/// Errors originating from exchange connector operations.
#[derive(Error, Debug)]
pub enum ExchangeError {
    #[error("WebSocket connection failed: {0}")]
    WebSocket(String),

    #[error("WebSocket stream ended unexpectedly")]
    StreamEnded,

    #[error("JSON parse error: {0}")]
    JsonParse(String),

    #[error("Invalid symbol format: {0}")]
    InvalidSymbol(String),

    #[error("Rate limited by exchange")]
    RateLimited,

    #[error("Exchange error: {0}")]
    Exchange(String),

    #[error("HTTP request error: {0}")]
    Http(String),

    #[error("API key not configured")]
    ApiKeyNotConfigured,

    #[error("Core error: {0}")]
    Core(#[from] velox_core::CoreError),

    #[error("Internal error: {0}")]
    Internal(String),
}
