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

## Benchmark Tests (criterion)

- Benchmark all hot paths: tick parsing, OHLCV aggregation, indicator streaming
- Benchmark GPU operations: draw call overhead, buffer updates
- Track regressions in CI (compare against stored baseline)

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
