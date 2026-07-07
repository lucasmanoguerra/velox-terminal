# Assumptions — velox-terminal

Suposiciones explícitas del proyecto. Validadas periódicamente.

---

| # | Assumption | Confidence | Last validated | Risk if wrong |
|---|-----------|-----------|---------------|---------------|
| 1 | Rust es la herramienta correcta para una terminal de trading de alta performance en desktop | Alta | 2026-07-06 | Bajo — Rust es comprobadamente efectivo para sistemas de baja latencia |
| 2 | wgpu proporciona rendimiento suficiente para charting con 50,000+ velas visibles | Media | 2026-07-06 | Medio — Alternativa: OpenGL directo vía glow, o Vulkano |
| 3 | egui immediate-mode escala a interfaces de trading complejas (DOM, multi-panel, dockable) | Media | 2026-07-06 | Medio — Alternativa: migrar a Druid/Slint si egui no escala |
| 4 | El modelo de concurrencia (tokio + crossbeam + hilo único UI) es viable sin data races | Alta | 2026-07-06 | Bajo — Es un patrón probado |
| 5 | El mercado objetivo (traders profesionales) existente en Windows y macOS principalmente | Alta | 2026-07-06 | Bajo — El soporte Linux es un diferenciador, no requisito |
| 6 | Los brokers expondrán APIs FIX/WebSocket estables y documentadas | Media | 2026-07-06 | Medio — Cambios en APIs de brokers son frecuentes |
| 7 | Los traders aceptarán una terminal nueva si cumple con el feature set mínimo de la industria | Media | 2026-07-06 | Alto — Feature parity con NinjaTrader/ATAS es un riesgo |
| 8 | property-based testing detecta suficientemente bugs en OMS/Risk | Alta | 2026-07-06 | Bajo — Complementar con integration tests |
