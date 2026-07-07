//! Core error types.

use thiserror::Error;

/// Generic core error.
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),

    #[error("Invalid price: {0}")]
    InvalidPrice(f64),

    #[error("Invalid quantity: {0}")]
    InvalidQuantity(f64),

    #[error("Invalid order: {0}")]
    InvalidOrder(String),

    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}
