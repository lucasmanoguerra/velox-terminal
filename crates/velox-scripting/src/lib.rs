//! # velox-scripting
//!
//! User scripting engine for algorithmic trading strategies.
//!
//! Supports Lua (via mlua) with sandboxed execution environment.
//! Resource limits: CPU time, memory, call depth, execution timeout.

pub mod engine;
pub mod sandbox;
pub mod api;

/// Placeholder for scripting implementation.
/// Full implementation in Phase 6.
pub fn init() {
    tracing::info!("velox-scripting initialized");
}
