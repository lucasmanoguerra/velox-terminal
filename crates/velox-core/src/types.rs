//! Common types and type aliases.

use serde::{Deserialize, Serialize};

/// A symbol identifier, stored as ASCII bytes for zero-copy.
pub type Symbol = [u8; 8];

/// Timestamp in nanoseconds since Unix epoch (for performance).
pub type TimestampNanos = i64;

/// Price with 4 decimal places (stored as integer for exact representation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Price4(pub i64);

impl Price4 {
    pub fn from_f64(value: f64) -> Self {
        Self((value * 10_000.0).round() as i64)
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64 / 10_000.0
    }
}

/// Account summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub cash: f64,
    pub buying_power: f64,
    pub equity: f64,
    pub margin_used: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub currency: String,
}

/// A position in a symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
}
