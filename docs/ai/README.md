# AI Documentation

Guías para agentes de IA que trabajan en velox-terminal.

## Documents

| File | Purpose | Read when |
|------|---------|-----------|
| `AI_CONTEXT_LOADING.md` | Sistema de carga por niveles para optimizar tokens | Starting a work session |
| `AI_GUIDELINES.md` | Comportamiento esperado de agentes en el dominio financiero | First session, or when uncertain about approach |
| `AI_MEMORY.md` | Memoria cross-session: decisiones, anti-patrones, riesgos conocidos | Every session start |
| `AI_FORBIDDEN_ACTIONS.md` | Acciones prohibidas en el dominio financiero (seguridad ante todo) | Before modifying OMS/Risk/P&L code |
| `AI_WORKFLOW.md` | Flujo de trabajo completo para tareas multi-agente | When coordinating multiple agents |

## Recommended Loading Order

1. `AI_MEMORY.md` — what we've learned
2. `AI_CONTEXT_LOADING.md` — what to load for this session
3. `AI_FORBIDDEN_ACTIONS.md` — what NOT to do
4. `AI_GUIDELINES.md` — how to approach the work

## Codebase Knowledge Graph

This project uses **codebase-memory-mcp**. Before searching code, always prefer:
1. `search_graph` — pattern matching
2. `trace_path` — dependency tracing
3. `get_code_snippet` — reading source

See `docs/AGENTS.md` for the full MCP reference.

## Agent Team

26 specialized agents defined in `.opencode/agents/`. The orquestador (`lead`) routes tasks to the correct specialist.
