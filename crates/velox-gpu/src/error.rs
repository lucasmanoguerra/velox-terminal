//! GPU error types.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("No suitable GPU adapter found")]
    NoAdapter,

    #[error("Failed to request device: {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),

    #[error("Shader '{name}' not found: {details}")]
    ShaderNotFound { name: String, details: String },

    #[error("Pipeline creation failed for '{name}': {details}")]
    PipelineCreation { name: String, details: String },

    #[error("Surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),

    #[error("No surface texture available")]
    NoSurfaceTexture,

    #[error("Create surface error: {0}")]
    CreateSurface(String),
}
