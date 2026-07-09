//! Market data primitives — Tick, Quote, OHLCV.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single trade tick.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: [u8; 8], // ASCII symbol, zero-padded
    pub price: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
    pub conditions: u32, // bitmask of trade conditions
}

/// A top-of-book quote.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: [u8; 8],
    pub bid_price: f64,
    pub bid_size: f64,
    pub ask_price: f64,
    pub ask_size: f64,
    pub timestamp: DateTime<Utc>,
}

/// OHLCV candle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Candle {
    pub symbol: [u8; 8],
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
    pub timeframe_secs: i64,
    pub trade_count: Option<u64>,
    pub vwap: Option<f64>,
}

impl Candle {
    pub fn is_bullish(&self) -> bool {
        self.close >= self.open
    }

    pub fn body(&self) -> f64 {
        (self.close - self.open).abs()
    }

    pub fn upper_wick(&self) -> f64 {
        self.high - self.open.max(self.close)
    }

    pub fn lower_wick(&self) -> f64 {
        self.open.min(self.close) - self.low
    }
}

/// Trading status for a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingStatus {
    Trading,
    Halted,
    Paused,
    Closed,
}

/// A single level in the order book (price + quantity).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
}

/// An order book snapshot (bids and asks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    /// Bids sorted descending by price (best bid first).
    pub bids: Vec<OrderBookLevel>,
    /// Asks sorted ascending by price (best ask first).
    pub asks: Vec<OrderBookLevel>,
    /// Exchange-provided update ID.
    pub last_update_id: u64,
}
