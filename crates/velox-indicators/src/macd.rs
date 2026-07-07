//! MACD (Moving Average Convergence Divergence) — incremental O(1).

use crate::traits::Indicator;

/// MACD indicator.
pub struct Macd {
    // Placeholder for full implementation
}

impl Macd {
    pub fn new(_fast_period: usize, _slow_period: usize, _signal_period: usize) -> Self {
        Self {}
    }
}

impl Indicator<f64> for Macd {
    type Output = f64;

    fn update(&mut self, _value: f64) -> f64 {
        f64::NAN
    }

    fn reset(&mut self) {}

    fn is_ready(&self) -> bool {
        false
    }
}
