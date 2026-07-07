# Crate Boundaries — velox-terminal

Layout del workspace Cargo y boundaries entre crates.

---

## Workspace Structure

```
velox-terminal/
├── Cargo.toml                  # Workspace root
├── crates/
│   ├── core/                   # Tipos compartidos, traits, event bus
│   ├── feed/                   # Market data feed (real-time)
│   ├── oms/                    # Order Management System
│   ├── risk/                   # Risk Management
│   ├── broker/                 # Broker connectors (FIX/WS/REST)
│   ├── charting/               # Charting engine (wgpu)
│   ├── gui/                    # egui UI panels
│   ├── storage/                # Time-series storage
│   ├── indicators/             # Technical indicators
│   ├── backtest/               # Backtesting engine
│   └── scripting/              # Scripting engine (Lua)
├── docs/                       # Project documentation
└── .opencode/                  # OpenCode agent configuration
```

---

## Dependency Graph

```
core ──┬──> feed ──> charting
       ├──> oms ──> risk
       ├──> storage ──> backtest
       ├──> indicators ──> charting
       ├──> broker ──> feed, oms
       ├──> scripting ──> oms, indicators
       └──> gui ──> charting, feed, oms

(No cyclical dependencies — verified by cargo-deny)
```

---

## Boundary Rules

### `crates/core`
- Define tipos base: `Tick`, `Quote`, `Trade`, `Candle`, `Order`, `OrderId`, `Symbol`
- Define traits: `BrokerClient`, `FeedConsumer`, `RiskValidator`, `OrderStore`
- Define errores compartidos: `CoreError`, `FeedError`, `OrderError`
- **No depende de ningún otro crate del workspace**

### `crates/feed`
- Depende de: `core` (tipos), `broker` (conexión)
- **No depende de**: `oms`, `risk`, `gui`, `charting`
- Envía eventos via crossbeam channel

### `crates/oms`
- Depende de: `core` (tipos, `OrderStore` trait), `risk` (validación)
- **No depende de**: `feed`, `gui`, `charting`
- Llama a `risk::validate()` antes de enviar cualquier orden

### `crates/risk`
- Depende de: `core` (tipos)
- **No depende de ningún otro crate del workspace** (debe ser puramente funcional)
- Sin I/O, sin estado mutable compartido

### `crates/broker`
- Depende de: `core` (tipos, `BrokerClient` trait)
- **No depende de**: `oms`, `risk`, `gui`
- Implementa el trait `BrokerClient` para cada broker

### `crates/charting`
- Depende de: `core` (tipos `Tick`, `Candle`), `indicators` (overlays)
- **No depende de**: `oms`, `risk`, `broker`
- Renderiza en GPU vía wgpu

### `crates/gui`
- Depende de: `core`, `charting` (comparte contexto wgpu), `feed` (datos)
- **No depende de**: `oms` directamente (recibe estado via core)
- UI immediate-mode con egui

### `crates/storage`
- Depende de: `core` (tipos)
- Independiente de otros crates de negocio

### `crates/indicators`
- Depende de: `core` (tipos)
- **No depende de**: ningún otro crate
- Cálculo puro, sin estado externo

### `crates/backtest`
- Depende de: `core`, `indicators`, `storage`
- **No depende de**: `oms`, `risk` — reusa la misma lógica

### `crates/scripting`
- Depende de: `core`, `indicators`, `oms` (API sandboxeada)
- Aísla scripts de usuario en hilos separados
