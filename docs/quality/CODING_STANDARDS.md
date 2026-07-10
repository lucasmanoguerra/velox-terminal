# Coding Standards — velox-terminal

Estándares de código Rust para el proyecto. Sigue arquitectura hexagonal + UNIX philosophy.

---

## Hexagonal Package Principles

| Capa | Regla |
|------|-------|
| **Domain Core** | `#![forbid(unsafe_code)]`. Zero imports de infraestructura (tokio, wgpu, egui, crossbeam, reqwest). Depende solo de std + crates de dominio puro. |
| **Ports** | Traits en su propio módulo o crate. Dependen solo de domain types. |
| **Adapters** | Implementan traits de port. Pueden depender de infraestructura. |
| **Application** | Composition root. Wrea ports → adapters. |

### Hot path exceptions
Ocurren donde el dispatch virtual afectaría rendimiento medible:
ring buffers, GPU upload loops, OMS state transitions.
Documentar cada excepción: `// HEXAGONAL-EXEMPT: <razón>`.

## Allocator Strategy

| Layer | Allocator | Rationale |
|-------|-----------|-----------|
| **Domain Core** (velox-core, -oms, -risk, -indicators) | `std::alloc::System` | Minimal allocation; mostly stack data. No behavioral surprises. |
| **Adapters** (velox-exchange, -chart, -ui) | `mimalloc` | Heavy allocation from JSON parsing, GPU buffers, WebSocket messages. |
| **Hot path** (RingBuffer, aggregator) | Pre-allocated buffers | Zero allocation in steady state. Pre-allocated ring buffer slots, reused Vecs via `pop_n` + `.clear()` + `.drain(..)`. |

```toml
# Cargo.toml (workspace) — feature-gated mimalloc
mimalloc = { version = "0.1", optional = true }

[features]
adapter-allocator = ["velox-exchange/mimalloc", "velox-chart/mimalloc", "velox-ui/mimalloc"]
```

**Rules**:
- Domain core crates MUST NOT set `#[global_allocator]`.
- Adapter crates MAY use mimalloc, gated behind a feature flag.
- Profiling (dhat) must confirm improvement before enabling mimalloc for a new crate.

## Zero-Copy Guidelines

All hot-path data transformation must use zero-copy techniques.

### Approved Patterns

```rust
// ✅ GOOD: bytemuck Pod struct for zero-copy from wire bytes
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
struct TickRaw {
    symbol: [u8; 8],       // Padded to fixed width
    price: f64,             // Native-endian from wire
    volume: u64,
    timestamp_ns: u64,
    flags: u32,
}

// Zero-copy cast from network buffer
fn parse_tick(bytes: &[u8]) -> Result<&TickRaw, Error> {
    bytemuck::try_from_bytes(bytes)
        .map_err(|_| Error::InvalidSize)
}

// ❌ BAD: serde_json deserialization on hot path
fn parse_tick_wrong(json: &str) -> Result<Tick, Error> {
    serde_json::from_str(json)  // Allocates strings, slow
}

// ✅ GOOD: bytes::Bytes for zero-copy slice sharing
use bytes::Bytes;
struct Message {
    header: Bytes,  // Zero-copy borrow from buffer
    payload: Bytes,
}
```

### Per-Path Requirements

| Path | Technique | Verification |
|------|-----------|-------------|
| Network bytes → Tick/Quote | `bytemuck::try_from_bytes` on `#[repr(C)]` struct | `criterion` benchmark shows < 100ns |
| IPC (RingBuffer) | `MarketEvent` enum with `Box<[u8]>` or `bytemuck::Pod` payload | No `serde` crate in hot path Cargo.toml |
| GPU buffer upload | `bytemuck::cast_slice` from SoA arrays | No per-element copy in render loop |
| Persistence load (backtest) | `rkyv::check_archived_root` | Deserialize < 50ns per candle |

### When Zero-Copy Is Optional

- Config files, exchangeInfo responses, account details: use `serde_json`.
- User input forms: use `serde` (negligible frequency).
- Logging/tracing: no constraint (outside hot path).

## Profiling-First Rule

**No performance optimization is accepted without profiling data.**

1. **Before**: Profile with `perf` + `cargo flamegraph` or Tracy. Establish baseline latency (p50, p99, p99.9).
2. **During**: Use `criterion` benchmarks to measure the specific change in isolation.
3. **After**: Profile again. Compare to baseline. Document improvement in commit message.

```bash
# Profiling workflow
cargo flamegraph --bin velox-terminal -- -profile    # Generate flamegraph
cargo bench --bench hot_path                          # Criterion regression check
cargo run --bin velox-terminal -- --dhat              # Heap allocation profile
```

**Mandatory criterion benchmarks** (checked in CI):
- Tick parsing latency
- Ring buffer push/pop throughput
- Candle aggregation throughput
- OMS state machine transitions/μs
- GPU buffer upload time

## Fuzzing Requirements

All parsers and protocol handlers must be fuzzed with `cargo-fuzz`.

| Target | Fuzz Input | Priority |
|--------|-----------|----------|
| Binance WebSocket JSON | Malformed trade/depth messages | HIGH |
| FIX protocol parser | Invalid tag-value sequences | HIGH |
| Configuration file parser | Malformed TOML/YAML | MEDIUM |
| User command parser | Garbled command strings | MEDIUM |
| Lua script engine | Scripts with infinite loops (timeout) | MEDIUM |

```rust
// Example fuzz target (crates/velox-exchange/fuzz/fuzz_targets/trade_parser.rs)
//! Fuzz target for Binance trade message parser
#![no_main]
use libfuzzer_sys::fuzz_target;
use velox_exchange::binance::parse_trade;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_trade(s); // Must not panic on any input
    }
});
```

Run with:
```bash
cargo +nightly fuzz run trade_parser   # Must run 1M+ iterations without crash
cargo +nightly fuzz run depth_parser
```

**Rule**: Every PR that touches a parser must include a fuzz target or extend an existing one. CI runs fuzz targets for 30s per target per PR.

## Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Types (structs, enums) | PascalCase | `OrderState`, `BrokerClient` |
| Traits | PascalCase (prefijo `Port` si es hexagonal) | `BrokerClient`, `OrderExecutionPort` |
| Functions/Methods | snake_case | `submit_order()`, `validate_trade()` |
| Variables | snake_case | `filled_qty`, `avg_price` |
| Modules | snake_case | `oms`, `risk_management` |
| Crates | snake_case, hyphenated | `velox-exchange`, `velox-broker-fix` |
| Error enums | PascalCase | `OrderError`, `RiskError` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_ORDER_QTY` |
| Type parameters | Single uppercase | `T`, `V`, `E` |

## File Structure

```
src/
├── lib.rs                    # Public API, re-exports
├── mod.rs                    # Module declaration (or inline in lib.rs)
├── state_machine.rs          # One concept per file
├── state_machine/
│   ├── mod.rs
│   ├── transitions.rs
│   └── tests.rs              # Unit tests in companion module
```

### File Size Rule (Atomización)

**Cada archivo `.rs` debe tener < 200 líneas efectivas de código de producción**
(excluyendo imports de crate, doc comments de módulo, líneas en blanco y
bloques `#[cfg(test)]`).

Si un archivo crece por encima de 200 líneas efectivas, dividilo en archivos
más pequeños por responsabilidad (UNIX: una cosa por archivo). Ver
`docs/quality/ATOMIZED_FILES.md` para la guía completa y el inventario de
deuda actual.

```rust
// ✅ BUENO: archivo pequeño, una responsabilidad
// src/state_machine/transitions.rs (~50 líneas efectivas)

// ❌ MALO: archivo gigante con múltiples responsabilidades
// src/order_manager.rs (800 líneas)
```

### Single Responsibility por Archivo (SRP-File)

Cada archivo debe representar **un concepto lógico**. Señales de que un archivo
viola SRP:

- El nombre del archivo usa "y" o "&" implícitos (p.ej. `parsing_and_validation.rs`)
- Más de 3 `pub` items no-triviales (structs o funciones) en el mismo archivo
- `impl` blocks para más de 2 structs diferentes en el mismo archivo
- Mezcla de tipos de datos, lógica de negocio, y helpers de infraestructura
- Funciones que se pueden agrupar en categorías claramente distintas

```rust
// ❌ MALO: 3 responsabilidades en 1 archivo
pub struct RiskValidator { ... }          // responsabilidad 1: risk
pub struct PositionLimits { ... }         // responsabilidad 2: positions
pub fn validate_symbol(sym: &str) {}     // responsabilidad 3: validation

// ✅ BUENO: 3 archivos, 1 responsabilidad cada uno
// risk/validators.rs    → pub struct RiskValidator
// risk/limits.rs        → pub struct PositionLimits
// risk/symbols.rs       → pub fn validate_symbol
```

**Excepciones documentadas**: Archivos que exceden 200 líneas pero tienen
un refactor planeado pueden llevar `// FILE-EXEMPT: refactor planned (issue #XXX)`.

## Error Handling

```rust
// ✅ GOOD: Typed errors with thiserror
#[derive(thiserror::Error, Debug)]
enum OrderError {
    #[error("Order {order_id} is already filled")]
    AlreadyFilled { order_id: OrderId },
    #[error("Risk validation failed: {reason}")]
    RiskRejected { reason: String, rule: String },
    #[error("Invalid transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
}

// ❌ BAD: Stringly-typed errors
fn submit_order() -> Result<(), String> { ... }

// ❌ BAD: Panics in production paths
fn process_fill() {
    let last = data.last().unwrap();  // NO: can panic
    let last = data.last().ok_or(OrderError::NoData)?;  // YES
}
```

## Documentation

- All `pub` items must have doc comments (`///` or `//!`)
- `// SAFETY:` comments required for every `unsafe` block
- `// HEXAGONAL-EXEMPT:` comments for hot path exceptions to hexagonal rules
- Complex business logic should have inline comments explaining "why", not "what"

## Clippy

All code must pass:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Exceptions must be documented with `#[allow(clippy::...)]` and a comment explaining why.

## Formatting

```bash
cargo fmt --check
```

Must pass in CI. No tabs, 4-space indentation, 100 char line limit.

## git + gh CLI Workflow

Para operaciones de repositorio, usar `gh` CLI:

```bash
# Crear issue
gh issue create --title "feat: ..." --body "..."

# Ver PR checks
gh pr checks

# Merge PR
gh pr merge --squash

# Ver estado del repo
gh repo view

# CI status on current commit
gh run list --limit 5
```

## Conventional Commits

```
<type>(<scope>): <description>

<optional body>
```

| Type | Scope examples | Use |
|------|---------------|-----|
| `feat` | oms, md, chart, ui, exchange | Nueva feature |
| `fix` | oms, risk, md, exchange | Bugfix |
| `refactor` | core, oms, chart | Refactor sin cambio funcional |
| `test` | oms, risk, indicators | Tests |
| `docs` | adrs, architecture, project | Documentación |
| `perf` | chart, md, exchange | Performance |
| `chore` | ci, deps, build | Mantenimiento |
