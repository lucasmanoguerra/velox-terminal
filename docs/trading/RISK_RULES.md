# Risk Rules — velox-terminal

Validaciones pre-trade, límites de exposición, circuit breakers.

---

## Risk Validation Pipeline

Toda orden pasa por este pipeline antes de salir hacia el broker:

```
Order → [Position Limits] → [Margin Check] → [Order Size] → [Circuit Breakers] → [User Limits]
                                                                                        │
                                                                                   Pass/Fail
                                                                                        │
                                                                                   Allow/Reject
```

Si **cualquier** validación falla, la orden es rechazada con el motivo específico. No existe "best effort".

---

## Position Limits

| Rule | Configurable | Default | Description |
|------|------------|---------|-------------|
| Max position per symbol | Sí | 100 contracts | Máxima posición neta (long + short) en un símbolo |
| Max gross exposure | Sí | $500,000 | Valor nocional total de todas las posiciones |
| Max leverage | Sí | 4:1 | Apalancamiento máximo permitido |
| Max daily loss | Sí | $10,000 | Pérdida máxima por día de trading |
| Max drawdown from peak | Sí | 20% | Drawdown máximo desde el pico de la sesión |
| Concentración por sector | Sí | 40% | Máximo porcentaje de capital en un sector |

## Margin Check

| Rule | Description |
|------|-------------|
| Initial margin | Margen requerido para abrir nueva posición |
| Maintenance margin | Margen mínimo para mantener posición abierta |
| Available cash | Efectivo disponible para nuevas posiciones |
| Overnight margin | Margen incrementado para posiciones overnight |

**Fail-safe**: Si no se puede obtener el margen actual del broker, la orden se rechaza.

## Order Size Limits

| Rule | Description |
|------|-------------|
| Max order quantity | Límite absoluto por orden individual |
| Max order notional | Límite en valor monetario por orden |
| Min order quantity | Cantidad mínima (evita órdenes demasiado pequeñas) |
| Max order rate | Máximo de órdenes por segundo/minuto |

## Circuit Breakers

| Breaker | Trigger | Action | Auto-reset |
|---------|---------|--------|------------|
| **Feed Disconnect** | Feed de precios desconectado > 5s en horario activo | Rechazar todas las órdenes nuevas | Sí, cuando feed reconecta + 1 tick |
| **Spread Alert** | Bid-ask spread > 5x el spread promedio del día | Rechazar Market orders, permitir Limit | Sí, cuando spread normaliza |
| **Order Flood** | > 10 órdenes por segundo durante 5 segundos | Rate-limit a 1 orden/segundo | Sí, después de 60s sin exceder |
| **Error Spike** | > 20% de órdenes rechazadas en 1 minuto | Detener todo envío por 60s | Sí, después del timeout |
| **Price Gap** | Precio se mueve > 5% en < 1 segundo | Rechazar Market orders, alertar usuario | Manual |
| **Session End** | Menos de 5 minutos para el cierre de sesión | Rechazar DAY orders, permitir GTC | No (fin de sesión) |

### Circuit Breaker State Machine

```
                    ┌──────────┐
                    │ Inactive │
                    └─────┬────┘
                          │ trigger condition met
                          ▼
                    ┌──────────┐
                    │  Active  │◄──────────┐
                    └─────┬────┘          │
                          │                │
                    ┌─────┴─────┐         │
                    │           │          │
                    ▼           ▼          │
              ┌─────────┐ ┌─────────┐     │
              │Auto-reset│ │Manual   │     │
              │(timeout) │ │(user)   │     │
              └────┬────┘ └────┬────┘     │
                   │            │          │
                   └─────┬──────┘          │
                         │ condition again?│
                         └─────────────────┘
                         │ condition clear
                         ▼
                    ┌──────────┐
                    │ Inactive │
                    └──────────┘
```

## User-Configurable Limits

| Limit | Default | Scope |
|-------|---------|-------|
| Max loss per trade | No limit | Per order |
| Max loss per day | No limit | Session |
| Max loss per week | No limit | Rolling week |
| Max positions open | 10 | Total |
| Max positions per symbol | 2 | Per symbol |
| Max contracts per order | 50 | Per order |
| Allowed order types | All | Select which types |
| Allowed symbols | All | Whitelist/blacklist |

## Audit Trail

Cada validación de riesgo debe registrar:

```rust
struct RiskAuditEntry {
    timestamp: Timestamp,      // when validation occurred
    order_id: OrderId,         // which order
    rule_id: String,           // which rule was checked
    rule_config: String,       // rule parameters at time of check
    result: RiskResult,        // Pass / Fail
    reason: Option<String>,    // why it failed
    user_override: Option<bool>, // was bypassed?
    session_id: SessionId,     // trading session identifier
}
```

## Implementation Notes

- **Risk es síncrono**: Se ejecuta en el mismo hilo que OMS antes de enviar una orden. No hay async/await en risk.
- **Risk es puro**: No hace I/O. Si necesita datos del feed, le llegan como parámetro de entrada.
- **Risk no puede desactivarse**: Sin una modificación explícita de configuración y reinicio.
- **Límites en tiempo real**: Los límites (max pérdida, max drawdown) se verifican contra P&L actualizado en cada orden.
