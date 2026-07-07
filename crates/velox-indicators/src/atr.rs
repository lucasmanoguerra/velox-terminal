//! ATR (Average True Range) — incremental O(1).

use crate::traits::Indicator;

/// Average True Range.
pub struct Atr {
    // Placeholder for full implementation
}

impl Atr {
    pub fn new(_period: usize) -> Self {
        Self {}
    }
}

impl Indicator<f64> for Atr {
    type Output = f64;

    fn update(&mut self, _value: f64) -> f64 {
        f64::NAN
    }

    fn reset(&mut self) {}

    fn is_ready(&self) -> bool {
        false
    }
}
