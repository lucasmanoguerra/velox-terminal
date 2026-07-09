//! # velox-exchange
//!
//! Exchange connectors for real-time market data and order execution.
//!
//! Provides WebSocket, REST, and FIX-based connectors to crypto and
//! traditional brokers. Each exchange implements the [`ExchangeFeed`] trait
//! for push-based market data consumption.
//!
//! # Architecture
//!
//! Each exchange connector:
//! 1. Connects to the exchange's public WebSocket API
//! 2. Subscribes to trade/quote streams for requested symbols
//! 3. Parses exchange-specific JSON/protobuf into [`velox_core::Tick`]/[`velox_core::Quote`]
//! 4. Pushes [`MarketEvent`]s into a shared lock-free [`RingBuffer`]
//!
//! The ring buffer is consumed by [`velox_md`]'s aggregation pipeline,
//! which produces OHLCV candles for the charting engine.
//!
//! # Supported Exchanges
//!
//! | Exchange | Trades | Quotes | Status |
//! |----------|--------|--------|--------|
//! | Binance  | ✅     | ✅     | Active |
//! | Kraken   | ❌     | ❌     | Planned |
//! | Coinbase | ❌     | ❌     | Planned |

pub mod binance;
pub mod binance_broker;
pub mod binance_rest;
pub mod binance_user_data;
pub mod error;
pub mod r#trait;

pub use error::ExchangeError;
pub use r#trait::ExchangeFeed;
