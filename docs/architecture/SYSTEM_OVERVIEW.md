# System Overview — velox-terminal

Hexagonal (ports & adapters) architecture + UNIX philosophy.

---

## Architecture Philosophy

Two principles guide the architecture:

### 1. Hexagonal (Ports & Adapters)

The trading domain is pure Rust — no I/O, no frameworks, no GPU. Domain crates don't know about WebSockets, databases, or rendering. They operate on plain data structures and return results.

Adapters implement traits (ports) defined by the domain. The composition root (`velox-terminal`) wires ports to adapters at startup.

### 2. UNIX Philosophy

Each stage does **one thing** and does it well:

| Stage | Responsibility | I/O? |
|-------|---------------|------|
| `velox-exchange` | Connect to exchange, parse wire protocol, push ticks | WebSocket (tokio) |
| `velox-md` | Aggregate ticks → OHLCV candles | None (pure) |
| `velox-indicators` | Compute SMA/EMA/RSI/MACD/Bollinger/ATR | None (pure) |
| `velox-oms` | Validate order state transitions | None (pure) |
| `velox-risk` | Pre-trade validation, circuit breakers | None (pure) |
| `velox-chart` | Render candles/overlays via wgpu | GPU (wgpu) |
| `velox-ui` | Build UI panels via egui | GPU (egui-wgpu) |
| `velox-terminal` | Wire everything, run event loop | All of the above |

Data flows through channels (crossbeam SPSC ring buffers, tokio mpsc). Each stage reads from its input channel, transforms the data, and writes to its output channel.

---

## Hexagonal Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        INFRASTRUCTURE LAYER                              │
│  ┌────────────────┐  ┌───────────────┐  ┌────────────────────────────┐  │
│  │  tokio runtime  │  │  wgpu GPU     │  │  crossbeam channels       │  │
│  │  (async I/O)    │  │  (Vulkan/Mtl) │  │  (lock-free SPSC)         │  │
│  └────────────────┘  └───────────────┘  └────────────────────────────┘  │
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    ADAPTER LAYER                                  │   │
│  │                                                                   │   │
│  │  ┌──────────────┐  ┌───────────┐  ┌────────┐  ┌───────────┐    │   │
│  │  │ velox-exchange│  │velox-brk │  │velox-  │  │ velox-ui  │    │   │
│  │  │ BinanceFeed  │  │FIX Adapter│  │chart   │  │ egui      │    │   │
│  │  └──────┬───────┘  └─────┬─────┘  └───┬────┘  └─────┬─────┘    │   │
│  │         │ implements     │ implements │        │ implements    │   │
│  │         ▼                ▼            ▼        ▼              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                        │              │           │                    │
│                        ▼              ▼           ▼                    │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      PORT LAYER (Traits)                         │   │
│  │                                                                   │   │
│  │  ExchangeFeed    BrokerClient    ChartPort    UIPort              │   │
│  │  (velox-exch)    (velox-broker)  (implicit)  (implicit)          │   │
│  │                                                                   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                        ▲              ▲           ▲                    │
│                        │              │           │                    │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                  DOMAIN / APPLICATION LAYER                       │   │
│  │                                                                   │   │
│  │  ┌──────────┐  ┌────────┐  ┌───────────┐  ┌──────────────┐     │   │
│  │  │velox-core │  │velox-md│  │ velox-oms  │  │ velox-risk   │     │   │
│  │  │ Tick,Ord, │  │RingBuf │  │ State      │  │ Validators   │     │   │
│  │  │ Candle    │  │Aggreg. │  │ Machine    │  │ CircuitBrk   │     │   │
│  │  └──────────┘  └────────┘  └───────────┘  └──────────────┘     │   │
│  │                                                                   │   │
│  │  ┌──────────────┐  ┌────────────┐  ┌───────────────┐           │   │
│  │  │velox-inds    │  │velox-strge │  │ velox-backtest│           │   │
│  │  │ SMA,EMA,RSI..│  │Engine trait│  │ Replay engine │           │   │
│  │  └──────────────┘  └────────────┘  └───────────────┘           │   │
│  │                                                                   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                               ▲
                               │ wires ports → adapters
                               │
                   ┌──────────────────────┐
                   │   COMPOSITION ROOT   │
                   │   velox-terminal     │
                   │   (App struct)       │
                   │                      │
                   │   - Creates GpuDevice │
                   │   - Creates BinanceFeed│
                   │   - Creates Pipeline  │
                   │   - Creates egui      │
                   │   - Runs event loop   │
                   └──────────────────────┘
```

---

## Ports & Adapters Table

| Port (Trait) | Defined In | Adapter Implementations | Consumed By |
|-------------|-----------|------------------------|-------------|
| `ExchangeFeed` | `velox-exchange::trait` | `BinanceFeed` (WebSocket, live) | `velox-terminal` (App) |
| `BrokerClient` | `velox-broker::client` | `MockBroker` (in-memory), `FixBrokerClient` (velox-broker-fix — WIP) | `velox-oms` (via composition) |
| `RiskValidator` | *(internal — `velox-risk::validators`)* | `RiskValidator` (position limits, circuit breaker) | `velox-oms` (order flow) |
| `StorageEngine` | `velox-storage::engine` | *(stub — SQLite planned)* | `velox-backtest` (historical data) |
| Chart Port | *(implicit — `velox-chart::renderer::ChartRenderer`)* | `ChartRenderer` (wgpu-based) | `velox-ui` (composite render) |
| UI Port | *(implicit — `velox-ui::panels::PanelManager`)* | `PanelManager` (egui-based) | `velox-terminal` (event loop) |

> **Note**: Not all ports have formal Rust traits. Some (Chart, UI) are implicit — the adapter *is* the implementation. This is pragmatic: these adapters are the only implementation and have no reason to be swapped. If a second charting backend emerges, extract `ChartPort` into a trait.

---

## Layer Rules (Enforced)

| Rule | Description | Violation Detection |
|------|-------------|-------------------|
| **Domain → Port only** | Domain crates may depend on trait definitions but never on adapter implementations | `cargo check --workspace` + `cargo-deny` |
| **Adapter → Port** | Adapters depend on the trait they implement + domain types | Cargo.toml dependency audit |
| **Domain never imports adapter** | No `extern crate velox_exchange` in velox-oms, velox-risk, etc. | grep for adapter crate names in domain crates |
| **Nothing depends on infra** | Domain crates don't depend on tokio, wgpu, egui | Cargo.toml dependency audit |
| **Composition root depends on everything** | `velox-terminal` depends on all crates | Intentional — it wires all layers |

### Actual dependency status (from Cargo.toml files)

| Crate | Depends on infra? | Infra deps | Passes rules? |
|-------|-------------------|------------|---------------|
| `velox-core` | No | None | ✅ |
| `velox-md` | No | crossbeam, tokio::sync | ✅ *(crossbeam is necessary for ring buffer — acknowledged exception)* |
| `velox-indicators` | No | None | ✅ |
| `velox-oms` | No | crossbeam | ✅ *(crossbeam for event dispatch — acknowledged)* |
| `velox-risk` | No | dashmap | ✅ *(dashmap for concurrent position tracking)* |
| `velox-broker` | Yes | tokio, async-trait | ✅ *(port crate needs async for trait)* |
| `velox-exchange` | Yes | tokio, tokio-tungstenite | ✅ *(adapter — connects to exchange)* |
| `velox-broker-fix` | Yes | tokio | ✅ *(adapter — FIX protocol I/O)* |
| `velox-storage` | No | None (rusqlite optional) | ✅ *(domain-level engine trait)* |
| `velox-gpu` | Yes | wgpu, winit, glyphon | ✅ *(infrastructure crate — GPU abstraction)* |
| `velox-chart` | Yes | wgpu, bytemuck | ✅ *(adapter — GPU rendering)* |
| `velox-ui` | Yes | egui, egui-wgpu, wgpu, tokio | ✅ *(adapter — UI framework)* |
| `velox-terminal` | Yes | tokio, winit, wgpu, egui-* | ✅ *(composition root)* |

---

## Event Bus Architecture

Central pub/sub system for cross-module communication without coupling.

```
┌─────────────────────────────────────────────────────────────────┐
│                        EVENT BUS                                  │
│  tokio::sync::broadcast::Sender<AppEvent>                        │
│                                                                   │
│  AppEvent enum:                                                   │
│    ├── Market(Tick | Quote | Candle | Depth)                     │
│    ├── Order(Submitted | Filled | Cancelled | Rejected)          │
│    ├── Account(BalanceChanged | PositionChanged)                  │
│    ├── Connection(Connected | Disconnected | Reconnecting)       │
│    ├── Risk(LimitBreached | CircuitTripped)                      │
│    ├── User(Command | Alert)                                     │
│    └── System(Shutdown | ConfigChanged)                          │
└──────────────────────┬──────────────────────────────────────────┘
         │                          ▲
         │ publish                  │ subscribe
    ┌────▼────┐              ┌──────┴──────┐
    │ Module  │              │  Module      │
    │ A       │────•────────►│  B (filter)  │
    └─────────┘   │          └─────────────┘
                  │    ┌──────────────┐
                  └───►│  Module C    │
                       └──────────────┘
```

**Implementation**: `tokio::sync::broadcast` with capacity 256. Hot-path events (ticks, fills) bypass the bus and go through dedicated channels to avoid broadcast overhead. The bus handles higher-level events: connection state, risk alerts, user commands, account updates.

**Hot path bridge**: Critical low-latency events (ticks, order fills) use dedicated lock-free channels (RingBuffer, crossbeam) and **optionally** publish a lightweight notification to the Event Bus for non-critical consumers (logging, UI indicators, alert system).

## Allocator Strategy

| Layer | Allocator | Rationale |
|-------|-----------|-----------|
| **Domain Core** (`velox-core`, `velox-oms`, `velox-risk`, `velox-indicators`) | System allocator | Minimal allocation; mostly stack data. Predictable, no behavioral changes. |
| **Adapters** (`velox-exchange`, `velox-chart`, `velox-ui`) | `mimalloc` | Heavy allocation from JSON parsing, GPU buffers, WebSocket messages. mimalloc reduces fragmentation and improves throughput under load. |
| **Hot path** (RingBuffer, aggregator) | Pre-allocated buffers | Zero allocation in steady state. Pre-allocated ring buffer slots, batched Vec reuse (`pop_n` with Vec drain). |

```toml
# Cargo.toml (workspace) — mimalloc for adapter crates
[dependencies]
mimalloc = { version = "0.1", optional = true }

[features]
default = ["adapter-allocator"]
adapter-allocator = ["velox-exchange/mimalloc", "velox-chart/mimalloc", "velox-ui/mimalloc"]
```

## Zero-Copy Strategy

| Path | Technique | Crate |
|------|-----------|-------|
| Network bytes → Tick/Quote | `#[repr(C)]` struct + `bytemuck::Pod` | `bytemuck` |
| IPC between threads | Zero-copy cast of `Pod` bytes | `bytemuck`, crossbeam |
| Persistence load | `rkyv` zero-copy deserialization | `rkyv` |
| GPU buffer upload | `bytemuck::cast_slice` from SoA arrays | `bytemuck` |
| String handling | `bytes::Bytes` for zero-copy borrow from network buffers | `bytes` |

**Rule**: No `serde_json` deserialization on hot path. Zero-copy parsing first; fall back to `serde_json` only for low-frequency endpoints (exchangeInfo, account details).

## Plugin System (Future)

Two-tier extensibility:

1. **Internal plugins** (Lua scripting): Sandboxed user strategies via `mlua`. Evaluated per-tick or per-candle. `velox-scripting` crate.
2. **Dynamic plugins** (cdylib): For advanced users/enterprise. Shared libraries loaded at runtime via `libloading`. Each plugin implements a C-ABI `Plugin` trait. Host and plugin must share Rust compiler version (ABI instability). WASM (via `wasmtime`) considered as safer alternative for untrusted plugins.

Design principle: The core doesn't know about plugins. Plugins observe events via the Event Bus and interact through well-defined port traits.

---

## UNIX Pipeline Analogy

```
stdin ──> filter1 ──> pipe ──> filter2 ──> pipe ──> filter3 ──> stdout
 │                                  │
 │    ┌─────────────────────────────┘
 │    │  Each filter:
 ▼    │  - Reads from stdin (input channel)
pipe  │  - Transforms data (pure function)
 │    │  - Writes to stdout (output channel)
 │    ▼
 │  No shared state. No side effects.
 │  Testable in isolation.
 ▼
Binance WS ──> ExchangeFeed ──> RingBuffer ──> CandleAggregator ──> mpsc ──> ChartRenderer
 (tokio)         (adapter)       (SPSC pipe)     (pure domain)       (pipe)   (adapter)
```

Each stage:
1. Is independently testable (mock its input, assert its output)
2. Has a single responsibility
3. Communicates via bounded channels (backpressure)
4. Can be replaced without changing neighboring stages

---

## Concurrency Model

```
┌────────────────────────────────────────────────────────────────────�-m────
│                         MAIN THREAD                                        │
│  ┌──────────────────────────────────────────────────────────────────┐      │
│  │  winit Event Loop                                                │      │
│  │  ┌────────────┐  ┌───────────┐  ┌──────────┐  ┌─────────────┐ │      │
│  │  │ poll_market│→│ PanelMgr  │→│ ChartRend│→│ composite   │ │      │
│  │  │ _data()    │  │ (egui)    │  │ (wgpu)   │  │ _render()   │ │      │
│  │  └─────┬──────┘  └───────────┘  └──────────┘  └─────────────┘ │      │
│  └────────┼────────────────────────────────────────────────────────┘      │
│           │ mpsc::UnboundedReceiver<Candle>                                │
│           │ (polled every frame, non-blocking)                             │
├───────────┼────────────────────────────────────────────────────-──────────┤
│  TOKIO    │                                                                │
│  THREADS  ▼                                                                │
│  ┌──────────────────────────────────────────────────────────────────┐      │
│  │  BinanceFeed::run_loop() — WebSocket I/O                        │      │
│  │  ┌────────────┐  ┌──────────────┐  ┌────────────────────────┐  │      │
│  │  │connect_async│→│ read stream  │→│ handle_message → push  │  │      │
│  │  │ (WS)        │  │ (tokio task) │  │ to RingBuffer         │  │      │
│  │  └────────────┘  └──────────────┘  └───────────┬────────────┘  │      │
│  └─────────────────────────────────────────────────┼────────────────┘      │
│                                                     │ RingBuffer (SPSC)     │
│                                                     │ lock-free, no wait    │
│  ┌──────────────────────────────────────────────────┼────────────────────┐  │
│  │  MarketDataPipeline (polled from main thread)    │                    │  │
│  │  reads RingBuffer → aggregate → send via mpsc    │                    │  │
│  └──────────────────────────────────────────────────┘                    │  │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Key Architectural Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Domain crates depend only on `velox-core` | Zero I/O debt — testable at native speed |
| 2 | Ring buffers for hot path (ticks) | Lock-free SPSC, 1μs p50 latency |
| 3 | `pop_n` batching for ring buffer consumption | 2 atomics per batch (vs 3 per tick). Reuses Vec allocation. |
| 4 | mpsc channels for cross-thread candles | Tokio unbounded channel — tradeoff: safe, simple, bounded by frame rate |
| 5 | `unsafe` for egui-wgpu `RenderPass<'static>` | egui-wgpu API requires it; proven safe pattern; never stores the ref |
| 6 | Ports co-located with adapters (ExchangeFeed in velox-exchange) | Pragmatic: only one implementation exists; extract if second appears |
| 7 | Event Bus via `tokio::sync::broadcast` | Decoupled pub/sub for non-critical events. Hot path bypasses bus via dedicated channels. |
| 8 | mimalloc for adapters, system allocator for domain core | Adapters allocate heavily (JSON, GPU buffers); domain core is mostly stack. |
| 9 | Zero-copy via bytemuck/rkyv on hot path | No serde deserialization on tick/quote paths. `#[repr(C)]` structs for safe casting. |
| 10 | `velox-terminal` imports all crates | Composition root: intentionally depends on everything to wire it |

---

## What Makes This Hexagonal?

1. **Domain has no framework imports**: `velox-core`, `velox-oms`, `velox-risk`, `velox-indicators` don't import tokio, wgpu, or egui. They compile to any environment.

2. **Adapters are swappable**: Replace `BinanceFeed` with `KrakenFeed` — no domain code changes. Replace `MockBroker` with `FixBrokerClient` — no OMS changes.

3. **Composition root owns wiring**: `App::new()` in `velox-terminal` creates all adapters and injects them. No adapter is ever instantiated by domain code.

4. **Tests test the domain, not the framework**: OMS tests create orders directly. Indicator tests feed raw numbers. No WebSocket, no GPU, no egui.

---

## Crate Dependency Graph (Hexagonal View)

```
                    ┌──────────────────┐
                    │   velox-terminal  │  Composition Root
                    └────┬──────┬───────┘
                         │      │
              ┌──────────┘      └──────────┐
              ▼                             ▼
    ┌──────────────────┐         ┌──────────────────┐
    │   ADAPTERS        │         │   DOMAIN          │
    │                   │         │                   │
    │ velox-exchange    │         │ velox-core        │
    │ velox-broker-fix  │    ┌───►│ velox-oms         │
    │ velox-chart       │    │    │ velox-risk        │
    │ velox-ui          │    │    │ velox-indicators  │
    │ velox-storage     │────┤    │ velox-md          │
    │ velox-gpu         │    │    │ velox-storage     │
    └──────────────────┘    │    │ velox-backtest    │
                            │    │ velox-scripting   │
                            │    └──────────────────┘
                            │
                            │    ┌──────────────────┐
                            └───►│   PORTS           │
                                 │ velox-broker      │ (BrokerClient trait)
                                 │ velox-exchange    │ (ExchangeFeed trait)
                                 └──────────────────┘
```

> **Key insight**: Arrows point in the direction of dependency. Domain never points to Adapter. Adapters depend on Domain (types) and Ports (traits). Ports depend only on Domain types.
