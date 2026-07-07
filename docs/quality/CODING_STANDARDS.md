# Coding Standards — velox-terminal

Estándares de código Rust para el proyecto.

---

## Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Types (structs, enums) | PascalCase | `OrderState`, `BrokerClient` |
| Traits | PascalCase | `BrokerClient`, `RiskValidator` |
| Functions/Methods | snake_case | `submit_order()`, `validate_trade()` |
| Variables | snake_case | `filled_qty`, `avg_price` |
| Modules | snake_case | `oms`, `risk_management` |
| Crates | snake_case, hyphenated | `market-data-feed`, `time-series-storage` |
| Error enums | PascalCase | `OrderError`, `RiskError` |
| Error variants | PascalCase | `AlreadyFilled`, `MissingCredentials` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_ORDER_QTY`, `DEFAULT_TIMEOUT_MS` |
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

## Error Handling

```rust
// ✅ GOOD: Typed errors with thiserror
#[derive(thiserror::Error, Debug)]
enum OrderError {
    #[error("Order {order_id} is already filled")]
    AlreadyFilled { order_id: OrderId },

    #[error("Risk validation failed: {reason}")]
    RiskRejected { reason: String, rule: String },

    #[error("Broker rejected: {message}")]
    BrokerRejected { code: String, message: String },

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
- Complex business logic should have inline comments explaining "why", not "what"

## Clippy

All code must pass:
```bash
cargo clippy -- -D warnings
```

Exceptions must be documented with `#[allow(clippy::...)]` and a comment explaining why.

## Formatting

```bash
cargo fmt --check
```

Must pass in CI. No tabs, 4-space indentation, 100 char line limit.
