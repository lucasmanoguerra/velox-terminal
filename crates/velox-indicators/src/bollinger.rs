//! Bollinger Bands — incremental O(1).

use crate::traits::Indicator;

/// Bollinger Bands output.
#[derive(Debug, Clone)]
pub struct BollingerOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

/// Bollinger Bands indicator.
pub struct BollingerBands {
    // Placeholder for full implementation
}

impl BollingerBands {
    pub fn new(_period: usize, _std_dev: f64) -> Self {
        Self {}
    }
}

impl Indicator<f64> for BollingerBands {
    type Output = BollingerOutput;

    fn update(&mut self, _value: f64) -> BollingerOutput {
        BollingerOutput {
            upper: f64::NAN,
            middle: f64::NAN,
            lower: f64::NAN,
        }
    }

    fn reset(&mut self) {}

    fn is_ready(&self) -> bool {
        false
    }
}
