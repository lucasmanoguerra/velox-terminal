# Next Actions — velox-terminal

Próximas acciones priorizadas después del último avance.

---

## ✅ Completed

| Item | Phase | Date |
|------|-------|------|
| Workspace Cargo con 14 crates + estructura bounded-context | P0 | 2026-07-06 |
| Modelo de concurrencia (tokio async + hilo único UI + crossbeam hot paths) | P0 | 2026-07-06 |
| Estructuras Tick/Quote/OHLCV con cache-locality (SoA) | P0 | 2026-07-06 |
| Pipeline agregación tick → OHLCV multi-timeframe | P0 | 2026-07-06 |
| ADRs (workspace, concurrencia, wgpu) + MSRV + perfiles ReleaseSafe/ReleaseFast | P0 | 2026-07-06 |
| CI básico (GitHub Actions: lint, test, build) | P0 | 2026-07-06 |
| Market data feed con ring buffer lock-free (SPSC) | P1 | 2026-07-06 |
| Máquina de estados OMS con enums de Rust (20 tests + proptest) | P1 | 2026-07-06 |
| Validaciones pre-trade Risk Management + circuit breaker | P1 | 2026-07-06 |
| Charting engine con wgpu (geom instanciada, zoom/pan en vertex shader) | P1 | 2026-07-07 |
| Paneles básicos egui (chart, order entry, positions, status bar) | P1 | 2026-07-07 |
| Integración egui-wgpu + charting engine (mismo contexto) | P1 | 2026-07-07 |
| Indicadores: SMA, EMA, RSI, MACD, Bollinger, ATR (incrementales O(1)) | P2 | 2026-07-06 |
| Property-based tests OMS (proptest) | P2 | 2026-07-06 |
| WebSocket market data feed real (Binance) | P2 | 2026-07-08 |
| Multi-timeframe (1m/5m/1h) con selector UI | P2 | 2026-07-08 |
| Auto-reconnect WebSocket con backoff exponencial + jitter | P2 | 2026-07-08 |
| **OMS + UI Integration** (PaperTrader, Buy/Sell, Positions, P&L) | **P1** | **2026-07-08** |
| **Indicadores overlay en chart** (SMA/EMA/RSI GPU lines) | **P1** | **2026-07-08** |
| **DOM ladder / Order Book Depth** (Binance @depth20@100ms) | **P1** | **2026-07-08** |
| **Scrollbar horizontal + follow mode** | **P1** | **2026-07-09** |
| **RingBuffer::pop_n batch tick consumption** | **P1** | **2026-07-09** |

---

## 🔥 Priority Queue

### P0 — Ahora (cimientos para features avanzados)

- [ ] **OVERRIDE — NADA. Los P0 están completos.**

### P1 — Inmediato (construir sobre el flujo end-to-end)

- [ ] **OVERRIDE — NADA. Todos los P1 están completos.**

### P2 — Próximo (features de trading real)

- [ ] Conector Binance REST + WebSocket para Account/Balances
- [ ] OMS integrado con Broker (enviar órdenes a Binance)
- [ ] Backtesting con slippage realista
- [ ] Tests OMS/Risk con proptest (expandir coverage)
- [ ] CI/CD multiplataforma (Windows, macOS, Linux)

### P3 — Camino a producción

- [ ] Keyring para credenciales seguras
- [ ] Benchmarks de latencia end-to-end (criterion + Tracy)
- [ ] Documentación compliance (MiFID II, SEC)
- [ ] Compilación cruzada y empaquetado (MSI/DMG/AppImage)
- [ ] Hotkeys configurables
- [ ] Motor de scripting (Lua embebido)

### P4 — Mantenimiento continuo

- [ ] Release management (SemVer + changelogs)
- [ ] Sistema de licencias
- [ ] SRE/Observabilidad (tracing, Sentry)
- [ ] cargo-audit + cargo-outdated periódico
- [ ] Triage de bugs

---

## Suggested Next Task

**P2 · Conector Binance REST + WebSocket para Account/Balances**

Todos los P1 están completos. El siguiente paso lógico es integrar la terminal
con cuentas reales de Binance:

1. Implementar `BinanceAccountFeed` — conecta a streams de balance/ordenes (`@account`)
2. Implementar `BinanceRestClient` — consultar balances, histórico de trades, comisiones
3. OMS → envío real de órdenes via REST a Binance (con autenticación HMAC-SHA256)
4. Manejo seguro de API keys (keyring nativo del SO)
5. Pruebas con sandbox/testnet de Binance
