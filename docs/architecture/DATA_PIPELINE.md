# Data Pipeline — velox-terminal

Flujo de datos desde el broker hasta la pantalla.

---

## Pipeline de Market Data

```
Broker ──FIX/WS──> [crates/broker]
                     │
                     ▼ raw bytes
              [crates/feed] ── Ring Buffer ──> Parse Tick
                     │                              │
                     │                              ▼
                     │                    ┌─────────────────┐
                     │                    │ Tick struct     │
                     │                    │ bytemuck cast    │
                     │                    └────────┬────────┘
                     │                             │
                     ├── crossbeam channel ────────┤
                     │                             │
                     ▼                             ▼
              ┌────────────┐              ┌──────────────┐
              │ OMS        │              │ OHLCV Aggr.  │
              │ (last tick) │              │ (multi-TF)   │
              └────────────┘              └──────┬───────┘
                                                  │
                                         crossbeam channel
                                                  │
                                                  ▼
                                        ┌─────────────────┐
                                        │ Charting Engine  │
                                        │ (wgpu update)    │
                                        └─────────────────┘
                                                  │
                                                  ▼
                                        ┌─────────────────┐
                                        │ egui UI         │
                                        │ (panels)         │
                                        └─────────────────┘
```

## Pipeline de Órdenes

```
User Action (hotkey/click)
         │
         ▼
    [crates/gui] ── crossbeam channel ──> [crates/oms]
         │                                      │
         │                                      ▼
         │                              [crates/risk] validate()
         │                                      │
         │                               ┌──────┴──────┐
         │                               │ Pass?        │
         │                               └──────┬──────┘
         │                                      │
         │                              Yes     │     No
         │                                      ▼
         │                              [crates/oms]
         │                              update state → PendingSubmit
         │                                      │
         │                                      ▼
         │                              [crates/broker]
         │                              send via FIX/WS/REST
         │                                      │
         │                                      ▼
         │                              Broker Response
         │                                      │
         │                              [crates/oms]
         │                              update state → Working/Filled/Rejected
         │                                      │
         │                                      ▼
         │                              [crates/gui]
         │                              update UI panels
```

## Pipeline de Backtesting

```
[crates/storage] ── read historical ticks batch
         │
         ▼
    [crates/backtest] ── replay ticks through same pipeline as live
         │                      │
         │                      ▼
         │              [crates/indicators] (streaming mode)
         │                      │
         │                      ▼
         │              Strategy Logic (same as live)
         │                      │
         │                      ▼
         │              [crates/oms] (simulated fills)
         │                      │
         │                      ▼
         │              P&L, Metrics, Report
```

## Objetivos de Latencia

| Etapa | p50 | p99 | Medición |
|-------|-----|-----|----------|
| Broker → raw bytes | 50μs | 200μs | tracing span |
| Parse → Tick struct | 5μs | 20μs | criterion bench |
| Tick → OHLCV update | 1μs | 5μs | criterion bench |
| Tick → Chart (GPU) | 100μs | 500μs | Tracy frame profiler |
| Order send → Broker | 5ms | 20ms | OMS tracing span |
| Broker → Order fill | 50ms | 500ms | OMS tracing span |

## Estrategia de Backpressure

- **Feed → Consumidores**: Ring buffer circular con política `drop_latest` si el consumidor no sigue. Marcar gap en datos.
- **OMS → Broker**: Cola bounded con timeout. Si el broker no responde, la orden pasa a estado `PendingCancel` después de timeout configurable.
- **UI → OMS**: Canal bounded con política `blocking` (el usuario debe esperar confirmación antes de nueva acción).
