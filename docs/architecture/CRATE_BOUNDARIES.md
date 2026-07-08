# Hexagonal Crate Boundaries — velox-terminal

Which crates belong to which hexagonal layer, and the rules governing their relationships.

---

## Layer Classification

```
┌─────────────────────────────────────────────────────────────────────┐
│                     INFRASTRUCTURE                                  │
│  tokio runtime, wgpu device, crossbeam channels, winit event loop   │
│  (Not crates — external dependencies that adapters use)             │
└─────────────────────────────────────────────────────────────────────┘
                              ▲
                              │ uses
                              │
┌─────────────────────────────────────────────────────────────────────┐
│                        ADAPTERS                                      │
│  velox-exchange     BinanceFeed (WebSocket → RingBuffer)            │
│  velox-broker-fix   FixBrokerClient (FIX protocol → broker)        │
│  velox-chart        ChartRenderer (wgpu pipeline → GPU)            │
│  velox-ui           PanelManager (egui → screen)                   │
│  velox-storage      StorageEngine (SQLite → disk) [WIP]            │
│                                                                     │
│  Traits: These crates IMPLEMENT ports                               │
│  Dependencies: domain types + infra(tokio, wgpu, egui)             │
└─────────────────────────────────────────────────────────────────────┘
                              ▲
                              │ implements
                              │
┌─────────────────────────────────────────────────────────────────────┐
│                          PORTS                                       │
│  velox-broker        BrokerClient trait (async)                     │
│  velox-exchange      ExchangeFeed trait (sync start/stop/sub)      │
│  (velox-storage)     StorageEngine trait (future)                   │
│                                                                     │
│  Traits: These crates DEFINE ports                                  │
│  Dependencies: domain types only                                    │
└─────────────────────────────────────────────────────────────────────┘
                              ▲
                              │ depends on
                              │
┌─────────────────────────────────────────────────────────────────────┐
│                     DOMAIN / APPLICATION                             │
│                                                                     │
│  velox-core         Pure types: Tick, Quote, Candle, Order,         │
│                     OrderState, Side, Fill, Position, AccountInfo   │
│                     Zero I/O, zero infra deps                       │
│                                                                     │
│  velox-md           Market data: RingBuffer (SPSC), CandleAggregator│
│                     MarketDataPipeline (tick→OHLCV)                 │
│                                                                     │
│  velox-indicators   Technical indicators: SMA, EMA, RSI, MACD,     │
│                     Bollinger Bands, ATR — all incremental O(1)     │
│                                                                     │
│  velox-oms          Order Management: State machine (10 states),    │
│                     OrderManager, fill management, replace/cancel    │
│                                                                     │
│  velox-risk         Risk Management: Validators, position limits,   │
│                     circuit breaker — forbid(unsafe_code)            │
│                                                                     │
│  velox-backtest     Backtesting: Replay engine, P&L calculation     │
│                                                                     │
│  velox-scripting    Lua scripting: Sandboxed strategy execution     │
│                                                                     │
│  Dependencies: limited to domain crates only                        │
└─────────────────────────────────────────────────────────────────────┘
                              ▲
                              │ composes
                              │
┌─────────────────────────────────────────────────────────────────────┐
│                    COMPOSITION ROOT                                  │
│  velox-terminal     App struct — wires ports to adapters            │
│                     Creates tokio runtime, window, GPU surface,     │
│                     market data pipeline, exchange feed, egui state │
│                     Runs winit event loop                           │
│                     Depends on EVERYTHING (intentional)             │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Classification Matrix

| Crate | Layer | Pure? | Infra deps | Tests |
|-------|-------|-------|-----------|-------|
| `velox-core` | **Domain** | ✅ Pure | None | proptest |
| `velox-md` | **Domain** | ✅ Pure | crossbeam (ring buf), tokio::sync (mpsc) | proptest, criterion |
| `velox-indicators` | **Domain** | ✅ Pure | None | proptest, criterion |
| `velox-oms` | **Domain** | ✅ Pure | crossbeam (event dispatch) | proptest |
| `velox-risk` | **Domain** | ✅ Pure | dashmap (concurrent positions) | proptest |
| `velox-backtest` | **Application** | ✅ Pure | chrono | proptest, criterion |
| `velox-scripting` | **Application** | ~ | mlua (optional Lua VM) | proptest |
| `velox-broker` | **Port** | ~ | tokio, async-trait | mockall |
| `velox-exchange` | **Port + Adapter** | ❌ I/O | tokio, tokio-tungstenite, serde_json | unit |
| `velox-broker-fix` | **Adapter** | ❌ I/O | tokio, tracing | unit |
| `velox-storage` | **Port + Adapter** | ~ | rusqlite (optional) | proptest, criterion |
| `velox-gpu` | **Infrastructure** | ❌ GPU | wgpu, winit, glyphon | — |
| `velox-chart` | **Adapter** | ❌ GPU | wgpu, bytemuck, velox-gpu | criterion |
| `velox-ui` | **Adapter** | ❌ GPU | egui, egui-wgpu, wgpu, tokio, crossbeam | unit |
| `velox-terminal` | **Composition** | ❌ All | All infra + all adapters | — |

---

## Boundary Rules

### Rule 1: Domain never depends on Adapter

```
✅ CORRECT:  velox-oms → velox-core
✅ CORRECT:  velox-risk → velox-core
✅ CORRECT:  velox-md → velox-core

❌ FORBIDDEN:  velox-oms → velox-exchange
❌ FORBIDDEN:  velox-risk → velox-ui
❌ FORBIDDEN:  velox-md → velox-chart
```

Verification: grep for adapter crate names in domain crate Cargo.toml files.

```bash
# No domain crate should import an adapter crate
for domain in velox-core velox-md velox-indicators velox-oms velox-risk velox-backtest velox-storage; do
    if grep -q "velox-exchange\|velox-broker-fix\|velox-chart\|velox-ui\|velox-gpu" "crates/$domain/Cargo.toml"; then
        echo "VIOLATION: $domain depends on an adapter!"
    fi
done
```

### Rule 2: Adapters depend on Ports (or Domain types)

```
✅ CORRECT:  velox-exchange → velox-core, velox-md (types + RingBuffer)
✅ CORRECT:  velox-broker-fix → velox-core, velox-broker (domain + BrokerClient trait)
✅ CORRECT:  velox-chart → velox-core, velox-indicators (domain types)
✅ CORRECT:  velox-ui → velox-core, velox-md, velox-oms, velox-chart, velox-gpu
```

Adapters may depend on other adapters when there's a clear dependency chain:
- `velox-chart` depends on `velox-gpu` (GPU abstraction)
- `velox-ui` depends on `velox-chart` and `velox-gpu` (composite rendering)
- `velox-ui` depends on `velox-md` and `velox-oms` (display state)

### Rule 3: Port crates define traits, may provide mocks

```
velox-broker:
  - Defines: BrokerClient trait
  - Provides: MockBroker (dev-dependency)
  - Depends on: velox-core

velox-exchange:
  - Defines: ExchangeFeed trait
  - Provides: BinanceFeed (the only implementation)
  - Note: Trait and adapter are co-located (pragmatic — only one impl exists)
```

### Rule 4: Composition root depends on everything

```
velox-terminal:
  - Depends on ALL crates (domain + ports + adapters + infra)
  - This is intentional — the composition root wires the dependency graph
  - No other crate has this privilege
  - If velox-terminal imports something, it's because it wires it
```

---

## Exception: Ring Buffer on the Hot Path

The ring buffer (`velox_md::ring_buffer::RingBuffer`) is a **lock-free SPSC** structure that bridges the tokio WebSocket thread (producer) and the main thread (consumer).

```
Producer (tokio)                  Consumer (main thread)
BinanceFeed::handle_trade()       MarketDataPipeline::poll()
         │                                │
         │ ring.push(MarketEvent::Tick)   │ ring.pop()
         ▼                                ▼
  ┌──────────────────────────────────────────┐
  │          RingBuffer (SPSC)                │
  │  - Fixed-size circular buffer            │
  │  - No locks, no atomics on hot path      │
  │  - Single producer, single consumer      │
  │  - Drop-latest if consumer is slow       │
  └──────────────────────────────────────────┘
```

**Why this is an exception**: The ring buffer is used by both an adapter (`velox-exchange`, producer side) and domain logic (`velox-md`, consumer side). This cross-layer usage is justified because:

1. The ring buffer is a pure data structure (no I/O, no syscalls)
2. The coupling is minimal — both sides only share the buffer's memory address
3. No domain logic leaks into the adapter, and vice versa
4. The alternative (a trait + adapter pattern) would add latency on the hot path

**Other acknowledged infra-in-domain dependencies**:

| Domain Crate | Infra Dependency | Justification |
|-------------|-----------------|---------------|
| `velox-md` | `crossbeam` | Ring buffer requires atomic ordering |
| `velox-md` | `tokio::sync` | mpsc channel for candle delivery to main thread |
| `velox-oms` | `crossbeam` | Event dispatch for order state changes |
| `velox-risk` | `dashmap` | Concurrent position tracking across symbols |

---

## Detailed Crate Boundaries

### `velox-core` — Domain Foundation

```
Purpose:   Fundamental types shared by all crates
Layer:     Domain
Pure:      Yes — no I/O, no infra, no GPU
Depends:   Nothing (crate root of the dependency DAG)
Exports:   Tick, Quote, Candle, Order, OrderState, Fill, Position,
           Side, OrderType, TimeInForce, OrderId, NewOrder,
           CoreError, AccountInfo
Tests via: proptest (property-based)
```

### `velox-md` — Market Data Processing

```
Purpose:   Ring buffer, candle aggregation, market data pipeline
Layer:     Domain
Pure:      Mostly — no I/O, no GPU
Depends:   velox-core (types)
Uses:      crossbeam (RingBuffer atomic ops)
           tokio::sync (mpsc channel for pipeline output)
Exports:   RingBuffer<T>, MarketEvent, CandleAggregator, MarketDataPipeline
Tests via: proptest, criterion
```

### `velox-indicators` — Technical Indicators

```
Purpose:   Incremental O(1) technical indicators
Layer:     Domain
Pure:      Yes — pure math, zero I/O
Depends:   velox-core (Candle type)
Exports:   SMA, EMA, RSI, MACD, BollingerBands, ATR
Tests via: proptest, criterion
```

### `velox-oms` — Order Management System

```
Purpose:   Order state machine, order manager, fill management
Layer:     Domain
Pure:      Yes — no I/O, no GPU
Depends:   velox-core (Order, OrderState, Fill, Side)
Uses:      crossbeam (event dispatch for state changes)
Exports:   OrderManager, OrderStateMachine, FillManager, OrderError
Tests via: proptest (20+ unit tests + property-based)
```

### `velox-risk` — Risk Management

```
Purpose:   Pre-trade validation, position limits, circuit breaker
Layer:     Domain
Pure:      Yes — forbid(unsafe_code), no I/O
Depends:   velox-core (Order, Position)
           velox-oms (OrderState)
Uses:      dashmap (concurrent position tracking)
Exports:   RiskValidator, PositionLimits, CircuitBreaker
Tests via: proptest
```

### `velox-backtest` — Backtesting Engine

```
Purpose:   Historical replay, P&L calculation, strategy evaluation
Layer:     Application
Pure:      Mostly — no real I/O, replays ticks from disk
Depends:   velox-core, velox-md, velox-oms, velox-risk, velox-indicators
Exports:   BacktestEngine, ReplayPipeline, Metrics
Tests via: proptest, criterion
```

### `velox-scripting` — Lua Scripting

```
Purpose:   User-defined strategies via Lua
Layer:     Application
Depends:   velox-core, velox-indicators
Uses:      mlua (optional, feature-gated)
Exports:   ScriptEngine, LuaContext (sandboxed)
```

### `velox-broker` — Broker Port

```
Purpose:   BrokerClient trait definition + mock
Layer:     Port
Depends:   velox-core (Order, Position, AccountInfo)
Uses:      tokio (async trait), async-trait
Exports:   BrokerClient trait, MockBroker, BrokerConfig, ConnectionHandle
```

### `velox-exchange` — Exchange Adapter

```
Purpose:   ExchangeFeed trait + Binance WebSocket implementation
Layer:     Port + Adapter
Depends:   velox-core (Tick, Quote, CoreError)
           velox-md (RingBuffer, MarketEvent)
Uses:      tokio, tokio-tungstenite, futures-util, serde_json
Exports:   ExchangeFeed trait, BinanceFeed, ExchangeError
```

### `velox-broker-fix` — FIX Protocol Adapter

```
Purpose:   FIX protocol broker client
Layer:     Adapter
Depends:   velox-core (Order, OrderId, CoreError)
           velox-broker (BrokerClient trait)
Uses:      tokio, tracing
Status:    WIP — stub/adapter structure in place
```

### `velox-storage` — Time-Series Storage

```
Purpose:   Persistent storage for ticks, candles, orders
Layer:     Port + Adapter
Depends:   velox-core (Tick, Candle, Order)
Uses:      rusqlite (optional, feature-gated), rkyv (serialization)
Exports:   StorageEngine trait (future), tick/candle/order storage
Status:    WIP — basic engine stub, trait not yet extracted
```

### `velox-gpu` — GPU Infrastructure

```
Purpose:   wgpu device/queue/instance creation, shader compilation
Layer:     Infrastructure
Depends:   wgpu, glyphon, winit
Exports:   GpuDevice, GpuError, shader management
Tests:     — (hardware-dependant; tested via integration)
```

### `velox-chart` — Chart Rendering

```
Purpose:   Candle chart, grid, volume rendering via wgpu
Layer:     Adapter (GPU)
Depends:   velox-core (Candle), velox-md (Candle types)
           velox-indicators (overlays), velox-gpu (device)
Uses:      wgpu, bytemuck
Exports:   ChartRenderer, ChartInteraction, ChartView, OverlayManager
Tests via: criterion
```

### `velox-ui` — User Interface

```
Purpose:   Trading UI panels via egui
Layer:     Adapter (UI)
Depends:   velox-core, velox-md, velox-oms, velox-chart, velox-gpu
Uses:      egui, egui-wgpu, egui-winit, wgpu, crossbeam, tokio
Exports:   PanelManager, AppState, theme, panels (top bar, order entry,
           positions, status bar, chart area)
```

### `velox-terminal` — Composition Root

```
Purpose:   Binary entry point, wires all layers together
Layer:     Composition
Depends:   ALL crates (intentional)
Uses:      winit (event loop), wgpu (surface), egui-* (UI rendering)
           tokio (async runtime), pollster (block-on), fastrand (dithering)
Exports:   App struct, main(), input routing
```

---

## Dependency Graph (Verified)

```
velox-core (no deps)
  ├── velox-indicators → velox-core
  ├── velox-md → velox-core
  │     └── velox-exchange → velox-core, velox-md
  ├── velox-oms → velox-core
  │     ├── velox-risk → velox-core, velox-oms
  │     ├── velox-broker → velox-core
  │     │     └── velox-broker-fix → velox-core, velox-broker
  │     └── velox-backtest → velox-core, velox-md, velox-oms, velox-risk, velox-indicators
  ├── velox-storage → velox-core
  ├── velox-scripting → velox-core, velox-indicators
  └── velox-gpu (no workspace deps)
        └── velox-chart → velox-core, velox-md, velox-indicators, velox-gpu
              └── velox-ui → velox-core, velox-md, velox-oms, velox-chart, velox-gpu
                    └── velox-terminal → ALL (composition root)

(No cyclical dependencies — verified by cargo-deny)
```
