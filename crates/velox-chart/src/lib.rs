//! # velox-chart
//!
//! Charting engine with wgpu GPU rendering.
//!
//! Renders candlestick charts, line indicators, volume bars, and overlays
//! using instanced geometry and WGSL shaders.

pub mod renderer;
pub mod overlay;
pub mod interaction;

/// Placeholder for charting implementation.
/// Full implementation in Phase 4.
pub fn init() {
    tracing::info!("velox-chart initialized");
}
