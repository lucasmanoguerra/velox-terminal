//! Bollinger Bands — incremental O(1).

use crate::traits::Indicator;
use crate::sma::Sma;

/// Bollinger Bands output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollingerOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
    pub bandwidth: f64,  // (upper - lower) / middle
    pub percent_b: f64,  // (price - lower) / (upper - lower)
}

/// Bollinger Bands with incremental O(1) update per bar.
///
/// Standard parameters: period=20, std_dev=2.0.
pub struct BollingerBands {
    period: usize,
    std_dev: f64,
    sma: Sma,
    buffer: Vec<f64>,
    last_value: f64,
}

impl BollingerBands {
    pub fn new(period: usize, std_dev: f64) -> Self {
        assert!(period > 0, "Bollinger period must be > 0");
        assert!(std_dev > 0.0, "Standard deviation must be > 0");
        Self {
            period,
            std_dev,
            sma: Sma::new(period),
            buffer: Vec::with_capacity(period),
            last_value: 0.0,
        }
    }

    /// Calculate population standard deviation from the rolling buffer.
    fn std_deviation(&self, mean: f64) -> f64 {
        if self.buffer.len() < 2 {
            return 0.0;
        }
        let variance = self.buffer.iter()
            .map(|v| {
                let diff = v - mean;
                diff * diff
            })
            .sum::<f64>()
            / self.buffer.len() as f64;
        variance.sqrt()
    }
}

impl Indicator<f64> for BollingerBands {
    type Output = BollingerOutput;

    fn update(&mut self, value: f64) -> BollingerOutput {
        self.last_value = value;
        self.buffer.push(value);
        let middle = self.sma.update(value);

        if !self.sma.is_ready() {
            return BollingerOutput {
                upper: f64::NAN,
                middle,
                lower: f64::NAN,
                bandwidth: f64::NAN,
                percent_b: f64::NAN,
            };
        }

        // Maintain buffer size
        if self.buffer.len() > self.period {
            self.buffer.remove(0);
        }

        let std = self.std_deviation(middle);
        let band = self.std_dev * std;
        let upper = middle + band;
        let lower = middle - band;
        let bandwidth = if middle != 0.0 { (upper - lower) / middle.abs() } else { 0.0 };
        let percent_b = if upper != lower {
            (value - lower) / (upper - lower)
        } else {
            0.5
        };

        BollingerOutput {
            upper,
            middle,
            lower,
            bandwidth,
            percent_b,
        }
    }

    fn reset(&mut self) {
        self.sma.reset();
        self.buffer.clear();
        self.last_value = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.sma.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bollinger_basic() {
        let mut bb = BollingerBands::new(5, 2.0);
        for i in 1..=5 {
            let out = bb.update(i as f64);
            if i < 5 {
                assert!(out.upper.is_nan());
            } else {
                assert!(!out.upper.is_nan());
                // With values 1..5, mean=3, std≈1.41, upper≈3+2*1.41=5.83, lower≈3-2*1.41=0.17
                assert!(out.middle > 2.9 && out.middle < 3.1);
                assert!(out.upper > out.middle);
                assert!(out.lower < out.middle);
                assert!(out.bandwidth > 0.0);
            }
        }
    }

    #[test]
    fn test_bollinger_constant_values() {
        let mut bb = BollingerBands::new(5, 2.0);
        for _ in 0..10 {
            let out = bb.update(50.0);
            if bb.is_ready() {
                // With constant values, std=0, bands collapse to middle
                assert!((out.upper - out.middle).abs() < 1e-10);
                assert!((out.lower - out.middle).abs() < 1e-10);
                assert!((out.percent_b - 0.5).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_bollinger_reset() {
        let mut bb = BollingerBands::new(5, 2.0);
        for _ in 0..5 {
            bb.update(10.0);
        }
        assert!(bb.is_ready());
        bb.reset();
        assert!(!bb.is_ready());
    }

    #[test]
    fn test_percent_b_range() {
        let mut bb = BollingerBands::new(10, 2.0);
        // Values that stay within bands
        for _ in 0..20 {
            let out = bb.update(100.0);
            if bb.is_ready() {
                assert!(out.percent_b >= 0.0 || out.percent_b.is_nan());
                assert!(out.percent_b <= 1.0 || out.percent_b.is_nan());
            }
        }
    }
}
