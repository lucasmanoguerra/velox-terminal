# ADR-001: Workspace Crate Structure

| | |
|---|---|
| **ADR** | 001 |
| **Title** | Cargo workspace con crates con bounded context |
| **Status** | Accepted |
| **Date** | 2026-07-06 |
| **Author** | systems-architect |

## Context

El proyecto necesita una estructura que permita:
- Compilación incremental rápida (crates pequeños y bien delimitados)
- Reutilización entre componentes (indicadores usados tanto en vivo como en backtesting)
- Aislamiento de subsistemas críticos (OMS/Risk no deben depender de UI)
- Compilación cruzada multiplataforma

## Decision

Usar un Cargo workspace con los siguientes crates:

```
velox-terminal/
├── Cargo.toml                    # workspace manifest
├── crates/
│   ├── velox-core/               # Domain primitives: Order, Trade, Quote, Symbol
│   ├── velox-md/                 # Market data structures, aggregation, ring buffers
│   ├── velox-indicators/         # Technical indicators (incremental O(1))
│   ├── velox-oms/                # Order Management System (zero unsafe)
│   ├── velox-risk/               # Risk Management (zero unsafe)
│   ├── velox-broker/             # Broker connector trait + impls
│   ├── velox-broker-fix/         # FIX protocol implementation
│   ├── velox-storage/            # Time-series storage engine
│   ├── velox-backtest/           # Backtesting engine
│   ├── velox-scripting/          # User scripting (Lua/DSL)
│   ├── velox-gpu/                # wgpu rendering primitives
│   ├── velox-chart/              # Charting engine
│   ├── velox-ui/                 # egui panels & UI components
│   └── velox-terminal/           # Main binary
├── docs/
└── .opencode/
```

```rust
// Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/*",
]
edition = "2024"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
repository = "https://github.com/lucasmanoguerra/velox-terminal"
```

## Consequences

### Positive
- Compilación paralela y caching granular
- Cada crate puede tener su propio perfil de release (ReleaseSafe para velox-oms, velox-risk)
- Dependencias explícitas y visibles en Cargo.toml de cada crate
- Fácil reemplazo de implementaciones (otro broker? solo un nuevo crate)

### Negative
- Mayor boilerplate inicial vs un solo crate
- Refactors cross-crate requieren actualizar varias dependencias

### Trade-offs
- Se eligió workspace con muchos crates pequeños en lugar de monorepo sin estructura: mayor overhead inicial pero mejor escalabilidad
- No se usan `pub` indiscriminados: cada crate expone solo una API mínima

## Compliance

- CI verifica que ningún crate en ReleaseSafe profile contenga `unsafe`
- Clippy lint `cargo clippy --workspace --all-targets` obligatorio
- `cargo deny` chequea licencias de dependencias

## Notes

### Related ADRs
- ADR-002: Concurrency Model

### References
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

### Change History
- 2026-07-06: Initial draft
