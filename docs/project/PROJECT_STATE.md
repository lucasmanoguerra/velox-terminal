# Project State — velox-terminal

Current state of the velox-terminal project.

---

## Completed (Fases 0-1)

- **Agent team setup**: 26 OpenCode agents with routing + MCP integration
- **Workspace**: 14 crates with bounded contexts, profiles (ReleaseSafe para OMS/Risk)
- **CI/CD**: GitHub Actions (lint, test, build, security audit, cross-platform)
- **Repository**: GitHub público `lucasmanoguerra/velox-terminal`
- **Documentation**: 72 markdown files across 17 sections + progressive-disclosure
- **ADRs**: 3 registrados (workspace, concurrency, wgpu)
- **Domain primitives**: Order, Tick, Quote, Candle, OrderState machine
- **Market data**: Ring buffer lock-free, aggregation tick→OHLCV multi-timeframe
- **Indicators**: SMA, EMA, RSI incrementales O(1)
- **OMS**: State machine validation, order manager, fill management
- **Risk**: Validators, position limits, circuit breaker
- **Broker**: BrokerClient trait + MockBroker
- **Connectivity docs**: FIX protocol, WebSocket feed, reconnection strategy
- **Licensing**: 3-tier model (Community/Pro/Enterprise)

## In Progress (Fase 2: Motor de Trading)

- Implementation completa de indicadores (MACD, Bollinger, ATR)
- Property-based tests para OMS/Risk
- Harden order manager (replace, edge cases)

## Next Up

Per `NEXT_ACTIONS.md`:
- Market data feed real (WebSocket)
- Charting engine con wgpu
- Paneles egui funcionales
- Conector FIX/WebSocket para broker

## Known Blockers

- None yet.

---

_Last updated: 2026-07-06_
