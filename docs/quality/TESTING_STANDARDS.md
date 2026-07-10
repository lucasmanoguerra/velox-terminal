# Testing Standards — velox-terminal

Estándares de testing para el proyecto.

---

## Testing Pyramid

```
        ╱╲
       ╱  ╲          E2E / Integration (few)
      ╱    ╲
     ╱──────╲
    ╱        ╲       Property-based (medium)
   ╱          ╲
  ╱────────────╲
 ╱              ╲    Unit tests (many)
╱────────────────╲
```

## Unit Tests

- Every module should have unit tests in a companion `tests` module or `tests/` directory
- Focus on: pure logic, state machines, validation rules, indicator calculations
- Benchmark: fast (< 1ms per test)

## Property-Based Tests (proptest)

**Obligatorio para**: OMS state machine, Risk validations, P&L calculations.

```rust
proptest! {
    #[test]
    fn oms_state_machine_invariants(transitions in any::<Vec<Transition>>()) {
        let mut oms = OMS::new();
        for t in transitions {
            let result = oms.apply(t);
            // Invariant 1: Position must never be negative
            prop_assert!(oms.position() >= 0, "Negative position detected");
            // Invariant 2: Total filled qty must never exceed order qty
            prop_assert!(oms.total_filled() <= oms.order_qty());
            // Invariant 3: Order state must be valid after any transition
            prop_assert!(oms.state().is_valid());
        }
    }

    #[test]
    fn risk_limits_never_exceeded(orders in any::<Vec<NewOrder>>()) {
        let mut risk = RiskEngine::new(config);
        let mut position: i64 = 0;
        for order in orders {
            if risk.validate(&order, position).is_ok() {
                position += order.signed_qty();
            }
            prop_assert!(position.abs() <= config.max_position);
        }
    }
}
```

## Integration Tests

- One integration test suite per crate (in `tests/` directory)
- Mock all external dependencies (broker, feeds)
- Test: end-to-end order flow, full candle pipeline, backtest vs known results

## Benchmark Tests (criterion) — Mandatory

**Every hot path must have a criterion benchmark.** Changes that regress hot path performance are rejected unless the tradeoff is explicitly documented.

### Mandatory Benchmarks

| Benchmark | Crate | Threshold | CI Check |
|-----------|-------|-----------|----------|
| Tick parsing | `velox-exchange` | < 100ns p50 | `cargo bench --bench tick_parse` |
| RingBuffer push | `velox-md` | < 50ns p50 | `cargo bench --bench ring_buffer` |
| RingBuffer pop_n | `velox-md` | < 200ns per batch of 128 | `cargo bench --bench ring_buffer` |
| Candle aggregation | `velox-md` | < 1μs per tick per TF | `cargo bench --bench aggregation` |
| OMS state transition | `velox-oms` | < 500ns per transition | `cargo bench --bench oms` |
| Indicator update (SMA) | `velox-indicators` | < 50ns per tick | `cargo bench --bench indicators` |
| GPU buffer upload | `velox-chart` | < 100μs per frame | `cargo bench --bench chart` |

### Criterion Baseline

```bash
# Record baseline (run once per major version)
cargo bench -- --save-baseline v0.3.0

# Compare against baseline in CI
cargo bench -- --baseline v0.3.0
```

CI fails if any benchmark regresses > 10% from baseline (configurable per benchmark).

## Fuzzing (cargo-fuzz) — Mandatory for Parsers

All protocol parsers, deserializers, and user input handlers must be fuzzed.

### Fuzz Targets

| Fuzz Target | Crate | Input Source | CI Duration |
|-------------|-------|-------------|-------------|
| `trade_parser` | `velox-exchange` | Binance trade JSON | 30s per PR |
| `depth_parser` | `velox-exchange` | Binance depth JSON | 30s per PR |
| `fix_parser` | `velox-broker-fix` | FIX tag-value | 60s per PR |
| `config_parser` | `velox-terminal` | TOML config | 15s per PR |
| `user_command` | `velox-ui` | User command strings | 15s per PR |

### Rules

1. Every parser has a fuzz target in `crates/<name>/fuzz/`.
2. CI runs each target for minimum 30 seconds per PR.
3. A crash in CI blocks the PR — must be fixed before merge.
4. Add new fuzz targets when adding new protocol handlers.

```bash
# Run fuzzer locally
cargo +nightly fuzz run trade_parser -- -max_len=4096

# Run all targets
cargo +nightly fuzz list | xargs -I{} cargo +nightly fuzz run {} -- -max_len=4096
```

## Coverage Requirements

| Module | Branch Coverage | Line Coverage |
|--------|---------------|--------------|
| OMS | 95% | 95% |
| Risk Management | 95% | 95% |
| P&L Calculation | 95% | 95% |
| Market Data Feed | 80% | 85% |
| Charting Engine | 70% | 80% |
| Broker Connectors | 85% | 85% |
| Technical Indicators | 90% | 90% |
| Backtesting | 85% | 85% |
| UI (egui) | 50% | 60% |
