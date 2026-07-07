//! Relative Strength Index — incremental O(1).

use crate::traits::Indicator;

/// Relative Strength Index.
pub struct Rsi {
    period: usize,
    gain_sum: f64,
    loss_sum: f64,
    prev_value: Option<f64>,
    count: usize,
}

impl Rsi {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "RSI period must be > 0");
        Self {
            period,
            gain_sum: 0.0,
            loss_sum: 0.0,
            prev_value: None,
            count: 0,
        }
    }
}

impl Indicator<f64> for Rsi {
    type Output = f64;

    fn update(&mut self, value: f64) -> f64 {
        if let Some(prev) = self.prev_value {
            let diff = value - prev;
            let gain = diff.max(0.0);
            let loss = (-diff).max(0.0);

            self.count += 1;

            if self.count <= self.period {
                self.gain_sum += gain;
                self.loss_sum += loss;
            } else {
                self.gain_sum = (self.gain_sum * (self.period as f64 - 1.0) + gain) / self.period as f64;
                self.loss_sum = (self.loss_sum * (self.period as f64 - 1.0) + loss) / self.period as f64;
            }
        }

        self.prev_value = Some(value);

        if self.count >= self.period && self.loss_sum > 0.0 {
            100.0 - 100.0 / (1.0 + self.gain_sum / self.loss_sum)
        } else if self.count >= self.period {
            100.0 // no losses, RSI = 100
        } else {
            f64::NAN
        }
    }

    fn reset(&mut self) {
        self.gain_sum = 0.0;
        self.loss_sum = 0.0;
        self.prev_value = None;
        self.count = 0;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }
}
