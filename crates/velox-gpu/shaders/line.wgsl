// Line overlay shader
// Renders polyline indicator overlays (SMA, EMA, RSI, etc.)
// Each line is a set of connected segments using LineList or LineStrip.

struct Uniforms {
    viewport_width: f32,
    viewport_height: f32,
    price_scale: f32,
    price_offset: f32,
    time_scale: f32,
    time_offset: f32,
    line_width: f32,
    color: vec3<f32>,
}

struct LinePoint {
    timestamp: f32,
    price: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> points: array<LinePoint>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

fn data_to_screen_x(timestamp: f32) -> f32 {
    return (timestamp - uniforms.time_offset) * uniforms.time_scale;
}

fn data_to_screen_y(price: f32) -> f32 {
    return (price - uniforms.price_offset) * uniforms.price_scale;
}

fn screen_to_clip(pos: vec2<f32>) -> vec4<f32> {
    let clip_x = 2.0 * pos.x / uniforms.viewport_width - 1.0;
    let clip_y = 2.0 * pos.y / uniforms.viewport_height - 1.0;
    return vec4<f32>(clip_x, clip_y, 0.0, 1.0);
}

// Each line segment uses 2 vertices (line list)
@vertex
fn vs_line(@builtin(vertex_index) vertex_id: u32, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let idx = instance_id * 2u + vertex_id;
    if idx >= arrayLength(&points) {
        return VertexOutput(vec4<f32>(0.0, 0.0, 0.0, 1.0), uniforms.color);
    }
    let pt = points[idx];
    let sx = data_to_screen_x(pt.timestamp);
    let sy = data_to_screen_y(pt.price);
    let clip = screen_to_clip(vec2<f32>(sx, sy));
    return VertexOutput(clip, uniforms.color);
}

@fragment
fn fs_line(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
