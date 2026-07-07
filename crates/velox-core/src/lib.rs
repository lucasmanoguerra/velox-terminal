//! # velox-core
//!
//! Domain primitives for velox-terminal trading platform.
//!
//! This crate defines the foundational types used across all other crates:
//! orders, trades, quotes, symbols, and error types.
//! No I/O, no complex logic — just types.

pub mod order;
pub mod market;
pub mod error;
pub mod types;

pub use order::*;
pub use market::*;
pub use error::*;
pub use types::*;
