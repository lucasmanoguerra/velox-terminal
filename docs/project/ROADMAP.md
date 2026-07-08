# Roadmap — velox-terminal

Hoja de ruta del proyecto basada en arquitectura hexagonal.
Intercambiable por capas: domain core → ports → adapters.

---

## 🎯 Fase 0: Fundación (Completada)
**Objetivo**: Proyecto scaffolded, toolchain listo, equipo de agentes operativo.

- [x] Definición del equipo de 26 agentes en `.opencode/agents/`
- [x] Estructura de documentación `docs/` (17 secciones)
- [x] Workspace Cargo con crate layout (15 crates)
- [x] Integración con codebase-memory-mcp
- [x] CI básico (lint + test + build + security audit)
- [x] GitHub repo público + community files (CONTRIBUTING, CODE_OF_CONDUCT, templates)

## 🧱 Fase 1: Domain Core (Completada)
**Objetivo**: Lógica de dominio pura, sin dependencias de infraestructura.

### Domain
- [x] `velox-core`: Tipos compartidos (Tick, Quote, Candle, Order, OrderState)
- [x] `velox-oms`: Máquina de estados de órdenes con enums de Rust
- [x] `velox-risk`: Validaciones pre-trade, circuit breakers
- [x] `velox-indicators`: Indicadores incrementales O(1) (SMA, EMA, RSI, MACD, Bollinger, ATR)
- [x] `velox-backtest`: Motor de simulación histórica (en progreso)

### Ports
- [x] `velox-broker`: Trait `BrokerClient` para conectores de broker
- [x] `ExchangeFeed` trait definido en `velox-exchange`
- [ ] `OrderExecutionPort` trait en `velox-oms`
- [ ] `MarketDataPort` trait en `velox-risk`
- [ ] `StoragePort` trait en `velox-storage`

### Testing
- [x] Property-based tests para OMS (proptest)
- [ ] Property-based tests para Risk
- [ ] Mock adapters para todos los ports

## 🔌 Fase 2: Adapters — Conectividad Real (En Progreso)
**Objetivo**: Conectar el domain core con el mundo real a través de adaptadores.

### Exchanges
- [x] `velox-exchange` adapter: Binance WebSocket (trades en vivo)
- [x] Reconexión automática con exponential backoff + jitter
- [ ] `velox-exchange` adapter: Binance REST (cuentas, balances, órdenes)
- [ ] `velox-exchange` adapter: Order book depth (`@depth20@100ms`)
- [ ] `velox-exchange` adapter: BingX
- [ ] `velox-exchange` adapter: Bybit
- [ ] Adapter: Indices y forex (fuentes a definir)

### Broker
- [ ] `velox-broker-fix` adapter: FIX protocol (conexión, heartbeats, mensajes)
- [ ] OMS → OrderExecutionPort → Broker adapter (enviar órdenes reales)

### Storage
- [ ] `velox-storage` adapter: Base de datos time-series embebida
- [ ] Compresión y particionado de históricos

## 🖥️ Fase 3: Adapters — UI/GPU (En Progreso)
**Objetivo**: Visualización y control del trading en pantalla.

- [x] `velox-gpu`: Primitivas de renderizado wgpu (shaders WGSL)
- [x] `velox-chart`: Charting engine con geometría instanciada
- [x] `velox-ui`: Paneles egui (order entry, positions, status bar)
- [x] Integración egui-wgpu: dos render passes (chart → UI)
- [x] Multi-timeframe (1m/5m/1h) con selector en UI
- [ ] **Indicadores overlay en chart** (SMA/EMA/RSI como líneas GPU)
- [ ] DOM ladder / order book
- [ ] Scrollbar horizontal para navegación histórica
- [ ] Hotkeys configurables
- [ ] Temas customizables

## 🔬 Fase 4: Backtesting & Scripting
**Objetivo**: Estrategias algorítmicas con simulación realista.

- [ ] Backtesting completo con slippage comisiones
- [ ] Walk-forward analysis
- [ ] Métricas: Sharpe, Sortino, Calmar, drawdown
- [ ] Motor de scripting (Lua embebido vía mlua)
- [ ] Sandboxing de scripts de usuario

## 🚀 Fase 5: Producción
**Objetivo**: Producto tradeable con seguridad, performance y comunidad.

- [ ] Seguridad de credenciales (keyring, cargo-audit)
- [ ] Benchmarks de latencia end-to-end (criterion, Tracy)
- [ ] Compilación cruzada y empaquetado nativo (MSI/DMG/AppImage)
- [ ] Sistema de licencias
- [ ] SRE/Observabilidad (tracing, Sentry, dashboards)
- [ ] Release management (SemVer + changelogs)
- [ ] Compliance (MiFID II, SEC)

## Timeline

| Fase | Duración | Estado |
|------|----------|--------|
| Fase 0: Fundación | 1-2 días | ✅ Completada |
| Fase 1: Domain Core | 1-2 semanas | ✅ Completada |
| Fase 2: Adapters (Conectividad) | 3-4 semanas | 🔄 En progreso |
| Fase 3: Adapters (UI/GPU) | 2-3 semanas | 🔄 En progreso |
| Fase 4: Backtesting & Scripting | 3-4 semanas | ⏳ Pendiente |
| Fase 5: Producción | 4-6 semanas | ⏳ Pendiente |

---

*Última actualización: 2026-07-08*
