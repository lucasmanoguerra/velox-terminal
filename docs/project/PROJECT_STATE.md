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
- **Exchange adapter**: Binance WebSocket feed (trades) con auto-reconnect (backoff + jitter)
- **Charting**: wgpu renderer, WGSL shaders, instanced geometry, zoom/pan
- **UI**: egui panels over wgpu, dark theme, order entry, positions, multi-timeframe selector
- **Connectivity docs**: FIX protocol, WebSocket feed, reconnection strategy
- **Licensing**: 3-tier model (Community/Pro/Enterprise) — design doc
- **Community**: CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, issue/PR templates

## In Progress (Phase 2-3: Adapters)

- Conectores a más exchanges (BingX, Bybit, Kraken)
- Order book depth stream
- OMS conectado con UI (botón Place Order → orden real)
- Backtesting con slippage

## Completed Recently

- **Indicadores overlay en chart**: SMA/EMA/RSI renderizados como líneas GPU
  (LineList vertex-buffer, per-vertex colors, NaN handling, UI toggles)

## Next Up

Ver `NEXT_ACTIONS.md` para la cola priorizada.

## Known Blockers

- None yet.

---

_Last updated: 2026-07-08_
