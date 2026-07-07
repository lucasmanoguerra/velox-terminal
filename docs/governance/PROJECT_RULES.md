# Project Rules — velox-terminal

Reglas fundamentales del proyecto.

---

## Core Principles

1. **Correctness in money paths > performance > velocity**: OMS, Risk y P&L no admiten errores. Performance se optimiza después. Velocidad de desarrollo es lo último.

2. **ReleaseSafe para OMS/Risk**: El código de OMS y Risk Management se compila con perfil ReleaseSafe. El resto puede usar ReleaseFast. Esto desactiva optimizaciones agresivas que puedan alterar semántica.

3. **Fail-safe por defecto**: Si no se puede verificar un límite, no se ejecuta la orden. Si no se puede determinar el estado, se asume lo peor.

4. **Máquina de estados explícita**: El estado de las órdenes se modela con enums de Rust, no con booleanos. Las transiciones inválidas son errores de compilación.

5. **Sin estado duplicado en UI**: egui es immediate-mode. El estado de UI se deriva del estado de la aplicación en cada frame.

6. **Unsafe mínimo y documentado**: Cada bloque unsafe debe tener un comentario `// SAFETY:`. Prohibido unsafe en OMS y Risk.

7. **Testing obligatorio**: OMS/Risk/P&L requieren property-based testing. No se mergea sin tests.

## Communication

- Issues de ejecución de órdenes son siempre prioridad CRÍTICA
- Los cambios de arquitectura requieren ADR
- Cambios en APIs públicas requieren revisión cross-crate

## Dependencies

- Tokio, wgpu, egui se consideran dependencias críticas — no actualizar sin testing de regresión
- cargo-audit se corre semanalmente
- Dependencias con CVSS >= 7 se actualizan inmediatamente
