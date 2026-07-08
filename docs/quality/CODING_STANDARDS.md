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

### File Size Rule

**Cada archivo debe tener < 200 líneas, sin contar imports ni doc comments de módulo.**

Si un archivo crece por encima de 200 líneas efectivas, dividilo en archivos más pequeños por responsabilidad (UNIX: una cosa por archivo).

```rust
// ✅ BUENO: archivo pequeño, una responsabilidad
// src/state_machine/transitions.rs (~50 líneas efectivas)

// ❌ MALO: archivo gigante con múltiples responsabilidades
// src/order_manager.rs (800 líneas)
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
