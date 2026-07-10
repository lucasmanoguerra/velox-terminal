# ADR-008: Allocator Strategy

| | |
|---|---|
| **ADR** | 008 |
| **Title** | Layered allocator strategy: system for domain, mimalloc for adapters |
| **Status** | Accepted |
| **Date** | 2026-07-09 |

## Context

Different parts of the system have different allocation patterns:

- **Domain core** (`velox-core`, `velox-oms`, `velox-risk`, `velox-indicators`): mostly stack data, minimal heap allocation. Predictable, low-throughput.
- **Adapters** (`velox-exchange`, `velox-chart`, `velox-ui`): heavy allocation from JSON parsing, GPU buffer uploads, WebSocket message handling. High-throughput, varied allocation sizes.
- **Hot path** (RingBuffer, tick aggregation): must have zero allocations in steady state.

Using a single global allocator for all layers forces a tradeoff that doesn't serve any layer optimally.

## Decision

Use a layered allocator approach:

| Layer | Allocator | Rationale |
|-------|-----------|-----------|
| Domain Core | `std::alloc::System` | Minimal allocation, predictable. No behavioral changes from default. |
| Adapters | `mimalloc` (feature-gated) | Reduces fragmentation, improves throughput under heavy allocation. |
| Hot path | Pre-allocated buffers | Zero allocation in steady state. `pop_n` with Vec reuse. |

### Implementation

Mimalloc is feature-gated at the workspace level:

```toml
# Cargo.toml (workspace root)
[dependencies]
mimalloc = { version = "0.1", optional = true }

[features]
default = ["adapter-allocator"]
adapter-allocator = ["velox-exchange/mimalloc", "velox-chart/mimalloc", "velox-ui/mimalloc"]
```

Each adapter crate that enables mimalloc must do so explicitly:

```rust
// In adapter crate (e.g., velox-exchange/src/lib.rs)
#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

Domain core crates MUST NOT set `#[global_allocator]`.

### Verification

Before enabling mimalloc for a new crate, profile with `dhat` to confirm improvement:

```bash
cargo run --features dhat-heap --bin velox-terminal
# Compare: total allocations, max heap, fragmentation
```

## Consequences

### Positive
- Adapters get performance-optimized allocation without affecting domain core
- Feature-gated — easy to disable if mimalloc causes issues on a platform
- Hot path guaranteed zero-alloc (independently verified)

### Negative
- Two global allocators in the binary is impossible (only one `#[global_allocator]` per binary). Solution: only one adapter crate sets `#[global_allocator]` at a time, or domain crates never compete.
- Actually, only ONE `#[global_allocator]` can exist per binary. All crates share it. The "layered" approach means: domain crates don't set one, an adapter crate does. The allocator is global, but the allocation *patterns* differ by layer.

**Correction**: There is only one global allocator. The decision is: set `mimalloc` at the binary level (`velox-terminal`) when the `adapter-allocator` feature is enabled. Domain core crates never set a global allocator.

### Trade-offs
- mimalloc adds ~200KB to binary size
- Must test on all 3 platforms (Windows/macOS/Linux)
- `dhat` heap profiling must confirm improvement before enabling

## Compliance

- Domain core crates MUST NOT import mimalloc or set `#[global_allocator]`
- Only `velox-terminal` (binary) MAY set `#[global_allocator]` via feature flag
- `cargo check` verifies no duplicate `#[global_allocator]`

## Notes

### Related ADRs
- ADR-004: Hexagonal Architecture — domain core purity

### References
- `docs/architecture/SYSTEM_OVERVIEW.md` — Allocator Strategy section
- `mimalloc` crate docs
- `dhat` heap profiling

### Change History
- 2026-07-09: Initial draft
