# Regulatory Requirements — velox-terminal

Requisitos regulatorios aplicables. **No constituye asesoría legal**.

---

## MiFID II (UE)

| Requirement | Technical Implementation | Priority |
|-------------|------------------------|----------|
| **Order recording** (Art. 25) | Audit log inmutable para cada orden, modificación, cancelación y fill | MUST |
| **Best execution** (Art. 27) | Logging de precios de mercado al momento de ejecución para demostrar best execution | MUST |
| **Transaction reporting** (Art. 26) | Formato de datos requerido para reportes transaccionales | SHOULD |
| **Record keeping** (Art. 16) | Retención de datos de órdenes por 5 años | MUST |
| **Clock synchronization** (RTS 25) | Sincronización NTP, precisión de < 1ms | MUST |
| **Business continuity** (Art. 29) | Plan de reconexión, respaldo de datos | SHOULD |

## SEC/FINRA (EE.UU.)

| Requirement | Technical Implementation | Priority |
|-------------|------------------------|----------|
| **Order audit trail** (Rule 613) | CAT (Consolidated Audit Trail) compliant recording | MUST (if US equities) |
| **Best execution** (Rule 5310) | Market data at time of execution for best ex analysis | MUST |
| **Record keeping** (Rule 17a-4) | Retention of electronic records, non-rewritable/non-erasable | MUST |
| **Supervision** (Rule 3110) | Review of orders, alerts for unusual activity | SHOULD |
| **Business continuity** (Rule 4370) | BCP plan, data backup | SHOULD |

## Technical Implementation Summary

| Area | Implementation | Reference |
|------|---------------|-----------|
| Clock sync | NTP daemon, timestamps en nanosegundos UTC | `docs/architecture/DATA_PIPELINE.md` |
| Audit log | Append-only, hash-encadenado (SHA-256) | `docs/security/AUDIT_LOG.md` |
| Data retention | Particionado por fecha, purga automática | `docs/compliance/DATA_RETENTION.md` |
| Order recording | Cada evento de orden logueado en audit trail | `docs/trading/OMS_STATE_MACHINE.md` |
| Best execution | Market data snapshot stored with each fill | `docs/trading/MARKET_DATA_MODEL.md` |
