# System Overview — velox-terminal

Visión general de la arquitectura del sistema.

---

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        velox-terminal                        │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ egui UI  │  │ Charting │  │  OMS     │  │  Risk    │   │
│  │ (panels) │  │ (wgpu)   │  │ (orders) │  │ (limits) │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘   │
│       │             │             │             │          │
│  ┌────┴─────────────┴─────────────┴─────────────┴────┐     │
│  │              Core State / Event Bus                 │     │
│  └────┬─────────────┬─────────────┬─────────────┬────┘     │
│       │             │             │             │          │
│  ┌────┴────┐  ┌─────┴─────┐  ┌───┴────┐  ┌────┴─────┐    │
│  │ Market  │  │ Time-Ser. │  │ Broker │  │ Script  │    │
│  │ Data Fd │  │ Storage   │  │ Connec │  │ Engine  │    │
│  └─────────┘  └───────────┘  └────────┘  └──────────┘    │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              tokio async runtime                      │   │
│  │  (network I/O: feeds, brokers, REST APIs)            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Subsistemas Principales

| Subsistema | Crate | Responsabilidad | Tecnología |
|-----------|-------|----------------|------------|
| **Core** | `crates/core` | Tipos compartidos, traits, event bus | Rust, crossbeam |
| **Market Data Feed** | `crates/feed` | Ingesta de ticks en tiempo real | tokio, crossbeam, ring buffers |
| **OMS** | `crates/oms` | Máquina de estados de órdenes | Rust enums, thiserror |
| **Risk** | `crates/risk` | Validaciones pre-trade, circuit breakers | Rust traits |
| **Broker Connection** | `crates/broker` | Conectores FIX/WS/REST | fefix, tokio-tungstenite, reqwest |
| **Charting** | `crates/charting` | Renderizado GPU de velas y overlays | wgpu, WGSL, glyphon |
| **GUI** | `crates/gui` | Paneles de trading en egui | egui, eframe, egui-wgpu |
| **Storage** | `crates/storage` | Time-series embebida | redb/sled, bytemuck, rkyv |
| **Indicators** | `crates/indicators` | Indicadores técnicos streaming | Rust generics |
| **Backtesting** | `crates/backtest` | Simulación histórica | rayon |
| **Scripting** | `crates/scripting` | Estrategias de usuario | mlua |
| **CLI** | `crates/cli` | Interfaz de línea de comandos | clap |

## Flujo de Datos Simplificado

```
Broker ──(FIX/WS)──> Broker Connector ──> Market Data Feed ──> Charting Engine
                                        └──> OMS ──> Risk ──> Broker Connector ──> Broker
                                        └──> Storage ──> Backtesting
```

## Principios Arquitectónicos

1. **Crates con boundaries claros**: Cada crate sabe lo mínimo necesario sobre los demás. Las dependencias entre crates son un DAG acíclico.
2. **Interfaces trait-based**: Los conectores de broker, almacenamiento, y fuentes de datos se definen como traits para permitir swapping e testing.
3. **Event-driven**: Los cambios de estado (nuevo tick, cambio de orden, actualización de posición) se propagan vía un event bus interno.
4. **Hilo único de UI**: egui corre en el hilo principal. Todo lo demás (red, procesamiento, cálculo) corre en threads separados.
5. **Fail-safe en todo lo financiero**: Si algo no se puede verificar, se rechaza/bloquea por defecto.
