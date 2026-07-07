# Architecture Documentation — velox-terminal

Estructura, concurrencia, boundaries y pipeline de datos del sistema.

## Documents

| File | Purpose | Read when |
|------|---------|-----------|
| `SYSTEM_OVERVIEW.md` | Visión general del sistema, subsistemas y flujo de datos | Starting any architecture-related task |
| `CONCURRENCY_MODEL.md` | Modelo de concurrencia: tokio async, crossbeam, threads dedicados, hilo único de UI | Designing concurrent components, debugging race conditions |
| `CRATE_BOUNDARIES.md` | Layout del workspace Cargo, boundaries entre crates, visibilidad | Adding a new crate, refactoring module boundaries |
| `DATA_PIPELINE.md` | Flujo de datos desde el broker hasta la pantalla | Understanding end-to-end data flow, debugging latency |
| `DEPENDENCY_MAP.md` | Mapa de dependencias críticas entre subsistemas | Planning cross-cutting changes, understanding impact |

## Recommended Loading Order

1. `SYSTEM_OVERVIEW.md` — big picture
2. `DEPENDENCY_MAP.md` — how parts relate
3. `CRATE_BOUNDARIES.md` — code organization
4. `CONCURRENCY_MODEL.md` — threading (if task involves performance)
5. `DATA_PIPELINE.md` — data flow (if task involves data processing)
