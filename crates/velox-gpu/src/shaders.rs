//! WGSL shader module compilation and caching.

use crate::error::GpuError;
use std::cell::RefCell;
use std::collections::HashMap;
use tracing::info;

/// Compiles and caches WGSL shader modules.
///
/// Shaders are identified by name strings. The same name always returns
/// the cached module. Use `load_shader` for dynamic sources and
/// `load_builtin` for shaders embedded at compile time.
///
/// Interior mutability via `RefCell` allows shared access for pipeline
/// creation without requiring `&mut self`.
pub struct ShaderManager {
    device: wgpu::Device,
    cache: RefCell<HashMap<String, wgpu::ShaderModule>>,
}

impl ShaderManager {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            device: device.clone(),
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Load a shader from a WGSL source string. Cached by name.
    pub fn load_shader(&self, name: &str, source: &str) -> Result<(), GpuError> {
        let mut cache = self.cache.borrow_mut();
        if cache.contains_key(name) {
            return Ok(());
        }

        info!("Compiling shader: {}", name);

        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("shader_{}", name)),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        cache.insert(name.to_string(), module);

        info!("Shader '{}' compiled successfully", name);
        Ok(())
    }

    /// Load a built-in shader that was included at compile time via `include_str!`.
    pub fn load_builtin(&self, name: &str) -> Result<(), GpuError> {
        {
            let cache = self.cache.borrow();
            if cache.contains_key(name) {
                return Ok(());
            }
        }

        let source = match name {
            "candle" => include_str!("../shaders/candle.wgsl"),
            "grid" => include_str!("../shaders/grid.wgsl"),
            "volume" => include_str!("../shaders/volume.wgsl"),
            "line" => include_str!("../shaders/line.wgsl"),
            _ => {
                return Err(GpuError::ShaderNotFound {
                    name: name.to_string(),
                    details: String::from("Unknown built-in shader"),
                });
            }
        };

        self.load_shader(name, source)
    }

    /// Get a reference to a cached shader module by name.
    /// Returns None if the shader hasn't been loaded yet.
    pub fn get(&self, name: &str) -> Option<wgpu::ShaderModule> {
        self.cache.borrow().get(name).cloned()
    }

    /// Check if a shader is already cached.
    pub fn is_cached(&self, name: &str) -> bool {
        self.cache.borrow().contains_key(name)
    }

    /// Remove a shader from the cache.
    pub fn evict(&self, name: &str) {
        self.cache.borrow_mut().remove(name);
    }

    /// Clear all cached shaders.
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }

    /// Number of cached shaders.
    pub fn len(&self) -> usize {
        self.cache.borrow().len()
    }

    /// Returns true if no shaders are cached.
    pub fn is_empty(&self) -> bool {
        self.cache.borrow().is_empty()
    }
}
