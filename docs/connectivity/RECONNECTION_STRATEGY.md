# Reconnection Strategy — velox-terminal

Estrategia de reconexión automática para conectores de broker y feed.

---

## Exponential Backoff

```
Attempt 1:  wait 1s
Attempt 2:  wait 2s
Attempt 3:  wait 4s
Attempt 4:  wait 8s
Attempt 5:  wait 16s
Attempt 6:  wait 30s  (capped)
...
Attempt N:  wait 30s  (steady state)
```

```rust
struct BackoffStrategy {
    initial_delay: Duration,      // 1 second
    max_delay: Duration,          // 30 seconds
    multiplier: f64,               // 2.0
    jitter: f64,                   // 0.1 (10% randomness)
    reset_after: Duration,         // 5 minutos sin error → reset
}
```

## Idempotency

| Operation | Idempotency Key | Validation |
|-----------|----------------|------------|
| Submit order | `ClOrdId` (client order ID) | Duplicate ClOrdId → return existing order status |
| Cancel order | `OrigClOrdId` | Already cancelled → return success |
| Modify order | `ClOrdId` + `OrigClOrdId` | Alread modified → return current state |

## Session Recovery

```
1. Detect disconnect (heartbeat timeout or TCP reset)
2. Set state: ALL_ORDERS_UNKNOWN
3. Start backoff reconnection loop
4. On reconnect:
   a. Authenticate with stored credentials
   b. Request all open orders from broker
   c. Request current positions
   d. Request account info
   e. Resubscribe to market data
   f. Reconcile internal state with broker state
   g. Set state: RECONCILED
5. If reconciliation detects discrepancy:
   - Reject all orders that broker doesn't know about
   - Create position correction orders if needed
   - Log detailed audit record
```

## Heartbeat

| Connection type | Heartbeat interval | Timeout |
|----------------|-------------------|---------|
| FIX (session) | 30s (FIX Heartbeat msg) | 60s (2 * interval) |
| WebSocket | 15s (custom ping/pong) | 30s |
| REST | N/A (request/response) | 10s per request |

## Graceful Degradation

| Connection State | Trading Capability | Risk Behavior |
|-----------------|-------------------|---------------|
| Connected | Full | Normal checks |
| Reconnecting (backoff) | Order submission paused | Circuit breaker: block new orders |
| Reconnecting (long) | Read-only mode | Allow cancels only |
| Disconnected (manual) | No trading | Flatten positions option |
