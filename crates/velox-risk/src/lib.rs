//! # velox-risk
//!
//! Risk Management — pre-trade validation, position limits, circuit breakers.
//!
//! **Critical subsystem**: zero `unsafe` allowed. Last barrier before an order
//! reaches the real market. Profile: ReleaseSafe.

#![forbid(unsafe_code)]

pub mod validators;
pub mod limits;
pub mod circuit_breaker;
pub mod error;

pub use validators::*;
pub use limits::*;
pub use circuit_breaker::*;
pub use error::*;
