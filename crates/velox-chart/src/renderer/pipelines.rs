//! wgpu render pipeline builders for chart elements.

use std::mem;
use wgpu;
use velox_gpu::error::GpuError;
use velox_gpu::pipeline::RenderPipelineManager;

use super::types::*;

/// Create a candle body or wick pipeline.
pub fn create_candle_pipeline(
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

/// Create the grid pipeline (uniform-only + vertex buffer).
pub fn create_grid_pipeline(
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

/// Create the volume pipeline.
pub fn create_volume_pipeline(
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

/// Create the line overlay pipeline (uniform-only + vertex buffer).
pub fn create_line_pipeline(
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
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<LineVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 4,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 8,
                        shader_location: 2,
                    },
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
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

/// Create a bind group with uniform + storage buffer bindings.
pub fn create_storage_bind_group(
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
