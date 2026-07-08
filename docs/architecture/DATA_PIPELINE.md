# Data Pipeline — velox-terminal

Data flow from external exchanges to the screen, through hexagonal layers.

---

## Architecture: UNIX Pipes through Hexagonal Layers

Each stage is:
1. **One responsibility** (UNIX philosophy: do one thing well)
2. **Self-contained** (testable with mocked inputs/outputs)
3. **Layer-pure** (domain stages have zero I/O, adapters wrap I/O)

```
External World               Adapters                  Domain                   Adapters
═══════════════         ╔══════════════╗         ╔══════════════╗         ╔══════════════╗
                     ╔══╝              ╚══╗   ╔══╝              ╚══╗   ╔══╝              ╚══╗
Binance WebSocket ──►║ BinanceFeed      ║──►║ RingBuffer       ║──►║ CandleAggregator ║──►║ AppState
(JSON trade msgs)    ║ (adapter)        ║   ║ (SPSC pipe)      ║   ║ (pure domain)    ║   ║ (adapter)
                     ╚══╗              ╔══╝   ╚══╗              ╔══╝   ╚══╗              ╔══╝
                        ╚══════════════╝         ╚══════════════╝         ╚══════════════╝
                              │                                              │
                         Layer: Adapter                                  Layer: Domain
                         I/O: tokio WebSocket                            I/O: none
                         Transforms: JSON → Tick                         Transforms: Tick → Candle

                                    │                                            │
                                    ▼                                            ▼
                              ╔══════════════╗                           ╔══════════════╗
                              ║ mpsc channel  ║────(cross-thread)──────►║ ChartRenderer║──► Screen
                              ║ (tokio pipe)  ║                         ║ (wgpu)       ║
                              ╚══════════════╝                         ╚══════════════╝
                                                                        Layer: Adapter
                                                                        I/O: GPU (wgpu)
                                                                        Transforms: Candle → triangles
```

---

## Market Data Pipeline (End-to-End)

```
EXTERNAL WORLD                    LAYER: INFRASTRUCTURE
═══════════════
  Binance WebSocket  ──────────► tokio task (spawned by ExchangeFeed::start())
  wss://stream.binance.com:9443  │
                                 │ WebSocket message (JSON text frame)
                                 ▼
╔══════════════════════════════════════════════════════════════════════╗
║                    LAYER: ADAPTER (velox-exchange)                    ║
║                                                                       ║
║  BinanceFeed::handle_message()                                        ║
║    │                                                                  ║
║    ├── serde_json::from_str() → serde_json::Value                     ║
║    │                                                                  ║
║    ├── Extract: "e" (event type = "trade")                           ║
║    │            "s" (symbol = "BTCUSDT")                             ║
║    │            "p" (price = "45000.25")                             ║
║    │            "q" (quantity = "0.001")                             ║
║    │            "T" (trade_time = 1672515782136)                     ║
║    │            "m" (is_maker_buy = true)                            ║
║    │                                                                  ║
║    └── Construct velox_core::Tick { symbol, price, volume, timestamp } ║
║                                                                       ║
║  ring.push(MarketEvent::Tick(tick)) ──────────────────────────────────║
║    │                        (lock-free, ~50ns)                        ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    │  ┌────────────────────────────────────────────────────────┐
    │  │  RingBuffer (SPSC) — crossbeam::segqueue::SegQueue     │
    │  │  Fixed-size, lock-free, single-producer single-consumer│
    │  │  Drop-latest if consumer is slow (backpressure)        │
    │  └────────────────────────────────────────────────────────┘
    │
    ▼
╔══════════════════════════════════════════════════════════════════════╗
║                 LAYER: DOMAIN (velox-md)                              ║
║                                                                       ║
║  MarketDataPipeline::poll()  ← called every frame from main thread   ║
║    │                                                                  ║
║    │  loop {                                                         ║
║    │      ring.pop() → Some(MarketEvent::Tick(tick))                 ║
║    │                       │                                          ║
║    │                       ▼                                          ║
║    │              CandleAggregator::process_tick(&tick)               ║
║    │                │                                                 ║
║    │                ├── Update current candle (O(1))                  ║
║    │                │   (open → high → low → close → volume)        ║
║    │                │                                                 ║
║    │                └── If timestamp crosses → emit completed Candle  ║
║    │                    │                                             ║
║    │                    └── candle_tx.send(candle) ──────────►        ║
║    │                             (mpsc::UnboundedSender)              ║
║    │  }                                                               ║
║                                                                       ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    │  mpsc::UnboundedReceiver<Candle>  (tokio::sync)
    │  Cross-thread: tokio producer → main thread consumer
    │
    ▼
╔══════════════════════════════════════════════════════════════════════╗
║                 LAYER: ADAPTER (velox-ui)                             ║
║                                                                       ║
║  AppState::poll_candles()  ← called every frame                      ║
║    │                                                                  ║
║    │  loop { rx.try_recv() → candle }                                 ║
║    │      │                                                           ║
║    │      ├── Store in candles_by_tf[timeframe_secs]                  ║
║    │      └── If matches active timeframe → append to self.candles    ║
║    │                                                                  ║
║    └── If first candle → ChartInteraction::reset_view()              ║
║                                                                       ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    ▼
╔══════════════════════════════════════════════════════════════════════╗
║               LAYER: ADAPTER (velox-chart → velox-gpu)               ║
║                                                                       ║
║  composite_render()  ← every frame                                   ║
║    │                                                                  ║
║    ├── PASS 1: ChartRenderer::render()                                ║
║    │   ├── update_from_state() → upload candle data to GPU buffers   ║
║    │   ├── Scissor rect to chart area                                 ║
║    │   ├── Clear background (#121218)                                ║
║    │   ├── Render grid (vertex buffer → grid.wgsl)                   ║
║    │   ├── Render candles (instanced → candle.wgsl)                  ║
║    │   └── Render volume (instanced → volume.wgsl)                   ║
║    │                                                                  ║
║    └── PASS 2: egui_wgpu::Renderer::render()                         ║
║        ├── LoadOp::Load (alpha blend over chart)                     ║
║        └── UI panels: top bar, order entry, positions, status bar    ║
║                                                                       ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    └──► wgpu::Surface present ──► GPU ──► Monitor
```

---

## Order Pipeline

```
(User clicks Buy/Sell)

╔══════════════════════════════════════════════════════════════════════╗
║                 LAYER: ADAPTER (velox-ui / egui)                      ║
║                                                                       ║
║  PanelManager::show() → order_entry panel                            ║
║    │                                                                  ║
║    └── User fills: side, quantity, price (optional)                  ║
║        clicks "Place Order"                                          ║
║                                                                       ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    │  crossbeam channel or direct function call
    │
    ▼
╔══════════════════════════════════════════════════════════════════════╗
║                    LAYER: DOMAIN (velox-oms)                          ║
║                                                                       ║
║  OrderManager::submit_order(new_order)                                ║
║    │                                                                  ║
║    ├── 1. Validate state transition: None → PendingNew               ║
║    │       │                                                          ║
║    │       ▼                                                          ║
║    ├── 2. Pre-trade validation via RiskValidator                      ║
║    │       │                                                          ║
║    │       ├── RiskValidator::validate_order()                        ║
║    │       │     │                                                    ║
║    │       │     ├── Position limit check (open + new ≤ max)          ║
║    │       │     ├── Circuit breaker check (volatility halt?)         ║
║    │       │     └── Symbol allowed?                                  ║
║    │       │                                                          ║
║    │       ├── PASS → continue                                        ║
║    │       └── FAIL → return OrderError, UI shows rejection           ║
║    │                                                                  ║
║    ├── 3. Create Order { id, state: PendingNew }                      ║
║    │                                                                  ║
║    └── 4. Send to broker adapter via BrokerClient trait               ║
║                      │                                                ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    │  trait call (async, dispatched through composition root)
    │
    ▼
╔══════════════════════════════════════════════════════════════════════╗
║                LAYER: ADAPTER (velox-broker)                          ║
║                                                                       ║
║  MockBroker::submit_order()  ── or ──  FixBrokerClient::submit_order()║
║    │                                                                  ║
║    ├── In dev: MockBroker immediately returns a simulated fill        ║
║    └── In production: FIX/WebSocket message to exchange               ║
║                                                                       ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    │  Broker response (or timeout)
    │
    ▼
╔══════════════════════════════════════════════════════════════════════╗
║                    LAYER: DOMAIN (velox-oms)                          ║
║                                                                       ║
║  OrderManager::handle_broker_response(order_id, response)             ║
║    │                                                                  ║
║    ├── State transition: PendingNew → New (accepted)                  ║
║    │                     PendingNew → Rejected (broker rejected)      ║
║    │                     New → PartiallyFilled (partial fill)         ║
║    │                     PartiallyFilled → Filled (fully filled)     ║
║    │                                                                  ║
║    └── Event published to state channels                              ║
║                      │                                                ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    ▼
╔══════════════════════════════════════════════════════════════════════╗
║                 LAYER: ADAPTER (velox-ui / egui)                      ║
║                                                                       ║
║  AppState updated → PanelManager reads new state next frame          ║
║    Positions panel updates                                            ║
║    Order status shown in order entry panel                            ║
║                                                                       ║
╚══════════════════════════════════════════════════════════════════════╝
```

---

## Backtesting Pipeline

```
╔══════════════════════════════════════════════════════════════════════╗
║              LAYER: ADAPTER (velox-storage)  [WIP]                    ║
║                                                                       ║
║  StorageEngine::read_ticks(symbol, start, end)                       ║
║    │                                                                  ║
║    └── Returns Vec<Tick> from SQLite/REDB                            ║
║                                                                       ║
╚══════════════════════════════════════════════════════════════════════╝
    │
    ▼
╔══════════════════════════════════════════════════════════════════════╗
║                 LAYER: DOMAIN (velox-backtest)                        ║
║                                                                       ║
║  BacktestEngine::run(start, end, strategy)                            ║
║    │                                                                  ║
║    ├── Replay ticks through same pipeline as live:                    ║
║    │     Tick → CandleAggregator → Indicators → Strategy → OMS       ║
║    │                                                                  ║
║    ├── Simulated fills with configurable slippage                     ║
║    │                                                                  ║
║    └── P&L, Sharpe ratio, max drawdown, win rate                     ║
║                                                                       ║
║  Reuses domain crates (velox-md, velox-oms, velox-indicators)         ║
║  directly — same logic as live trading, different data source.        ║
║                                                                       ║
╚══════════════════════════════════════════════════════════════════════╝
```

---

## Timeframe Expansion: Multi-Timeframe Pipeline

The `CandleAggregator` maintains independent state for each timeframe.

```
Stream of ticks:
  │
  ├──► Timeframe 1m (60s)  ──► candles[60]
  ├──► Timeframe 5m (300s) ──► candles[300]
  └──► Timeframe 1h (3600s)──► candles[3600]

Each bucket: Vec<Candle> with 500-candle window (ring buffer).
User selects active timeframe → AppState::set_timeframe(tf) swaps the view.
```

---

## Transformation Summary

| Stage | Input | Output | Transform | Layer |
|-------|-------|--------|-----------|-------|
| BinanceFeed | JSON text frame | `MarketEvent::Tick` | Deserialize + field extract | Adapter |
| RingBuffer | Push (tokio) / Pop (main) | None (pass-through) | Lock-free transfer | Pipe |
| CandleAggregator | `&Tick` | `Vec<Candle>` (completed) | Time-windowed OHLCV aggregation | Domain |
| mpsc channel | Candle (tokio task) | Candle (main thread) | Cross-thread transfer | Pipe |
| AppState | Candle (channel) | `self.candles: Vec<Candle>` | Multi-TF storage + window | Adapter |
| ChartRenderer | `&[Candle]` | GPU buffer upload | SoA → WGSL struct → vertex/instance data | Adapter |
| wgpu pipeline | Vertex/Instance buffers | Screen pixels | Instanced rendering (candle.wgsl, grid.wgsl, volume.wgsl) | Infrastructure |

---

## Latency Budget

| Stage | p50 | p99 | Where measured |
|-------|-----|-----|---------------|
| WebSocket → Tick | 50μs | 200μs | tracing span |
| RingBuffer push | 50ns | 200ns | criterion |
| RingBuffer pop | 50ns | 200ns | criterion |
| Tick → Candle update | 1μs | 5μs | criterion |
| Candle → AppState | 5μs | 20μs | tracing span |
| Chart GPU upload | 100μs | 500μs | Tracy frame profiler |
| wgpu render + present | 1ms | 5ms | Tracy frame profiler |
| **End-to-end (tick → screen)** | **~1.5ms** | **~6ms** | Tracy |

---

## Backpressure Strategy

| Stage | Mechanism | Policy |
|-------|-----------|--------|
| BinanceFeed → RingBuffer | Ring buffer full → `push` returns error | Mark gap, drop latest |
| Pipeline → mpsc channel | `UnboundedSender::send` never blocks | **Risk**: unbounded growth if main thread lags. Mitigation: channel drained every frame (~16ms). |
| UI → OMS (future) | Bounded channel with timeout | User waits for confirmation before next action |

---

## Key hex, not hex

| Property | Live Pipeline | Backtest Pipeline |
|----------|--------------|-------------------|
| Data source | Binance WebSocket | Storage engine (disk) |
| Tick source | ExchangeFeed adapter | Vec<Tick> replay |
| Candle aggregator | Real-time (poll per frame) | Batch (simulated time) |
| OMS | Real fills from broker | Simulated fills with slippage |
| Indicators | Streaming O(1) | Same code |
| UI | egui panels | Report generation |
| **Domain code reused** | — | **100%** (same crates) |
