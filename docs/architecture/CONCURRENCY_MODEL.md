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

## Reglas

1. **Nunca bloquees el hilo principal**: No I/O, no cálculo pesado, no lock acquisition en el hilo de UI.
2. **Hot path sin allocations**: El path tick → OHLCV → indicador debe tener 0 allocations en steady state (pre-allocar buffers).
3. **Backpressure explícita**: Los canales tienen tamaño limitado. Definir política de drop (latest vs earliest) cuando el consumidor no sigue el ritmo.
4. **hilos de scripting con timeout**: Ejecutar scripts con timeout forzoso (panic + abort del thread si excede). No hay unbounded execution.
5. **Mutex solo en config**: El único Mutex permitido es para acceso a configuración compartida. Los hot paths usan solo crossbeam o Atomics.
