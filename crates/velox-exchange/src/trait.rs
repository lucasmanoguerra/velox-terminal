//! Exchange connector trait — common interface for all exchange implementations.
//!
//! Each exchange (Binance, Kraken, Coinbase, etc.) provides a feed of market data
//! events (ticks, quotes) via the [`ExchangeFeed`] trait. The feed writes into a
//! shared [`RingBuffer`] for lock-free SPSC consumption by the market data pipeline.

use std::sync::Arc;
use velox_core::CoreError;
use velox_md::ring_buffer::RingBuffer;

/// Common interface for real-time exchange data feeds.
///
/// Implementations connect to an exchange's WebSocket API and push
/// [`MarketEvent`]s (ticks/quotes) into the provided ring buffer.
pub trait ExchangeFeed: Send + Sync {
    /// Start the feed, writing events into the provided ring buffer.
    ///
    /// This typically spawns one or more tokio tasks. The feed continues
    /// until [`stop`](Self::stop) is called or the connection drops.
    fn start(&self, ring: Arc<RingBuffer>) -> Result<(), CoreError>;

    /// Gracefully stop the feed and disconnect.
    fn stop(&self) -> Result<(), CoreError>;

    /// Subscribe to a symbol's trade and/or quote stream.
    fn subscribe(&self, symbol: &str) -> Result<(), CoreError>;

    /// Unsubscribe from a symbol.
    fn unsubscribe(&self, symbol: &str) -> Result<(), CoreError>;
}
