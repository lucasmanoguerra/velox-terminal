# Documentation Index — velox-terminal

Start here, then open the section README for the area you need.

**velox-terminal** es una terminal de trading de escritorio multiplataforma (Windows/macOS/Linux) en Rust, con renderizado GPU vía wgpu, UI en egui y texto vía glyphon.

---

## Sections

| Section | Read first | Covers |
|---------|-----------|--------|
| Project | `docs/project/README.md` | Estado actual, roadmap, alcance, métricas, riesgos |
| AI | `docs/ai/README.md` | Guías para agentes de IA, memoria, contexto, acciones prohibidas |
| Architecture | `docs/architecture/README.md` | Visión general del sistema, concurrencia, boundaries entre crates, pipeline de datos |
| Trading | `docs/trading/README.md` | OMS, Risk, datos de mercado, tipos de orden, backtesting |
| Connectivity | `docs/connectivity/README.md` | Brokers, FIX, WebSocket, REST, reconexión resiliente |
| GPU | `docs/gpu/README.md` | Renderizado wgpu, charting engine, shaders WGSL, texto glyphon |
| UI | `docs/ui/README.md` | Paneles dockables, DOM ladder, order entry, hotkeys |
| Quality | `docs/quality/README.md` | Estándares de código, Definition of Done, checklists |
| Governance | `docs/governance/README.md` | Reglas del proyecto, proceso de decisiones, releases |
| Observability | `docs/observability/README.md` | Logging, métricas, crash reporting, alertas |
| Security | `docs/security/README.md` | Credenciales, threat model, auditoría |
| Compliance | `docs/compliance/README.md` | MiFID II, SEC, retención de datos, auditoría regulatoria |
| Operations | `docs/operations/README.md` | CI/CD, cross-compilation, licencias, releases |
| ADRs | `docs/adrs/README.md` | Architecture Decision Records |
| Timelines | `docs/timelines/README.md` | Registro cronológico de cambios significativos |
| Reference | `docs/reference/INSPIRATION.md` | Proyectos open source y plataformas de inspiración |

## Core Entry Files

| File | Purpose |
|------|---------|
| `docs/project/PROJECT_STATE.md` | Estado actual del proyecto, completado y en progreso |
| `docs/project/NEXT_ACTIONS.md` | Próximas acciones priorizadas |
| `docs/AGENTS.md` | MCP codebase-memory-mcp reference para agentes de IA |
| `docs/ai/AI_MEMORY.md` | Memoria cross-session para agentes |
| `docs/architecture/SYSTEM_OVERVIEW.md` | Visión general de la arquitectura del sistema |

## Agent Ecosystem

Este proyecto utiliza **codebase-memory-mcp** para mantener un grafo de conocimiento del código.
Los 26 agentes del equipo están definidos en `.opencode/agents/` y referencian este sistema de documentación.

Ver `docs/AGENTS.md` para instrucciones de uso del MCP y `docs/ai/README.md` para guías de comportamiento de agentes.

## Loading Rule

1. Read this file first.
2. Read the relevant section README.
3. Read only the specific docs needed for the task.
4. Use `docs/AGENTS.md` for MCP tool reference when searching code.
