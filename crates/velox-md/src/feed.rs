//! Feed abstraction for market data sources.

use std::sync::Arc;
use velox_core::CoreError;

use crate::ring_buffer::{RingBuffer, MarketEvent};

/// Trait for market data feed implementations.
pub trait MarketFeed: Send + Sync {
    /// Start the feed, writing events into the provided ring buffer.
    fn start(&self, ring: Arc<RingBuffer>) -> Result<(), CoreError>;

    /// Stop the feed gracefully.
    fn stop(&self) -> Result<(), CoreError>;

    /// Subscribe to a symbol.
    fn subscribe(&self, symbol: &str) -> Result<(), CoreError>;

    /// Unsubscribe from a symbol.
    fn unsubscribe(&self, symbol: &str) -> Result<(), CoreError>;
}
