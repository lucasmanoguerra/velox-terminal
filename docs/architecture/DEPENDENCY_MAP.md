# Hexagonal Dependency Map — velox-terminal

Complete dependency map between crates, organized by hexagonal layer.

---

## Hexagonal Layering Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         COMPOSITION ROOT                                │
│  velox-terminal (App struct, main, input routing)                       │
│  Depends on: ALL crates (intentional wiring)                            │
└─────────────────────────────────────────────────────────────────────────┘
                                ▲
                    ┌───────────┴───────────┐
                    │                       │
                    ▼                       ▼
┌───────────────────────────────┐ ┌───────────────────────────────┐
│       ADAPTERS                │ │       DOMAIN                   │
│                               │ │                               │
│  velox-exchange  (Binance)    │ │  velox-core  (types)          │
│  velox-broker-fix (FIX)      │ │  velox-md    (ring/aggregate) │
│  velox-chart     (wgpu)       │ │  velox-indicators (math)      │
│  velox-ui        (egui)       │ │  velox-oms   (state machine)  │
│  velox-storage   (SQLite)     │ │  velox-risk  (validation)     │
│  velox-gpu       (infra)      │ │  velox-backtest (replay)      │
│                               │ │  velox-scripting (Lua)        │
└───────────────────────────────┘ └───────────────────────────────┘
                                ▲
                                │
                    ┌───────────┴───────────┐
                    │         PORTS          │
                    │                       │
                    │  velox-broker (BrokerClient trait)  │
                    │  velox-exchange (ExchangeFeed trait)│
                    └─────────────────────────────────────┘
```

---

## Domain Layer (Zero Infra Dependencies)

### `velox-core`
```
Purpose:     Domain types shared by all crates
Depends on:  Nothing (root of the DAG)
Used by:     EVERY other crate
┌──────────────────────────────────────┐
│  velox-core                          │
│                                      │
│  market.rs:    Tick, Quote, Candle   │
│  order.rs:     Order, OrderState,    │
│                OrderId, Fill, Side,  │
│                OrderType, TimeInForce │
│  types.rs:     Position, AccountInfo │
│  error.rs:     CoreError             │
└──────────────────────────────────────┘
```

### `velox-md`
```
Purpose:     Market data processing
Depends on:  velox-core (Tick, Candle, MarketEvent)
             crossbeam (RingBuffer atomic ops)
             tokio::sync (mpsc channel)
Used by:     velox-exchange, velox-chart, velox-ui, velox-backtest
┌───────────────────────────────────────────┐
│  velox-md                                 │
│                                           │
│  ring_buffer.rs:  RingBuffer<T>,         │
│                   MarketEvent             │
│  aggregation.rs:  CandleAggregator        │
│  pipeline.rs:     MarketDataPipeline      │
│  feed.rs:         (reserved)              │
└───────────────────────────────────────────┘
```

### `velox-indicators`
```
Purpose:     Technical indicators
Depends on:  velox-core (Candle)
Used by:     velox-chart, velox-backtest, velox-scripting
┌──────────────────────────────────────┐
│  velox-indicators                    │
│                                      │
│  sma.rs:     SMA (O(1) incremental)  │
│  ema.rs:     EMA (O(1) incremental)  │
│  rsi.rs:     RSI (O(1) incremental)  │
│  macd.rs:    MACD (O(1) incremental) │
│  bollinger.rs: Bollinger Bands (O(1)) │
│  atr.rs:     ATR (O(1) incremental)  │
└──────────────────────────────────────┘
```

### `velox-oms`
```
Purpose:     Order management
Depends on:  velox-core (Order, OrderState, Fill, Side)
             crossbeam (event dispatch)
Used by:     velox-risk, velox-ui, velox-backtest
┌──────────────────────────────────────┐
│  velox-oms                           │
│                                      │
│  state_machine.rs: OrderStateMachine │
│  order_manager.rs:  OrderManager     │
│  error.rs:          OrderError       │
└──────────────────────────────────────┘
```

### `velox-risk`
```
Purpose:     Risk management
Depends on:  velox-core (Order, Position)
             velox-oms (OrderState types)
             dashmap (concurrent position tracking)
Used by:     velox-oms, velox-backtest
┌──────────────────────────────────────┐
│  velox-risk                          │
│                                      │
│  validators.rs:     RiskValidator    │
│  limits.rs:         PositionLimits   │
│  circuit_breaker.rs: CircuitBreaker  │
│  error.rs:          RiskError        │
└──────────────────────────────────────┘
```

### `velox-backtest`
```
Purpose:     Backtesting engine
Depends on:  velox-core (Tick, Candle, Order)
             velox-md (CandleAggregator)
             velox-oms (OrderManager)
             velox-risk (RiskValidator)
             velox-indicators (indicators)
Used by:     velox-terminal (via composition)
┌──────────────────────────────────────┐
│  velox-backtest                      │
│                                      │
│  engine.rs:  BacktestEngine          │
│  slippage.rs: SlippageModel          │
│  metrics.rs: PnL, Sharpe, Drawdown   │
└──────────────────────────────────────┘
```

### `velox-scripting`
```
Purpose:     Lua scripting for user strategies
Depends on:  velox-core (Tick, Candle, Order)
             velox-indicators (indicator values)
             mlua (optional, feature-gated)
Used by:     velox-terminal (via composition)
┌──────────────────────────────────────┐
│  velox-scripting                     │
│                                      │
│  engine.rs:  ScriptEngine            │
│  context.rs: Sandboxed LuaContext    │
└──────────────────────────────────────┘
```

---

## Port Layer (Traits)

### `velox-broker`
```
Purpose:     BrokerClient trait definition
Layer:       Port
Depends on:  velox-core (Order, OrderId, Position, AccountInfo, CoreError)
             tokio (async trait), async-trait crate
Used by:     velox-broker-fix (implements), velox-oms (calls via trait),
             velox-terminal (wires)
┌──────────────────────────────────────┐
│  velox-broker                        │
│                                      │
│  client.rs:  BrokerClient trait      │
│              BrokerConfig struct     │
│              ConnectionHandle struct │
│  error.rs:   BrokerError             │
│  mock.rs:    MockBroker (impl)       │
└──────────────────────────────────────┘
```

Port definition:
```rust
#[async_trait]
pub trait BrokerClient: Send + Sync {
    async fn connect(&self, config: BrokerConfig) -> Result<ConnectionHandle, CoreError>;
    async fn disconnect(&self, handle: &ConnectionHandle) -> Result<(), CoreError>;
    async fn submit_order(&self, order: NewOrder) -> Result<OrderId, CoreError>;
    async fn cancel_order(&self, order_id: OrderId) -> Result<(), CoreError>;
    async fn get_positions(&self) -> Result<Vec<Position>, CoreError>;
    async fn get_account_info(&self) -> Result<AccountInfo, CoreError>;
}
```

### `velox-exchange` (co-located port + adapter)
```
Purpose:     ExchangeFeed trait + BinanceFeed implementation
Layer:       Port + Adapter
Depends on:  velox-core (Tick, Quote, CoreError)
             velox-md (RingBuffer, MarketEvent)
             tokio, tokio-tungstenite, serde_json
Used by:     velox-terminal (wires)

NOTE: Trait and adapter are co-located because there's currently only
one implementation (BinanceFeed). If a second exchange (Kraken, Coinbase)
is added, the trait should be extracted to a standalone port crate.
```

Port definition:
```rust
pub trait ExchangeFeed: Send + Sync {
    fn start(&self, ring: Arc<RingBuffer>) -> Result<(), CoreError>;
    fn stop(&self) -> Result<(), CoreError>;
    fn subscribe(&self, symbol: &str) -> Result<(), CoreError>;
    fn unsubscribe(&self, symbol: &str) -> Result<(), CoreError>;
}
```

---

## Adapter Layer (Implementations)

### `velox-exchange` (adapter part)
```
Purpose:     Binance WebSocket market data feed
Layer:       Adapter
Implements:  ExchangeFeed trait (co-located)
Depends on:  velox-core (Tick, CoreError)
             velox-md (RingBuffer, MarketEvent)
             tokio, tokio-tungstenite, futures-util, serde_json
Provides:    BinanceFeed — real-time trade ticks via WebSocket
             Auto-reconnect with exponential backoff + full jitter
             Symbol normalization (BTC-USDT → btcusdt)
```

### `velox-broker-fix` (FIX adapter)
```
Purpose:     FIX protocol broker client
Layer:       Adapter
Implements:  BrokerClient trait (from velox-broker)
Depends on:  velox-core (Order, OrderId, CoreError)
             velox-broker (BrokerClient trait, BrokerConfig, etc.)
             tokio
Status:      WIP — structure in place, implementation pending
```

### `velox-chart`
```
Purpose:     GPU-accelerated candlestick chart rendering
Layer:       Adapter
Depends on:  velox-core (Candle types)
             velox-md (Candle types)
             velox-indicators (indicator overlay values)
             velox-gpu (GpuDevice, shaders)
             wgpu, bytemuck
Provides:    ChartRenderer — renders candles/grid/volume via wgpu
             ChartInteraction — zoom/pan (GPU-side, vertex shader)
             OverlayManager — indicator line overlays (future)
```

### `velox-ui`
```
Purpose:     Trading UI panels via egui (immediate mode)
Layer:       Adapter
Depends on:  velox-core (Tick, Candle, Order types)
             velox-md (candle data for display)
             velox-oms (order state for display)
             velox-chart (ChartRenderer for composite render)
             velox-gpu (GpuDevice)
             egui, egui-wgpu, egui-winit, wgpu, crossbeam, tokio
Provides:    PanelManager — top bar, order entry, chart area, positions, status bar
             AppState — shared mutable state (main thread only, no locks)
             Theme — dark trading professional palette
```

### `velox-storage`
```
Purpose:     Time-series data storage
Layer:       Adapter (will implement StorageEngine port)
Depends on:  velox-core (Tick, Candle, Order)
             rkyv (serialization)
             rusqlite (optional, feature-gated)
Status:      WIP — basic StorageEngine stub, trait not yet extracted
```

### `velox-gpu`
```
Purpose:     GPU abstraction layer
Layer:       Infrastructure / Adapter helper
Depends on:  wgpu, glyphon, winit
Provides:    GpuDevice — wraps wgpu::Instance, Device, Queue
             Shader compilation helpers
             Not a port — it's infrastructure shared by chart and UI
```

---

## Composition Root

### `velox-terminal`
```
Purpose:     Binary entry point, wiring everything together
Layer:       Composition
Depends on:  ALL workspace crates (intentional — it's the wiring point)
             + winit, egui, egui-winit, egui-wgpu, wgpu, tokio,
               pollster, clap, fastrand, tracing-subscriber
Provides:    App struct — owns window, GPU, chart, UI, pipeline, feed
             main() — CLI parsing, tokio runtime, event loop
             input.rs — winit → egui/chart event routing
```

`App::new()` wiring:
```rust
// In velox-terminal/src/app.rs (composition root):
pub fn new(event_loop: &EventLoop<()>) -> Result<Self> {
    // 1. Create window
    // 2. Create GpuDevice (wgpu instance + device + queue)
    // 3. Create surface from window
    // 4. Create RingBuffer + MarketDataPipeline (domain)
    // 5. Create BinanceFeed (adapter), subscribe, start
    // 6. Create ChartRenderer (adapter, depends on GPU)
    // 7. Create AppState (adapter, connected to pipeline output)
    // 8. Create egui context + state + renderer (adapter, depends on GPU)
    // 9. Run winit event loop
}
```

---

## Compile-Time Dependency Graph

```
velox-core (zero workspace deps)
  ├── velox-gpu (zero workspace deps — wgpu, glyphon, winit)
  │
  ├── velox-broker ─── velox-broker-fix
  │       │                │
  │       │                └── (depends on velox-core + velox-broker)
  │       │
  │       └── velox-scripting
  │
  ├── velox-md ─── velox-exchange
  │       │          │
  │       │          └── (depends on velox-core + velox-md)
  │       │
  │       ├── velox-chart ─── velox-ui
  │       │       │              │
  │       │       │              └── (depends on velox-core, -md, -oms, -chart, -gpu)
  │       │       │
  │       │       └── (depends on velox-core, -md, -indicators, -gpu)
  │       │
  │       └── velox-backtest
  │
  ├── velox-indicators ─── velox-chart, velox-scripting, velox-backtest
  │
  ├── velox-oms ─── velox-risk
  │       │           │
  │       │           └── (depends on velox-core + velox-oms)
  │       │
  │       ├── velox-ui
  │       │
  │       └── velox-backtest
  │
  └── velox-storage

velox-terminal (depends on ALL — composition root)

(No cycles — verified by cargo-deny)
```

---

## Data Dependency Table

| Data Type | Produced By | Consumed By | Layer Transitions | Format |
|-----------|------------|-------------|-------------------|--------|
| `Tick` | `velox-exchange` (BinanceFeed) | `velox-md` (CandleAggregator) | Adapter → Domain | `MarketEvent::Tick` via RingBuffer |
| `Candle` | `velox-md` (CandleAggregator) | `velox-ui` (AppState) → `velox-chart` (ChartRenderer) | Domain → Adapter | `mpsc::UnboundedSender<Candle>` |
| `Order` | `velox-ui` (user input) → `velox-oms` (OrderManager) | `velox-risk` (validation), `velox-broker` (send) | Adapter → Domain → Port | `velox_core::Order` |
| `Fill` | `velox-oms` (FillManager) | `velox-ui` (display) | Domain → Adapter | `velox_core::Fill` |
| `Position` | `velox-oms` (derived from fills) | `velox-ui`, `velox-risk` | Domain → Adapter + Domain | `velox_core::Position` |
| `Indicator value` | `velox-indicators` | `velox-chart` (overlay) | Domain → Adapter | `f64` / `Vec<f64>` |
| Historical ticks | `velox-storage` | `velox-backtest` | Adapter → Domain | `velox_core::Tick` (rkyv) |
| User command | `velox-ui` (PanelManager) | `velox-oms` (OrderManager) | Adapter → Domain | crossbeam channel enum |
| `MarketEvent` | `velox-exchange` (BinanceFeed) | `velox-md` (MarketDataPipeline) | Adapter → Domain | RingBuffer (lock-free SPSC) |

---

## Dependency Inversion Examples

The hexagonal architecture uses dependency inversion at these points:

### Example 1: Broker (Port)
```
Without hexagonal:  velox-oms → velox-broker-fix (direct coupling)
With hexagonal:     velox-oms → BrokerClient trait (port)
                    velox-broker-fix → BrokerClient (implements)
                    velox-terminal wires: OMS → MockBroker (dev)
                                          OMS → FixBrokerClient (prod)
```

### Example 2: Exchange Feed (Port + Adapter)
```
Without hexagonal:  velox-md → velox-exchange (direct coupling)
With hexagonal:     velox-exchange → ExchangeFeed trait + RingBuffer
                    velox-md ← RingBuffer (data, not dependency)
                    velox-terminal wires: Pipeline → BinanceFeed
```

### Example 3: Storage (future)
```
Without hexagonal:  velox-backtest → SQLite (direct coupling)
With hexagonal:     velox-storage → StorageEngine trait
                    velox-backtest → StorageEngine (via trait)
                    velox-terminal wires: Backtest → SQLiteStorage,
                                          Backtest → InMemoryStorage (tests)
```

---

## Impact Analysis Quick Reference

| If you change... | Check these dependents | Layer impact |
|-----------------|-----------------------|-------------|
| `velox_core::Tick` | `velox-md`, `velox-exchange`, `velox-indicators`, `velox-storage` | Domain types — affects all layers |
| `velox_core::Order` | `velox-oms`, `velox-risk`, `velox-broker`, `velox-ui`, `velox-storage` | Domain types — affects all layers |
| `BrokerClient` trait | `velox-broker-fix` (impl), `velox-oms` (caller), `velox-terminal` (wiring) | Port change — 2 adapters + 1 domain |
| `ExchangeFeed` trait | `velox-exchange::binance` (impl), `velox-terminal` (wiring) | Port change — 1 adapter + composition |
| `RingBuffer` struct | `velox-exchange` (push), `velox-md` (pop) | Data structure — affects producer + consumer |
| `CandleAggregator` | `velox-md::pipeline`, `velox-backtest` | Domain logic — affects aggregation |
| `ChartRenderer` pipeline | `velox-gpu` shaders, `velox-ui` composite render | Adapter internal — contained |
| `PanelManager` layout | `velox-ui` panels, `velox-terminal` App | Adapter internal — contained |
| `AppState` fields | `velox-ui` panels, `velox-terminal` App | Adapter — affects UI + composition |
