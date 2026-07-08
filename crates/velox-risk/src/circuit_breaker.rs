//! Circuit breaker — halts trading for a symbol when triggered.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Mutex;

/// Reason for circuit breaker trigger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerReason {
    MaxDailyLoss,
    ConsecutiveLosses(u32),
    ManualOverride,
    VolatilitySpike,
}

/// Circuit breaker state for a symbol.
#[derive(Debug, Clone)]
struct SymbolBreaker {
    triggered: bool,
    #[expect(dead_code)]
    reason: CircuitBreakerReason,
    #[expect(dead_code)]
    triggered_at: Option<DateTime<Utc>>,
    cooldown_until: Option<DateTime<Utc>>,
}

/// Circuit breaker manager.
pub struct CircuitBreaker {
    breakers: Mutex<HashMap<String, SymbolBreaker>>,
    cooldown_secs: i64,
    #[expect(dead_code)]
    max_consecutive_losses: u32,
}

impl CircuitBreaker {
    pub fn new(cooldown_secs: i64, max_consecutive_losses: u32) -> Self {
        Self {
            breakers: Mutex::new(HashMap::new()),
            cooldown_secs,
            max_consecutive_losses,
        }
    }

    /// Check if circuit breaker is triggered for a symbol.
    pub fn is_triggered(&self, symbol: &str) -> bool {
        let breakers = self.breakers.lock().unwrap();
        if let Some(breaker) = breakers.get(symbol) {
            if !breaker.triggered {
                return false;
            }
            // Check cooldown
            if let Some(cooldown) = breaker.cooldown_until
                && Utc::now() >= cooldown
            {
                return false; // cooldown expired
            }
            return true;
        }
        false
    }

    /// Trigger the circuit breaker for a symbol.
    pub fn trigger(&self, symbol: &str, reason: CircuitBreakerReason) {
        let mut breakers = self.breakers.lock().unwrap();
        let now = Utc::now();
        breakers.insert(
            symbol.to_string(),
            SymbolBreaker {
                triggered: true,
                reason,
                triggered_at: Some(now),
                cooldown_until: Some(now + chrono::Duration::seconds(self.cooldown_secs)),
            },
        );
    }

    /// Reset the circuit breaker for a symbol.
    pub fn reset(&self, symbol: &str) {
        let mut breakers = self.breakers.lock().unwrap();
        breakers.remove(symbol);
    }
}
