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

pub mod renderer;
pub mod overlay;
pub mod interaction;

pub use renderer::{
    ChartRenderer, ChartUniforms, CandleGpuData, VolumeGpuData,
    GridVertex, LinePointGpu, IndicatorOverlay,
};
pub use interaction::{ChartInteraction, ChartView};
pub use overlay::OverlayManager;

/// Initialize the charting engine.
pub fn init() {
    tracing::info!("velox-chart initialized");
}
