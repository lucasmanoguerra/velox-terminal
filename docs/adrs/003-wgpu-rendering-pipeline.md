# ADR-003: wgpu Rendering Pipeline

| | |
|---|---|
| **ADR** | 003 |
| **Title** | wgpu como backend gráfico único con fallback automático |
| **Status** | Accepted |
| **Date** | 2026-07-06 |
| **Author** | charting-engine |

## Context

La terminal necesita renderizado GPU para:
- Charts de velas con alta densidad de datos (millones de puntos)
- DOM ladder con actualizaciones por tick
- Texto con glyphon para labels de precio/eje
- 60+ FPS constantes

Requerimientos multiplataforma:
- Windows: DirectX 12 (o 11 fallback)
- macOS: Metal
- Linux: Vulkan (o OpenGL fallback como último recurso)

## Decision

Usar **wgpu** como backend gráfico único.

```rust
// wgpu handles backend selection automatically
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: wgpu::Backends::all(), // Vulkan | Metal | DX12 | GL
    dx12_shader_compiler: wgpu::Dx12Compiler::Dxc { dxil_path, dxc_path },
    ..Default::default()
});
let adapter = instance.request_adapter(&adapter_options).await?;
let (device, queue) = adapter.request_device(&device_descriptor).await?;
```

### Pipeline Architecture

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   egui UI    │    │ Chart Engine │    │   Glyphon    │
│  (immediate) │    │ (wgsl shader)│    │ (text atlas) │
└──────┬───────┘    └──────┬───────┘    └──────┬───────┘
       │                   │                   │
       ▼                   ▼                   ▼
┌──────────────────────────────────────────────────────┐
│                    wgpu Renderer                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │ SwapChain│  │  Frame   │  │  Pipeline│           │
│  │  (surface)│  │  Graph   │  │  Manager │           │
│  └──────────┘  └──────────┘  └──────────┘           │
└──────────────────────────────────────────────────────┘
```

### Render Pass Order

1. **Clear** — color + depth
2. **Chart** — candle geometry (instanced triangles)
3. **Grid** — lines via line list
4. **Indicators** — overlay lines/bands
5. **Crosshair** — cursor lines
6. **DOM Ladder** — bid/ask bars + text
7. **egui** — all UI panels (rendered as last pass)
8. **Glyphon** — text overlay (integrated in egui pass)

### Shader Strategy (WGSL)

```wgsl
// Vertex shader: instanced candle rendering
struct CandleInstance {
    @location(0) open: f32,
    @location(1) high: f32,
    @location(2) low: f32,
    @location(3) close: f32,
    @location(4) volume: f32,
    @location(5) index: f32,
    @location(6) color: vec4<f32>,
}

@vertex
fn vs_candle(@builtin(instance_index) instance: u32, ...) -> ...
```

Los shaders reciben matrices de transformación (zoom/pan/pixel ratio) vía uniforms, actualizadas por frame.

## Consequences

### Positive
- Código único para 3+ backends gráficos
- wgpu es mantenido por Mozilla/Embark, comunidad activa
- Type-safe API de Rust (no pointers raw como OpenGL)
- Integración nativa con egui (egui-wgpu)

### Negative
- wgpu no expone todas las features de Vulkan/Metal (pero suficiente para 2D charting)
- WGSL es más verboso que GLSL para shaders simples
- Debugging de shaders requiere tools específicas (RenderDoc, NSight)

### Trade-offs
- Se consideró OpenGL 4.6 (más maduro, tooling extenso) pero no tiene futuro a largo plazo
- Se consideró Vulkan directo (máximo control) pero el desarrollo sería 3x más lento
- wgpu es el punto dulce entre control y productividad

## Compliance

- Todos los shaders en WGSL (nunca GLSL/HLSL compilado)
- Las transformaciones zoom/pan se hacen en vertex shader (no CPU)
- `wgpu::ShaderModule` se compila en build time con `include_wgsl!()` o `include_str!()`

## Notes

### Related ADRs
- ADR-001: Workspace Crate Structure (velox-gpu, velox-chart crates)

### References
- [wgpu documentation](https://wgpu.rs/)
- [egui-wgpu integration](https://github.com/emilk/egui/tree/master/crates/egui-wgpu)
- [glyphon](https://github.com/grovesNL/glyphon)

### Change History
- 2026-07-06: Initial draft
