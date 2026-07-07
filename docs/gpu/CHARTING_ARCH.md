# Charting Architecture — velox-terminal

Arquitectura del motor de charting con wgpu.

---

## Geometry Strategy

Usamos **geometría instanciada** para maximizar efficiency de GPU:

| Element | Geometry | Instancing | Draw Calls |
|---------|----------|-----------|------------|
| Candles | 1 quad (6 vertices) | 1 instance per candle | 1 draw call for all visible candles |
| Volume bars | 1 quad (6 vertices) | 1 instance per bar | 1 draw call |
| Grid lines | Line segments | 1 instance per line | 2 draw calls (horizontal + vertical) |
| SMA lines | Line strip segments | 1 instance per segment | 1 draw call per SMA |
| Bollinger bands | Filled polygon strip | 1 instance per band | 1 draw call per band |

**Principio**: Nunca un draw call por elemento. Todo se agrupa en batches.

## Data Flow

```
[crates/feed] ── crossbeam channel ──> Candle data
                                              │
                                              ▼
                                    ┌─────────────────┐
                                    │ CandleBuffer    │
                                    │ (ring in GPU)   │
                                    └────────┬────────┘
                                              │
                                              ▼
                                    ┌─────────────────┐
                                    │ InstanceBuffer  │
                                    │ (updated per    │
                                    │  frame if new   │
                                    │  data)          │
                                    └────────┬────────┘
                                              │
                                              ▼
                                    ┌─────────────────┐
                                    │ Vertex Shader   │
                                    │ (WGSL)          │
                                    │ • Camera MVP    │
                                    │ • Per-instance  │
                                    │   transform     │
                                    └────────┬────────┘
                                              │
                                              ▼
                                    ┌─────────────────┐
                                    │ Fragment Shader │
                                    │ (WGSL)          │
                                    │ • Candle color  │
                                    │   (bull/bear)   │
                                    │ • Wick, body    │
                                    └─────────────────┘
```

## Zoom and Pan

Zoom y pan se implementan **exclusivamente en el vertex shader**:

```wgsl
struct CameraUniform {
    view_projection: mat4x4<f32>;
    viewport_size: vec2<f32>;
    time_frame_start: f32;
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec2<f32>,  // Local position in candle space
    @location(1) instance_offset: vec2<f32>,  // Candle's position in screen space
};

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    let world_pos = vec4(input.position + input.instance_offset, 0.0, 1.0);
    return camera.view_projection * world_pos;
}
```

- **Pan**: Se actualiza `camera.view_projection` → recargar uniform buffer
- **Zoom**: Se escala la matriz de proyección alrededor del punto central → recargar uniform buffer
- **No se recrea geometría** durante pan/zoom

## Indicator Overlays

Los overlays son capas de geometría independientes:

```rust
enum Overlay {
    Sma {
        period: u32,
        color: [f32; 4],
        line_width: f32,
    },
    BollingerBands {
        period: u32,
        std_dev: f32,
        color: [f32; 4],
        fill_color: [f32; 4],
    },
    // ...
}

struct OverlayRenderer {
    overlays: Vec<Box<dyn OverlayInstance>>,
    // Each overlay has its own instance buffer
    // Activation = add to vec, Deactivation = set visible=false
}
```

Cada overlay tiene su propio buffer de instancias que se sube a GPU solo cuando cambian los datos. La activación/desactivación no recompila shaders — solo cambia una flag de visibilidad.

## Performance Targets

| Metric | Target |
|--------|--------|
| Max visible candles without frame drop | 50,000 |
| Max simultaneous overlays | 10 |
| Frame update (new tick) | < 500μs CPU + GPU |
| Pan/zoom response | < 16ms (next frame) |
| Memory (candle buffer) | ~ 500KB per 50,000 candles |
