# Next Actions — velox-terminal

Próximas acciones priorizadas según la hoja de ruta de activación progresiva.

## Priority Legend

- **P0**: Núcleo inicial — resolver antes de paralelizar
- **P1**: Segunda ola — una vez fijada la arquitectura
- **P2**: Tercera ola — al tener un flujo end-to-end funcional
- **P3**: Cuarta ola — camino a producción
- **P4**: Mantenimiento continuo

---

## P0 — Núcleo Inicial

- [ ] Crear workspace Cargo con estructura de crates (`crates/core`, `crates/gui`, `crates/feed`, `crates/oms`, `crates/risk`, `crates/charting`, `crates/storage`)
- [ ] Definir modelo de concurrencia (tokio async vs threads vs hilo único de UI)
- [ ] Definir estructuras de tick/quote/OHLCV con optimización de cache-locality
- [ ] Diseñar pipeline de agregación tick → OHLCV multi-timeframe
- [ ] Documentar ADRs de arquitectura general (MSRV, edition, perfiles)
- [ ] Configurar CI básico (lint, test, build)

## P1 — Segunda Ola

- [ ] Implementar market data feed con ring buffers lock-free
- [ ] Implementar máquina de estados de OMS con enums de Rust
- [ ] Implementar validaciones pre-trade de Risk Management
- [ ] Implementar charting engine con wgpu (geometría instanciada)
- [ ] Implementar paneles básicos en egui (chart, order entry)
- [ ] Integrar egui-wgpu con charting engine (mismo contexto)

## P2 — Tercera Ola

- [ ] Implementar conector FIX/WebSocket para al menos un broker
- [ ] Implementar motor de backtesting con slippage realista
- [ ] Implementar indicadores técnicos (SMA, RSI, MACD, Bollinger)
- [ ] Escribir tests exhaustivos de OMS/Risk (proptest)
- [ ] Configurar CI/CD multiplataforma (Windows, macOS, Linux)

## P3 — Cuarta Ola

- [ ] Integrar keyring para almacenamiento seguro de credenciales
- [ ] Benchmarks de latencia end-to-end con criterion
- [ ] Documentación de compliance (MiFID II, SEC)
- [ ] Compilación cruzada y empaquetado nativo
- [ ] Refinamiento de UI/UX (DOM ladder, hotkeys, workspace layout)
- [ ] Motor de scripting (Lua embebido)

## P4 — Mantenimiento Continuo

- [ ] Release management y changelogs
- [ ] Sistema de licencias
- [ ] SRE/Observabilidad (tracing, Sentry)
- [ ] Mantenimiento de dependencias (cargo-audit, cargo-outdated)
- [ ] Triage de bugs
