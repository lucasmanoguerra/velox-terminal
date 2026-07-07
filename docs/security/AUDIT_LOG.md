# Audit Log — velox-terminal

Sistema de auditoría de operaciones.

---

## Design

Los logs de auditoría son **append-only e inmutables**. Cada entrada se escribe secuencialmente y no se puede modificar retroactivamente.

```rust
struct AuditEntry {
    sequence: u64,              // Monotonically increasing
    timestamp_ns: u64,          // UTC nanoseconds
    entry_type: AuditEntryType, // What happened
    data: serde_json::Value,    // Structured payload
    previous_hash: [u8; 32],    // SHA-256 of previous entry (blockchain-like chain)
    entry_hash: [u8; 32],       // SHA-256 of this entry
}
```

## Entry Types

| Type | Trigger | Data |
|------|---------|------|
| `OrderSubmitted` | User or algo submits order | OrderId, symbol, side, qty, price, type |
| `OrderFilled` | Broker confirms fill | OrderId, fill price, fill qty, remaining |
| `OrderPartiallyFilled` | Partial fill received | OrderId, cum_qty, remaining_qty, price |
| `OrderCancelled` | User or system cancels | OrderId, reason |
| `OrderRejected` | Broker or risk rejects | OrderId, reason, code |
| `OrderModified` | User modifies order | OrderId, old params, new params |
| `RiskValidation` | Pre-trade risk check | Rule, allowed, actual, result |
| `CircuitBreakerTriggered` | Circuit breaker activates | Breaker name, trigger value |
| `PositionChange` | Position updated | Symbol, old_qty, new_qty, reason |
| `ConnectionEvent` | Connection state change | Broker, endpoint, old_state, new_state |
| `ConfigChange` | Risk/trading config changed | Key, old_value, new_value, user |
| `Login` | User authentication | User, method |

## Integrity

La cadena de hashes permite detectar manipulación retrospectiva:

```
Entry 1: [data_1] → hash_1 = SHA256(data_1)
Entry 2: [data_2, prev_hash = hash_1] → hash_2 = SHA256(data_2 + hash_1)
Entry 3: [data_3, prev_hash = hash_2] → hash_3 = SHA256(data_3 + hash_2)
...
```

Si alguien modifica Entry 2, todos los hashes subsecuentes cambian → la manipulación es detectable.

## Retention

| Data | Retention | Rationale |
|------|-----------|-----------|
| Audit entries (trading) | 5 years | MiFID II |
| Audit entries (auth) | 2 years | GDPR |
| Audit entries (config) | 1 year | Best practice |
| Market data (tick) | 6 months | Storage tradeoff |
| Market data (daily) | 5 years | Backtesting + compliance |
