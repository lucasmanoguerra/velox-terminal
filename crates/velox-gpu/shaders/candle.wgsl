// Candle chart shader
// Renders candlestick bodies as quads and wicks as lines using instanced drawing.

struct Uniforms {
    price_scale: f32,
    price_offset: f32,
    time_scale: f32,
    time_offset: f32,
    candle_width: f32,
    viewport_width: f32,
    viewport_height: f32,
}

struct CandleData {
    open: f32,
    high: f32,
    low: f32,
    close: f32,
    timestamp: f32,
    is_bullish: u32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> candles: array<CandleData>;

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

fn get_candle_color(is_bullish: u32) -> vec3<f32> {
    if is_bullish == 1u {
        return vec3<f32>(0.0, 0.8, 0.0); // green
    } else {
        return vec3<f32>(0.8, 0.0, 0.0); // red
    }
}

// Candle body: 4 vertices forming a quad (triangle strip)
// Vertex layout:
//   0: bottom-left   (open or close, whichever is lower)
//   1: bottom-right
//   2: top-left      (open or close, whichever is higher)
//   3: top-right
@vertex
fn vs_candle_body(@builtin(vertex_index) vertex_id: u32, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let candle = candles[instance_id];
    let x_center = data_to_screen_x(candle.timestamp);
    let half_w = uniforms.candle_width * 0.5;

    let open_y = data_to_screen_y(candle.open);
    let close_y = data_to_screen_y(candle.close);
    let top_y = max(open_y, close_y);
    let bottom_y = min(open_y, close_y);

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

    let color = get_candle_color(candle.is_bullish);
    return VertexOutput(screen_to_clip(pos), color);
}

// Candle wick: 2 vertices forming a line (line list)
// vertex_id 0 = low, vertex_id 1 = high
@vertex
fn vs_candle_wick(@builtin(vertex_index) vertex_id: u32, @builtin(instance_index) instance_id: u32) -> VertexOutput {
    let candle = candles[instance_id];
    let x = data_to_screen_x(candle.timestamp);

    var y: f32;
    if vertex_id == 0u {
        y = data_to_screen_y(candle.low);
    } else {
        y = data_to_screen_y(candle.high);
    }

    let color = get_candle_color(candle.is_bullish);
    return VertexOutput(screen_to_clip(vec2<f32>(x, y)), color);
}

@fragment
fn fs_candle(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
