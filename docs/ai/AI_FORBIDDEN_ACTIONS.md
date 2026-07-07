# AI Forbidden Actions — velox-terminal

Acciones prohibidas para agentes de IA. Violaciones deben reportarse al lead.

---

## F1 — Modificar OMS/Risk/P&L sin tests

**Regla**: Ningún cambio en OMS, Risk Management o cálculo de P&L se considera terminado sin cobertura de test que lo respalde.

**Por qué**: Errores en estos subsistemas implican dinero real mal ejecutado.

**Excepción**: Refactors puramente mecánicos (renombrar, extraer función) con suite de tests existente que pasa.

---

## F2 — Usar `unwrap()` o `expect()` en rutas de producción de OMS/Risk

**Regla**: Prohibido `unwrap()`, `expect()`, indexación sin bounds check, o cualquier panic implícito en código de OMS, Risk, o pipeline de mercado.

**Por qué**: Un panic en medio de una orden en vuelo puede dejar el sistema en estado inconsistente.

**Alternativa**: `Result<T, E>` con errores tipados (`thiserror`), o `Option<T>` con manejo explícito del caso `None`.

---

## F3 — Loggear credenciales o datos sensibles

**Regla**: Ninguna credencial de broker (API keys, tokens, contraseñas), información personal, o datos de cuenta debe loguearse en ningún nivel de log.

**Por qué**: Compliance (MiFID II, SEC) y seguridad.

---

## F4 — Asumir fills perfectos en backtesting

**Regla**: El motor de backtesting nunca debe asumir que una orden se ejecuta al precio exacto de la señal sin slippage.

**Por qué**: Es la fuente más común de resultados de backtesting que no se replican en producción.

---

## F5 — Estado implícito en OMS

**Regla**: La máquina de estados de órdenes debe modelarse con enums de Rust, no con booleanos sueltos ni flags.

**Prohibido**:
```rust
// MAL: estado implícito
struct Order {
    is_submitted: bool,
    is_filled: bool,
    is_cancelled: bool,
    filled_qty: f64,
}
```

**Correcto**:
```rust
// BIEN: estado explícito
enum OrderState {
    New,
    Submitted,
    Working,
    PartiallyFilled { filled_qty: f64, remaining_qty: f64 },
    Filled,
    Cancelled,
    Rejected { reason: RejectReason },
}
```

---

## F6 — unsafe sin justificación documentada

**Regla**: Cada bloque `unsafe` debe tener un comentario `// SAFETY:` que explique por qué las precondiciones se cumplen.

**Además**: Prohibido `unsafe` en OMS, Risk Management, o cálculo de P&L bajo cualquier circunstancia.

---

## F7 — Mezclar cambios funcionales con refactors en el mismo commit

**Regla**: Un commit debe hacer una sola cosa: o arregla un bug, o agrega una feature, o refactoriza. Nunca las tres.

**Por qué**: Revisión y bisect de bugs.

---

## F8 — Deshabilitar Risk Management

**Regla**: Risk Management no se puede deshabilitar parcial ni totalmente sin reiniciar la aplicación en modo degradado explícito y documentado.

**Por qué**: Es la última barrera antes del mercado real.

---

## Enforcement

| Violación | Consecuencia |
|-----------|-------------|
| F1 (sin tests) | PR rechazado automáticamente |
| F2 (unwrap) | PR rechazado + revisión de seguridad |
| F3 (loggear credenciales) | Incidente de seguridad reportado |
| F4 (fills perfectos) | PR rechazado + revisión de arquitectura |
| F5 (estado implícito) | PR rechazado |
| F6 (unsafe injustificado) | PR rechazado + revisión de seguridad |
| F7 (mezcla) | PR rechazado, pedir split |
| F8 (deshabilitar risk) | Incidente crítico reportado |
