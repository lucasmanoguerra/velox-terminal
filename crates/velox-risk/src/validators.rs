//! Pre-trade validators.

use crate::error::RiskError;
use crate::limits::RiskLimits;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use velox_core::{NewOrder, OrderId};

/// Result of a risk check.
#[derive(Debug)]
pub enum RiskCheckResult {
    Approved,
    Rejected(RiskError),
}

/// Performs all pre-trade risk validations.
pub struct RiskValidator {
    limits: RiskLimits,
    #[expect(dead_code)]
    positions: DashMap<String, f64>, // symbol → current position
    #[expect(dead_code)]
    daily_pnl: DashMap<String, f64>, // symbol → daily P&L
    order_timestamps: DashMap<OrderId, chrono::DateTime<chrono::Utc>>,
    circuit_breaker: Option<Arc<crate::circuit_breaker::CircuitBreaker>>,
}

impl RiskValidator {
    pub fn new(limits: RiskLimits) -> Self {
        Self {
            limits,
            positions: DashMap::new(),
            daily_pnl: DashMap::new(),
            order_timestamps: DashMap::new(),
            circuit_breaker: None,
        }
    }

    pub fn with_circuit_breaker(mut self, cb: Arc<crate::circuit_breaker::CircuitBreaker>) -> Self {
        self.circuit_breaker = Some(cb);
        self
    }

    /// Validate a new order against all risk rules.
    pub fn validate_order(&self, order: &NewOrder) -> RiskCheckResult {
        // 1. Symbol check
        if !self.limits.allowed_symbols.is_empty()
            && !self.limits.allowed_symbols.contains(&order.symbol)
        {
            return RiskCheckResult::Rejected(RiskError::SymbolNotAllowed {
                symbol: order.symbol.clone(),
            });
        }

        // 2. Max position size
        if order.quantity > self.limits.max_position_size {
            return RiskCheckResult::Rejected(RiskError::MaxPositionExceeded {
                max: self.limits.max_position_size,
            });
        }

        // 3. Max notional
        if let Some(price) = order.price {
            let notional = price * order.quantity;
            if notional > self.limits.max_notional {
                return RiskCheckResult::Rejected(RiskError::MaxNotionalExceeded {
                    max: self.limits.max_notional,
                });
            }
        }

        // 4. Order frequency
        let now = Utc::now();
        let recent_count = self
            .order_timestamps
            .iter()
            .filter(|entry| {
                let age = now - *entry.value();
                age.num_milliseconds() < 1000
            })
            .count();

        if recent_count as u32 >= self.limits.max_orders_per_second {
            return RiskCheckResult::Rejected(RiskError::OrderFrequencyExceeded {
                max_orders_per_second: self.limits.max_orders_per_second,
            });
        }

        // 5. Circuit breaker
        if let Some(ref cb) = self.circuit_breaker
            && cb.is_triggered(&order.symbol)
        {
            return RiskCheckResult::Rejected(RiskError::CircuitBreakerTriggered {
                symbol: order.symbol.clone(),
                reason: "Circuit breaker active".into(),
            });
        }

        RiskCheckResult::Approved
    }

    /// Record an order for frequency tracking.
    pub fn record_order(&self, order_id: OrderId) {
        self.order_timestamps.insert(order_id, Utc::now());
    }
}
