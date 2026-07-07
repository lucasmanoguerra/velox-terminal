# Decision Process — velox-terminal

Cómo se toman las decisiones en el proyecto.

---

## Decision Levels

| Level | Examples | Process | Documentation |
|-------|----------|---------|---------------|
| **L1 — Trivial** | Renombrar variable, bugfix menor, test nuevo | Decisión directa | Ninguna |
| **L2 — Menor** | Nuevo indicador, panel UI, configuración | Discusión breve + implementación | Mencionar en AI_MEMORY.md |
| **L3 — Significativa** | Nuevo crate, cambio en trait público, feature nueva | Diseño por escrito + revisión | ADR + AI_MEMORY.md |
| **L4 — Arquitectónica** | Cambio de tecnología, modelo de concurrencia, API breaking | ADR completo + aprobación del lead | ADR obligatorio |

## L3/L4 Process

```
1. Identify need → document context + problem
2. Propose solution → write ADR draft
3. Alternatives considered → at least 2 other options with trade-offs
4. Review → lead + affected agents review
5. Decide → accept / modify / reject
6. Implement → per Definition of Done
7. Document → ADR finalized, AI_MEMORY.md updated
```

## ADR Template

See `docs/adrs/TEMPLATE.md`.
