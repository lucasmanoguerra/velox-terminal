// Line overlay shader v2 — vertex-buffer-based with per-vertex colors.
// Renders polyline indicator overlays (SMA, EMA, RSI, etc.).
// Uses LineList topology (each pair of vertices = one segment).
// NaN values split the line into separate segments (handled CPU-side).
//
// Uniforms struct MUST match ChartUniforms field order (7 × f32):
//   price_scale, price_offset, time_scale, time_offset,
//   candle_width, viewport_width, viewport_height

struct Uniforms {
    price_scale: f32,
    price_offset: f32,
    time_scale: f32,
    time_offset: f32,
    candle_width: f32,
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) timestamp: f32,
    @location(1) price: f32,
    @location(2) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    // Convert data coordinates to screen space
    let sx = (in.timestamp - uniforms.time_offset) * uniforms.time_scale;
    let sy = (in.price - uniforms.price_offset) * uniforms.price_scale;
    // Convert screen space to clip space
    let clip_x = 2.0 * sx / uniforms.viewport_width - 1.0;
    let clip_y = 2.0 * sy / uniforms.viewport_height - 1.0;
    return VertexOutput(vec4<f32>(clip_x, clip_y, 0.0, 1.0), in.color);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
