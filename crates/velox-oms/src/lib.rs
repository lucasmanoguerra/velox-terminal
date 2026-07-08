//! # velox-oms
//!
//! Order Management System.
//!
//! **Critical subsystem**: zero `unsafe` allowed. All orders validated by
//! risk management before execution. Profile: ReleaseSafe.
//!
//! ## State Machine
//!
//! Every order follows this lifecycle:
//!
//! ```text
//! PendingNew ──► New ──► PartiallyFilled ──► Filled
//!                 │                              ▲
//!                 ├──► Canceled                  │
//!                 ├──► Rejected                  │
//!                 ├──► Expired                   │
//!                 └──► PendingCancel ──► Canceled
//! ```

#![forbid(unsafe_code)]

pub mod error;
pub mod order_manager;
pub mod paper_trader;
pub mod state_machine;

pub use error::*;
pub use order_manager::*;
pub use paper_trader::*;
pub use state_machine::*;
