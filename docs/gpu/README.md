# GPU Documentation — velox-terminal

Renderizado GPU vía wgpu, charting engine, shaders WGSL, texto con glyphon.

## Documents

| File | Purpose | Read when |
|------|---------|-----------|
| `RENDER_PIPELINE.md` | Pipeline de renderizado wgpu, layout, pases | Implementing GPU rendering, debugging visual issues |
| `CHARTING_ARCH.md` | Arquitectura del charting engine, geometría instanciada, overlays | Building the chart, adding new visual elements |
| `SHADER_DESIGN.md` | Shaders WGSL para velas, líneas, overlays | Writing/modifying shaders |
| `TEXT_RENDERING.md` | Integración glyphon para texto en chart | Adding labels, axis, annotations |

## Recommended Loading Order

1. `RENDER_PIPELINE.md` — understand the GPU pipeline
2. `CHARTING_ARCH.md` — the charting architecture
3. `SHADER_DESIGN.md` — shader specifics (if needed)
4. `TEXT_RENDERING.md` — text overlay (if needed)
