//! Risk limits configuration.

/// Configuration for all risk limits.
#[derive(Debug, Clone)]
pub struct RiskLimits {
    /// Maximum position size in contracts/shares.
    pub max_position_size: f64,
    /// Maximum notional value per order.
    pub max_notional: f64,
    /// Maximum daily loss before trading is halted.
    pub max_daily_loss: f64,
    /// Maximum orders per second.
    pub max_orders_per_second: u32,
    /// Symbols allowed for trading (empty = all).
    pub allowed_symbols: Vec<String>,
    /// Maximum position concentration (% of account).
    pub max_concentration_pct: f64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            max_position_size: 100.0,
            max_notional: 1_000_000.0,
            max_daily_loss: 10_000.0,
            max_orders_per_second: 10,
            allowed_symbols: Vec::new(),
            max_concentration_pct: 0.25,
        }
    }
}
