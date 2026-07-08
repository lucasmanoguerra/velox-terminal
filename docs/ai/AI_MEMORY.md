# AI Memory — velox-terminal

Persistent knowledge store for cross-session continuity.

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

### Technical Constraints

1. **Perfiles de compilación**: OMS y Risk Management deben compilarse con perfil `ReleaseSafe`. El resto puede usar `ReleaseFast`.
2. **MSRV**: Minimum Supported Rust Version definido por `systems-architect`. Evaluar toolchain estable más reciente.
3. **Dependencias GPU**: wgpu requiere Vulkan (Linux), DirectX 12 (Windows), o Metal (macOS). No hay fallback a OpenGL.
4. **Distribución**: Rust genera binarios estáticos sin runtime externo — facilitan distribución pero requieren compilación cruzada por plataforma.
5. **egui-wgpu `RenderPass<'static>`**: `egui_wgpu::Renderer::render()` requiere `&mut RenderPass<'static>` porque internamente puede entregarlo a paint callbacks. Workaround safe: helper `unsafe fn` con transmute lifetime, siempre que no se usen paint callbacks.
6. **Dual wgpu en lockfile**: egui-wgpu 0.31 y nuestro código usan ambos wgpu 24.0.5 (misma versión). La `Surface` de wgpu 24 requiere `create_surface_unsafe()` con `SurfaceTargetUnsafe` para obtener lifetime `'static`.
7. **WGSL Uniforms struct layout**: Todos los shaders que comparten un `BindGroupLayout` deben tener el mismo struct de `Uniforms` (mismos campos, mismo orden, mismo tamaño). Usar `min_binding_size: None` para desactivar validación de tamaño, y crear el uniform buffer suficientemente grande (256 bytes) para cubrir todos los shaders.
8. **Separar layouts por pipeline tipo**: Pipelines que usan `storage buffers` (candle/volume con instanced rendering) y pipelines que usan `vertex buffers` (grid con vertex input) no pueden compartir el mismo `BindGroupLayout` — el tipo de binding (Storage vs Vertex) está codificado en el layout. Cada pipeline debe tener el layout que coincida con cómo recibe sus datos.
