//! MACD (Moving Average Convergence Divergence) — incremental O(1).

use crate::traits::Indicator;
use crate::ema::Ema;

/// MACD output values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdOutput {
    pub macd_line: f64,
    pub signal_line: f64,
    pub histogram: f64,
}

/// MACD indicator with incremental O(1) update.
///
/// Standard parameters: fast=12, slow=26, signal=9.
pub struct Macd {
    fast_ema: Ema,
    slow_ema: Ema,
    signal_ema: Ema,
}

impl Macd {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        assert!(fast_period > 0 && slow_period > 0 && signal_period > 0,
            "MACD periods must be > 0");
        assert!(fast_period < slow_period,
            "fast_period ({}) must be < slow_period ({})", fast_period, slow_period);
        Self {
            fast_ema: Ema::new(fast_period),
            slow_ema: Ema::new(slow_period),
            signal_ema: Ema::new(signal_period),
        }
    }
}

impl Indicator<f64> for Macd {
    type Output = MacdOutput;

    fn update(&mut self, value: f64) -> MacdOutput {
        let fast = self.fast_ema.update(value);
        let slow = self.slow_ema.update(value);

        if !self.slow_ema.is_ready() {
            return MacdOutput {
                macd_line: fast - slow,
                signal_line: f64::NAN,
                histogram: f64::NAN,
            };
        }

        let macd_line = fast - slow;
        let signal_line = self.signal_ema.update(macd_line);
        let histogram = macd_line - signal_line;

        MacdOutput {
            macd_line,
            signal_line,
            histogram,
        }
    }

    fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.signal_ema.reset();
    }

    fn is_ready(&self) -> bool {
        self.signal_ema.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macd_basic() {
        let mut macd = Macd::new(3, 6, 2);
        let mut ready_count = 0;

        for i in 1..=20 {
            let output = macd.update(i as f64);
            if macd.is_ready() {
                ready_count += 1;
                assert!(!output.macd_line.is_nan());
                assert!(!output.signal_line.is_nan());
                assert!(!output.histogram.is_nan());
            }
        }

        // With slow=6, need at least 6 + signal=2 - 1 = 7 values for signal to be ready
        assert!(ready_count >= 12);
    }

    #[test]
    fn test_macd_zero_growth() {
        let mut macd = Macd::new(3, 6, 2);
        for _ in 0..10 {
            macd.update(100.0);
        }
        let output = macd.update(100.0);
        assert!(macd.is_ready());
        // When price is constant, macd_line should approach 0
        assert!(output.macd_line.abs() < 1.0);
    }

    #[test]
    fn test_macd_upward_trend() {
        let mut macd = Macd::new(3, 6, 2);
        for i in 1..=20 {
            macd.update(i as f64);
        }
        let output = macd.update(21.0);
        assert!(macd.is_ready());
        // In an upward trend, macd_line should be positive (fast > slow)
        assert!(output.macd_line > 0.0);
    }

    #[test]
    fn test_macd_downward_trend() {
        let mut macd = Macd::new(3, 6, 2);
        for i in (1..=20).rev() {
            macd.update(i as f64);
        }
        let output = macd.update(0.0);
        assert!(macd.is_ready());
        // In a downward trend, macd_line should be negative (fast < slow)
        assert!(output.macd_line < 0.0);
    }

    #[test]
    fn test_macd_reset() {
        let mut macd = Macd::new(3, 6, 2);
        for _ in 0..10 {
            macd.update(100.0);
        }
        assert!(macd.is_ready());
        macd.reset();
        assert!(!macd.is_ready());
    }
}
