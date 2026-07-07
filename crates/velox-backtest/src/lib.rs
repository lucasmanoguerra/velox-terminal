//! # velox-backtest
//!
//! Backtesting engine that reuses live trading logic.
//!
//! Uses the same OMS, Risk, and indicators as the live system.
//! Configurable slippage, commission, and market impact models.

pub mod engine;
pub mod metrics;
pub mod slippage;

/// Placeholder for backtesting implementation.
/// Full implementation in Phase 5.
pub fn init() {
    tracing::info!("velox-backtest initialized");
}
