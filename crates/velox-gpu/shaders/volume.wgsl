// Volume bar shader
// Renders volume bars as quads in the bottom portion of the viewport.

struct Uniforms {
    price_scale: f32,
    price_offset: f32,
    time_scale: f32,
    time_offset: f32,
    candle_width: f32,
    viewport_width: f32,
    viewport_height: f32,
}

struct VolumeData {
    timestamp: f32,
    volume: f32,
    is_up: u32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> volumes: array<VolumeData>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

fn data_to_screen_x(timestamp: f32) -> f32 {
    return (timestamp - uniforms.time_offset) * uniforms.time_scale;
}

// Volume bar quad: 4 vertices, triangle strip
// Vertex layout:
//   0: bottom-left,  1: bottom-right,  2: top-left,  3: top-right
@vertex
fn vs_volume(@builtin(vertex_index) vertex_id: u32, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let vol = volumes[instance_id];
    let x_center = data_to_screen_x(vol.timestamp);
    let half_w = uniforms.candle_width * 0.4; // slightly narrower than candles

    // Volume area is at the bottom 20% of the viewport
    let volume_area_bottom = 0.0;
    let volume_area_height = uniforms.viewport_height * 0.2;

    // Volume data is pre-normalized 0..1 by the CPU; just scale to area height
    let bar_height = vol.volume * volume_area_height;

    // Invert Y so volume grows upward from the bottom
    let bottom_y = volume_area_bottom;
    let top_y = volume_area_bottom + bar_height;

    var pos: vec2<f32>;
    if vertex_id == 0u {
        pos = vec2<f32>(x_center - half_w, bottom_y);
    } else if vertex_id == 1u {
        pos = vec2<f32>(x_center + half_w, bottom_y);
    } else if vertex_id == 2u {
        pos = vec2<f32>(x_center - half_w, top_y);
    } else {
        pos = vec2<f32>(x_center + half_w, top_y);
    }

    let clip_x = 2.0 * pos.x / uniforms.viewport_width - 1.0;
    let clip_y = 2.0 * pos.y / uniforms.viewport_height - 1.0;

    var color: vec3<f32>;
    if vol.is_up == 1u {
        color = vec3<f32>(0.0, 0.6, 0.0);
    } else {
        color = vec3<f32>(0.6, 0.0, 0.0);
    }

    return VertexOutput(vec4<f32>(clip_x, clip_y, 0.0, 1.0), color);
}

@fragment
fn fs_volume(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 0.7);
}
