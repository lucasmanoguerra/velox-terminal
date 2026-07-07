# Text Rendering — velox-terminal

Renderizado de texto sobre wgpu vía glyphon.

---

## Integration

glyphon se integra como una capa de renderizado adicional en el mismo contexto wgpu:

```
Charting (wgpu render pass)
    │
    ├── Draw candles, volume, grid
    │
    ├── Begin glyphon pass
    │   ├── Price labels (Y-axis)
    │   ├── Timestamps (X-axis)
    │   ├── Indicator values (overlay annotations)
    │   └── Symbol name / timeframe
    │
    ├── End glyphon pass
    │
    ├── egui render pass (overlay)
    │
    └── Present
```

## Font Atlas

- **Cargar al inicio**: Una fuente monoespaciada para números (precios), otra sans-serif para etiquetas
- **Tamaños pre-renderizados**: 10px, 12px, 14px (labels), 20px, 24px (big numbers)
- **glyphon cache**: Maneja automáticamente el atlas de glyphs en GPU

## Text Layout Strategy

| Element | Font | Size | Alignment | Update frequency |
|---------|------|------|-----------|-----------------|
| Price labels (Y axis) | Mono | 12px | Right | On pan/zoom |
| Timestamps (X axis) | Sans | 10px | Center | On pan/zoom |
| Symbol name top-left | Sans bold | 14px | Left | On symbol change |
| Indicator values | Mono | 11px | Left | On every tick |
| Cursor crosshair | Mono | 12px | Follows cursor | On mouse move |
| Volume labels | Sans | 10px | Right | On data change |

## Performance

- **glyphon batch**: Todos los textos de un frame se dibujan en un solo pase → 1 draw call
- **Evitar recrear**: Los textos estáticos (symbol name) se cachean; solo los dinámicos (price labels) se actualizan cada frame
- **Target**: < 200μs para todo el texto de un frame
