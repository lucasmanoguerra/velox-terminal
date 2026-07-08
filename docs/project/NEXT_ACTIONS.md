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

---

## 🔥 Priority Queue

### P0 — Ahora (cimientos para features avanzados)

- [ ] **OVERRIDE — NADA. Los P0 están completos.**

### P1 — Inmediato (construir sobre el flujo end-to-end)

- [ ] **Conectar OMS con la UI** — botón Buy/Sell/Place Order debe crear órdenes reales contra MockBroker
- [ ] **Indicadores overlay en chart** — SMA/EMA/RSI renderizados como líneas sobre las velas (nuevos pipelines wgpu)
- [ ] **DOM depth / order book ladder** — primer panel de order book (bid/ask) con datos reales via Binance `@depth20@100ms`
- [ ] **Scrollbar horizontal** — para navegar velas históricas sin zoom infinito
- [ ] **CandleAggregator multi-tick fix** — pipeline consume ticks en lote (RingBuffer.pop_n) en vez de 1 por frame

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
- [ ] DOM ladder (no solo order book plano)
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

**P1 · Indicadores overlay en chart (SMA/EMA/RSI)**

El chart ya renderiza velas y volumen. El siguiente paso lógico es agregar líneas de indicadores técnicos como overlays. Esto implica:
1. `velox-indicators` → exponer valores como `Vec<f32>` en un buffer linearizado
2. `velox-gpu` → nuevo shader `line.wgsl` (ya existe stub) con pipeline de líneas
3. `velox-chart` → `IndicatorRenderer` que administra buffers de vértices de líneas
4. `AppState` → indicadores activos + dirty flag
5. UI → toggle de indicadores en menú o panel lateral

Estimación: ~2-3 sesiones de implementación.
