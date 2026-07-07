# AI Context Loading — velox-terminal

Sistema de carga de contexto por niveles para optimizar tokens.

---

## Tier 1: Carga Obligatoria (cada sesión)

_Leer siempre al inicio de cada sesión. ~500 tokens total._

| File | Why |
|------|-----|
| `docs/README.md` | Master navigation index |
| `docs/AGENTS.md` | MCP codebase-memory-mcp reference |
| `docs/ai/AI_MEMORY.md` | Cross-session memory — latest entries |
| `docs/project/NEXT_ACTIONS.md` | What to work on now |

## Tier 2: Alta Prioridad (según tarea)

_Leer cuando la tarea toque estos dominios._

| Domain | Files |
|--------|-------|
| Architecture decisions | `docs/architecture/SYSTEM_OVERVIEW.md`, `docs/architecture/CONCURRENCY_MODEL.md` |
| Trading logic | `docs/trading/OMS_STATE_MACHINE.md`, `docs/trading/RISK_RULES.md` |
| Market data | `docs/trading/MARKET_DATA_MODEL.md`, `docs/architecture/DATA_PIPELINE.md` |
| GPU/Charting | `docs/gpu/RENDER_PIPELINE.md`, `docs/gpu/CHARTING_ARCH.md` |
| Quality | `docs/quality/CODING_STANDARDS.md`, `docs/quality/DEFINITION_OF_DONE.md` |
| Security | `docs/ai/AI_FORBIDDEN_ACTIONS.md`, `docs/security/CREDENTIAL_MANAGEMENT.md` |

## Tier 3: Bajo Prioridad (solo si es necesario)

_Leer solo cuando la tarea específicamente lo requiera._

| Domain | Files |
|--------|-------|
| Connectivity details | `docs/connectivity/BROKER_INTERFACE.md`, `docs/connectivity/RECONNECTION_STRATEGY.md` |
| UI specifics | `docs/ui/PANEL_SYSTEM.md`, `docs/ui/HOTKEYS.md` |
| Observability | `docs/observability/LOGGING_POLICY.md`, `docs/observability/METRICS_DASHBOARD.md` |
| Compliance | `docs/compliance/REGULATORY_REQUIREMENTS.md` |
| Operations | `docs/operations/CI_CD_PIPELINE.md`, `docs/operations/CROSS_COMPILATION.md` |
| Governance | `docs/governance/PROJECT_RULES.md` |
| Risk register | `docs/project/RISK_REGISTER.md` |

## Tier 4: Bajo Demanda

_Leer solo cuando se necesite referencia específica._

- ADR records: `docs/adrs/ADR-*.md`
- Timelines: `docs/timelines/YYYY-MM-DD-*.md`
- Individual section READMEs for navigation

---

## Session Start Workflow

```
1. Read Tier 1 files                                          ← mandatory
2. Identify which domain(s) the task belongs to               ← classify
3. Read Tier 2 files for each domain                          ← high priority
4. If task is complex, index the project with MCP             ← code discovery
   → `index_repository(repo_path="/home/lucas/Documentos/Code/velox-terminal")`
5. Work on task, using `search_graph` / `trace_path` / `get_code_snippet`
6. On completion, update AI_MEMORY.md with key learnings      ← persistence
```

## Checklist

- [ ] Read `docs/README.md`
- [ ] Read `docs/AGENTS.md`
- [ ] Read `docs/ai/AI_MEMORY.md` (latest entries)
- [ ] Read `docs/project/NEXT_ACTIONS.md`
- [ ] Indexed project with codebase-memory-mcp
- [ ] Read Tier 2 files for task domain
- [ ] Reviewed `AI_FORBIDDEN_ACTIONS.md` (especially F1-F8)
