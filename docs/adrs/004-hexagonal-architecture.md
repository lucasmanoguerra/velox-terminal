# ADR-004: Hexagonal (Ports & Adapters) Architecture

| | |
|---|---|
| **ADR** | 004 |
| **Title** | Hexagonal (Ports & Adapters) Architecture |
| **Status** | Accepted |
| **Date** | 2026-07-08 |
| **Author** | systems-architect |

## Context

The project currently has 15 crates organized by domain with some already following a port-adapter-like separation (e.g., `velox-broker` trait + `velox-broker-fix` implementation; `velox-exchange` trait + `BinanceFeed`). However, this is applied opportunistically rather than as a structured principle. This leads to:

1. **Tight coupling between domain logic and infrastructure** — OMS depends on concrete broker types in places; charting engine is inseparably tied to wgpu; UI panels depend directly on egui internals.
2. **Testing friction** — No systematic way to swap real infrastructure for test doubles. Mocking requires ad-hoc trait extraction after implementation.
3. **Portability risk** — If we need to swap the renderer (Vulkan→Metal), storage engine (sled→redb), or GUI framework (egui→dear imgui), the blast radius is undefined.
4. **Blurred domain boundaries** — Without explicit ports, it's unclear where market data processing ends and broker connectivity begins.

**Forces**:
- Hard real-time GPU rendering (wgpu) requires zero-cost abstraction — trait dispatch on the render path is unacceptable.
- Ring buffer tick ingestion is a hot path (~1µs per tick) — pointer indirection via dyn traits would add measurable latency.
- OMS/Risk are zero-unsafe, zero-infrastructure pure domain logic — ideal candidates for strict hexagonal isolation.
- The project already has traits that look like ports (`BrokerClient`, `ExchangeFeed`, `Indicator`) — we formalize, not replace.
- Rust's generics with monomorphization allow zero-cost abstraction where trait dispatch *would* cost — we can have both.

## Decision

Adopt **Hexagonal (Ports & Adapters) Architecture** as the high-level structuring principle for the workspace, layered on top of the existing crate organization.

### Definitions

| Term | Meaning | Rust Equivalent |
|------|---------|-----------------|
| **Port** | A boundary interface defining a domain capability | `trait` (in domain crate) |
| **Adapter** | A concrete implementation of a port using infrastructure | `struct` + `impl Trait for Struct` (in adapter crate) |
| **Domain Core** | Pure business logic with zero external dependencies | Crates with `#![forbid(unsafe_code)]`, no I/O, no tokio/wgpu |
| **Infrastructure** | I/O, network, GPU, storage, OS-specific code | Crates that depend on tokio, wgpu, crossbeam, etc. |

### Crate Classification

Every crate in the workspace is classified into exactly one of three layers:

```
┌──────────────────────────────────────────────────────────────────┐
│                    ADAPTERS (Infrastructure)                      │
│                                                                   │
│  velox-exchange  velox-broker-fix  velox-gpu  velox-ui           │
│  velox-terminal  velox-scripting   velox-storage                  │
│                                                                   │
│  Depend on: tokio, wgpu, egui, crossbeam, OS APIs                │
│  Implement ports defined in domain/application crates             │
└────────────────────────┬─────────────────────────────────────────┘
                         │  depends on ports & core types
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│                 APPLICATION (Use Cases / Orchestration)           │
│                                                                   │
│  velox-chart   velox-md    velox-backtest                         │
│                                                                   │
│  Orchestrates domain logic with infrastructure adapters.          │
│  Defines application-level ports (e.g., ChartRendererPort,       │
│  MarketDataPort). Contains no infrastructure imports directly.    │
└────────────────────────┬─────────────────────────────────────────┘
                         │  depends on core types + ports
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│                  DOMAIN CORE (Pure Business Logic)                │
│                                                                   │
│  velox-core    velox-oms    velox-risk    velox-indicators        │
│                                                                   │
│  Zero external dependencies. #![forbid(unsafe_code)].             │
│  No I/O, no tokio, no wgpu, no egui. Pure Rust.                  │
│  Defines ports as traits where the domain needs an                 │
│  external capability (e.g., OrderExecutionPort for sending        │
│  orders to a broker).                                             │
└──────────────────────────────────────────────────────────────────┘
```

### Port/Adapter Boundaries Per Crate

#### Domain Core (Ports defined here)

| Crate | Ports (Traits) | Why a port? |
|-------|---------------|-------------|
| `velox-core` | — (pure types only) | Types are the universal language; no behavior to abstract. |
| `velox-oms` | `OrderExecutionPort` | OMS needs to send orders somewhere (broker, backtest, paper) without knowing where. |
| `velox-risk` | `MarketDataPort` | Risk validators need current prices/mark-to-market without depending on feed crate. |
| `velox-indicators` | `Indicator<T>` (already exists) | Generic over f32/f64 price input; works identically in live and backtest. |

```rust
// Domain port in velox-oms
/// Port: how orders are sent to the execution venue.
/// The domain defines it; adapters (broker, backtest, paper) implement it.
pub trait OrderExecutionPort: Send + Sync {
    fn submit(&self, order: NewOrder) -> Result<OrderId, OmsError>;
    fn cancel(&self, order_id: OrderId) -> Result<(), OmsError>;
    fn replace(&self, order_id: OrderId, modify: OrderModification) -> Result<OrderId, OmsError>;
}
```

```rust
// Domain port in velox-risk
/// Port: how risk obtains current market prices.
pub trait MarketDataPort {
    fn last_price(&self, symbol: &Symbol) -> Result<Price, RiskError>;
    fn mark_to_market(&self, positions: &[Position]) -> Result<PortfolioValuation, RiskError>;
}
```

#### Application (Ports defined here)

| Crate | Ports (Traits) | Why a port? |
|-------|---------------|-------------|
| `velox-chart` | `ChartRendererPort` | Chart engine needs a GPU surface to draw on; wgpu is one adapter. |
| `velox-md` | `FeedIngestionPort`, `StoragePort` | Market data pipeline needs input source + output sink. |
| `velox-backtest` | `SimulationClockPort`, `FillModelPort` | Backtest needs pluggable clock + fill/slippage models. |

```rust
// Application port in velox-chart
/// Port: GPU-accelerated chart rendering capability.
/// Adapters: wgpu-based renderer, software fallback (testing), screenshot.
pub trait ChartRendererPort {
    fn render(&mut self, view: &ChartView, candles: &[Candle]) -> Result<(), ChartError>;
    fn resize(&mut self, width: u32, height: u32);
    fn take_screenshot(&self) -> Vec<u8>;
}
```

#### Adapters (Implement the ports)

| Crate | Implements | Infrastructure Used |
|-------|-----------|-------------------|
| `velox-broker` | `OrderExecutionPort` | tokio, reqwest |
| `velox-broker-fix` | `OrderExecutionPort` | tokio, fefix |
| `velox-exchange` | Market data sources | tokio, tokio-tungstenite |
| `velox-gpu` | `ChartRendererPort` (indirectly via velox-chart) | wgpu, WGSL, glyphon |
| `velox-ui` | UI rendering (egui adapter over wgpu) | egui, egui-wgpu, egui-winit |
| `velox-storage` | `StoragePort` | redb/sled, rkyv |
| `velox-backtest` | `FillModelPort` (slippage/commission) | rayon |

```rust
// Adapter in velox-broker — implements domain port
pub struct BrokerAdapter {
    client: Arc<dyn BrokerClient>,
}

impl OrderExecutionPort for BrokerAdapter {
    fn submit(&self, order: NewOrder) -> Result<OrderId, OmsError> {
        // Translates domain → broker-specific, calls async under the hood
        self.client.blocking_submit(order)
    }
}
```

### Rules for Trait vs Direct Call (Hot Path Exceptions)

This is the critical section. Not everything needs a port. The following rules govern when to use a trait boundary vs direct call.

**Rule 1 — Hot paths use zero-cost abstraction (generics, not `dyn`).**

```rust
// ✅ ACCEPTABLE: Generic over port — zero-cost via monomorphization
pub struct OrderManager<P: OrderExecutionPort> {
    port: P,
    // ...
}

// ❌ AVOIDED ON HOT PATHS: dyn dispatch adds indirection and prevents inlining
pub struct OrderManager {
    port: Box<dyn OrderExecutionPort>,  // only acceptable for cold paths
    // ...
}
```

**Rule 2 — Render path bypasses port abstraction entirely.**

The GPU render path (`velox-chart` → `velox-gpu`) is exempt from port abstraction. The chart renderer calls wgpu directly because:

- Each frame is ~60 function calls to wgpu — dyn dispatch on every call adds measurable overhead.
- `ChartRenderer` is already the single implementation; swapping it would mean rewriting wgpu → Vulkan, which is a full crate replacement anyway.
- GPGPU/interop patterns require concrete types (`wgpu::Device`, `wgpu::Queue`, `wgpu::BindGroup`) that don't admit meaningful abstraction without massive complexity.

```rust
// ✅ DIRECT CALL (expected on render path)
impl ChartRenderer {
    pub fn render(&mut self, surface_texture: &wgpu::TextureView) {
        // Direct wgpu calls — no port indirection
        let mut encoder = self.device.create_command_encoder(...);
        let mut pass = encoder.begin_render_pass(...);
        pass.set_pipeline(&self.pipeline);
        pass.draw_indexed(0..self.num_indices, 0, 0..self.num_candles);
    }
}
```

**Rule 3 — Ring buffer / tick pipeline uses concrete types.**

The tick ingestion path (WebSocket → ring buffer → candle aggregation) is classified as infrastructure and does not require ports. The `RingBuffer` is a concrete `crossbeam::SegQueue` wrapper and is used directly. Cross-crate boundaries at this layer pass `Arc<RingBuffer>` directly.

**Rule 4 — OMS/Risk state machines are pure domain — no ports needed internally.**

The internal state machines (`OrderState`, `RiskValidator`) contain no I/O and no references to adapters. They operate entirely on domain types from `velox-core`. The *only* ports are at the boundary where OMS needs to send an order (to broker/backtest) or risk needs a price (from market data).

```
                          Port boundary (Rule 4)
                    ┌──────────────────────────────┐
                    │        DOMAIN CORE            │
                    │  ┌──────────────────────┐    │
                    │  │  OrderStateMachine   │    │  ← Pure enum transitions,
                    │  │  RiskValidator       │    │     no I/O, no traits
                    │  └──────────────────────┘    │
                    │         │                     │
                    │         │ OrderExecutionPort  │
                    │         ▼ (trait)             │
                    │  ┌──────────────────────┐    │
                    │  │  OrderManager<P>     │    │  ← Generic over port
                    │  └──────────────────────┘    │
                    └──────────────────────────────┘
                               │
                    ┌──────────▼───────────┐
                    │   BrokerAdapter      │  ← Implements port, has I/O
                    │   (tokio + HTTP/FIX) │
                    └──────────────────────┘
```

**Rule 5 — Testing adapters must exist for every port.**

Each port must have at least two implementations: one production, one test (mock or fake). Test implementations live inside the crate that defines the port (as `pub mod testing`) or in a companion crate:

```rust
// In velox-oms/src/testing.rs
pub struct MockOrderExecutionPort;

impl OrderExecutionPort for MockOrderExecutionPort {
    fn submit(&self, order: NewOrder) -> Result<OrderId, OmsError> {
        Ok(OrderId::from(42))  // Simulated success
    }
}
```

### Dependency Flow Diagram

```
 ┌────────────────────────────────────────────────────────────────────┐
 │                       velox-terminal (binary)                       │
 │  Wires: storage adapter → market data pipeline → chart renderer     │
 │         → UI panel manager; creates runtime, event loop             │
 └────┬───────────────────────────────────────────────────────────────┘
      │ depends on any crate
      ▼
┌──────────────────────────────────────────────────────────────────────┐
│  ADAPTERS                                                           │
│                                                                     │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
│  │ velox-broker │   │ velox-gpu    │   │ velox-ui     │            │
│  │ (tokio, HTTP)│   │ (wgpu, WGSL) │   │ (egui, winit)│            │
│  └──────┬───────┘   └──────┬───────┘   └──────┬───────┘            │
│         │ implements       │ implements       │ depends on          │
│         ▼                  ▼                  ▼                    │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
│  │ velox-exchg  │   │ velox-stor   │   │ velox-scrpt  │            │
│  │ (tungstenite)│   │ (redb/rkyv)  │   │ (mlua)       │            │
│  └──────┬───────┘   └──────┬───────┘   └──────────────┘            │
│         │ implements       │ implements                             │
│         ▼                  ▼                                        │
└─────────┼──────────────────┼────────────────────────────────────────┘
          │                  │
          ▼                  ▼
┌──────────────────────────────────────────────────────────────────────┐
│  APPLICATION (Orchestration)                                        │
│                                                                     │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
│  │ velox-chart  │   │ velox-md     │   │ velox-bcktest│            │
│  │ (overlays)   │   │ (aggregation)│   │ (sim engine) │            │
│  └──────┬───────┘   └──────┬───────┘   └──────────────┘            │
│         │ depends on       │ depends on                             │
│         ▼                  ▼                                        │
└─────────┼──────────────────┼────────────────────────────────────────┘
          │                  │
          ▼                  ▼
┌──────────────────────────────────────────────────────────────────────┐
│  DOMAIN CORE                                                        │
│                                                                     │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
│  │ velox-core   │   │ velox-oms    │   │ velox-risk   │            │
│  │ (types only) │   │ (state mach) │   │ (validators) │            │
│  └──────────────┘   └──────┬───────┘   └──────┬───────┘            │
│                            │ defines ports    │ defines ports       │
│                            │ OrderExecution   │ MarketData          │
│                            └──────────────────┘                     │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │ velox-indicators                                             │   │
│  │ (SMA, EMA, RSI, MACD, Bollinger, ATR — zero deps, pure)     │   │
│  └──────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
```

### Crate Dependency Map (enforced at Cargo.toml level)

```
DOMAIN CORE (no I/O deps):
  velox-core          → (none)
  velox-oms           → velox-core
  velox-risk          → velox-core
  velox-indicators    → velox-core

APPLICATION:
  velox-chart         → velox-core, velox-gpu (GPU is adapter)
  velox-md            → velox-core, velox-indicators
  velox-backtest      → velox-core, velox-oms, velox-risk, velox-md

ADAPTERS:
  velox-broker        → velox-core
  velox-broker-fix    → velox-core, velox-broker
  velox-exchange      → velox-core, velox-md
  velox-gpu           → (wgpu, glyphon)
  velox-storage       → velox-core
  velox-ui            → velox-core, velox-chart, velox-gpu
  velox-scripting     → velox-core, velox-oms, velox-risk
  velox-terminal      → (all — wiring crate)
```

**Critical rule**: No domain crate may depend on an adapter crate. The compiler enforces this: if `velox-oms/Cargo.toml` lists `velox-broker`, CI fails.

## Consequences

### Positive

- **Testability**: Every port has a mock/fake implementation. OMS can be tested without a broker connection; chart can be tested without a GPU.
- **Swapability**: Replace `velox-broker-fix` with `velox-broker-rest` by implementing the same `OrderExecutionPort` — zero changes to OMS.
- **Clear upgrade path**: When a new exchange or storage backend is needed, it's a new adapter crate, not a modification of domain logic.
- **Better reasoning about unsafe**: Domain crates (`velox-oms`, `velox-risk`, `velox-indicators`) remain `#![forbid(unsafe_code)]`. Unsafe is contained in adapter crates.
- **Reinforces existing strengths**: The project already has traits-as-ports (`BrokerClient`, `ExchangeFeed`, `Indicator`). This ADR codifies and expands that pattern.
- **Dependency inversion**: `velox-oms` defines what it needs (`OrderExecutionPort`) instead of importing broker libraries.

### Negative

- **More boilerplate**: Each port needs at least one trait, one production impl, one test impl. For simple wrappers this is overhead.
- **Learning curve**: Contributors must understand which layer a crate belongs to and which direction dependencies flow.
- **Refactoring cost**: Extracting a trait from existing concrete code takes effort. Some existing code (chart renderer) is explicitly exempted per Rule 2.

### Trade-offs

- **Trait dispatch vs monomorphization**: We chose generics over `dyn` for hot paths. This increases compile time (more monomorphization) but gives zero-cost runtime. Cold paths (config loading, account info requests) can use `dyn`.
- **Not pure hexagonal**: The GPU render path (Rule 2) and ring buffer pipeline (Rule 3) are explicitly exempted. This is a pragmatic concession to performance requirements. The project is "hexagonal-inspired" where it matters (domain logic, I/O, testability) and "direct" where performance demands.
- **Existing code**: Some existing code (e.g., `ChartRenderer` calling wgpu directly) pre-dates this ADR. It's grandfathered in per Rule 2. Future refactors should consider port extraction only if a second render implementation (software fallback, test mock) is needed.

## Compliance

### Automated Checks

1. **Crate dependency lint** — CI enforces that no domain crate depends on an adapter crate. Implemented as a custom script (`scripts/check-hexagonal-deps.sh`) that parses `Cargo.toml` files and verifies the dependency matrix above.

2. **Unsafe audit** — Domain crates (`velox-oms`, `velox-risk`, `velox-indicators`, `velox-core`) are scanned for `unsafe` keywords. Any introduction of `unsafe` in these crates fails CI.

3. **Port coverage** — CI runs `cargo test --workspace` and verifies that every port trait has at least one test that uses a mock/fake implementation (measured via `grep -r "impl.*for.*Mock"`). Initially a soft warning, hard failure by Phase 4.

4. **Clippy** — All crates must pass `cargo clippy --workspace --all-targets`. Additional lints in domain crates: `clippy::pedantic`, `unsafe_code` forbid.

5. **ReleaseSafe profile check** — All domain crates compiled with `profile.release-safe`. CI verifies no domain crate uses `unsafe` under this profile.

### Exceptions Registry

| Exception | ADR Rule | Rationale | Expiry |
|-----------|----------|-----------|--------|
| ChartRenderer → wgpu (direct call) | Rule 2 | Render path performance; single impl | Permanent |
| RingBuffer (SegQueue) direct usage | Rule 3 | Hot path; zero-cost already via lock-free | Permanent |
| egui panel rendering (direct egui calls) | Rule 2 | UI framework tightly coupled to immediate mode | Permanent |
| `velox-broker` vs `velox-broker-fix` split | — | Pre-dates ADR; both are adapters, acceptable | Permanent |

## Notes

### Related ADRs

- ADR-001: Workspace Crate Structure — established the crate boundaries that hexagonal layers build upon
- ADR-002: Concurrency Model — tokio for I/O (adapters), crossbeam for hot paths (Rule 3 exemptions)
- ADR-003: wgpu Rendering Pipeline — wgpu as adapter for GPU rendering (Rule 2 exemption context)

### References

- [Alistair Cockburn — Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [Ports & Adapters Pattern](https://www.thinktocode.com/2018/07/19/ports-and-adapters-architecture/)
- [Rust and Hexagonal Architecture](https://alexis-lozano.com/hexagonal-architecture-in-rust/)
- [Zero-Cost Abstractions in Rust](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)

### Change History

- 2026-07-08: Initial draft — formalized hexagonal architecture over existing workspace, with hot path exceptions
