# ADR-007: Event Bus Architecture

| | |
|---|---|
| **ADR** | 007 |
| **Title** | Event Bus for cross-module pub/sub communication |
| **Status** | Accepted |
| **Date** | 2026-07-09 |

## Context

As the terminal grows, modules need to communicate without direct coupling:

- UI needs to know about order fills (from OMS) and connection state (from feeds)
- Risk management needs to react to market data events
- Logging/audit needs visibility into all system events
- Future plugins need a hook into system events

Direct module-to-module calls create a tight dependency graph and make it hard to add new consumers without modifying producers.

## Decision

Use a central Event Bus based on `tokio::sync::broadcast` for non-critical events.
Hot-path events (ticks, fills) **bypass the bus** through dedicated lock-free channels.

### Architecture

```rust
use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub enum AppEvent {
    Tick { symbol: String, price: f64, volume: f64 },
    CandleClosed { symbol: String, candle: Candle },
    OrderUpdate { order_id: OrderId, state: OrderState },
    Fill { order_id: OrderId, fill: Fill },
    ConnectionStatus { exchange: String, status: ConnectionState },
    AccountUpdate { balances: Vec<Balance> },
    RiskAlert { severity: AlertSeverity, message: String },
    UserCommand { command: UserCommand },
    SystemShutdown,
}

pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }
}
```

### Hot Path Bridge

| Event Class | Hot Path Channel | Bus Proxy |
|-------------|-----------------|-----------|
| Tick/Quote | Lock-free RingBuffer (crossbeam) | Optional summary event |
| Order Fill | Direct crossbeam channel | Optional order summary |
| Connection State | `AtomicBool` + polling | Full event via bus |
| Account Update | mpsc channel | Full event via bus |
| User Command | crossbeam bounded | — |

### Channel Capacities

| Channel | Capacity | Policy When Full |
|---------|----------|-----------------|
| `broadcast` event bus | 256 | Drop oldest (slow subscriber gets `Lagged(n)`) |

## Consequences

### Positive
- Decouples producers and consumers — add new subscribers without modifying producers
- Single place for audit/logging to observe all events
- Plugin system can subscribe without modifying core
- Standardized `AppEvent` enum documents all system events

### Negative
- Broadcast adds overhead — not suitable for high-frequency events
- Slow subscribers miss events (`Lagged` error)
- Event enum must be in a shared crate (or duplicated)

### Trade-offs
- `tokio::sync::broadcast` chosen over `crossbeam_channel` for async compatibility
- Hot path events bypass the bus intentionally — the bus is for awareness, not for time-critical data

## Compliance

- All cross-module communication should go through either: (a) dedicated lock-free channels for hot paths, or (b) the Event Bus for awareness events
- New event variants must be added to `AppEvent` enum
- Review: grep for `tokio::sync::broadcast::Sender` usage patterns

## Notes

### Related ADRs
- ADR-002: Concurrency Model (thread architecture)
- ADR-004: Hexagonal Architecture (layer rules)

### References
- `docs/architecture/SYSTEM_OVERVIEW.md` — Event Bus Architecture section
- `docs/architecture/CONCURRENCY_MODEL.md` — Event Bus section
- `tokio::sync::broadcast` docs

### Change History
- 2026-07-09: Initial draft
