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

### Technical Constraints

1. **Perfiles de compilación**: OMS y Risk Management deben compilarse con perfil `ReleaseSafe`. El resto puede usar `ReleaseFast`.
2. **MSRV**: Minimum Supported Rust Version definido por `systems-architect`. Evaluar toolchain estable más reciente.
3. **Dependencias GPU**: wgpu requiere Vulkan (Linux), DirectX 12 (Windows), o Metal (macOS). No hay fallback a OpenGL.
4. **Distribución**: Rust genera binarios estáticos sin runtime externo — facilitan distribución pero requieren compilación cruzada por plataforma.

---

_Append new entries chronologically. Format: `## YYYY-MM-DD — Title`_
