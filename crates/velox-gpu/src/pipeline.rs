//! Render pipeline creation and caching.

use std::collections::HashMap;
use wgpu;
use crate::error::GpuError;
use crate::shaders::ShaderManager;
use tracing::info;

/// Describes a render pipeline for creation.
pub struct PipelineDesc<'a> {
    /// Vertex shader name (must be loaded in ShaderManager).
    pub vs_name: String,
    /// Fragment shader name (must be loaded in ShaderManager).
    pub fs_name: String,
    /// Vertex buffer layouts for the pipeline.
    pub vertex_layouts: Vec<wgpu::VertexBufferLayout<'a>>,
    /// Primitive topology and state.
    pub primitive: wgpu::PrimitiveState,
    /// Fragment blend state.
    pub blend: wgpu::BlendState,
    /// Optional depth/stencil state.
    pub depth_stencil: Option<wgpu::DepthStencilState>,
    /// Multisample state.
    pub multisample: wgpu::MultisampleState,
}

impl Default for PipelineDesc<'_> {
    fn default() -> Self {
        Self {
            vs_name: String::new(),
            fs_name: String::new(),
            vertex_layouts: Vec::new(),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            blend: wgpu::BlendState::REPLACE,
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        }
    }
}

impl PipelineDesc<'_> {
    /// Create a desc for a standard triangle-list pipeline with alpha blending.
    pub fn transparent(vs: &str, fs: &str) -> Self {
        Self {
            vs_name: vs.to_string(),
            fs_name: fs.to_string(),
            blend: wgpu::BlendState::ALPHA_BLENDING,
            ..Default::default()
        }
    }

    /// Create a desc for a line-list pipeline.
    pub fn lines(vs: &str, fs: &str) -> Self {
        Self {
            vs_name: vs.to_string(),
            fs_name: fs.to_string(),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

/// Manages creation and caching of wgpu render pipelines.
///
/// Pipelines are identified by string names. Construction requires
/// a `PipelineDesc` and a `PipelineLayout`. The resulting pipeline
/// is cached and reused on subsequent requests with the same name.
pub struct RenderPipelineManager {
    device: wgpu::Device,
    pipelines: HashMap<String, wgpu::RenderPipeline>,
    shader_manager: ShaderManager,
}

impl RenderPipelineManager {
    pub fn new(device: &wgpu::Device, shader_manager: ShaderManager) -> Self {
        Self {
            device: device.clone(),
            pipelines: HashMap::new(),
            shader_manager,
        }
    }

    /// Get or create a render pipeline.
    ///
    /// `format` is the target surface texture format.
    pub fn get_or_create(
        &mut self,
        name: &str,
        desc: &PipelineDesc,
        layout: &wgpu::PipelineLayout,
        format: wgpu::TextureFormat,
    ) -> Result<&wgpu::RenderPipeline, GpuError> {
        if self.pipelines.contains_key(name) {
            return Ok(self.pipelines.get(name).unwrap());
        }

        info!("Creating render pipeline: {}", name);

        self.shader_manager.load_builtin(&desc.vs_name)?;
        self.shader_manager.load_builtin(&desc.fs_name)?;

        let vs_module = self.shader_manager.get(&desc.vs_name)
            .ok_or_else(|| GpuError::PipelineCreation {
                name: name.to_string(),
                details: format!("Vertex shader '{}' not found after loading", desc.vs_name),
            })?;
        let fs_module = self.shader_manager.get(&desc.fs_name)
            .ok_or_else(|| GpuError::PipelineCreation {
                name: name.to_string(),
                details: format!("Fragment shader '{}' not found after loading", desc.fs_name),
            })?;

        let pipeline_desc = wgpu::RenderPipelineDescriptor {
            label: Some(&format!("pipeline_{}", name)),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &desc.vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(desc.blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: desc.primitive,
            depth_stencil: desc.depth_stencil.clone(),
            multisample: desc.multisample,
            multiview: None,
            cache: None,
        };

        let pipeline = self.device.create_render_pipeline(&pipeline_desc);
        self.pipelines.insert(name.to_string(), pipeline);

        info!("Pipeline '{}' created", name);
        Ok(self.pipelines.get(name).unwrap())
    }

    /// Access the shader manager.
    pub fn shaders(&self) -> &ShaderManager {
        &self.shader_manager
    }

    /// Check if a pipeline is cached.
    pub fn has_pipeline(&self, name: &str) -> bool {
        self.pipelines.contains_key(name)
    }

    /// Remove a pipeline from the cache (triggers recreation on next use).
    pub fn evict(&mut self, name: &str) {
        self.pipelines.remove(name);
    }

    /// Clear all cached pipelines (useful after device recreation).
    pub fn clear(&mut self) {
        self.pipelines.clear();
    }
}
