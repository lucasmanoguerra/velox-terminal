# Concurrency Model — velox-terminal

Modelo de concurrencia del sistema: qué corre dónde y cómo se comunican los threads.

---

## Principio General

Usamos el modelo de concurrencia adecuado para cada subsistema, no un enfoque único:

| Subsistema | Modelo | Razón |
|-----------|--------|-------|
| Network I/O (feeds, brokers) | tokio async | E/S con mucha espera, multiplexación eficiente |
| Hot path (tick parsing, OMS) | crossbeam channels + threads dedicados | Latencia predecible, sin overhead de async |
| Backtesting | rayon | Parallelismo de datos en CPU, dividir y conquistar |
| UI/Renderizado | Hilo único main loop | egui y wgpu requieren un solo hilo |
| Scripting de usuario | Thread separado con timeout | Aislamiento, evitar que un script bloquee el sistema |

---

## Arquitectura de Threads

```
┌─────────────────────────────────────────────────────────┐
│   Hilo Principal (Main Thread)                          │
│   ┌─────────────────────────────────────────────────┐  │
│   │  wgpu Render Loop    ← 16ms target (60fps)      │  │
│   │  egui Update Loop    ← 8ms target (120fps)      │  │
│   │  Input Processing    (mouse, keyboard, hotkeys)  │  │
│   └─────────────────────────────────────────────────┘  │
│         ▲                               ▲              │
│         │ crossbeam channel             │ crossbeam     │
│         ▼                               ▼              │
┌────────────────┐               ┌────────────────────┐  │
│ tokio Runtime  │               │ Tick Processing    │  │
│ (worker pool)  │               │ (dedicated thread) │  │
│                │               │                    │  │
│ • Broker Conn  │               │ • Parse FIX/WS     │  │
│ • REST APIs    │               │ • Ring buffer ins  │  │
│ • Feed Reconn  │               │ • -> OHLCV aggreg  │  │
│ • Health Check │               │ • -> Indicator upd │  │
└────────────────┘               └────────────────────┘  │
                                      │                   │
                           ┌──────────┴──────────┐        │
                           │  crossbeam channel   │        │
                           ▼                      ▼        │
                   ┌──────────────┐    ┌──────────────┐   │
                   │ OMS Thread   │    │ Scripting    │   │
                   │ (dedicated)  │    │ (per-script) │   │
                   │              │    │ \w timeout   │   │
                   │ • State mach │    │ • Execute    │   │
                   │ • Risk check │    │ • Signal gen │   │
                   │ • Send order │    │ • Position   │   │
                   └──────┬───────┘    └──────────────┘   │
                          │                                 │
                          ▼                                 │
               ┌────────────────────┐                       │
               │ tokio Runtime      │                       │
               │ (order to broker)  │                       │
               └────────────────────┘                       │
└─────────────────────────────────────────────────────────┘
```

## Canales de Comunicación

| Origen → Destino | Mecanismo | Contenido | Latencia Objetivo |
|-----------------|-----------|-----------|-------------------|
| Feed → Tick Processing | crossbeam channel (MPSC) | Raw bytes de red | < 10μs |
| Tick Processing → OMS | crossbeam channel (MPSC) | `Tick` struct parsed | < 50μs |
| Tick Processing → UI/Charting | crossbeam channel (broadcast) | `Tick` / `CandleUpdate` | < 100μs |
| OMS → Risk | Llamada síncrona | `Order` + `AccountState` | < 1ms |
| OMS → Broker (send) | tokio oneshot | OrderMessage serializado | < 5ms |
| Broker → OMS (recv) | tokio mpsc | ExecutionReport | < 5ms |
| UI → OMS (user action) | crossbeam channel (bounded) | `UserCommand` | < 1ms |

## Event Bus

Sistema pub/sub central que desacopla módulos sin añadir latencia al hot path.

### Arquitectura

```
                    ┌──────────────────────┐
                    │     Event Bus         │
                    │ broadcast::Sender<S>   │
                    └──────┬───────────────┘
                           │
          ┌────────────────┼────────────────┐
          │                │                │
   ┌──────▼─────┐  ┌──────▼─────┐  ┌──────▼─────┐
   │  OMS        │  │  Feed      │  │  UI        │
   │  (publish)  │  │  (publish) │  │  (sub)     │
   └────────────┘  └────────────┘  └────────────┘
          │                │
          │                │
   ┌──────▼─────┐  ┌──────▼─────┐
   │  Risk      │  │  Logging   │
   │  (sub)     │  │  (sub)     │
   └────────────┘  └────────────┘
```

### Hot Path Bridge

Critical low-latency events **bypass the bus** and use dedicated channels:

| Event Class | Channel | Event Bus Proxy? |
|-------------|---------|-----------------|
| Tick/Quote | Lock-free RingBuffer (crossbeam) | Optional lightweight notification |
| Order Fill | Direct crossbeam channel | Optional order summary event |
| Connection State | `AtomicBool` + polling | Full event via bus |
| Account Update | mpsc channel | Full event via bus |
| User Command | crossbeam bounded channel | — |
| Risk Alert | Direct call + bus event | Full event via bus |

### AppEvent Enum

```rust
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
```

## Backpressure Strategy

### Channel Capacities and Policies

| Channel | Type | Capacity | Policy When Full |
|---------|------|----------|-----------------|
| BinanceFeed → RingBuffer | Lock-free SPSC (ring) | 4096 slots | Drop newest (gap marked) |
| Pipeline → AppState (candles) | `tokio::mpsc::UnboundedSender` | Unbounded | **Risk**: Mitigated by draining every frame (~16ms). Monitor with tracing. |
| Event Bus | `tokio::sync::broadcast` | 256 slots | Drop oldest for slow subscribers (lagged = true) |
| User commands (UI → OMS) | `crossbeam::bounded` | 64 slots | Block sender (user waits for confirmation) |
| Order updates (OMS → UI) | `crossbeam::bounded` | 128 slots | Drop oldest (frame skip) |

### Backpressure Rules

1. **Bounded channels by default**: Every cross-module channel must have a defined capacity. `UnboundedSender` is an acknowledged exception with documented mitigation.
2. **Drop policy must be explicit**: Each channel documents whether it drops newest, oldest, or blocks. No silent unbounded growth.
3. **Monitor lag**: Log `WARN` when consumer falls behind. Log `ERROR` on data loss.
4. **Backpressure propagates**: If RingBuffer is full, the WebSocket frame is dropped (not queued). The producer never blocks.
5. **Batch consumption**: Use `pop_n(batch, max)` on RingBuffer to amortize atomic overhead — 2 atomics per batch instead of 3 per tick.

```rust
// Ring buffer batch consumption (2 atomics per batch):
fn poll(&mut self) -> usize {
    let mut batch = Vec::with_capacity(128);
    let mut total = 0;
    loop {
        batch.clear();
        let n = self.ring.pop_n(&mut batch, 128);
        if n == 0 { break; }
        for event in batch.drain(..) {
            self.aggregator.process_tick(&event);
        }
        total += n;
    }
    total
}
```

## Allocator Strategy

| Subsystem | Allocator | Why |
|-----------|-----------|-----|
| **Domain core** (velox-core, -oms, -risk, -indicators) | `std::alloc::System` | Minimal allocations. Predictable behavior. |
| **Adapters** (velox-exchange, -chart, -ui) | `mimalloc` | Heavy allocation from JSON, GPU buffers, WebSocket messages. mimalloc reduces fragmentation. |
| **Hot path** (ring buffer, aggregator, indicators) | Pre-allocated buffers | Zero heap allocations in steady state. Pre-allocated slots, reused Vecs via `pop_n`. |
| **Scripting** (velox-scripting) | `std::alloc::System` | Lua VM manages its own memory. Avoid double allocator overhead. |

Selected via feature flag:
```toml
[features]
adapter-allocator = ["velox-exchange/mimalloc", "velox-chart/mimalloc"]
```

## Profiling Rules (Mandatory)

1. **Before any optimization**: Profile first. Use `perf` + `cargo flamegraph` or Tracy. No change accepted without baseline data.
2. **Benchmark hot paths**: Every hot path function must have a `criterion` benchmark. Track regressions in CI.
3. **Zero-alloc proof**: The hot path (tick → OHLCV → indicator update) must run with zero heap allocations in steady state. Verify with `dhat` or `alloc_counter`.
4. **p99 over mean**: Always optimize for tail latency (p99), not average. Trading systems are sensitive to jitter.

## Reglas

1. **Nunca bloquees el hilo principal**: No I/O, no cálculo pesado, no lock acquisition en el hilo de UI.
2. **Hot path sin allocations**: El path tick → OHLCV → indicador debe tener 0 allocations en steady state (pre-allocar buffers).
3. **Backpressure explícita**: Todo canal debe tener capacidad definida y política de drop documentada. Preferir `pop_n` batching para amortiguar overhead atómico.
4. **Profiling antes de optimizar**: Toda mejora de rendimiento debe demostrarse con datos (criterion, flamegraph, dhat). No optimizar por corazonada.
5. **Mutex solo en config**: El único Mutex permitido es para acceso a configuración compartida. Los hot paths usan solo crossbeam o Atomics.
6. **Zero-copy en hot path**: Usar bytemuck/rkyv para evitar deserialización en el pipeline de ticks. `#[repr(C)]` structs para casting directo.
7. **hilos de scripting con timeout**: Ejecutar scripts con timeout forzoso (panic + abort del thread si excede). No hay unbounded execution.
