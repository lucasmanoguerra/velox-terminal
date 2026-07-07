# Decision Log — velox-terminal

Registro de decisiones del proyecto. Para decisiones arquitectónicas significativas, ver `docs/adrs/`.

---

| Date | Decision | Rationale | Author |
|------|----------|-----------|--------|
| 2026-07-06 | Rust + wgpu + egui + glyphon + tokio como stack base | Rendimiento GPU, multiplataforma, ecosistema Rust maduro | lead |
| 2026-07-06 | 26 agentes OpenCode organizados en 8 fases | Cobertura completa del dominio trading con especialización | lead |
| 2026-07-06 | El lead agent es el router default del proyecto | El orquestador puede delegar al agente correcto según la tarea | lead |
| 2026-07-06 | Documentación con modelo progressive-disclosure | Optimiza consumo de tokens por agentes de IA | lead |
| 2026-07-06 | codebase-memory-mcp integrado para grafo de conocimiento | Búsqueda semántica de código, trazabilidad de dependencias | lead |
| 2026-07-06 | ADR-001: Workspace con 14 crates + bounded context | Compilación incremental, aislamiento de subsistemas críticos | systems-architect |
| 2026-07-06 | ADR-002: Tokio para I/O + crossbeam para hot paths | Latencia vs ergonomía, dos modelos coexistiendo | systems-architect |
| 2026-07-06 | ADR-003: wgpu como backend gráfico único con fallback | Código único para 3+ backends, type-safe | charting-engine |
| 2026-07-06 | Creación de GitHub repo público `velox-terminal` | Distribución open-source, CI/CD, colaboración | lead |
| 2026-07-06 | Git workflow: Conventional Commits + PRs + CI obligatorio | Profesionalismo, trazabilidad, calidad | lead |

---

_Append new decisions chronologically. For architectural decisions, create an ADR in docs/adrs/._
