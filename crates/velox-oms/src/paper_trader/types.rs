//! Internal types for the paper trading engine.

/// Bracket configuration stored for an entry order.
#[derive(Debug, Clone)]
pub(crate) struct BracketConfig {
    pub take_profit_price: f64,
    pub stop_loss_price: f64,
}
