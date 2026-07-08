//! # velox-risk
//!
//! Risk Management — pre-trade validation, position limits, circuit breakers.
//!
//! **Critical subsystem**: zero `unsafe` allowed. Last barrier before an order
//! reaches the real market. Profile: ReleaseSafe.

#![forbid(unsafe_code)]

pub mod circuit_breaker;
pub mod error;
pub mod limits;
pub mod validators;

pub use circuit_breaker::*;
pub use error::*;
pub use limits::*;
pub use validators::*;
