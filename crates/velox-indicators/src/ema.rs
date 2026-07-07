//! Exponential Moving Average — incremental O(1).

use crate::traits::Indicator;

/// Exponential Moving Average.
pub struct Ema {
    #[expect(dead_code)]
    period: usize,
    multiplier: f64,
    current: f64,
    count: usize,
}

impl Ema {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "EMA period must be > 0");
        let multiplier = 2.0 / (period as f64 + 1.0);
        Self {
            period,
            multiplier,
            current: 0.0,
            count: 0,
        }
    }
}

impl Indicator<f64> for Ema {
    type Output = f64;

    fn update(&mut self, value: f64) -> f64 {
        self.count += 1;

        if self.count == 1 {
            self.current = value;
        } else {
            self.current = (value - self.current) * self.multiplier + self.current;
        }

        self.current
    }

    fn reset(&mut self) {
        self.current = 0.0;
        self.count = 0;
    }

    fn is_ready(&self) -> bool {
        self.count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_basic() {
        let mut ema = Ema::new(3);
        let v = ema.update(1.0);
        assert_eq!(v, 1.0);
        let v = ema.update(2.0);
        assert!((v - 1.5).abs() < 1e-10);
        let v = ema.update(3.0);
        assert!((v - 2.25).abs() < 1e-10);
    }
}
