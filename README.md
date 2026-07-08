# velox-terminal

[![CI](https://github.com/lucasmanoguerra/velox-terminal/actions/workflows/ci.yml/badge.svg)](https://github.com/lucasmanoguerra/velox-terminal/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-2024%2B-blue)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
[![GitHub issues](https://img.shields.io/github/issues/lucasmanoguerra/velox-terminal)](https://github.com/lucasmanoguerra/velox-terminal/issues)

**velox-terminal** es una terminal de trading profesional de escritorio, multiplataforma (Windows/macOS/Linux), construida en Rust con renderizado GPU nativo. Combina la latencia de herramientas como NinjaTrader/ATAS con la extensibilidad de un proyecto open-source moderno.

## Features

- ⚡ **Renderizado GPU** con wgpu (DirectX/Metal/Vulkan) — velas, volumen, indicadores overlay
- 🎯 **UI inmediata** con egui — paneles dockables, DOM ladder, order entry
- 📡 **Market data en vivo** — WebSocket a Binance (próximamente: BingX, Bybit, Kraken, índices y forex)
- 🧮 **Indicadores técnicos** — SMA, EMA, RSI, MACD, Bollinger, ATR (incrementales O(1))
- 📊 **Multi-timeframe** — 1m, 5m, 1h con selector en UI
- 🔄 **Reconexión automática** — exponential backoff + jitter para conexiones WebSocket
- 🏗️ **Arquitectura hexagonal** — Ports & Adapters para intercambiar brokers, storages y exchanges

## Arquitectura

```
┌──────────────────────────────────────────────────────┐
│                    Application                         │
│  ┌────────────────────────────────────────────────┐  │
│  │  velox-terminal (composition root)              │  │
│  │  velox-chart (chart orchestrator)               │  │
│  │  velox-md (market data pipeline)                │  │
│  └────────────────────────────────────────────────┘  │
│                          │                            │
│  ┌────────────────────────────────────────────────┐  │
│  │           Domain Core (zero infra deps)          │  │
│  │  velox-core  velox-oms  velox-risk               │  │
│  │  velox-indicators  velox-backtest                │  │
│  └────────────────────────────────────────────────┘  │
│                          │                            │
│  ┌────────────────────────────────────────────────┐  │
│  │              Adapters (implement ports)          │  │
│  │  velox-exchange  velox-broker-fix  velox-chart  │  │
│  │  velox-ui  velox-storage  velox-scripting       │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

Ver [docs/architecture/SYSTEM_OVERVIEW.md](docs/architecture/SYSTEM_OVERVIEW.md) para el detalle completo.

## Stack

| Componente | Tecnología |
|-----------|------------|
| Lenguaje | Rust (edición 2024+) |
| Gráficos | wgpu 24 (DirectX/Metal/Vulkan) + glyphon |
| UI | egui 0.31 (immediate-mode) |
| Async I/O | tokio |
| Concurrencia | crossbeam (hot paths) |
| Serialización | rkyv, bytemuck (zero-copy IPC) |
| Testing | proptest (property-based), criterion (benchmarks) |
| CI/CD | GitHub Actions, cargo-cross |

## Quick Start

```bash
# Requisitos: Rust 2024+, GPU con soporte Vulkan/DirectX 12/Metal
git clone https://github.com/lucasmanoguerra/velox-terminal.git
cd velox-terminal
cargo build --workspace
cargo test --workspace
cargo run -p velox-terminal
```

## Project Status

| Fase | Estado |
|------|--------|
| 🏗️ Arquitectura (ADRs, workspace, CI) | ✅ Completa |
| 📡 Market data pipeline | ✅ Completa (Binance WebSocket + multi-timeframe) |
| 🧮 Indicadores | ✅ SMA, EMA, RSI, MACD, Bollinger, ATR |
| 🏦 OMS + Risk | ✅ State machine + validaciones + circuit breaker |
| 🖥️ Charting engine + UI | ✅ wgpu + egui integrados |
| 🔄 Reconexión automática | ✅ Exponential backoff + jitter |
| 🌐 Más exchanges | 🚧 BingX, Bybit, Kraken (próximamente) |
| 📈 Backtesting | 🚧 En desarrollo |
| 🔌 FIX Protocol | 🚧 En desarrollo |

## Contributing

Nos encantan las contribuciones. Leé [CONTRIBUTING.md](CONTRIBUTING.md) para empezar.

Principios del proyecto:
- **Arquitectura hexagonal**: Ports & Adapters. Domain core no depende de infraestructura.
- **Filosofía UNIX**: Cada componente hace una cosa y la hace bien.
- **Files < 200 líneas**: Archivos pequeños, responsabilidades únicas.
- **Conventional Commits**: Commits atómicos con formato estándar.
- **CI siempre verde**: Todo PR debe pasar build + test + clippy + deny.

## Comunidad

- [Issues](https://github.com/lucasmanoguerra/velox-terminal/issues) — Reportá bugs, pedí features
- [Discussions](https://github.com/lucasmanoguerra/velox-terminal/discussions) — Preguntá, compartí ideas
- [Pull Requests](https://github.com/lucasmanoguerra/velox-terminal/pulls) — Enviá tu código

## Seguridad

Si encontrás una vulnerabilidad, reportala según [SECURITY.md](SECURITY.md).
No uses issues públicos para reportes de seguridad.

## Licencia

Distribuido bajo licencia MIT o Apache 2.0 (a tu elección). Ver `LICENSE` para más información.

---

*velox-terminal — Trading terminal, engineered in Rust.*
