//! # velox-gpu
//!
//! GPU rendering primitives using wgpu.
//!
//! Provides the foundation for chart rendering, text rendering via glyphon,
//! and GPU-accelerated UI. This crate manages the wgpu instance, adapter,
//! device, shader compilation, and render pipeline creation.
//!
//! # Architecture
//!
//! - `GpuDevice` — wgpu instance, adapter, device, and queue
//! - `ShaderManager` — compile and cache WGSL shaders
//! - `RenderPipelineManager` — create and cache render pipelines
//! - `GpuError` — unified error type
//!
//! # Example
//!
//! ```rust,no_run
//! use velox_gpu::device::GpuDevice;
//!
//! # async fn example() {
//! let gpu = GpuDevice::new().await.unwrap();
//! // Use gpu.device, gpu.queue for rendering...
//! # }
//! ```

pub mod device;
pub mod error;
pub mod shaders;
pub mod pipeline;

pub use device::GpuDevice;
pub use error::GpuError;
pub use shaders::ShaderManager;
pub use pipeline::{RenderPipelineManager, PipelineDesc};
