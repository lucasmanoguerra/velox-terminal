# ADR-002: Concurrency Model

| | |
|---|---|
| **ADR** | 002 |
| **Title** | Tokio para I/O asíncrono + crossbeam para hot paths |
| **Status** | Accepted |
| **Date** | 2026-07-06 |
| **Author** | systems-architect |

## Context

Una terminal de trading requiere manejar múltiples fuentes de concurrencia:
- Conexiones de red (WebSocket, FIX, REST) — manejadas con async/await
- Pipeline de market data (ring buffers, agregación) — milisegundo a nanosegundo
- UI (egui) — single-threaded, 60 FPS
- Cálculo de indicadores — incremental, O(1) por tick
- Scripts de usuario — sandboxeados, timeout forzoso

Necesitamos una estrategia de concurrencia que balancee latencia vs ergonomía.

## Decision

**Dos modelos de concurrencia coexistiendo**:

### 1. Tokio (async/await) para I/O de red

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // One runtime, multiple tasks
    let md_task = tokio::spawn(market_data_feed_loop());
    let broker_task = tokio::spawn(broker_connection_loop());
    let ui_task = tokio::spawn(ui_event_loop());

    // ... wait for shutdown
    Ok(())
}
```

Usar tokio para:
- Conexiones WebSocket/FIX
- REST API calls
- File I/O asíncrono
- Timers y heartbeats

### 2. Crossbeam (lock-free channels) para hot paths

```rust
use crossbeam::channel::{unbounded, Sender, Receiver};

// Market data pipeline (lock-free, single-producer multi-consumer)
let (md_tx, md_rx): (Sender<Tick>, Receiver<Tick>) = unbounded();

// Ring buffer for zero-copy IPC between threads
let ring = Arc::new(SegQueue::new());
```

Usar crossbeam para:
- Tick data entre feed → agregador → indicadores → charting
- Órdenes entre UI → OMS → Risk → Broker
- Eventos de fill/update entre Broker → OMS → UI

### Thread Model

```
┌─────────────────────────────────────────────────────┐
│  Main Thread (UI)                                   │
│  - egui update/render                               │
│  - Keyboard/mouse input                             │
│  - Panel management                                 │
│  Channel: recv from OMS/Risk updates                │
└─────────────┬──────────────────────────────────────-┘
              │ crossbeam channels
              ▼
┌─────────────────────────────────────────────────────┐
│  Tokio Runtime (async tasks)                        │
│  Task 1: Market data feed (WebSocket)               │
│  Task 2: Broker connection (FIX)                    │
│  Task 3: Order execution loop                       │
│  Task 4: REST API calls                             │
│  Channel: send ticks to ring buffer                 │
└─────────────┬──────────────────────────────────────-┘
              │ SegQueue / crossbeam
              ▼
┌─────────────────────────────────────────────────────┐
│  Market Data Thread                                  │
│  - Consume ticks from ring buffer                   │
│  - Build candles (1min, 5min, etc.)                 │
│  - Compute indicators                               │
│  - Forward to UI/charting                           │
│  Priority: LOW_LATENCY                              │
└─────────────────────────────────────────────────────┘
```

## Consequences

### Positive
- I/O-bound (red) no bloquea CPU-bound (indicadores)
- UI nunca se bloquea por operaciones de red
- Crossbeam channels en hot paths evitan overhead de async
- Escalable a más brokers/feeds sin modificar la arquitectura

### Negative
- Dos modelos de concurrencia → dos formas de debugging
- State compartido entre threads requiere cuidado (Arc<Mutex> o message passing)
- Tokio tasks dentro del runtime no pueden ser crossbeam senders directos

### Trade-offs
- Se consideró usar solo tokio (con `spawn_blocking` para CPU-bound) pero el overhead de waker para ticks de 1µs es demasiado alto
- Se consideró actores (Axum/Thespian) pero la complejidad no se justifica para este tamaño de proyecto

## Compliance

- Todas las operaciones de red usan tokio
- Todos los hot paths de market data usan crossbeam
- Clippy lint `await_holding_lock` para detectar held locks across await points
- Benchmarks de latency en cada canal crítico

## Notes

### Related ADRs
- ADR-001: Workspace Crate Structure

### References
- [Tokio: Going down the rabbit hole](https://tokin.rs/)
- [crossbeam: Lock-free data structures](https://github.com/crossbeam-rs/crossbeam)

### Change History
- 2026-07-06: Initial draft
