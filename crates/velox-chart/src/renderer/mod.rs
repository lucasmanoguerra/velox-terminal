//! Chart renderer — sends candle/indicator geometry to GPU.

pub mod types;
pub(crate) mod pipelines;

use std::mem;
use tracing::info;
use velox_core::Candle;
use velox_gpu::device::GpuDevice;
use velox_gpu::error::GpuError;
use velox_gpu::pipeline::RenderPipelineManager;
use velox_gpu::shaders::ShaderManager;
use wgpu;

use crate::interaction::ChartView;
pub use types::{CandleGpuData, ChartUniforms, GridVertex, LineDescriptor, LineVertex, VolumeGpuData};

/// Renders a candlestick chart, grid, volume bars, and indicator overlays
/// using wgpu instanced rendering.
pub struct ChartRenderer {
    // Pipelines
    candle_body_pipeline: wgpu::RenderPipeline,
    candle_wick_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    volume_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    // Candle/volume layout (uniform + storage)
    bind_group_layout: wgpu::BindGroupLayout,
    #[expect(dead_code)]
    pipeline_layout: wgpu::PipelineLayout,
    // Grid-only layout (uniform only)
    grid_bind_group_layout: wgpu::BindGroupLayout,
    #[expect(dead_code)]
    grid_pipeline_layout: wgpu::PipelineLayout,
    // Buffers
    uniform_buffer: wgpu::Buffer,
    candle_buffer: wgpu::Buffer,
    volume_buffer: wgpu::Buffer,
    grid_vertex_buffer: wgpu::Buffer,
    line_vertex_buffer: wgpu::Buffer,
    // Bind groups
    candle_bind_group: wgpu::BindGroup,
    volume_bind_group: wgpu::BindGroup,
    grid_bind_group: wgpu::BindGroup,
    line_bind_group: wgpu::BindGroup,
    // Counts
    num_candles: u32,
    num_volume_bars: u32,
    num_grid_vertices: u32,
    num_line_vertices: u32,
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
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("chart_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: types::BIND_UNIFORMS,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: types::BIND_STORAGE,
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("chart_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Grid uses its own layout: uniform only (no storage binding)
        let grid_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("grid_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: types::BIND_UNIFORMS,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let grid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("grid_pipeline_layout"),
            bind_group_layouts: &[&grid_bind_group_layout],
            push_constant_ranges: &[],
        });

        // ── Buffers ───────────────────────────────────────────────
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("chart_uniforms"),
            size: 256,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let candle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("candle_data"), size: 1024,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let volume_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("volume_data"), size: 1024,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let grid_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid_vertices"), size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let line_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line_vertices"), size: 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Create pipelines ─────────────────────────────────────
        let mut pipeline_manager = RenderPipelineManager::new(device, shader_manager);

        let candle_body_pipeline = pipelines::create_candle_pipeline(
            device, &mut pipeline_manager, &pipeline_layout, format, "vs_candle_body",
        )?;
        let candle_wick_pipeline = pipelines::create_candle_pipeline(
            device, &mut pipeline_manager, &pipeline_layout, format, "vs_candle_wick",
        )?;
        let grid_pipeline = pipelines::create_grid_pipeline(
            device, &mut pipeline_manager, &grid_pipeline_layout, format,
        )?;
        let volume_pipeline = pipelines::create_volume_pipeline(
            device, &mut pipeline_manager, &pipeline_layout, format,
        )?;
        let line_pipeline = pipelines::create_line_pipeline(
            device, &mut pipeline_manager, &grid_pipeline_layout, format,
        )?;

        // ── Create bind groups ───────────────────────────────────
        let candle_bind_group = pipelines::create_storage_bind_group(
            device, &bind_group_layout, &uniform_buffer, &candle_buffer, Some("candle_bg"),
        );
        let volume_bind_group = pipelines::create_storage_bind_group(
            device, &bind_group_layout, &uniform_buffer, &volume_buffer, Some("volume_bg"),
        );

        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid_bg"),
            layout: &grid_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: types::BIND_UNIFORMS,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let line_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("line_bg"),
            layout: &grid_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: types::BIND_UNIFORMS,
                resource: uniform_buffer.as_entire_binding(),
            }],
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
            grid_bind_group_layout,
            grid_pipeline_layout,
            uniform_buffer,
            candle_buffer,
            volume_buffer,
            grid_vertex_buffer,
            line_vertex_buffer,
            candle_bind_group,
            volume_bind_group,
            grid_bind_group,
            line_bind_group,
            num_candles: 0,
            num_volume_bars: 0,
            num_grid_vertices: 0,
            num_line_vertices: 0,
            device: device.clone(),
            queue: queue.clone(),
        })
    }

    // ── Data updates ──────────────────────────────────────────────

    /// Update candle data from a slice of `Candle`.
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

        self.queue
            .write_buffer(&self.candle_buffer, 0, bytemuck::cast_slice(&gpu_data));

        self.candle_bind_group = pipelines::create_storage_bind_group(
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

        let max_vol = candles.iter().map(|c| c.volume).fold(0.0_f64, |a, b| a.max(b));

        let gpu_data: Vec<VolumeGpuData> = candles
            .iter()
            .map(|c| VolumeGpuData {
                timestamp: c.timestamp.timestamp() as f32,
                volume: (c.volume / max_vol) as f32,
                is_up: if c.is_bullish() { 1 } else { 0 },
                _padding: 0,
            })
            .collect();

        let data_size = (gpu_data.len() * mem::size_of::<VolumeGpuData>()) as u64;
        Self::ensure_buffer_size(&mut self.volume_buffer, &self.device, "volume_data", data_size);
        self.queue
            .write_buffer(&self.volume_buffer, 0, bytemuck::cast_slice(&gpu_data));

        self.volume_bind_group = pipelines::create_storage_bind_group(
            &self.device,
            &self.bind_group_layout,
            &self.uniform_buffer,
            &self.volume_buffer,
            Some("volume_bg"),
        );

        self.num_volume_bars = gpu_data.len() as u32;
    }

    /// Update grid lines (horizontal price levels, vertical time levels).
    pub fn update_grid(&mut self, price_levels: &[f32], time_levels: &[f32]) {
        let mut vertices = Vec::new();

        for &price in price_levels {
            vertices.push(GridVertex { x: 0.0, y: price });
            vertices.push(GridVertex { x: 1.0, y: price });
        }

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
        self.queue
            .write_buffer(&self.grid_vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        self.grid_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("grid_bg"),
            layout: &self.grid_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: types::BIND_UNIFORMS,
                resource: self.uniform_buffer.as_entire_binding(),
            }],
        });

        self.num_grid_vertices = vertices.len() as u32;
    }

    /// Update line vertex data for indicator overlays.
    pub fn update_lines(&mut self, overlay_data: &[LineDescriptor]) {
        let mut vertices: Vec<LineVertex> = Vec::new();

        for (_name, values, color) in overlay_data {
            let mut segment: Vec<LineVertex> = Vec::new();

            for &(ts, val) in values {
                if val.is_nan() {
                    Self::flush_line_segment(&mut vertices, &segment);
                    segment.clear();
                } else {
                    segment.push(LineVertex {
                        timestamp: ts as f32,
                        price: val as f32,
                        r: color.0,
                        g: color.1,
                        b: color.2,
                    });
                }
            }
            Self::flush_line_segment(&mut vertices, &segment);
        }

        if vertices.is_empty() {
            self.num_line_vertices = 0;
            return;
        }

        let data_size = (vertices.len() * mem::size_of::<LineVertex>()) as u64;

        if self.line_vertex_buffer.size() < data_size {
            let new_size = data_size.next_power_of_two();
            self.line_vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("line_vertices"),
                size: new_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        self.queue
            .write_buffer(&self.line_vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        self.line_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("line_bg"),
            layout: &self.grid_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: types::BIND_UNIFORMS,
                resource: self.uniform_buffer.as_entire_binding(),
            }],
        });

        self.num_line_vertices = vertices.len() as u32;
    }

    /// Flush a segment of connected points into the vertex buffer as LineList pairs.
    fn flush_line_segment(out: &mut Vec<LineVertex>, segment: &[LineVertex]) {
        if segment.len() < 2 {
            return;
        }
        for i in 0..segment.len() - 1 {
            out.push(segment[i]);
            out.push(segment[i + 1]);
        }
    }

    /// Update uniform buffer with viewport transform.
    pub fn update_uniforms(&self, uniforms: &ChartUniforms) {
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    // ── Rendering ─────────────────────────────────────────────────

    /// Render the chart into a render pass.
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
            pass.draw(0..2, 0..self.num_candles);
        }

        // 3. Candle bodies
        if self.num_candles > 0 {
            pass.set_pipeline(&self.candle_body_pipeline);
            pass.set_bind_group(0, &self.candle_bind_group, &[]);
            pass.draw(0..4, 0..self.num_candles);
        }

        // 4. Volume bars
        if self.num_volume_bars > 0 {
            pass.set_pipeline(&self.volume_pipeline);
            pass.set_bind_group(0, &self.volume_bind_group, &[]);
            pass.draw(0..4, 0..self.num_volume_bars);
        }

        // 5. Indicator overlay lines
        if self.num_line_vertices > 0 {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_bind_group(0, &self.line_bind_group, &[]);
            pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            pass.draw(0..self.num_line_vertices, 0..1);
        }
    }

    // ── Helpers ───────────────────────────────────────────────────

    /// Update all GPU data from application state.
    pub fn update_from_state(
        &mut self,
        candles: &[Candle],
        view: &ChartView,
        phys_width: f32,
        phys_height: f32,
    ) {
        self.update_candles(candles);
        self.update_volume(candles);

        let price_range = view.price_range();
        let price_step = Self::nice_step(price_range / 10.0);
        let mut price_levels = Vec::new();
        let mut p = (view.price_min / price_step).floor() * price_step;
        while p <= view.price_max {
            price_levels.push(p as f32);
            p += price_step;
        }

        let time_range = view.time_range();
        let time_step = Self::nice_step(time_range / 8.0).max(1.0);
        let mut time_levels = Vec::new();
        let mut t = (view.time_start / time_step).floor() * time_step;
        while t <= view.time_end {
            time_levels.push(t as f32);
            t += time_step;
        }
        self.update_grid(&price_levels, &time_levels);

        let num_candles = candles.len().max(1) as f32;
        let uniforms = ChartUniforms {
            price_scale: phys_height / view.price_range() as f32,
            price_offset: view.price_min as f32,
            time_scale: phys_width / view.time_range() as f32,
            time_offset: view.time_start as f32,
            candle_width: (phys_width / num_candles) * 0.6,
            viewport_width: phys_width,
            viewport_height: phys_height,
        };
        self.update_uniforms(&uniforms);
    }

    /// Compute a "nice" round step value.
    fn nice_step(step: f64) -> f64 {
        let mag = 10.0_f64.powf(step.abs().log10().floor());
        let normalized = step / mag;
        if normalized < 1.5 {
            mag
        } else if normalized < 3.5 {
            2.0 * mag
        } else if normalized < 7.5 {
            5.0 * mag
        } else {
            10.0 * mag
        }
    }

    /// Resize a buffer if it's too small for the required data size.
    fn ensure_buffer_size(
        buffer: &mut wgpu::Buffer,
        device: &wgpu::Device,
        label: &str,
        required_size: u64,
    ) {
        if buffer.size() >= required_size {
            return;
        }
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
