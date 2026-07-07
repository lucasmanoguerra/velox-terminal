//! Chart renderer — sends candle/indicator geometry to GPU.

use std::mem;
use std::num::NonZeroU64;
use wgpu;
use bytemuck::{Pod, Zeroable};
use velox_core::Candle;
use velox_gpu::device::GpuDevice;
use velox_gpu::error::GpuError;
use velox_gpu::pipeline::RenderPipelineManager;
use velox_gpu::shaders::ShaderManager;
use tracing::info;

// ── Data structures matching WGSL shaders ──────────────────────────

/// Uniform buffer contents (matches `Uniforms` in candle.wgsl).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ChartUniforms {
    pub price_scale: f32,
    pub price_offset: f32,
    pub time_scale: f32,
    pub time_offset: f32,
    pub candle_width: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

/// Per-candle instance data (matches `CandleData` in candle.wgsl).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CandleGpuData {
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    pub timestamp: f32,
    pub is_bullish: u32,
    pub _padding: u32,
}

/// Per-volume-bar instance data (matches `VolumeData` in volume.wgsl).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VolumeGpuData {
    pub timestamp: f32,
    pub volume: f32,
    pub is_up: u32,
    pub _padding: u32,
}

/// Grid line vertex (matches grid shader).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GridVertex {
    pub x: f32,
    pub y: f32,
}

/// Line overlay point (matches line.wgsl).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LinePointGpu {
    pub timestamp: f32,
    pub price: f32,
}

// ── Bind group layout indices ──────────────────────────────────────

const BIND_UNIFORMS: u32 = 0;
const BIND_STORAGE: u32 = 1;

/// Renders a candlestick chart, grid, volume bars, and indicator overlays
/// using wgpu instanced rendering.
pub struct ChartRenderer {
    // Pipelines
    candle_body_pipeline: wgpu::RenderPipeline,
    candle_wick_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    volume_pipeline: wgpu::RenderPipeline,
    #[expect(dead_code)]
    line_pipeline: wgpu::RenderPipeline,
    // Pipeline layout (shared across all pipelines that use uniform + storage)
    bind_group_layout: wgpu::BindGroupLayout,
    #[expect(dead_code)]
    pipeline_layout: wgpu::PipelineLayout,
    // Buffers
    uniform_buffer: wgpu::Buffer,
    candle_buffer: wgpu::Buffer,
    volume_buffer: wgpu::Buffer,
    grid_vertex_buffer: wgpu::Buffer,
    // Bind groups
    candle_bind_group: wgpu::BindGroup,
    volume_bind_group: wgpu::BindGroup,
    grid_bind_group: wgpu::BindGroup,
    // Counts
    num_candles: u32,
    num_volume_bars: u32,
    num_grid_vertices: u32,
    // Device handle for buffer operations
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl ChartRenderer {
    /// Create a new chart renderer with all necessary pipelines.
    pub fn new(gpu: &GpuDevice, format: wgpu::TextureFormat) -> Result<Self, GpuError> {
        let device = &gpu.device;
        let queue = &gpu.queue;

        // ── Shader manager ────────────────────────────────────────
        let shader_manager = ShaderManager::new(device);

        // ── Bind group layout ─────────────────────────────────────
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("chart_bind_group_layout"),
                entries: &[
                    // Binding 0: Uniform buffer (transforms, viewport)
                    wgpu::BindGroupLayoutEntry {
                        binding: BIND_UNIFORMS,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(mem::size_of::<ChartUniforms>() as u64),
                        },
                        count: None,
                    },
                    // Binding 1: Storage buffer (instance data)
                    wgpu::BindGroupLayoutEntry {
                        binding: BIND_STORAGE,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("chart_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        // ── Buffers ───────────────────────────────────────────────
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chart_uniforms"),
            size: mem::size_of::<ChartUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Initial empty buffers (will be resized on first update)
        let candle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("candle_data"),
            size: 1024, // Start small, grow as needed
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let volume_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("volume_data"),
            size: 1024,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid_vertices"),
            size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Create pipelines ─────────────────────────────────────
        let mut pipeline_manager = RenderPipelineManager::new(device, shader_manager);

        // Candle body pipeline (triangle list, 4 vertices per instance)
        let candle_body_pipeline = Self::create_candle_pipeline(
            device,
            &mut pipeline_manager,
            &pipeline_layout,
            format,
            "vs_candle_body",
        )?;

        // Candle wick pipeline (line list, 2 vertices per instance)
        let candle_wick_pipeline = {
            Self::create_candle_pipeline(
                device,
                &mut pipeline_manager,
                &pipeline_layout,
                format,
                "vs_candle_wick",
            )?
        };

        // Grid pipeline
        let grid_pipeline = Self::create_grid_pipeline(
            device,
            &mut pipeline_manager,
            &pipeline_layout,
            format,
        )?;

        // Volume pipeline
        let volume_pipeline = Self::create_volume_pipeline(
            device,
            &mut pipeline_manager,
            &pipeline_layout,
            format,
        )?;

        // Line overlay pipeline
        let line_pipeline = Self::create_line_pipeline(
            device,
            &mut pipeline_manager,
            &pipeline_layout,
            format,
        )?;

        // ── Create bind groups ───────────────────────────────────
        let candle_bind_group = Self::create_storage_bind_group(
            device,
            &bind_group_layout,
            &uniform_buffer,
            &candle_buffer,
            Some("candle_bg"),
        );

        let volume_bind_group = Self::create_storage_bind_group(
            device,
            &bind_group_layout,
            &uniform_buffer,
            &volume_buffer,
            Some("volume_bg"),
        );

        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid_bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BIND_UNIFORMS,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BIND_STORAGE,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &grid_vertex_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        info!("ChartRenderer initialized");
        Ok(Self {
            candle_body_pipeline,
            candle_wick_pipeline,
            grid_pipeline,
            volume_pipeline,
            line_pipeline,
            bind_group_layout,
            pipeline_layout,
            uniform_buffer,
            candle_buffer,
            volume_buffer,
            grid_vertex_buffer,
            candle_bind_group,
            volume_bind_group,
            grid_bind_group,
            num_candles: 0,
            num_volume_bars: 0,
            num_grid_vertices: 0,
            device: device.clone(),
            queue: queue.clone(),
        })
    }

    // ── Pipeline builders ────────────────────────────────────────

    fn create_candle_pipeline(
        device: &wgpu::Device,
        pm: &mut RenderPipelineManager,
        layout: &wgpu::PipelineLayout,
        format: wgpu::TextureFormat,
        entry_point: &str,
    ) -> Result<wgpu::RenderPipeline, GpuError> {
        pm.shaders().load_builtin("candle")?;
        let vs_module = pm.shaders().get("candle").unwrap();
        let fs_module = pm.shaders().get("candle").unwrap();

        let is_body = entry_point == "vs_candle_body";
        let primitive = if is_body {
            wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            }
        } else {
            wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            }
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(entry_point),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: Some("fs_candle"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive,
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(pipeline)
    }

    fn create_grid_pipeline(
        device: &wgpu::Device,
        pm: &mut RenderPipelineManager,
        layout: &wgpu::PipelineLayout,
        format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, GpuError> {
        pm.shaders().load_builtin("grid")?;
        let module = pm.shaders().get("grid").unwrap();

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grid_pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_grid"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<GridVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_grid"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(pipeline)
    }

    fn create_volume_pipeline(
        device: &wgpu::Device,
        pm: &mut RenderPipelineManager,
        layout: &wgpu::PipelineLayout,
        format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, GpuError> {
        pm.shaders().load_builtin("volume")?;
        let module = pm.shaders().get("volume").unwrap();

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("volume_pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_volume"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_volume"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(pipeline)
    }

    fn create_line_pipeline(
        device: &wgpu::Device,
        pm: &mut RenderPipelineManager,
        layout: &wgpu::PipelineLayout,
        format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, GpuError> {
        pm.shaders().load_builtin("line")?;
        let module = pm.shaders().get("line").unwrap();

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line_pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_line"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_line"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(pipeline)
    }

    // ── Bind group helpers ────────────────────────────────────────

    fn create_storage_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform: &wgpu::Buffer,
        storage: &wgpu::Buffer,
        label: Option<&str>,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BIND_UNIFORMS,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BIND_STORAGE,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: storage,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        })
    }

    // ── Data updates ──────────────────────────────────────────────

    /// Update candle data from a slice of `Candle`.
    ///
    /// Converts OHLCV data to GPU-friendly `f32` and uploads to the storage buffer.
    /// Automatically resizes the GPU buffer if needed.
    pub fn update_candles(&mut self, candles: &[Candle]) {
        if candles.is_empty() {
            self.num_candles = 0;
            return;
        }

        let gpu_data: Vec<CandleGpuData> = candles
            .iter()
            .map(|c| {
                let ts = c.timestamp.timestamp() as f32;
                CandleGpuData {
                    open: c.open as f32,
                    high: c.high as f32,
                    low: c.low as f32,
                    close: c.close as f32,
                    timestamp: ts,
                    is_bullish: if c.is_bullish() { 1 } else { 0 },
                    _padding: 0,
                }
            })
            .collect();

        let data_size = (gpu_data.len() * mem::size_of::<CandleGpuData>()) as u64;
        Self::ensure_buffer_size(&mut self.candle_buffer, &self.device, "candle_data", data_size);

        self.queue.write_buffer(&self.candle_buffer, 0, bytemuck::cast_slice(&gpu_data));

        // Recreate bind group with updated buffer
        self.candle_bind_group = Self::create_storage_bind_group(
            &self.device,
            &self.bind_group_layout,
            &self.uniform_buffer,
            &self.candle_buffer,
            Some("candle_bg"),
        );

        self.num_candles = gpu_data.len() as u32;
    }

    /// Update volume bar data.
    pub fn update_volume(&mut self, candles: &[Candle]) {
        if candles.is_empty() {
            self.num_volume_bars = 0;
            return;
        }

        let max_vol = candles
            .iter()
            .map(|c| c.volume)
            .fold(0.0_f64, |a, b| a.max(b));

        let gpu_data: Vec<VolumeGpuData> = candles
            .iter()
            .map(|c| {
                VolumeGpuData {
                    timestamp: c.timestamp.timestamp() as f32,
                    volume: (c.volume / max_vol) as f32,
                    is_up: if c.is_bullish() { 1 } else { 0 },
                    _padding: 0,
                }
            })
            .collect();

        let data_size = (gpu_data.len() * mem::size_of::<VolumeGpuData>()) as u64;
        Self::ensure_buffer_size(&mut self.volume_buffer, &self.device, "volume_data", data_size);
        self.queue.write_buffer(&self.volume_buffer, 0, bytemuck::cast_slice(&gpu_data));

        self.volume_bind_group = Self::create_storage_bind_group(
            &self.device,
            &self.bind_group_layout,
            &self.uniform_buffer,
            &self.volume_buffer,
            Some("volume_bg"),
        );

        self.num_volume_bars = gpu_data.len() as u32;
    }

    /// Update grid lines (horizontal price levels, vertical time levels).
    pub fn update_grid(
        &mut self,
        price_levels: &[f32],
        time_levels: &[f32],
    ) {
        let mut vertices = Vec::new();

        // Horizontal lines (price levels): (0, price) → (viewport_width, price)
        for &price in price_levels {
            vertices.push(GridVertex { x: 0.0, y: price });
            vertices.push(GridVertex { x: 1.0, y: price });
        }

        // Vertical lines (time levels): (timestamp, 0) → (timestamp, viewport_height)
        for &ts in time_levels {
            vertices.push(GridVertex { x: ts, y: 0.0 });
            vertices.push(GridVertex { x: ts, y: 1.0 });
        }

        if vertices.is_empty() {
            self.num_grid_vertices = 0;
            return;
        }

        let data_size = (vertices.len() * mem::size_of::<GridVertex>()) as u64;
        Self::ensure_buffer_size(&mut self.grid_vertex_buffer, &self.device, "grid_vertices", data_size);
        self.queue.write_buffer(
            &self.grid_vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices),
        );

        // Rebuild grid bind group
        self.grid_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BIND_UNIFORMS,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: BIND_STORAGE,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.grid_vertex_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        self.num_grid_vertices = vertices.len() as u32;
    }

    /// Update uniform buffer with viewport transform.
    pub fn update_uniforms(&self, uniforms: &ChartUniforms) {
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(uniforms),
        );
    }

    // ── Rendering ─────────────────────────────────────────────────

    /// Render the chart into a render pass.
    ///
    /// Call this between `render_pass.begin()` and `render_pass.end()`.
    /// The render pass must target a texture with the format used at construction.
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        // 1. Grid (behind candles)
        if self.num_grid_vertices > 0 {
            pass.set_pipeline(&self.grid_pipeline);
            pass.set_bind_group(0, &self.grid_bind_group, &[]);
            pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
            pass.draw(0..self.num_grid_vertices, 0..1);
        }

        // 2. Candle wicks
        if self.num_candles > 0 {
            pass.set_pipeline(&self.candle_wick_pipeline);
            pass.set_bind_group(0, &self.candle_bind_group, &[]);
            // 2 vertices per instance (line list: low→high)
            pass.draw(0..2, 0..self.num_candles);
        }

        // 3. Candle bodies
        if self.num_candles > 0 {
            pass.set_pipeline(&self.candle_body_pipeline);
            pass.set_bind_group(0, &self.candle_bind_group, &[]);
            // 4 vertices per instance (triangle strip quad)
            pass.draw(0..4, 0..self.num_candles);
        }

        // 4. Volume bars
        if self.num_volume_bars > 0 {
            pass.set_pipeline(&self.volume_pipeline);
            pass.set_bind_group(0, &self.volume_bind_group, &[]);
            pass.draw(0..4, 0..self.num_volume_bars);
        }
    }

    // ── Helpers ───────────────────────────────────────────────────

    /// Resize a buffer if it's too small for the required data size.
    fn ensure_buffer_size(buffer: &mut wgpu::Buffer, device: &wgpu::Device, label: &str, required_size: u64) {
        if buffer.size() >= required_size {
            return;
        }
        // Round up to next power of two for amortized growth
        let new_size = required_size.next_power_of_two();
        *buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: new_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }
}

// ── Default indicator overlay support ──────────────────────────────

/// Trait for rendering an indicator overlay (SMA, EMA, RSI, etc.).
pub trait IndicatorOverlay: Send + Sync {
    fn name(&self) -> &str;
    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, uniforms: &ChartUniforms);
}
