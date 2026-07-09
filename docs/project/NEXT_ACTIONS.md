# Next Actions — velox-terminal

Próximas acciones priorizadas. Basado en el roadmap completo en `ROADMAP.md`.

---

## ✅ Completed (hasta 2026-07-09)

### Fundación
- [x] Equipo de 26 agentes OpenCode | ADRs fundacionales
- [x] Workspace 15 crates + CI/CD + GitHub community files
- [x] Documentación `docs/` (17 secciones)

### Domain Core
- [x] Tipos compartidos (Tick, Quote, Candle, Order, OrderState)
- [x] OMS: máquina de estados, fills parciales, proptest
- [x] Risk: validadores pre-trade, circuit breaker, position limits
- [x] Indicadores O(1): SMA, EMA, RSI, MACD, Bollinger, ATR
- [x] BrokerClient trait (port) + MockBroker

### Market Data
- [x] RingBuffer lock-free SPSC con `pop_n` batch optimization
- [x] Pipeline tick→OHLCV multi-timeframe (1m/5m/1h)
- [x] Binance WebSocket: trades en vivo + depth20
- [x] Auto-reconnect con exponential backoff + jitter

### Trading
- [x] PaperTrader: mock execution engine con fills automáticos
- [x] Position tracking: weighted-average cost basis
- [x] P&L: realizado + no realizado
- [x] Account equity tracking
- [x] Buy/Sell market buttons desde UI
- [x] Limit/Stop orders (Market, Limit, Stop Market, Stop Limit)
- [x] Bracket orders (TP/SL auto-create/cancel)
- [x] Binance REST Client y User Data Stream
- [x] BinanceBroker (BrokerClient impl.)
- [x] Live/Paper trading toggle + API keyring storage

### UI/GPU
- [x] ChartRenderer wgpu: grid, candle, volume, line pipelines
- [x] Zoom/pan en vertex shader + scrollbar + follow mode
- [x] Integración egui-wgpu (2 render passes)
- [x] Paneles: order entry, positions, DOM ladder, status bar
- [x] Indicadores overlay: SMA, EMA, RSI (líneas GPU)
- [x] Multi-timeframe selector
- [x] Dark trading theme

---

## 🔥 Priority Queue

### P0 — Emergencia (bugs críticos, seguridad)
> _Nada actualmente._

### P1 — Inmediato (próximo release, ~1-2 semanas)

| # | Feature | Categoría | Dependencias | Esfuerzo |
|---|---------|-----------|-------------|----------|
| 1 | **Binance REST Client** — balances, histórico, comisiones | Conectividad | — | M |
| 2 | **Binance User Data Stream** (`@account`, `@balance`) | Conectividad | — | M |
| 3 | **OMS → Broker real** — enviar órdenes a Binance via REST | Trading | #1, #2 | L |
| 4 | ~~**API Keys seguras** — keyring nativo del SO (keyring-rs)~~ | ✅ | — | S |
| 5 | **Sandbox/Testnet Binance** — entorno de pruebas | Conectividad | — | S |

### P2 — Próximo (~2-4 semanas)

| # | Feature | Categoría | Dependencias | Esfuerzo |
|---|---------|-----------|-------------|----------|
| 6 | ~~**Limit/Stop orders** — entrada en UI y OMS~~ | ✅ | — | M |
| 7 | ~~**Bracket orders** (SL/TP + entrada)~~ | ✅ | #6 | M |
| 8 | **VWAP + VWAP anclado** | Indicadores | — | S |
| 9 | **Volume Profile** (session, anchored, visible range) | Order Flow | — | L |
| 10 | **Heikin Ashi chart type** | Charting | — | M |
| 11 | **DOM profesional** — niveles completos + heatmap | Order Flow | — | L |
| 12 | **Watchlist panel** + favorites + sections | UX | — | M |
| 13 | **Price alerts** + desktop notifications | UX | — | M |
| 14 | **Hotkeys configurables** | UX | — | M |
| 15 | **Position size calculator** (SL/ATR/% account) | Trading | — | S |
| 16 | **Close All / Kill Switch** | Trading | — | S |
| 17 | **Multi-panel charts** (split vertical/horizontal) | UX | — | L |
| 18 | **Conexión BingX** | Conectividad | P1#5 | M |
| 19 | **Conexión Bybit** | Conectividad | P1#5 | M |

### P3 — Camino a producción (~1-2 meses)

| # | Feature | Categoría | Dependencias |
|---|---------|-----------|-------------|
| 20 | **Drawing tools** — líneas, canales, fibonacci, rectángulos | Drawing | — |
| 21 | **Smart Money Concepts (ICT)** — FVG, OB, BOS, CHoCH, etc. | Análisis Técnico | — |
| 22 | **Footprint charts** — delta, cumulative delta, bid/ask split | Order Flow | P2#11 |
| 23 | **Time & Sales tape** — tick feed en tiempo real | Order Flow | — |
| 24 | **Scripting engine (Lua)** — estrategias de usuario | Automatización | — |
| 25 | **Session replay** — tick replay, DOM replay, variable speed | Replay | P2#11 |
| 26 | **Multi-window / pop-out charts** | UX | — |
| 27 | **Command palette** (Ctrl+K) | UX | — |
| 28 | **Custom timeframe** | Charting | — |
| 29 | **Chart trading** — pre-orden en gráfico | Trading | P2#6 |
| 30 | **OCO/OTO/trailing stop** | Trading | P2#6 |

### P4 — Visión (3-6+ meses)

| # | Feature | Categoría |
|---|---------|-----------|
| 31 | Renko, Range Bars, Tick Charts, Kagi, Point & Figure | Chart Types |
| 32 | Backtesting completo con slippage + comisiones | Backtesting |
| 33 | Walk-forward + parameter optimization | Backtesting |
| 34 | Indicadores: Cipher A/B, Market Profile, VPIN | Análisis Técnico |
| 35 | Economic calendar | Datos |
| 36 | AI local (Ollama, Gemini Nano) | IA |
| 37 | MCP server de trading | IA |
| 38 | Screener multi-filtro | UX |
| 39 | Trading journal + statistics | UX |
| 40 | Elliott Wave + Gann | Drawing |
| 41 | FIX protocol | Conectividad |
| 42 | Interactive Brokers connector | Conectividad |
| 43 | Multi-monitor soporte nativo | UX |
| 44 | Cross-compilation + empaquetado (MSI/DMG/AppImage) | Build |
| 45 | Auto-updater | Build |
| 46 | Compliance (MiFID II, SEC) | Compliance |

---

## Suggested Next Task

**P1 · Binance REST Client + User Data Stream**

El flujo end-to-end actual es paper trading. Para operar con dinero real necesitamos:

1. **`BinanceRestClient`** — `GET /api/v3/account` (balances), `GET /api/v3/myTrades` (historial),
   `POST /api/v3/order` (enviar órdenes), `GET /api/v3/exchangeInfo` (symbol rules)
2. **`BinanceAccountFeed`** — conectar a `wss://stream.binance.com:9443/ws/<listenKey>`
   para recibir actualizaciones de balance y estado de órdenes en tiempo real
3. **API key management** — input seguro + keyring-rs + encripción en memoria
4. **Testnet** — `wss://testnet.binance.vision` + `https://testnet.binance.vision/api`
5. **OMS → Broker bridge** — `OrderExecutionPort` trait implementado por `BinanceRestClient`

Esfuerzo tentativo: ~3-4 sesiones de implementación.

---

## Resumen de Crates Impactados

| Crate | Features P1-P2 |
|-------|---------------|
| `velox-exchange` | REST client, account feed, BingX, Bybit |
| `velox-oms` | Limit/stop orders, bracket, OCO, trailing stop |
| `velox-chart` | Heikin Ashi, multi-panel, custom TF, drawing tools |
| `velox-indicators` | VWAP, Volume Profile, FVG, OB, BOS/CHoCH |
| `velox-ui` | Watchlist, alerts, hotkeys, command palette, pop-out |
| `velox-gpu` | New shaders (footprint, volume profile, heatmap) |
| `velox-storage` | Time-series DB for historical data |
| `velox-scripting` | Lua engine, sandboxing |
| `velox-backtest` | Slippage, metrics, optimization |
| `velox-broker-fix` | FIX protocol connector |
| `velox-terminal` | Multi-monitor, multi-window |

---

*Última actualización: 2026-07-09*
