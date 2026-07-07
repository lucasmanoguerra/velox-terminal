// Grid line shader
// Renders horizontal and vertical grid lines as a line list.

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
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_grid(in: VertexInput) -> VertexOutput {
    let clip_x = 2.0 * in.position.x / uniforms.viewport_width - 1.0;
    let clip_y = 2.0 * in.position.y / uniforms.viewport_height - 1.0;
    return VertexOutput(
        vec4<f32>(clip_x, clip_y, 0.0, 1.0),
        vec3<f32>(0.3, 0.3, 0.35), // semi-transparent gray
    );
}

@fragment
fn fs_grid(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 0.4); // semi-transparent
}
