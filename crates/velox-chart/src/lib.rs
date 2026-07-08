//! # velox-chart
//!
//! Charting engine with wgpu GPU rendering.
//!
//! Renders candlestick charts, line indicators, volume bars, and overlays
//! using instanced geometry and WGSL shaders.
//!
//! # Architecture
//!
//! - `ChartRenderer` — GPU pipeline for candlestick charts, grid, and volume
//! - `ChartInteraction` — zoom/pan state machine
//! - `OverlayManager` — indicator overlay management
//! - `ChartView` — visible price/time range

pub mod interaction;
pub mod overlay;
pub mod renderer;

pub use interaction::{ChartInteraction, ChartView};
pub use overlay::OverlayManager;
pub use renderer::{
    CandleGpuData, ChartRenderer, ChartUniforms, GridVertex, LineDescriptor, LineVertex,
    VolumeGpuData,
};

/// Initialize the charting engine.
pub fn init() {
    tracing::info!("velox-chart initialized");
}
