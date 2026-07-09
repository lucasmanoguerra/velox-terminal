# AI Memory — velox-terminal

Persistent knowledge store for cross-session continuity.

---

## 2026-07-08 — OMS + UI Integration (PaperTrader mock execution engine)

**Decision**: Integrar la OMS (OrderManager) con la UI mediante un PaperTrader mock
execution engine. Los botones Buy/Sell crean órdenes market que se ejecutan automáticamente
al precio de cierre de la siguiente vela. Se implementa position tracking con weighted-average
cost basis y P&L realizado/no realizado.

**Problema resuelto**: Los botones Buy/Sell eran stubs (solo `tracing::info!`). No había
forma de operar desde la UI. Las órdenes creadas no se ejecutaban en ningún motor de
simulación.

**Arquitectura**:
- `PaperTrader` (nuevo, `velox-oms/src/paper_trader.rs`): Wrapper sobre OrderManager
  - `submit_market_order(symbol, side, qty)` → `OrderManager::submit_order()`
  - `execute_open_orders(symbol, price)` → fills `New` market orders en `Filled`
  - `positions()` → compute positions on-the-fly desde fill history (weighted avg cost basis)
  - `update_account()` → equity = cash + unrealized + realized P&L
  - 7 tests: submit+execute, multiple buys averaging, partial reduce realized P&L,
    account equity changes with price movement, cancel, zero-qty reject
- `AppState`: +PaperTrader, `order_entry_qty`, `order_error`/`order_success` feedback
- `panels.rs`: Order Entry (Buy green/Sell red buttons con qty slider + feedback),
  Positions (open orders con X cancel, positions con P&L, account summary)
- `app.rs`: `poll_market_data()` ejecuta `execute_open_orders(last_close)` cada frame

**Data flow**:
```
Buy button → AppState::buy_market() → PaperTrader::submit_market_order()
                                              ↓
<poll_market_data> → execute_open_orders(close) → OrderManager::apply_fill()
                                              ↓
Positions panel ← PaperTrader::positions() (on-the-fly from fills)
```

**Files changed**: 5 files, +668−21 líneas.
- `crates/velox-oms/src/paper_trader.rs` (nuevo, 267 líneas)
- `crates/velox-oms/src/lib.rs` (+paper_trader export)
- `crates/velox-ui/src/app_state.rs` (+PaperTrader, order_entry_qty, buy/sell/cancel methods)
- `crates/velox-ui/src/panels.rs` (+order entry wiring, positions/account display)
- `crates/velox-terminal/src/app.rs` (+mock execution hook in poll_market_data)

**Tests**: 59 pasando (+7 PaperTrader), 0 clippy warnings.

---

## 2026-07-08 — Order book depth (DOM ladder) with Binance @depth20@100ms

**Decision**: Implementar panel de DOM ladder (order book depth) renderizado en egui,
con datos en vivo de Binance via el stream `@depth20@100ms`. Los niveles de bid/ask
se muestran con barras de volumen (verde/rojo), spread, y precio medio.

**Problema resuelto**: No había visibilidad de la profundidad del mercado. La terminal
mostraba velas y trades pero no la liquidez disponible en cada nivel de precio.

**Key changes**:
- `velox-core/market.rs`: +`OrderBookLevel`(price,size), +`OrderBook`(bids,asks,last_update_id)
- `binance.rs`: rewrite de conexión — combined streams `/stream?streams=...` para
  soportar `@trade` + `@depth20@100ms` simultáneamente
  - `handle_depth()`: parsea snapshot de top-20 bids/asks
  - `order_book(symbol)`: accessor via `Arc<RwLock<HashMap>>` (try_read sin bloqueo)
  - `build_stream_path()`: ahora genera `?streams=sym@trade/sym@depth20@100ms`
  - `handle_message()`: unwrap de combined stream `{stream, data}`, routing por
    nombre de stream (terminado en `@trade` o `@depth20@100ms`)
- `app_state.rs`: +`depth: Option<OrderBook>`, incluye `OrderBook` en imports
- `panels.rs`: +DOM ladder panel (SidePanel::right entre chart y positions)
  - Asks en rojo (descendente, best ask primero), Bids en verde
  - Barras de volumen proporcionales al depth total
  - Precisión de precio autoescalada según magnitud (1000+ → 2 decimales)
  - Spread + Mid price en header
  - Scroll vertical con overflow
- `app.rs`: `poll_market_data()` lee `feed.order_book(symbol)` cada frame

**Files changed**: 5 files, +280−23 líneas, 59 tests, 0 clippy warnings.

---

## 2026-07-08 — Indicator overlays on chart (SMA/EMA/RSI GPU lines)

**Decision**: Implementar overlays de indicadores técnicos (SMA, EMA, RSI) renderizados como líneas GPU sobre el chart de velas. El pipeline de renderizado centralizado (`line_pipeline` en ChartRenderer) recibe datos desde OverlayManager, se procesan en CPU con NaN-aware segment splitting, y se renderizan con el pipeline `line.wgsl` v2 (vertex-buffer-based, per-vertex colors).

**Problema resuelto**: El chart mostraba velas y volumen pero no overlays de indicadores. `line.wgsl` era un stub con instancing incorrecto, y `IndicatorOverlay` trait estaba muerto.

**Key changes**:
- `line.wgsl`: rewrite completo — de storage-buffer instanced a vertex-buffer-based con per-vertex colors. Uniforms coinciden con `ChartUniforms` (7×f32). Entry points: `vs_main`/`fs_main`.
- `renderer.rs`: `LineVertex` (stride 20, 3 atributos), `LineDescriptor` type alias, buffer/bind group/vertex count. `create_line_pipeline()` con vertex layout. `update_lines()` con NaN handling + `flush_line_segment()`. Render pass 5: lines after volume.
- `overlay.rs`: `values()`/`color()` accessors en `OverlayInstance`. `has_overlay()` y `collect_line_data()`. Removido `IndicatorOverlay` trait.
- `app.rs`: wire-up overlay → line data → chart renderer en `composite_render()`.
- `panels.rs`: toggle buttons SMA(20) [green], EMA(20) [yellow], RSI(14) [orange].
- `Cargo.toml` UI: depende de `velox-indicators`.

**Files changed**: 8 files, +258−76 líneas.
- `crates/velox-gpu/shaders/line.wgsl` (rewrite)
- `crates/velox-chart/src/renderer.rs` (+LineVertex, update_lines, line pipeline)
- `crates/velox-chart/src/overlay.rs` (+accessors, remove stub)
- `crates/velox-chart/src/lib.rs` (exports)
- `crates/velox-ui/src/panels.rs` (+indicator toggles)
- `crates/velox-ui/Cargo.toml` (+velox-indicators dep)
- `crates/velox-terminal/src/app.rs` (+overlay wire-up)

---

## 2026-07-06 — Project initialization

**Decision**: Define velox-terminal as a Rust + wgpu + egui + glyphon + tokio trading terminal, organized as a Cargo workspace with clearly bounded crates.

**Stack**:
- Rust edition 2024+
- Graphics: wgpu (DirectX/Metal/Vulkan) + glyphon for text
- UI: egui (immediate-mode) over wgpu
- Async: tokio for network I/O
- Concurrency: crossbeam for lock-free channels on hot paths
- Serialization: rkyv/bincode for IPC, bytemuck for zero-copy
- Testing: proptest (property-based), criterion (benchmarks)
- CI/CD: GitHub Actions, cross-compilation with cargo-cross

**Key changes**:
- Created `.opencode/` directory with 26 agent definitions
- Created project-level `opencode.json` with agent routing
- Created `docs/` infrastructure with progressive-disclosure model
- Integrated codebase-memory-mcp for code knowledge graph

**Files changed**:
- `.opencode/opencode.json`
- `.opencode/agents/*.md` (26 agent files)
- `docs/README.md`, `docs/AGENTS.md`
- `docs/project/*.md`
- `docs/ai/*.md`
- `docs/architecture/*.md`
- `docs/trading/*.md`
- Various other `docs/` section files

---

## 2026-07-06 — Agent architecture decisions

**Decision**: The lead agent routes to 25 specialized subagents organized in 8 phases (0-7). All agents use `claude-sonnet-4-6` model. Critical agents (OMS, Risk Management, QA Financiero) have `edit: allow` permission. `soporte-triage` is read-only (`edit: deny`).

**Rationale**: The trading domain requires specialized knowledge that generic agents can't provide. The phased activation roadmap matches the natural dependency order: architecture → data → connectivity → trading logic → UI/GPU → quality → operations.

---

---

## 2026-07-06 — Indicadores MACD/Bollinger/ATR + OMS hardening

**Decision**: Implementar MACD, Bollinger Bands y ATR como indicadores incrementales O(1) con tests. Expandir OMS state machine con 7 nuevas transiciones, reemplazo de órdenes, y 10 tests de edge cases. Agregar property-based tests para OMS (proptest).

**Problema resuelto**: Los indicadores eran stubs sin implementación real. El OMS carecía de soporte para replace, cancel en estados intermedios, y tenía bugs en fills con side incorrecto.

**Key changes**:
- MACD: 9-period EMA, 26-period EMA, signal line (9-period EMA of difference), histogram
- Bollinger Bands: SMA ± k*σ con k configurable
- ATR: Wilder's smoothed ATR sobre True Range
- 16 tests de indicadores pasando (SMA, EMA, RSI, MACD, Bollinger, ATR)
- OMS state machine: +7 transiciones (PendingNew→PendingCancel, New→Stopped, New→PendingReplace, PartiallyFilled→Stopped, PendingCancel→New, PendingReplace→PendingCancel, Stopped→Expired)
- `replace_order()` con validación de precio, cantidad, y guard contra qty < filled
- 10 nuevos unit tests OMS + 2 proptest properties (fill exact-quantity, no-overfill)
- Bugfix: `make_fill` usaba siempre Side::Buy — ahora `make_fill_for_order` respeta el side
- Zero warnings en workspace (fix de campos no usados, imports)

**Files changed**:
- `crates/velox-indicators/src/macd.rs`, `bollinger.rs`, `atr.rs`
- `crates/velox-oms/src/order_manager.rs`, `state_machine.rs`, `error.rs`
- `crates/velox-risk/src/validators.rs`, `circuit_breaker.rs`
- `crates/velox-md/src/aggregation.rs`
- `docs/project/ROADMAP.md`, `PROJECT_STATE.md`

---

## 2026-07-07 — Integración egui-wgpu + ventana winit

**Decision**: Integrar el charting engine (velox-chart) con egui-wgpu 0.31 + winit 0.30 + wgpu 24 en un event loop completo, usando dos render passes (chart primero con Clear, egui encima con Load) con un `unsafe` helper justificado para el `RenderPass<'static>` que exige egui-wgpu.

**Problema resuelto**: No había forma de ver el chart en pantalla. Todo el pipeline GPU existía pero no se conectaba con una ventana, un event loop, ni egui.

**Key changes**:
- `App` struct: orquestador que posee window, GpuDevice, surface, ChartRenderer, egui context/renderer/state, PanelManager, AppState
- Event loop completo: resize (surface reconfigure), close, redraw, input routing
- Render pipeline: PASS 1 (chart vía ChartRenderer con scissor rect) + PASS 2 (egui con LoadOp::Load sobre chart)
- Input routing: winit → egui primero (consumed check vía `EventResponse.consumed`), si no consumido → ChartInteraction (zoom/pan/undo)
- `render_egui_with_pass()`: helper unsafe para workaround del `RenderPass<'static>` de egui-wgpu
- PanelManager funcional: top bar (último precio), order entry (side/quantity), chart area (rect tracking), positions, status bar
- Theme: dark trading professional con tonos oscuros (#121218 fondo)
- AppState: estado compartido entre UI panels y GPU renderer, sincronizado en mismo thread
- Mock data generada: 200 velas OHLCV con random walk seedeado, símbolo BTC/USD

**Technical constraints discovered**:
- `egui_wgpu::Renderer::render()` requiere `&mut RenderPass<'static>` — workaround necesario con transmute en helper `unsafe`
- egui 0.31 API: `on_window_event()` devuelve `EventResponse` (no bool), `Visuals` no tiene campo `dark`
- wgpu 24: `Surface` requiere `create_surface_unsafe()` para `'static` lifetime
- `fastrand::Rng` API: `u64(min..max)` retorna u64, `f64()` retorna [0,1)

**Files changed**:
- `Cargo.toml` (workspace): +winit, egui-winit, pollster
- `crates/velox-terminal/Cargo.toml`: +winit, egui, egui-winit, egui-wgpu, wgpu, pollster, fastrand
- `crates/velox-terminal/src/main.rs`: rewrite — event loop + CLI
- `crates/velox-terminal/src/app.rs`: new — App struct + composite render
- `crates/velox-terminal/src/input.rs`: new — event routing
- `crates/velox-ui/src/app_state.rs`: new — AppState
- `crates/velox-ui/src/panels.rs`: full PanelManager implementation
- `crates/velox-ui/src/theme.rs`: dark trading theme
- `crates/velox-chart/src/interaction.rs`: +is_dragging(), zoom_stack_size()
- `crates/velox-chart/src/renderer.rs`: +update_from_state()

---

## 2026-07-06 — ADRs fundacionales y estructura base

**Decision**: Se crearon 3 ADRs fundacionales (workspace, concurrencia, wgpu) y se completaron los gaps de documentación: FIX_PROTOCOL.md, WEBSOCKET_FEED.md, LICENSING.md.

**Decision**: Se creó el repositorio público `velox-terminal` en GitHub y se estableció el git workflow con Conventional Commits.

**Estructura del workspace**:
```
Cargo workspace con 14 crates (velox-core, velox-md, velox-indicators, velox-oms,
velox-risk, velox-broker, velox-broker-fix, velox-storage, velox-backtest,
velox-scripting, velox-gpu, velox-chart, velox-ui, velox-terminal)
```

**Git workflow**:
- Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`)
- Pull Requests obligatorios (excepto hotfixes críticos)
- CI obligatorio pre-merge (build + lint + test)
- Commits atómicos (un cambio = un commit)

**Files changed**:
- `docs/adrs/001-workspace-crate-structure.md` (nuevo)
- `docs/adrs/002-concurrency-model.md` (nuevo)
- `docs/adrs/003-wgpu-rendering-pipeline.md` (nuevo)
- `docs/adrs/README.md` (updated)
- `docs/connectivity/FIX_PROTOCOL.md` (nuevo)
- `docs/connectivity/WEBSOCKET_FEED.md` (nuevo)
- `docs/operations/LICENSING.md` (nuevo)
- `docs/governance/DECISION_LOG.md` (updated)
- `docs/ui/PANEL_SYSTEM.md`, `DOM_LADDER.md`, `ORDER_ENTRY.md` (nuevos)
- `crates/*/Cargo.toml` (14 nuevos)
- `Cargo.toml` (workspace root)
- `.gitignore`
- `.github/workflows/ci.yml`

---

### Anti-Patterns to Avoid

- ❌ **Implementar lógica financiera sin tests**: OMS/Risk/P&L sin property-based tests es inaceptable.
- ❌ **Mezclar cambios funcionales con refactors**: Cada cambio debe ser atómico y revisable.
- ❌ **Asumir fills perfectos en backtesting**: Slippage y comisiones son obligatorios, no opcionales.
- ❌ **Unsafe sin justificación**: Cada bloque unsafe debe tener comentario `// SAFETY:`.
- ❌ **Estado implícito en OMS**: La máquina de estados debe ser explícita con enums de Rust.
- ❌ **Loggear credenciales**: Nunca en ningún nivel de log.

### Known Risks

| Risk | Context | Mitigation |
|------|---------|------------|
| wgpu no soportado en HW antiguo | Windows 7, GPUs pre-2015 | Detectar capabilities al inicio, fallback a software rendering |
| egui immediate-mode lento con muchos paneles | 10+ paneles simultáneos con datos en tiempo real | Lazy update de paneles no visibles, rate limiting de repaints |
| FIX engine (fefix) incompleto para ciertos brokers | Algunos brokers usan extensiones FIX propietarias | Evaluar alternativa: construir sobre mensajes FIX base, extensiones como plugins |

## 2026-07-07 — Fix: WGSL Uniforms struct mismatch en pipelines compartidos

**Decision**: Estandarizar todos los shaders activos (candle, grid, volume) en un mismo `Uniforms` de 7×f32 (28 bytes) para que compartan el bind group layout sin errores de validación. La `line_pipeline` (dead-code) se deja para refactor futuro con su propio layout.

**Problema resuelto**: `volume.wgsl` tenía 9 campos en `Uniforms` (36 bytes) pero `ChartUniforms` en Rust tenía 7 (28 bytes). El `min_binding_size` del bind group layout (28) era menor que lo que esperaba el shader de volumen (36), causando:
```
Error matching ShaderStages(VERTEX)...Buffer structure size 36...
greater than the given min_binding_size, which is 28
```
Además, `grid.wgsl` tenía un `Uniforms` con solo `viewport_width/height/padding` en offsets distintos a `ChartUniforms`, leyendo datos incorrectos (`price_scale` como `viewport_width`).

**Key changes**:
- `volume.wgsl`: Eliminé `volume_height` y `max_volume` de Uniforms. El CPU ya normaliza volúmenes 0..1 en `update_volume()`. Usar fracción fija 0.2 del viewport.
- `grid.wgsl`: Cambié su `Uniforms` al struct común de 7 campos para que `viewport_width`/`viewport_height` estén en los offsets correctos (20 y 24).
- `renderer.rs`: `min_binding_size` cambiado de `NonZeroU64::new(28)` a `None` para desactivar validación de tamaño. Uniform buffer agrandado de 28 a 256 bytes.
- Removí import `NonZeroU64` no usado.
- Build + 39 tests + clippy: todo verde.

**Files changed**:
- `crates/velox-gpu/shaders/volume.wgsl`
- `crates/velox-gpu/shaders/grid.wgsl`
- `crates/velox-chart/src/renderer.rs`

---

## 2026-07-08 — Fix: grid pipeline layout separado (vertex vs storage bindings)

**Decision**: Separar el grid pipeline en su propio `BindGroupLayout` y `PipelineLayout` (solo uniform binding), mientras candle/volume comparten el layout original (uniform + storage). El grid shader recibe datos vía `set_vertex_buffer`, no storage buffers.

**Problema resuelto**: El grid bind group bindeaba su vertex buffer (`VERTEX | COPY_DST`) en binding 1 declarado como `Storage` en el bind group layout compartido. wgpu rechazaba el bind group con:
```
Buffer 'grid_vertices' usage flags BufferUsages(COPY_DST | VERTEX) 
do not contain required usage flags BufferUsages(STORAGE)
```

**Key changes**:
- `grid_bind_group_layout`: layout con solo binding 0 (uniform), sin binding 1 (storage)
- `grid_pipeline_layout`: pipeline layout para grid usando el nuevo bind group layout
- Grid bind group: solo bindea uniform buffer (ya no el vertex buffer como storage)
- `update_grid()` recrea bind group con `grid_bind_group_layout` (solo uniform)
- Candle/volume siguen compartiendo `bind_group_layout` (uniform + storage)

**Resultado**: App arranca limpia — todos los pipelines se crean sin errores de validación wgpu:
```
GPU device initialized
Shader 'candle' compiled
Shader 'grid' compiled  
Shader 'volume' compiled
Shader 'line' compiled
ChartRenderer initialized
```

**Files changed**:
- `crates/velox-chart/src/renderer.rs`

---

---

## 2026-07-08 — Real WebSocket market data feed (Binance) + pipeline chart live

**Decision**: Implementar pipeline de market data en vivo: Binance WebSocket → RingBuffer → CandleAggregator → mpsc channel → AppState → ChartRenderer.

**Problema resuelto**: El chart solo mostraba mock data (random walk generado en `generate_mock_candles()`). No había conexión real a ningún exchange.

**Arquitectura**:
- **Nuevo crate `velox-exchange`**: Contiene el trait `ExchangeFeed` y el conector `BinanceFeed`
  - `BinanceFeed` se conecta a `wss://stream.binance.com:9443/ws` vía `tokio-tungstenite`
  - Suscribe a streams `<symbol>@trade` (trades en tiempo real)
  - Parsea JSON de Binance → `velox_core::Tick` → `MarketEvent::Tick` → `RingBuffer`
  - Manejo de reconexión: `AtomicBool` running flag + `JoinHandle` para abort
  - Normalización de símbolos: `BTC-USDT` → `btcusdt`, `ETH/USDT` → `ethusdt`
  - 5 tests unitarios (stream path, subscribe normalization, duplicates, trade parsing)

- **Nuevo módulo `velox-md::pipeline`**: `MarketDataPipeline`
  - Consume `RingBuffer` (SPSC consumer, poll-based, no bloqueante)
  - Alimenta `CandleAggregator` para timeframe 1m (60s)
  - Envía velas completadas vía `tokio::sync::mpsc::unbounded_channel`
  - `poll()` → retorna count de nuevas velas. Llamado desde main thread cada frame.

- **`velox-ui::AppState` actualizado**:
  - `set_candle_receiver(rx)` — conecta el canal de velas
  - `poll_candles()` — drena el canal, actualiza `self.candles`, autoescala vista en primera vela
  - Buffer window: mantiene últimas 500 velas (descarta las viejas)
  - `empty()` constructor para arrancar sin datos mock
  - Nuevos campos: `symbol`, `ticks_processed`, `candles_produced`, `feed_connected`

- **`velox-terminal::App` cableado**:
  - `App::new()`: crea `RingBuffer(4096)` → `MarketDataPipeline` → `BinanceFeed` → subscribe BTC/USDT → start
  - `AboutToWait` hook: `poll_market_data()` corre cada frame (poll ring buffer + drain channel)
  - `CloseRequested`: `feed.stop()` con abort del JoinHandle
  - AppState arranca con `AppState::empty()` (sin datos mock)

- **`main.rs`**:
  - Crea `tokio::runtime::Runtime` al inicio
  - `runtime.enter()` para que `tokio::spawn()` funcione desde `BinanceFeed::start()`
  - Runtime se dropea al salir → cancela todas las tareas spawned

**Data flow final**:

```
Binance WS ──> RingBuffer ──> Pipeline.poll() ──> mpsc channel ──> AppState ──> ChartRenderer
 (tokio)       (SPSC)         (main thread poll)                    (candles)    (GPU upload)
```

**Files changed**:
- `Cargo.toml` (workspace): +tokio-tungstenite, futures-util
- `crates/velox-exchange/Cargo.toml` + `src/lib.rs` + `src/error.rs` + `src/trait.rs` + `src/binance.rs` (nuevos)
- `crates/velox-md/Cargo.toml`: +tokio, tracing
- `crates/velox-md/src/lib.rs`: +pipeline module
- `crates/velox-md/src/pipeline.rs` (nuevo — MarketDataPipeline)
- `crates/velox-ui/Cargo.toml`: +tokio
- `crates/velox-ui/src/app_state.rs`: rewrite — +channel receiver, poll_candles(), empty()
- `crates/velox-ui/src/panels.rs`: +live indicators (price, ticks, candles, connection status)
- `crates/velox-terminal/Cargo.toml`: +velox-exchange
- `crates/velox-terminal/src/app.rs`: rewrite — +pipeline, feed, poll_market_data
- `crates/velox-terminal/src/main.rs`: +tokio runtime, enter guard

**Tests**: 47 pasando (+5 exchange, +1 pipeline), 0 fallos, 0 warnings nuevos

---

---

## 2026-07-08 — Multi-timeframe support (1m/5m/1h) with UI selector

**Decision**: Expandir el pipeline de market data de 1m a multi-timeframe (1m/5m/1h) con almacenamiento en `HashMap<i64, Vec<Candle>>` y selector gráfico en la UI. AppState cambia de `Vec<Candle>` plano a un mapa keyed por `timeframe_secs`.

**Problema resuelto**: El chart solo soportaba un timeframe fijo (60s). Para análisis multi-resolución (scalping en 1m, tendencia en 1h) se necesitaba almacenar velas de múltiples timeframes y switchear entre ellas sin perder datos.

**Key changes**:
- `AppState::candles_by_tf`: `HashMap<i64, Vec<Candle>>` almacena velas por timeframe
- `AppState::empty(timeframes: &[i64])`: constructor parametrizado con lista de timeframes
- `set_timeframe(tf)`: swap del bucket activo, recrea `ChartInteraction` con la nueva vista
- `seconds_to_tf_label()`: formatea segundos a 1m/5m/1h/1D/1W/1M
- `timeframe_labels()`: devuelve lista `(i64, String)` para UI
- `poll_candles()`: drena canal a vec local para evitar conflictos de borrow checker con `self.reset_view()`
- Pipeline configurado con `&[60, 300, 3600]` (1m, 5m, 1h)
- UI: botones `selectable_label` en top bar, label de timeframe activo en status bar
- 4 unit tests nuevos: `empty_with_timeframes`, `seconds_to_label`, `set_timeframe_switches_candles`, `theme_applies_without_panic`
- 49 tests pasando, 0 warnings nuevos

**Files changed**:
- `crates/velox-ui/src/app_state.rs`: rewrite — HashMap multi-tf, set_timeframe, empty(timeframes), tests
- `crates/velox-ui/src/panels.rs`: +timeframe selector buttons, +timeframe label in status bar
- `crates/velox-terminal/src/app.rs`: `&[60, 300, 3600]` pipeline, `AppState::empty(timeframes)`
- `crates/velox-md/src/ring_buffer.rs`: removed redundant `.clone()` on Copy types (clippy fix)

---

## 2026-07-08 — Auto-reconnect WebSocket (exponential backoff + jitter)

**Decision**: Envolver `BinanceFeed::run_loop()` en un reconnect loop con exponential backoff y full jitter. Cuando la conexión se cae (cierre del exchange, error de red, timeouts), el feed se reconecta automáticamente sin intervención del usuario.

**Problema resuelto**: Binance desconecta WebSocket frecuentemente. Sin reconexión, el feed moría y el chart se quedaba congelado hasta que el usuario reiniciara la app.

**Key changes**:
- `run_loop` rewrite: `while running { try_connect → read_loop → on_disconnect → backoff → retry }`
- Backoff: `base * 2^(attempt-1)` con full jitter (0–window), min 500ms, max 60s
- `sleep_with_running_check()`: polls `running` cada 100ms — shutdown responsivo en <100ms
- `feed_connected: AtomicBool` en `BinanceFeedInner`, actualizado en connect/disconnect
- `BinanceFeed::connected()`: expone estado vía atomic load — sin locks
- `App::poll_market_data()`: actualiza `state.feed_connected` desde el feed cada frame
- `AtomicBool` define el estado de conexión real (WebSocket TCP conectado)
- 3 tests nuevos: backoff ranges, progression, initial state (8 total exchange)
- 50 tests pasando, 0 warnings nuevos

**Files changed**:
- `crates/velox-exchange/src/binance.rs`: rewrite — reconnect loop, backoff, sleep_with_running_check, connected(), feed_connected
- `crates/velox-terminal/src/app.rs`: poll feed connection state per frame

---

## 2026-07-09 — Horizontal scrollbar with follow mode

**Decision**: Add horizontal scrollbar at the bottom of the chart panel,
with a follow-mode toggle that auto-scrolls to newest data. The scrollbar
is an egui slider (0%–100%) rendered below the chart area in CentralPanel.

**Problema resuelto**: No way to navigate historical candles without zooming
out and panning. Users had to drag to scroll, and there was no visual indicator
of where the view was within the full data range.

**Key changes**:
- `interaction.rs`: +`scroll_pos()`, `set_scroll_pos()`, `data_range()`,
  `is_at_right_edge()` — normalized scroll position computed from view + data
- `app_state.rs`: +`scroll_pos: f64`, `follow_mode: bool`, `sync_scroll_pos()`,
  `set_scroll_pos()`, `toggle_follow_mode()`, auto-scroll in `poll_candles()`
  when follow_mode is active
- `panels.rs`: +egui slider (0..1) below chart area with percentage label,
  🔒Follow/🔓Free toggle button, auto-disable follow on manual scroll drag

**Data flow**:
```
New candle arrives → poll_candles()
  → if follow_mode & not at right edge → scroll view to latest
Next frame → sync_scroll_pos() → UI reads scroll_pos → slider updates
User drags slider → set_scroll_pos(fraction) → chart view adjusts
  → follow_mode disabled (user is browsing history)
User clicks Follow → follow_mode = true → snaps to newest
```

**Files changed**: 3 files, +301−0 líneas (interaction.rs, app_state.rs, panels.rs)
**Tests**: 66 pasando (+7 scrollbar), 0 clippy warnings.

---

## 2026-07-09 — RingBuffer::pop_n batch tick consumption

**Decision**: Agregar `RingBuffer::pop_n()` para consumir hasta `max` eventos
del ring buffer con solo 2 atomic loads + 1 store, reemplazando el loop de
`pop()` (3 atomics por evento) en MarketDataPipeline.

**Problema resuelto**: Cada tick requería 3 operaciones atómicas (2 loads +
1 store) en el ring buffer. Con cientos de ticks por frame, el overhead
atómico se acumulaba. `pop_n` lee el write_index una vez, extrae hasta
`max` eventos usando lecturas planas (no atómicas), y avanza el read_index
una sola vez.

**Key changes**:
- `ring_buffer.rs`: +`pop_n(&self, buf: &mut Vec<MarketEvent>, max: usize) → usize`
  - Lee read/write una vez, calcula count = min(available, max)
  - Extrae eventos con índice modular (power-of-two mask)
  - Almacena read_index una vez al final (Release ordering)
  - 6 tests: límite máximo, todos disponibles, orden FIFO, vacío,
    wrap-around completo, max=0
- `pipeline.rs`: `poll()` ahora usa `pop_n(&mut batch, 128)` en loop de
  drenaje con `Vec::drain()`, reusando el buffer entre iteraciones
- 71 tests pasando, 0 clippy warnings

---

## 2026-07-09 — Comprehensive roadmap + inspiration docs

**Decision**: Refactor completo del roadmap a 14 fases granulares basado en análisis
de 7 proyectos open source (Fincept, Nautilus, Freqtrade, OpenTerminalUI, OpenAlgo,
ProfitMaker, OS Engine) y 5 plataformas comerciales (NinjaTrader, MetaTrader,
TradingView, ATAS, DeepChart). Crear `docs/reference/INSPIRATION.md` documentando
cada proyecto con stack, features clave, y qué aprender.

**Documentos creados**:
- `docs/reference/INSPIRATION.md` — 7 proyectos OSS + 5 comerciales, extracción de
  ~100 features por categoría, tabla comparativa de stacks
- `docs/project/ROADMAP.md` — Rewrite completo: 14 fases con ~300 features organizados
  por categoría (charting, trading, drawing, SMC/ICT, order flow, scripting,
  backtesting, IA, etc.), todo priorizado y con estado ✅/🚧/📋
- `docs/project/NEXT_ACTIONS.md` — Backlog priorizado P1-P4 con 46 items y tabla de
  crates impactados
- `docs/project/PROJECT_STATE.md` — Fases extendidas de 5 a 14

---

## 2026-07-08 — Hexagonal architecture + community standards + CI fixes

**Decision**: Adopt Hexagonal (Ports & Adapters) + UNIX philosophy as the architectural guide. Create community files (CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md). Fix CI pipeline failures (fmt, clippy, cargo-deny).

**Problema resuelto**: El proyecto carecía de una guía arquitectónica unificada, no tenía community files open-source, y el CI estaba rojo por problemas de formato y configuración de cargo-deny 0.19.

**Key changes**:
- ADR-004: Hexagonal Architecture — 15 crates clasificados en Domain Core / Application / Adapters
- `#![forbid(unsafe_code)]` en Domain Core crates (velox-core, velox-oms, velox-risk, velox-indicators)
- Hot path exceptions documentadas con `// HEXAGONAL-EXEMPT: <razón>`
- UNIX philosophy: cada componente hace una cosa. Files < 200 líneas (sin imports).
- Community files: CONTRIBUTING.md (440 líneas), CODE_OF_CONDUCT.md, SECURITY.md, issue/PR templates
- Root README.md con badges, stack, quick start
- Architecture docs rewrite con overlay hexagonal (SYSTEM_OVERVIEW, CRATE_BOUNDARIES, DATA_PIPELINE, DEPENDENCY_MAP)
- GitHub issues #1 (indicadores overlay), #2 (OMS+UI), #3 (order book depth) creados con milestone
- CI fix: `cargo fmt` en 47 archivos, `cargo clippy --fix` en stubs, deny.toml version 2 para cargo-deny 0.19

**Files changed**:
- `docs/adrs/004-hexagonal-architecture.md` (nuevo)
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` (nuevos)
- `README.md`, `deny.toml`, `.github/ISSUE_TEMPLATE/*`, `.github/PULL_REQUEST_TEMPLATE.md` (nuevos)
- `docs/architecture/*.md` (rewrite con hexagonal overlay)
- `.opencode/agents/lead.md`, `systems-architect.md`, `opencode.json` (routing hexagonal)
- 47 archivos .rs (cargo fmt + cargo clippy --fix)

---

### Technical Constraints

1. **Perfiles de compilación**: OMS y Risk Management deben compilarse con perfil `ReleaseSafe`. El resto puede usar `ReleaseFast`.
2. **MSRV**: Minimum Supported Rust Version definido por `systems-architect`. Evaluar toolchain estable más reciente.
3. **Dependencias GPU**: wgpu requiere Vulkan (Linux), DirectX 12 (Windows), o Metal (macOS). No hay fallback a OpenGL.
4. **Distribución**: Rust genera binarios estáticos sin runtime externo — facilitan distribución pero requieren compilación cruzada por plataforma.
5. **egui-wgpu `RenderPass<'static>`**: `egui_wgpu::Renderer::render()` requiere `&mut RenderPass<'static>` porque internamente puede entregarlo a paint callbacks. Workaround safe: helper `unsafe fn` con transmute lifetime, siempre que no se usen paint callbacks.
6. **Dual wgpu en lockfile**: egui-wgpu 0.31 y nuestro código usan ambos wgpu 24.0.5 (misma versión). La `Surface` de wgpu 24 requiere `create_surface_unsafe()` con `SurfaceTargetUnsafe` para obtener lifetime `'static`.
7. **WGSL Uniforms struct layout**: Todos los shaders que comparten un `BindGroupLayout` deben tener el mismo struct de `Uniforms` (mismos campos, mismo orden, mismo tamaño). Usar `min_binding_size: None` para desactivar validación de tamaño, y crear el uniform buffer suficientemente grande (256 bytes) para cubrir todos los shaders.
8. **Separar layouts por pipeline tipo**: Pipelines que usan `storage buffers` (candle/volume con instanced rendering) y pipelines que usan `vertex buffers` (grid con vertex input) no pueden compartir el mismo `BindGroupLayout` — el tipo de binding (Storage vs Vertex) está codificado en el layout. Cada pipeline debe tener el layout que coincida con cómo recibe sus datos.
9. **cargo-deny 0.19 `version = 2`**: En cargo-deny 0.19, se requiere `version = 2` en `[advisories]` y `[licenses]` para optar al nuevo comportamiento. Los campos `vulnerability`, `unmaintained`, `yanked`, `notice`, `severity-threshold`, `unlicensed`, `copyleft`, y `allow-osi-fsf-free` han sido eliminados — todos los valores deny/warn son ahora fijos (deny por defecto). Las advisory-dbs se controlan solo via `ignore = []`.
10. **Transitive advisory management**: Los advisories de dependencias transitivas (paste via metal, quick-xml via wayland-scanner, ttf-parser via glyphon/egui) deben ignorarse explícitamente con `RUSTSEC-XXXX-XXXX` en `deny.toml[advisories].ignore` hasta que las dependencias upstream bumpen sus versiones.
