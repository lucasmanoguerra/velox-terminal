//! VWAP (Volume Weighted Average Price) — incremental O(1).

use crate::traits::Indicator;

/// Volume Weighted Average Price.
pub struct Vwap {
    // Placeholder for full implementation
}

impl Default for Vwap {
    fn default() -> Self {
        Self::new()
    }
}

impl Vwap {
    pub fn new() -> Self {
        Self {}
    }
}

impl Indicator<f64> for Vwap {
    type Output = f64;

    fn update(&mut self, value: f64) -> f64 {
        value // pass-through placeholder
    }

    fn reset(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }
}
