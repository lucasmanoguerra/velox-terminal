<!-- codebase-memory-mcp:start -->

# Codebase Knowledge Graph (codebase-memory-mcp)

This project uses **codebase-memory-mcp** to maintain a knowledge graph of the entire codebase.
**ALWAYS prefer MCP graph tools over grep/glob/file-search for code discovery.**

## Priority Order

1. `search_graph` — find functions, classes, routes, variables by pattern. Use for: "find the OrderHandler", "find the BrokerClient trait", "where is the RSI indicator implemented".
2. `trace_path` — trace who calls a function or what it calls. Use for: "trace how an order flows from UI to broker", "what calls the market data feed", "who uses the ring buffer".
3. `get_code_snippet` — read specific function/class source code. Use for: "show me the OMS state machine", "read the RiskValidator trait".
4. `query_graph` — run Cypher queries for complex patterns. Use for: "find functions with high cyclomatic complexity", "find all implementations of a trait".
5. `get_architecture` — high-level project summary showing crate dependencies, module clusters, and cross-service relationships.

## When to fall back to grep/glob

- Searching for string literals, error messages, config values, hardcoded constants
- Searching non-code files (Dockerfiles, shell scripts, CI configs, TOML manifests)
- When MCP tools return insufficient results (always try MCP first)

## Trading-specific search patterns

```cypher
// Find all files in the OMS crate
MATCH (f:File) WHERE f.path CONTAINS 'crates/oms' RETURN f.path

// Find all implementations of BrokerClient trait
MATCH (t:Trait {name: 'BrokerClient'})<-[:IMPLEMENTS]-(s:Struct) RETURN s.name

// Find hot-path functions (high complexity + deep loops)
MATCH (f:Function)
WHERE f.transitive_loop_depth >= 3 OR f.linear_scan_in_loop >= 1
RETURN f.qualified_name, f.transitive_loop_depth, f.linear_scan_in_loop
ORDER BY f.transitive_loop_depth DESC

// Find all wgpu shader modules
MATCH (f:File) WHERE f.path ENDS WITH '.wgsl' RETURN f.path

// Find unsafe blocks for security audit
MATCH (f:File)-[:CONTAINS]->(fn:Function)
WHERE fn.source CONTAINS 'unsafe'
RETURN f.path, fn.name
```

## Examples

- Find a handler: `search_graph(name_pattern=".*OrderHandler.*")`
- Who calls it: `trace_path(function_name="OrderHandler", direction="inbound")`
- Read source: `get_code_snippet(qualified_name="crates/oms/src/state_machine::OrderState")`
- Find high-complexity functions: `query_graph("MATCH (f:Function) WHERE f.cyclomatic_complexity > 15 RETURN f.qualified_name, f.cyclomatic_complexity ORDER BY f.cyclomatic_complexity DESC")`

## Agent Guidelines

- **Always index the project first**: Before working, ensure the project is indexed via `index_repository(repo_path="/home/lucas/Documentos/Code/velox-terminal")`.
- **Use agents for complex tasks**: When the task requires multi-step analysis across multiple files, delegate to a specialized agent (systems-architect, oms, risk-management, charting-engine, etc.) via the `task` tool.
- **Document decisions**: Every architectural decision should be recorded as an ADR in `docs/adrs/`.

---

# Git Workflow & Commit Conventions

## Core Rule

**Después de cada implementación, modificación o feature completa, se debe hacer commit y push al repositorio.** No acumular cambios sin versionar.

## Conventional Commits

Usar el formato [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <short description>

<optional body explaining what and why, not how>
```

### Types

| Type | When to use | Example |
|------|-------------|---------|
| `feat` | Nueva feature | `feat: add RSI indicator with incremental O(1) update` |
| `fix` | Bugfix | `fix: prevent overfill in partially filled orders` |
| `docs` | Documentación | `docs: add FIX protocol architecture doc` |
| `refactor` | Refactor sin cambio funcional | `refactor: extract order validation from OrderManager` |
| `test` | Tests | `test: add property-based tests for state machine` |
| `chore` | Mantenimiento | `chore: update wgpu to v24` |
| `perf` | Performance | `perf: replace Vec with SegQueue in hot path` |
| `style` | Formato/estilo | `style: cargo fmt` |

### Anatomy

```
feat(oms): add bracket order support

- One-Cancels-Other (OCO) pair management
- Profit target and stop loss as child orders
- Parent-child order relationship tracking

Closes #42
```

## Commits Atómicos

- **Un cambio lógico = un commit**. No mezclar formateo con lógica, ni refactors con features.
- Commits pequeños facilitan code review, bisect y rollback.

## Pull Requests

- Toda feature o fix significativo → PR a `main`
- CI debe pasar antes del merge (build + lint + test)
- Al menos 1 approval de reviewer (humano o agente senior)

## Ramas

| Branch | Purpose |
|--------|---------|
| `main` | Producción. Siempre estable, CI verde |
| `develop` | Integración de features |
| `feat/<name>` | Features nuevas |
| `fix/<name>` | Bugfixes |
| `docs/<name>` | Documentación |

## Antes de commitear

1. `cargo build --workspace` — compila?
2. `cargo test --workspace` — tests pasan?
3. `cargo clippy --workspace --all-targets` — sin warnings?
4. `git diff --check` — sin whitespace errors?
5. Hacer un commit por cambio lógico (si trabajaste en 2 cosas, separar)

## Etiquetado

- Todos los commits deben tener un tipo válido (`feat:`, `fix:`, `docs:`, etc.)
- El cuerpo debe explicar el **qué** y el **por qué**, no el cómo (el código se explica solo)

<!-- codebase-memory-mcp:end -->
