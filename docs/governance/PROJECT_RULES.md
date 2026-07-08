# Project Rules — velox-terminal

Reglas fundamentales del proyecto. Arquitectura hexagonal + UNIX philosophy + comunidad profesional.

---

## Core Principles

1. **Correctness in money paths > performance > velocity**: OMS, Risk y P&L no admiten errores. Performance se optimiza después. Velocidad de desarrollo es lo último.

2. **ReleaseSafe para OMS/Risk**: El código de OMS y Risk Management se compila con perfil ReleaseSafe. El resto puede usar ReleaseFast.

3. **Fail-safe por defecto**: Si no se puede verificar un límite, no se ejecuta la orden. Si no se puede determinar el estado, se asume lo peor.

4. **Máquina de estados explícita**: El estado de las órdenes se modela con enums de Rust, no con booleanos. Las transiciones inválidas son errores de compilación.

5. **Sin estado duplicado en UI**: egui es immediate-mode. El estado de UI se deriva del estado de la aplicación en cada frame.

6. **Unsafe mínimo y documentado**: Cada bloque unsafe debe tener un comentario `// SAFETY:`. Prohibido unsafe en OMS y Risk.

## Hexagonal Architecture Rules

7. **Domain Core no importa infraestructura**: Los crates de dominio (`velox-core`, `velox-oms`, `velox-risk`, `velox-indicators`) no pueden importar tokio, wgpu, egui, crossbeam, reqwest ni ningún crate de infraestructura. Son Rust puro + std.

8. **Ports son traits, adapters son implementaciones**: Toda comunicación entre domain y el mundo exterior pasa por un trait definido en el dominio o en un crate de ports. Los adapters implementan esos traits.

9. **Excepciones documentadas**: Las hot paths (ring buffers, GPU upload, OSM transitions) pueden usar tipos concretos en vez de traits, pero deben documentarse con `// HEXAGONAL-EXEMPT: <razón>`.

10. **File size < 200 líneas**: Sin contar imports ni doc comments de módulo. Un archivo = una responsabilidad.

## UNIX Philosophy

11. **Una cosa, bien hecha**: Cada crate, cada módulo, cada función hace exactamente una cosa. Si un componente hace dos cosas, dividilo.

12. **Compón, no heredes**: Preferir composición sobre herencia/estructuras jerárquicas. Los pipelines de datos se construyen conectando componentes pequeños.

13. **Mínimo acoplamiento**: Las dependencias entre crates son un DAG acíclico. El domain core no sabe nada de los adapters.

## Community & Repo Governance

14. **gh CLI para operaciones**: Usar `gh` para issues, PRs, checks, releases. No mezclar herramientas.

15. **CI obligatorio**: Todo PR debe pasar build + test + clippy + deny antes de mergear. Verificar con `gh pr checks`.

16. **Conventional Commits**: Todos los commits siguen el formato conventional commits. Commits atómicos (un cambio = un commit).

17. **PRs revisados**: Toda feature o fix significativo requiere PR a `main`. Al menos 1 approval. El autor no se mergea a sí mismo.

18. **Comunidad abierta**: Issues y PRs de la comunidad son bienvenidos. Las templates de issue/PR guían el formato. Toda contribución debe pasar CI.

## Communication

- Issues de ejecución de órdenes son siempre prioridad CRÍTICA
- Los cambios de arquitectura requieren ADR
- Cambios en APIs públicas requieren revisión cross-crate
- Reportes de seguridad van a SECURITY.md (privado)

## Dependencies

- Tokio, wgpu, egui se consideran dependencias críticas — no actualizar sin testing de regresión
- `cargo deny check advisories` se corre en CI y semanalmente
- Dependencias con CVSS >= 7 se actualizan inmediatamente
- Domain core debe minimizar dependencias externas (idealmente solo std + crates de dominio)
