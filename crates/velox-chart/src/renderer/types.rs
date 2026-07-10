//! GPU data structures matching WGSL shader layouts.

use bytemuck::{Pod, Zeroable};

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

/// Line vertex with per-vertex color (matches line.wgsl vertex input).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LineVertex {
    pub timestamp: f32,
    pub price: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// A descriptor for one indicator line overlay.
/// `(name, [(timestamp_unix, value)], (r, g, b) color)`.
pub type LineDescriptor = (String, Vec<(f64, f64)>, (f32, f32, f32));

// ── Bind group layout indices ──────────────────────────────────────

pub const BIND_UNIFORMS: u32 = 0;
pub const BIND_STORAGE: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_uniforms_size() {
        assert_eq!(std::mem::size_of::<ChartUniforms>(), 28);
    }

    #[test]
    fn test_candle_gpu_data_size() {
        assert_eq!(std::mem::size_of::<CandleGpuData>(), 28);
    }

    #[test]
    fn test_volume_gpu_data_size() {
        assert_eq!(std::mem::size_of::<VolumeGpuData>(), 16);
    }

    #[test]
    fn test_grid_vertex_size() {
        assert_eq!(std::mem::size_of::<GridVertex>(), 8);
    }

    #[test]
    fn test_line_vertex_size() {
        assert_eq!(std::mem::size_of::<LineVertex>(), 20);
    }
}
