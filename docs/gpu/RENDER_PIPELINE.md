# Render Pipeline — velox-terminal

Pipeline de renderizado wgpu compartido entre charting engine y egui.

---

## Pipeline Architecture

```
wgpu Instance
    │
    ├── Surface (window)
    │
    ├── Adapter → Device
    │
    ├── Render Pass 1: Charting
    │   ├── Clear (background)
    │   ├── Candle geometry (instanced)
    │   ├── Volume bars (instanced)
    │   ├── Grid lines
    │   ├── Indicator overlays (instanced per overlay)
    │   └── Text (glyphon)
    │
    ├── Render Pass 2: egui UI
    │   └── egui-wgpu integration
    │
    └── Present to surface
```

## Shared Context

Charting engine y egui comparten el mismo `wgpu::Device`, `wgpu::Queue`, y `wgpu::Surface`.

```rust
struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    format: wgpu::TextureFormat,
    // Shared uniform buffers
    camera: CameraUniform,
}
```

**Orden de renderizado**:
1. Charting engine dibuja primero (fondo, velas, overlays, texto)
2. egui dibuja encima (paneles, formularios, botones)
3. egui puede tener transparencia — el charting se ve a través

## Frame Budget

| Stage | Budget (60fps) | Budget (144fps) | Notes |
|-------|---------------|-----------------|-------|
| Charting render | 8ms | 3ms | GPU-bound |
| egui render | 4ms | 2ms | CPU + GPU |
| Headroom | 4ms | 2ms | Input, compose, present |
| **Total** | **16ms** | **7ms** | |

## Uniform Buffers

```rust
#[repr(C)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],  // MVP matrix
    viewport_size: [f32; 2],          // Current viewport in pixels
    time_frame_start: f32,            // Time since start (for animations)
}
```

Camera uniform se actualiza en CPU solo cuando el usuario hace pan/zoom (no en cada frame si no hay interacción).

## Texture Atlas

- Símbolos de velas (colores pre-computados)
- Indicadores de estado (conectado/desconectado)
- Pequeños iconos de UI (cargados en GPU al inicio)
