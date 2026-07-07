//! # velox-gpu
//!
//! GPU rendering primitives using wgpu.
//!
//! Provides the foundation for chart rendering, text rendering via glyphon,
//! and GPU-accelerated UI.

pub mod device;
pub mod shaders;
pub mod pipeline;

/// Placeholder for GPU implementation.
/// Full implementation in Phase 4.
pub fn init() {
    tracing::info!("velox-gpu initialized");
}
