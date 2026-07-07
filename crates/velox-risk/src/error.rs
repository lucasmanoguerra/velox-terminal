//! Risk error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RiskError {
    #[error("Order exceeds max position size: {max}")]
    MaxPositionExceeded { max: f64 },

    #[error("Order exceeds max notional value: {max}")]
    MaxNotionalExceeded { max: f64 },

    #[error("Insufficient buying power: available={available}, required={required}")]
    InsufficientBuyingPower { available: f64, required: f64 },

    #[error("Daily loss limit reached: {current_loss}")]
    DailyLossLimitReached { current_loss: f64 },

    #[error("Circuit breaker triggered for symbol {symbol}: {reason}")]
    CircuitBreakerTriggered { symbol: String, reason: String },

    #[error("Order frequency exceeded: {max_orders_per_second} orders/s")]
    OrderFrequencyExceeded { max_orders_per_second: u32 },

    #[error("Symbol {symbol} not in allowed list")]
    SymbolNotAllowed { symbol: String },

    #[error("Internal error: {0}")]
    Internal(String),
}
