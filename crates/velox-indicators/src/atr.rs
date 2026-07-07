//! ATR (Average True Range) — incremental O(1) using Wilder's smoothing.

use crate::traits::Indicator;

/// Average True Range with Wilder's smoothing.
///
/// Standard period: 14.
/// Input: (high, low, close) tuples.
pub struct Atr {
    period: usize,
    prev_close: Option<f64>,
    current_atr: f64,
    count: usize,
    first_tr_sum: f64,
}

impl Atr {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "ATR period must be > 0");
        Self {
            period,
            prev_close: None,
            current_atr: 0.0,
            count: 0,
            first_tr_sum: 0.0,
        }
    }

    /// Calculate True Range.
    fn true_range(high: f64, low: f64, prev_close: f64) -> f64 {
        let hl = high - low;
        let hc = (high - prev_close).abs();
        let lc = (low - prev_close).abs();
        hl.max(hc).max(lc)
    }
}

impl Indicator<(f64, f64, f64)> for Atr {
    type Output = f64;

    fn update(&mut self, ohlc: (f64, f64, f64)) -> f64 {
        let (high, low, close) = ohlc;

        if let Some(prev_close) = self.prev_close {
            let tr = Self::true_range(high, low, prev_close);

            self.count += 1;

            if self.count <= self.period {
                // First period: simple average of TR
                self.first_tr_sum += tr;
                if self.count == self.period {
                    self.current_atr = self.first_tr_sum / self.period as f64;
                }
            } else {
                // Wilder's smoothing: ATR = (prev_ATR * (period-1) + TR) / period
                self.current_atr =
                    (self.current_atr * (self.period as f64 - 1.0) + tr) / self.period as f64;
            }
        }

        self.prev_close = Some(close);

        if self.count >= self.period {
            self.current_atr
        } else {
            f64::NAN
        }
    }

    fn reset(&mut self) {
        self.prev_close = None;
        self.current_atr = 0.0;
        self.count = 0;
        self.first_tr_sum = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_basic() {
        let mut atr = Atr::new(5);
        // First value doesn't have prev_close, so NaN
        assert!(atr.update((10.0, 9.0, 9.5)).is_nan());

        for i in 0..10 {
            let val = atr.update((10.0 + i as f64, 9.0 + i as f64, 9.5 + i as f64));
            if i >= 4 {
                assert!(!val.is_nan(), "ATR should be ready at i={}", i);
                assert!(val > 0.0);
            }
        }
        assert!(atr.is_ready());
    }

    #[test]
    fn test_atr_constant_range() {
        let mut atr = Atr::new(5);
        // Need first value to set prev_close
        atr.update((10.0, 9.0, 9.5));
        // Constant high-low range of 2.0
        for _ in 0..10 {
            let tr = atr.update((12.0, 10.0, 11.0));
            if atr.is_ready() {
                // With constant TR of 2.0, ATR should converge to 2.0
                assert!((tr - 2.0).abs() < 0.5);
            }
        }
    }

    #[test]
    fn test_atr_reset() {
        let mut atr = Atr::new(5);
        atr.update((10.0, 9.0, 9.5));
        for _ in 0..5 {
            atr.update((11.0, 9.0, 10.0));
        }
        assert!(atr.is_ready());
        atr.reset();
        assert!(!atr.is_ready());
    }

    #[test]
    fn test_true_range_calculation() {
        // high-low is largest
        assert!((Atr::true_range(12.0, 10.0, 11.0) - 2.0).abs() < 1e-10);
        // high-prev_close is largest (gap up)
        assert!((Atr::true_range(15.0, 11.0, 10.0) - 5.0).abs() < 1e-10);
        // low-prev_close is largest (gap down)
        assert!((Atr::true_range(10.0, 5.0, 12.0) - 7.0).abs() < 1e-10);
    }
}
