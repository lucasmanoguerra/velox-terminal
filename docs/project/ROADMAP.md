# Roadmap — velox-terminal

Hoja de ruta del proyecto basada en el plan de activación progresiva de agentes.

---

## Fase 0: Inicialización
**Objetivo**: Proyecto scaffolded, toolchain listo, equipo de agentes operativo.

- [x] Definición del equipo de 26 agentes en `.opencode/agents/`
- [x] Estructura de documentación `docs/`
- [ ] Workspace Cargo con crate layout inicial
- [ ] Integración con codebase-memory-mcp
- [ ] CI básico (lint + test + build)

---

## Fase 1: Núcleo Inicial
**Objetivo**: Arquitectura fundacional definida, datos de mercado diseñados.

- Workspace Cargo con boundaries entre crates
- Modelo de concurrencia documentado (tokio, crossbeam, threads)
- Estructuras de tick/quote/OHLCV
- Pipeline de agregación tick → velas en timeframes múltiples
- ADRs de arquitectura publicados

**Agentes líderes**: `systems-architect`, `product-financiero`, `market-data-arch`

---

## Fase 2: Motor de Trading
**Objetivo**: Pipeline end-to-end funcional: feed → OMS → Risk → UI.

- Market data feed en tiempo real con ring buffers
- Máquina de estados de OMS
- Validaciones de Risk Management
- Charting engine con wgpu (velas, volumen, overlays)
- Paneles egui funcionales

**Agentes líderes**: `market-data-feed`, `oms`, `risk-management`, `charting-engine`, `frontend-egui`, `ui-ux-trading`

---

## Fase 3: Conectividad Real
**Objetivo**: Trading real contra al menos un broker.

- Conector FIX/WebSocket/REST
- Reconexión automática e idempotencia
- Backtesting con slippage realista
- Indicadores técnicos (SMA, RSI, MACD, Bollinger, VWAP, Volume Profile)
- Tests de OMS/Risk con proptest
- CI/CD multiplataforma

**Agentes líderes**: `broker-integration`, `backtesting`, `tech-indicators`, `qa-financiero`, `devops`

---

## Fase 4: Camino a Producción
**Objetivo**: Producto tradeable con seguridad, performance y compliance.

- Seguridad de credenciales (keyring, cargo-audit)
- Benchmarks de latencia (criterion, Tracy)
- Compliance (MiFID II, SEC)
- Compilación cruzada y empaquetado nativo (MSI/DMG/AppImage)
- Refinamiento UI (DOM ladder, hotkeys, workspace layouts)
- Motor de scripting (Lua embebido)

**Agentes líderes**: `seguridad`, `performance`, `compliance`, `cross-platform-build`, `scripting-engine`

---

## Fase 5: Mantenimiento Continuo
**Objetivo**: Operación sostenible a largo plazo.

- Release management con SemVer
- Sistema de licencias
- SRE/Observabilidad (tracing, Sentry, dashboards)
- Mantenimiento de dependencias
- Triage de bugs y soporte

**Agentes líderes**: `release-management`, `licensing`, `sre-observability`, `dependency-maint`, `soporte-triage`

---

## Timeline Estimado

| Fase | Duración estimada | Dependencias |
|------|------------------|-------------|
| Fase 0 | 1-2 días | Ninguna |
| Fase 1 | 1 semana | Fase 0 |
| Fase 2 | 2-3 semanas | Fase 1 |
| Fase 3 | 3-4 semanas | Fase 2 |
| Fase 4 | 2-3 semanas | Fase 3 |
| Fase 5 | Continuo | Fase 4 |

---

_Last updated: 2026-07-06_
