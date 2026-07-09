# Roadmap — velox-terminal

Hoja de ruta integral del proyecto. Organizada por fases (dependencia ascendente)
y categorías funcionales. Cada feature incluye prioridad y estado.

> **Ver también**: [`INSPIRATION.md`](../reference/INSPIRATION.md) — proyectos de referencia y extracción de features.

---

## 🏁 Fase 0: Fundación (✅ Completada)

**Objetivo**: Proyecto scaffolded, toolchain listo, equipo de agentes operativo.

### Infraestructura
- [x] Definición del equipo de 26 agentes en `.opencode/agents/`
- [x] Estructura de documentación `docs/` (17 secciones)
- [x] Workspace Cargo con crate layout (15 crates)
- [x] Integración con codebase-memory-mcp
- [x] CI básico (lint + test + build + security audit)
- [x] GitHub repo público + community files
- [x] ADRs fundacionales (workspace, concurrencia, wgpu, hexagonal)

---

## 🧱 Fase 1: Domain Core (✅ Completada)

**Objetivo**: Lógica de dominio pura, sin dependencias de infraestructura.

### Tipos Compartidos
- [x] `velox-core`: Tick, Quote, Candle, Order, OrderState, Position, Account
- [x] SoA (Struct of Arrays) layout para cache locality
- [x] Serialización zero-copy via bytemuck

### Order Management System
- [x] Máquina de estados de órdenes con enums de Rust
- [x] States: New, PendingNew, PartiallyFilled, Filled, Canceled, Rejected, etc.
- [x] Transiciones: submit, fill (parcial/total), cancel, replace, expire
- [x] Property-based tests (proptest) para invarianzas
- [x] PaperTrader: mock execution engine con fills automáticos
- [x] Position tracking: weighted-average cost basis
- [x] P&L: realizado + no realizado

### Indicadores Técnicos (Core)
- [x] SMA (Simple Moving Average)
- [x] EMA (Exponential Moving Average)
- [x] RSI (Relative Strength Index)
- [x] MACD (Moving Average Convergence Divergence)
- [x] Bollinger Bands
- [x] ATR (Average True Range)
- [x] Cálculo incremental O(1) en todos

### Risk Management
- [x] Validadores pre-trade
- [x] Límites de posición
- [x] Circuit breaker pattern
- [x] Property-based tests (TODO: expandir)

### Ports (Interfaces)
- [x] `BrokerClient` trait en `velox-broker`
- [x] `ExchangeFeed` trait en `velox-exchange`
- [ ] `OrderExecutionPort` trait en `velox-oms`
- [ ] `MarketDataPort` trait en `velox-md`
- [ ] `StoragePort` trait en `velox-storage`

---

## 🔌 Fase 2: Conectividad & Market Data (🚧 En Progreso)

**Objetivo**: Datos de mercado en tiempo real y conexión con brokers.

### Exchange Feeds: Trades
- [x] Binance WebSocket (`<symbol>@trade`)
- [x] RingBuffer lock-free SPSC (pop_n batch optimization)
- [x] Auto-reconnect con exponential backoff + full jitter
- [x] Multi-timeframe aggregation (1m, 5m, 1h)
- [x] Pipeline tick→OHLCV via mpsc channel

### Exchange Feeds: Order Book
- [x] Binance `@depth20@100ms` (top-20 bids/asks)
- [x] DOM ladder UI con barras de volumen
- [ ] **Order Book completo** — snapshots + incremental updates (depthUpdate)
- [ ] **Niveles completos** — soporte para +20 niveles

### Exchange Feeds: Account & Balances
- [ ] Binance User Data Stream (`@account`, `@balance`)
- [ ] OMS → ejecución real de órdenes vía REST (HMAC-SHA256)
- [ ] REST client: consultar balances, historial, comisiones
- [ ] Manejo seguro de API keys (keyring nativo del SO)
- [ ] Sandbox/testnet de Binance

### Más Exchanges
- [ ] BingX WebSocket + REST
- [ ] Bybit WebSocket + REST
- [ ] Kraken WebSocket + REST
- [ ] Intercambios descentralizados (DEX) — futura evaluación

### Brokers Tradicionales
- [ ] FIX protocol connector (velox-broker-fix)
- [ ] Interactive Brokers connector (TWS API)
- [ ] Soporte multi-broker simultáneo

### Calendario Económico
- [ ] Fuente de datos macroeconómicos
- [ ] Filtros: país, relevancia, categoría, fecha
- [ ] Notificaciones automáticas pre-evento

---

## 📊 Fase 3: Charting Core (🚧 En Progreso)

**Objetivo**: Renderizado GPU profesional de velas e indicadores.

### Chart Engine
- [x] ChartRenderer con wgpu (pipeline: grid, candle, volume, line)
- [x] Shaders WGSL con geometría instanciada
- [x] Zoom/pan en vertex shader
- [x] Scrollbar horizontal con follow mode
- [x] OverlayManager para indicadores en chart
- [x] Indicadores overlay: SMA, EMA, RSI (líneas GPU)

### Tipos de Gráfico
- [ ] **Candlestick** (✅ base, refinar)
- [ ] **Heikin Ashi** — cálculo suavizado sobre OHLC raw
- [ ] **Renko** — gráfico de ladrillos (volátil/regular)
- [ ] **Range Bars** — barras por rango de precio fijo
- [ ] **Tick Charts** — barras por N ticks
- [ ] **Volume Bars** — barras por N volumen
- [ ] **Kagi** — gráfico de líneas con cambios de dirección
- [ ] **Line Break** — N-line break charts
- [ ] **Point & Figure** — gráfico clásico de puntos y equis
- [ ] **Base/Versus** — toggle entre tipos sin perder datos

### Timeframes
- [x] Multi-timeframe selector (1m, 5m, 1h)
- [ ] **Custom timeframe** — entrada manual (ej: 47m, 3h)
- [ ] **Timeframes linked** — cambiar TF en un panel = cambia en todos
- [ ] **Comparative overlay** — dos activos superpuestos
- [ ] **Sync mode** — switch para sync time/price/timestamp

### Multi-panel & Multi-window
- [ ] **Gráficos multi-panel** — split vertical/horizontal
- [ ] **Paneles desacoplables (pop-out)** — arrastrar a nueva ventana
- [ ] **Multi-monitor** — ventanas independientes en distintos monitores
- [ ] **Paneles sincronizados** — símbolo, timeframe, precio
- [ ] **Switch sync** — toggle qué sincronizar (time, price, crosshair)

### Chart Trading
- [ ] **Pre-orden en gráfico** — click en precio para crear orden
- [ ] **Position lines** — línea horizontal en precio de entrada
- [ ] **Order lines** — líneas de SL/TP en chart
- [ ] **Drag & drop orders** — mover SL/TP arrastrando líneas

---

## 💹 Fase 4: Trading & Órdenes (🚧 En Progreso)

**Objetivo**: Ejecución profesional de órdenes con gestión de riesgo incorporada.

### Order Entry
- [x] Buy/Sell market buttons
- [x] Quantity slider
- [ ] **One-click trading** — click = orden sin confirmación (configurable)
- [ ] **Pre-orden en chart** — click en precio del chart
- [ ] **Order presets** — configuraciones guardadas de órdenes

### Tipos de Orden
- [x] Market order
- [ ] Limit order (precio límite)
- [ ] Stop market / Stop limit
- [ ] **Bracket order** — entrada + SL + TP simultáneos
- [ ] **OCO** (One-Cancels-Other) — par de órdenes
- [ ] **OTO** (One-Triggers-Other) — orden secuencial
- [ ] **Trailing stop** — SL dinámico que sigue al precio
- [ ] **Iceberg** — orden visible parcial + reserva
- [ ] **TWAP / VWAP** — órdenes algorítmicas de ejecución

### Gestión de Posiciones
- [x] Position tracking (weighted avg cost basis)
- [x] Realized / Unrealized P&L
- [ ] **Partial close** — cerrar fracción de la posición
- [ ] **Pyramiding** — añadir a posición existente
- [ ] **Reverse position** — cerrar y abrir en dirección opuesta
- [ ] **Scale in / Scale out** — entrada/salida escalonada
- [ ] **Auto break-even** — mover SL a breakeven cuando precio > umbral
- [ ] **Platten** — cubrir posición con instrumento correlacionado

### Acciones Masivas
- [ ] **Close All** — cerrar todas las posiciones abiertas
- [ ] **Kill Switch** — botón de emergencia: cancela todo, cierra todo
- [ ] **Flatten by symbol** — cerrar todo de un símbolo

### Risk Calculator
- [ ] **Position size calculator**:
  - Por stop loss (riesgo fijo $)
  - Por ATR (volatilidad)
  - Por porcentaje de cuenta
  - Por riesgo/reward ratio
- [ ] Max position size por símbolo
- [ ] Max drawdown alerta / stop
- [ ] Max daily loss limit

---

## 📈 Fase 5: Análisis Técnico Avanzado (📋 Planificado)

**Objetivo**: Indicadores y herramientas de smart money / ICT / order flow.

### Indicadores Core (expandir)
- [ ] **VWAP** (Volume-Weighted Average Price)
- [ ] **VWAP anclado** (Anchor VWAP) — desde un punto específico
- [ ] **Volume Profile** — perfil de volumen horizontal
- [ ] **Market Profile** — distribución de TPO (Time Price Opportunity)
- [ ] **SMA/EMA** (✅ básico, mejorar: multiples longitudes, sources)

### Smart Money Concepts (ICT)
- [ ] **FVG** (Fair Value Gap) — detección automática
- [ ] **iFVG** (Imbalance FVG) — FVG invertido
- [ ] **Mitigation FVG** — FVG mitigado/rellenado
- [ ] **Order Blocks** — bloques de órdenes (bullish/bearish)
- [ ] **Breaker Blocks** — order blocks que fallan
- [ ] **Mitigation Blocks** — bloques mitigados
- [ ] **Liquidity Pools** — pools de liquidez (above highs, below lows)
- [ ] **Liquidity Sweep** — barrido de liquidez
- [ ] **BOS** (Break of Structure) — quiebre de estructura de mercado
- [ ] **CHoCH** (Change of Character) — cambio de carácter
- [ ] **MSS** (Market Structure Shift) — cambio de estructura
- [ ] **Premium / Discount** — zonas de precio premium/discount respecto a VWAP
- [ ] **Dealing Range** — rango de negociación
- [ ] **Judas Swing** — movimiento engañoso antes de la dirección real

### Session & Time-based
- [ ] **Kill Zones** — zonas horarias de alta liquidez (London, New York, Asia)
- [ ] **Session Boxes** — rectángulos horarios de sesiones
- [ ] **Opening Range** — rango de apertura (1m, 5m, 15m)
- [ ] **Silver Bullet** — ventana horaria específica de ICT
- [ ] **AMD Model** — modelo Accumulation / Manipulation / Distribution
- [ ] **High/Low de sesión** — líneas automáticas
- [ ] **Ciclos de mercado** — detección de expansión/contracción

### Cipher Patterns
- [ ] **Cipher A** — patrón de reversa específico
- [ ] **Cipher B** — continuación / divergencia
- [ ] Convergencia/divergencia de múltiples timeframes

---

## 📉 Fase 6: Order Flow & Volumen (📋 Planificado)

**Objetivo**: Análisis de flujo de órdenes para lectura de mercado profesional.

### Depth of Market (DOM)
- [x] DOM ladder básico (Binance @depth20)
- [ ] **DOM profesional** — niveles completos, coloreado por volumen
- [ ] **Heatmap de liquidez** — gradient visual de profundidad
- [ ] **DOM histórico** — replay de depth en time-lapse
- [ ] **DOM aggregation** — agrupar niveles por tick size
- [ ] **Smart DOM** — detección de spoofing, iceberga, absorción

### Footprint Charts
- [ ] **Footprint** — velas con volumen por precio (bid/ask split)
- [ ] **Delta footprint** — diferencia compra - venta
- [ ] **Cumulative Delta** — delta acumulado en sesión
- [ ] **Bid/Ask volume ratio** — imbalance ratio
- [ ] **POC** (Point of Control) — precio con más volumen
- [ ] **VA/VAH/VAL** — Value Area, High, Low
- [ ] **Footprint profiles** — perfiles de footprint multi-timeframe

### Volume Profile
- [ ] **Session Volume Profile** — perfil por sesión
- [ ] **Anchored Volume Profile** — desde punto específico
- [ ] **Visible Range VP** — perfil del rango visible
- [ ] **Composite VP** — multi-sesión combinada
- [ ] **VPIN** (Volume-synchronized Probability of Informed Trading)

### Time & Sales
- [ ] **Tape** — tick a tick en tiempo real
- [ ] **Tape filters** — filtros por tamaño, precio, agresor
- [ ] **Tape highlighting** — prints grandes destacados
- [ ] **Tape histórico** — replay de tape

---

## ✏️ Fase 7: Herramientas de Dibujo (📋 Planificado)

**Objetivo**: Suite completa de drawing tools para análisis técnico.

### Líneas y Canales
- [ ] **Trend Line** — línea de tendencia
- [ ] **Channel** — canal paralelo
- [ ] **Horizontal Line** — línea horizontal en precio
- [ ] **Vertical Line** — línea vertical en tiempo
- [ ] **Ray** — línea infinita desde punto
- [ ] **Regression Trend** — tendencia por regresión lineal

### Fibonacci
- [ ] **Fibonacci Retracement** — niveles de retroceso
- [ ] **Fibonacci Extension** — extensión de tendencia
- [ ] **Fibonacci Channel** — canal de fibonacci
- [ ] **Fibonacci Time Zones** — zonas temporales
- [ ] **Fibonacci Fan** — abanico de fibonacci
- [ ] **Fibonacci Speed Fan** — velocidad de fibonacci

### Ciclos y Ondas
- [ ] **Gann Fan** — ángulos de Gann
- [ ] **Elliott Wave** — conteo de ondas (manual + automático)
- [ ] **Cycle Lines** — líneas de ciclo temporal

### Figuras Geométricas
- [ ] **Rectangle** — rectángulo de precio/tiempo
- [ ] **Circle** — círculo parabólico
- [ ] **Arrow** — flecha direccional
- [ ] **Path / Route** — ruta de múltiples puntos
- [ ] **Brush / Pencil** — dibujo libre
- [ ] **Image** — overlay de imagen

### Anotaciones
- [ ] **Text** — texto en cualquier punto
- [ ] **Label (Price)** — etiqueta anclada a precio
- [ ] **Note** — nota expandible
- [ ] **Table** — tabla de datos
- [ ] **Risk/Reward tool** — medición visual de RR

### Zonas y Rangos
- [ ] **Price Range** — rango horizontal de precio
- [ ] **Date Range** — rango vertical de fechas
- [ ] **Date & Price Range** — rectángulo completo
- [ ] **Position Long/Short** — zona de posición abierta

### Gestión de Dibujos
- [ ] Layer management
- [ ] Lock/Unlock drawings
- [ ] Snap to price/time
- [ ] Copy/paste drawings
- [ ] Template drawings
- [ ] Global drawing list

---

## 🖥️ Fase 8: UX Multi-ventana & Productividad (📋 Planificado)

**Objetivo**: Experiencia profesional con flujo de trabajo eficiente.

### Layout & Ventanas
- [ ] **Paneles dockables** — arrastrar y acoplar paneles
- [ ] **Workspace profiles** — layouts guardados (scalping, swing, etc.)
- [ ] **Pop-out chart** — chart en ventana independiente
- [ ] **Multi-monitor** — ventanas en distintos monitores
- [ ] **Tabbed panels** — múltiples paneles en tabs
- [ ] **Split view** — dividir panel en 2/3/4 vistas

### Watchlist & Symbols
- [ ] **Watchlist panel** — lista de símbolos con precio/change
- [ ] **Symbol search** — buscador con autocomplete
- [ ] **Favorites** — estrellas/favoritos en la watchlist
- [ ] **Sections** — agrupar símbolos por categoría
- [ ] **Market status** — indicador de mercado abierto/cerrado

### Screener
- [ ] **Multi-filter search** — filtrar por precio, volumen, cambio %
- [ ] **Screener presets** — guardados
- [ ] **Screener alerts** — alertar cuando un símbolo cumple condición

### Alerts & Notifications
- [ ] **Price alerts** — alerta cuando precio cruza nivel
- [ ] **Indicator alerts** — alerta cuando indicador cruza
- [ ] **Order alerts** — notificación de fill/cancel/reject
- [ ] **News alerts** — noticias relevantes al símbolo
- [ ] **Desktop notifications** — nativas del SO
- [ ] **Sound alerts** — sonidos configurables
- [ ] **Alert conditions** — lógica combinada (AND/OR)

### Hotkeys & Productividad
- [ ] **Hotkeys configurables** — todos los comandos bindeables
- [ ] **Command palette** — buscar y ejecutar comandos (Ctrl+K)
- [ ] **Quick order presets** — atajos para órdenes frecuentes
- [ ] **Undo/Redo en drawings** — historial de dibujos
- [ ] **Screenshot** — captura de chart con anotaciones

### Trading Journal
- [ ] **Session replay** — rebobinar sesión tick a tick
- [ ] **Trade diary** — registrar notas por trade
- [ ] **Trade statistics** — win rate, profit factor, avg RR
- [ ] **Export trades** — CSV/JSON
- [ ] **Save session** — guardar estado de la sesión

### Tiempo & Calendario
- [ ] **Countdown timer** — hasta cierre de sesión/evento
- [ ] **Market hours** — calendario de horarios de mercado
- [ ] **Economic calendar** — eventos económicos filtrables

---

## 🤖 Fase 9: Automatización & Scripting (📋 Planificado)

**Objetivo**: Estrategias algorítmicas definidas por el usuario.

### Motor de Scripting
- [ ] **Lua embebido** (mlua) — scripting sandboxeado
- [ ] **API de trading** — submit/cancel orders desde script
- [ ] **API de indicadores** — acceder a indicadores desde script
- [ ] **API de market data** — ticks, velas, order book
- [ ] **Eventos** — on_tick, on_candle, on_order_update, etc.
- [ ] **Sandboxing** — límites de CPU, memoria, calls por segundo
- [ ] **Debugger integrado** — step-through de scripts

### Estrategias Visuales
- [ ] **Node editor** — conectar bloques visualmente
- [ ] **Trigger/Action** — si condición → entonces acción
- [ ] **Templates** — estrategias pre-built

### Señales Externas
- [ ] **Webhook receiver** — recibir señales via HTTP
- [ ] **TradingView webhook** — compatible con alertas de TradingView
- [ ] **MQTT** — señales vía MQTT
- [ ] **Email/SMS alerts** — notificaciones externas

---

## 🔬 Fase 10: Backtesting & Optimización (📋 Planificado)

**Objetivo**: Simulación realista de estrategias.

### Backtesting Core
- [ ] **Tick-by-tick replay** — simulación sobre ticks reales
- [ ] **OHLCV backtest** — sobre velas (más rápido)
- [ ] **Slippage configurable** — fijo, porcentual, por volumen
- [ ] **Comisiones** — por orden, por volumen, por exchange
- [ ] **Multi-symbol backtesting** — cartera simultánea
- [ ] **Multi-timeframe** — estrategias multi-TF

### Métricas
- [ ] Sharpe Ratio
- [ ] Sortino Ratio
- [ ] Calmar Ratio
- [ ] Max Drawdown
- [ ] Win Rate
- [ ] Profit Factor
- [ ] Expectancy
- [ ] Avg Trade Duration
- [ ] Consecutive Wins/Losses
- [ ] Monte Carlo simulation

### Optimización
- [ ] **Walk-forward analysis** — ventanas de train/test
- [ ] **Parameter optimization** — grid search
- [ ] **Genetic optimization** — algoritmo genético
- [ ] **Overfitting detection** — alertas de sobreoptimización

### Replay Engine
- [ ] **Tick replay** — rebobinar tick a tick
- [ ] **Candle replay** — vela por vela
- [ ] **DOM replay** — order book histórico
- [ ] **Footprint replay** — footprint histórico
- [ ] **Order flow replay** — flujo histórico
- [ ] **Velocidad variable** — 1x, 2x, 5x, 10x, max
- [ ] **Rewind** — retroceder en el tiempo
- [ ] **Bookmarks** — marcar momentos clave

---

## 🛡️ Fase 11: Riesgo & Compliance (📋 Planificado)

**Objetivo**: Protección en rutas de dinero real.

### Risk Management Avanzado
- [ ] **Risk limits** — por símbolo, cartera, día
- [ ] **Max loss per day** — parar trading automático
- [ ] **Max loss per trade** — stop loss forzoso
- [ ] **Position sizing automático** — basado en SL, ATR, % cuenta
- [ ] **Correlation risk** — límite por sector correlacionado
- [ ] **Concentration risk** — % máximo en un activo

### Seguridad
- [ ] **API key encryption** — keyring nativo del SO
- [ ] **cargo-audit** — escaneo de vulnerabilidades
- [ ] **No logging de credenciales** — policy de tracing
- [ ] **Unsage review** — revisión de bloques unsafe
- [ ] **2FA support** — para conexión a exchange

### Compliance
- [ ] Audit log inmutable
- [ ] MiFID II transaction reporting
- [ ] SEC/Rule 606
- [ ] Data retention policies
- [ ] Order book recording for dispute resolution

---

## 🧠 Fase 12: IA & Innovación (📋 Planificado)

**Objetivo**: Integración de inteligencia artificial local.

### IA Local
- [ ] **Ollama integration** — modelos locales (Llama, Mistral, etc.)
- [ ] **Gemini Nano** — modelo en dispositivo
- [ ] **Chat contextual** — consultar sobre el chart actual
- [ ] **Pattern recognition** — IA para detectar patrones
- [ ] **Análisis de sentimiento** — sobre noticias del símbolo
- [ ] **Trade ideas** — sugerencias basadas en el contexto

### Agentes MCP
- [ ] **MCP server de trading** — exponer funcionalidad via MCP
- [ ] **Automatización por agentes** — agentes que ejecutan estrategias
- [ ] **Monitoreo por agente** — agente que vigila riesgo
- [ ] **Reportes por agente** — resúmenes periódicos

### Social & Comunidad
- [ ] **Share screenshots** — exportar chart con análisis
- [ ] **Trade ideas sharing** — compartir ideas con la comunidad
- [ ] **Copy trading** — seguir a otros traders

---

## 🚀 Fase 13: Producción & Distribución (📋 Planificado)

**Objetivo**: Producto tradeable, empaquetado y distribuible.

### Performance
- [ ] Benchmarks con criterion (end-to-end latency)
- [ ] Tracy profiling (frame timing, GPU time)
- [ ] Memory profiling (heap, fragmentation)
- [ ] ReleaseSafe para OMS/Risk
- [ ] ReleaseFast para GPU/Render

### Build & Distribución
- [ ] Cross-compilation (Win/Mac/Linux)
- [ ] MSI installer (Windows)
- [ ] DMG package (macOS)
- [ ] AppImage/Flatpak (Linux)
- [ ] Auto-updater
- [ ] Code signing

### Licenciamiento
- [x] Modelo definido (Community/Pro/Enterprise)
- [ ] License key system
- [ ] Trial period
- [ ] Feature gating
- [ ] Hardware lock

### Observabilidad
- [ ] Tracing with tokio-rs/tracing
- [ ] Crash reporting (Sentry)
- [ ] Health dashboard
- [ ] Connection monitoring
- [ ] Frame time monitoring

### Release Management
- [ ] Semantic versioning (SemVer)
- [ ] Changelogs (user + technical)
- [ ] Release candidates
- [ ] Canary releases
- [ ] Rollback plan

---

## 🏆 Leyenda

| Símbolo | Significado |
|---------|-------------|
| ✅ | Completado |
| 🚧 | En progreso |
| 📋 | Planificado |
| ★★★★★ | Prioridad máxima |
| ★★☆☆☆ | Nice to have |

---

*Última actualización: 2026-07-09*
