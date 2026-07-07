# Shader Design — velox-terminal

Shaders WGSL para el charting engine.

---

## Candle Shader

### Vertex Shader

```wgsl
struct CandleInstance {
    @location(2) open: f32,     // Y position of open
    @location(3) high: f32,     // Y position of high
    @location(4) low: f32,      // Y position of low
    @location(5) close: f32,    // Y position of close
    @location(6) x_pos: f32,    // X position (center)
    @location(7) width: f32,    // Half-width of candle body
    @location(8) is_bull: f32,  // 1.0 if bullish, 0.0 if bearish
};

struct VertexInput {
    @builtin(instance_index) instance_index: u32;
    @location(0) local_pos: vec2<f32>;  // Local position within candle geometry
};

@vertex
fn candle_vs(input: VertexInput, instance: CandleInstance) -> @builtin(position) vec4<f32> {
    // Each instance has 6 vertices (2 triangles forming a quad)
    // Vertex 0-3: body quad, Vertex 4-5: wick
    
    var world_y: f32;
    if input.local_pos.y >= 0.0 {
        // Body
        world_y = select(instance.close, instance.open, input.local_pos.y > 0.5);
        // local_pos.y 0-0.5 = bottom of body, 0.5-1.0 = top of body
        let body_bottom = min(instance.open, instance.close);
        let body_top = max(instance.open, instance.close);
        world_y = mix(body_bottom, body_top, fract(input.local_pos.y * 2.0));
    } else {
        // Wick: goes from body to high/low
        let body_mid = (instance.open + instance.close) / 2.0;
        if input.local_pos.y < 0.0 && input.local_pos.x == 0.0 {
            world_y = mix(body_mid, instance.high, -input.local_pos.y);
        } else {
            world_y = mix(body_mid, instance.low, -input.local_pos.y);
        }
    }
    
    let world_pos = vec4(instance.x_pos + input.local_pos.x * instance.width, world_y, 0.0, 1.0);
    return camera.view_projection * world_pos;
}
```

### Fragment Shader

```wgsl
@fragment
fn candle_fs(input: FragmentInput, instance: CandleInstance) -> @location(0) vec4<f32> {
    let bull_color = vec4<f32>(0.0, 0.8, 0.0, 1.0);   // Green
    let bear_color = vec4<f32>(0.8, 0.0, 0.0, 1.0);   // Red
    
    if instance.is_bull > 0.5 {
        return bull_color;
    } else {
        return bear_color;
    }
}
```

## Grid Shader

Grid lines con anti-aliasing en el fragment shader:

```wgsl
@fragment
fn grid_fs() -> @location(0) vec4<f32> {
    // Soft grid lines with alpha based on line distance
    let line_dist = abs(frag_coord - round(frag_coord));
    let alpha = 1.0 - smoothstep(0.0, 1.0, line_dist / 2.0);
    return vec4<f32>(0.3, 0.3, 0.3, alpha * 0.5);  // Semi-transparent gray
}
```

## Overlay Shaders

Los overlays (SMA, Bollinger, etc.) comparten un vertex shader genérico de líneas:

```wgsl
struct LineInstance {
    @location(2) from_pos: vec2<f32>;
    @location(4) to_pos: vec2<f32>;
    @location(6) color: vec4<f32>;
    @location(10) width: f32;
};

@vertex
fn line_vs(input: VertexInput<LineInstance>) -> @builtin(position) vec4<f32> {
    // Line with thickness using instanced geometry
    // 6 vertices per segment (2 triangles forming a thick line)
    let direction = normalize(input.instance.to_pos - input.instance.from_pos);
    let normal = vec2(-direction.y, direction.x);
    let offset = normal * input.instance.width * input.local_pos.y;
    let world_pos = mix(input.instance.from_pos, input.instance.to_pos, input.local_pos.x) + offset;
    return camera.view_projection * vec4(world_pos, 0.0, 1.0);
}
```

## Pipeline Layout

```rust
let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("Candle Pipeline"),
    vertex: wgpu::VertexState {
        module: &candle_shader,
        entry_point: "candle_vs",
        buffers: &[
            candle_vertex_buffer_layout,     // Per-vertex (6 shared vertices)
            candle_instance_buffer_layout,   // Per-instance (position, price, etc.)
        ],
    },
    fragment: Some(wgpu::FragmentState {
        module: &candle_shader,
        entry_point: "candle_fs",
        targets: &[Some(ColorTargetState {
            format: surface_config.format,
            blend: Some(BlendState::ALPHA_BLENDING),
            ..Default::default()
        })],
    }),
    primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
    },
    depth_stencil: None,
    multisample: wgpu::MultisampleState::default(),
    multiview: None,
});
```
