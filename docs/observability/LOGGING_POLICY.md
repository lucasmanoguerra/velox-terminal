# Logging Policy — velox-terminal

Política de logging estructurado con tracing.

---

## Levels

| Level | Usage | Example |
|-------|-------|---------|
| `ERROR` | Condición que requiere atención humana inmediata | Feed disconnect, OMS rejected, risk limit breached |
| `WARN` | Degradación que no impide operar | High latency, star skip, backpressure |
| `INFO` | Eventos normales importantes | Order submitted, fill received, connection established |
| `DEBUG` | Diagnóstico detallado | Tick parsed, state transition, validation result |
| `TRACE` | Tracing de flujo interno | Ring buffer push/pop, GPU buffer update |

## Structured Fields

Cada evento de log debe incluir:

```rust
// Required fields for all events
tracing::info!(
    target: "oms",
    order_id = %order.id(),
    symbol = %order.symbol(),
    action = "submit",
    result = "success",
    latency_ms = %latency.as_millis(),
    "Order submitted successfully"
);
```

**Required fields**: `target` (subsystem), `timestamp` (automático), `event` (action description)

**Never log**: Credentials, API keys, tokens, personal information, internal IPs

## Subsystem Targets

| Target | Subsystem |
|--------|-----------|
| `feed` | Market data feed (ticks, quotes, candles) |
| `oms` | Order management system |
| `risk` | Risk management |
| `broker` | Broker connection |
| `charting` | Charting engine |
| `ui` | egui panels |
| `storage` | Time-series storage |
| `system` | General system events |

## Audit Trail

Para OMS y Risk, se usa un logger dedicado que escribe a un archivo append-only:

```rust
// Audit log — writes to append-only file (immutable log)
tracing::info!(
    target: "audit",
    order_id = %order.id(),
    action = "fill",
    fill_price = %fill.price,
    fill_qty = %fill.qty,
    timestamp = %fill.time,
    "Order fill recorded"
);
```

Este archivo de auditoría está separado de los logs generales del sistema y tiene rotación diaria con retención configurada según compliance.
