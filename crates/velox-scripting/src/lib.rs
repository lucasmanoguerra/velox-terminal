//! # velox-scripting
//!
//! User scripting engine for algorithmic trading strategies.
//!
//! Supports Lua (via mlua) with sandboxed execution environment.
//! Resource limits: CPU time, memory, call depth, execution timeout.

pub mod api;
pub mod engine;
pub mod sandbox;

/// Placeholder for scripting implementation.
/// Full implementation in Phase 6.
pub fn init() {
    tracing::info!("velox-scripting initialized");
}
