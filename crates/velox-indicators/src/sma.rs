//! Simple Moving Average — incremental O(1).

use crate::traits::Indicator;

/// Simple Moving Average.
pub struct Sma {
    period: usize,
    values: Vec<f64>,
    sum: f64,
    ready: bool,
}

impl Sma {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "SMA period must be > 0");
        Self {
            period,
            values: Vec::with_capacity(period),
            sum: 0.0,
            ready: false,
        }
    }
}

impl Indicator<f64> for Sma {
    type Output = f64;

    fn update(&mut self, value: f64) -> f64 {
        self.values.push(value);
        self.sum += value;

        if self.values.len() > self.period {
            let removed = self.values.remove(0);
            self.sum -= removed;
        }

        if self.values.len() >= self.period {
            self.ready = true;
        }

        if self.ready {
            self.sum / self.period as f64
        } else {
            f64::NAN
        }
    }

    fn reset(&mut self) {
        self.values.clear();
        self.sum = 0.0;
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_basic() {
        let mut sma = Sma::new(3);
        assert!(sma.update(1.0).is_nan());
        assert!(sma.update(2.0).is_nan());
        assert_eq!(sma.update(3.0), 2.0); // (1+2+3)/3
        assert_eq!(sma.update(4.0), 3.0); // (2+3+4)/3
        assert_eq!(sma.update(5.0), 4.0); // (3+4+5)/3
    }

    #[test]
    fn test_sma_single_period() {
        let mut sma = Sma::new(1);
        assert_eq!(sma.update(42.0), 42.0);
    }
}
