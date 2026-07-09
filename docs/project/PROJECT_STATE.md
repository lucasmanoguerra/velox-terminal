# Project State — velox-terminal

Current state of the velox-terminal project.

---

## Completed (Domain Core + Adapters base)

- **Agent team setup**: 26 OpenCode agents with routing, hexagonal + UNIX philosophy, MCP integration
- **Workspace**: 15 crates organized in hexagonal layers (domain / ports / adapters / application)
- **CI/CD**: GitHub Actions (lint, test, build, security audit, cross-platform) + gh CLI workflow
- **Repository**: GitHub público `lucasmanoguerra/velox-terminal` con community files
- **Documentation**: 80+ files across 17 sections + progressive-disclosure + ADRs
- **ADRs**: 4 registrados (workspace, concurrency, wgpu rendering, hexagonal architecture)
- **Domain primitives**: Order, Tick, Quote, Candle, OrderState machine
- **Market data**: Ring buffer lock-free, aggregation tick→OHLCV multi-timeframe
- **Indicators**: SMA, EMA, RSI, MACD, Bollinger, ATR incrementales O(1)
- **OMS**: State machine validation, order manager, fill management, property-based tests
- **Risk**: Validators, position limits, circuit breaker
- **Broker**: BrokerClient trait (port) + MockBroker
- **Exchange adapter**: Binance WebSocket feed (trades + depth20) con auto-reconnect (backoff + jitter)
- **Charting**: wgpu renderer, WGSL shaders, instanced geometry, zoom/pan
- **UI**: egui panels over wgpu, dark theme, order entry, positions, multi-timeframe selector
- **Connectivity docs**: FIX protocol, WebSocket feed, reconnection strategy
- **Licensing**: 3-tier model (Community/Pro/Enterprise) — design doc
- **Community**: CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, issue/PR templates

## Fase Actual

| Fase | Estado |
|------|--------|
| Fase 0: Fundación | ✅ Completada |
| Fase 1: Domain Core | ✅ Completada |
| Fase 2: Conectividad & Market Data | 🔄 En progreso |
| Fase 3: Charting Core | 🔄 En progreso |
| Fase 4: Trading & Órdenes | 🔄 En progreso |
| Fase 5: Análisis Técnico Avanzado | 📋 Planificado |
| Fase 6: Order Flow & Volumen | 📋 Planificado |
| Fase 7: Herramientas de Dibujo | 📋 Planificado |
| Fase 8: UX Multi-ventana & Productividad | 📋 Planificado |
| Fase 9: Automatización & Scripting | 📋 Planificado |
| Fase 10: Backtesting & Optimización | 📋 Planificado |
| Fase 11: Riesgo & Compliance | 📋 Planificado |
| Fase 12: IA & Innovación | 📋 Planificado |
| Fase 13: Producción & Distribución | 📋 Planificado |

## Completed Recently

- **RingBuffer::pop_n batch tick consumption**: Batch-aware ring buffer reduces
  atomic operations from 3-per-tick to 3-per-batch. Pipeline uses pop_n(128) in
  a drain loop, reusing the Vec between frames.
- **Horizontal scrollbar + follow mode**: Scrollbar slider at bottom of chart
  for navigating historical candles. Follow mode auto-scrolls to newest data.
  Toggle with 🔒Follow/🔓Free button.
- **Order book depth (DOM ladder)**: Binance @depth20@100ms combined stream with
  trade feed. DOM panel shows top-20 bids/asks with green/red volume bars, spread,
  and mid price. Fixed BinanceFeed to use proper combined stream endpoint.
- **OMS + UI Integration**: PaperTrader mock execution engine with auto-fill at candle close
  price, weighted-average position tracking, realized/unrealized P&L, account equity.
  Buy/Sell buttons wired, Positions panel shows open orders (with cancel), positions with
  P&L, and account summary.
- **Indicadores overlay en chart**: SMA/EMA/RSI renderizados como líneas GPU
  (LineList vertex-buffer, per-vertex colors, NaN handling, UI toggles)

## Next Up

Ver `NEXT_ACTIONS.md` para la cola priorizada (P1: Binance REST + ejecución real de órdenes).

## Known Blockers

- None yet.

---

_Last updated: 2026-07-08_
