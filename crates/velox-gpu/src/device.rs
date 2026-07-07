//! wgpu device and surface management.

use crate::error::GpuError;
use tracing::{info, warn};

/// Available GPU backends for the current platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Vulkan,
    Metal,
    DirectX12,
    OpenGl,
    WebGpu,
}

impl Backend {
    pub fn name(&self) -> &'static str {
        match self {
            Backend::Vulkan => "Vulkan",
            Backend::Metal => "Metal",
            Backend::DirectX12 => "DirectX 12",
            Backend::OpenGl => "OpenGL",
            Backend::WebGpu => "WebGPU",
        }
    }
}

/// Manages the wgpu instance, adapter, device, and queue.
///
/// This is the foundation for all GPU operations. Does NOT own a surface;
/// surfaces are created separately from window handles so this struct
/// remains decoupled from any specific windowing system.
pub struct GpuDevice {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub backend: Backend,
}

impl GpuDevice {
    /// Create a new GPU device with a default instance.
    ///
    /// This is the simplest constructor. Use `with_instance` or `from_parts`
    /// if you need to customize the instance or provide an existing setup.
    pub async fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::default();
        Self::with_instance(instance).await
    }

    /// Create a GPU device using a custom instance.
    pub async fn with_instance(instance: wgpu::Instance) -> Result<Self, GpuError> {
        info!("Requesting high-performance GPU adapter...");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuError::NoAdapter)?;

        let backend = detect_backend(&adapter);
        info!(
            "Selected adapter: {} ({})",
            adapter.get_info().name,
            backend.name()
        );

        let adapter_info = adapter.get_info();
        if adapter_info.device_type == wgpu::DeviceType::Cpu {
            warn!("Using CPU-based adapter — performance will be limited");
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("velox-gpu-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await?;

        info!("GPU device initialized successfully");

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            backend,
        })
    }

    /// Recreate the device for a different surface (e.g., after window changes).
    /// This is a lightweight operation that just reuses existing adapter.
    pub async fn recreate_device(&self) -> Result<(wgpu::Device, wgpu::Queue), GpuError> {
        let (device, queue) = self
            .adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("velox-gpu-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await?;

        Ok((device, queue))
    }
}

/// Detect the GPU backend from the adapter info.
fn detect_backend(adapter: &wgpu::Adapter) -> Backend {
    let info = adapter.get_info();
    match info.backend {
        wgpu::Backend::Vulkan => Backend::Vulkan,
        wgpu::Backend::Metal => Backend::Metal,
        wgpu::Backend::Dx12 => Backend::DirectX12,
        wgpu::Backend::Gl => Backend::OpenGl,
        wgpu::Backend::BrowserWebGpu => Backend::WebGpu,
        _ => Backend::Vulkan,
    }
}
