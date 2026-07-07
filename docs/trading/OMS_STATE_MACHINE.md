# OMS State Machine — velox-terminal

Máquina de estados del Order Management System.

---

## States

```rust
enum OrderState {
    /// Initial state — order created but not yet submitted
    New,
    /// Order sent to broker, awaiting acknowledgment
    PendingSubmit,
    /// Order acknowledged and working in the market
    Submitted,
    /// Order working with partial fills
    Working {
        filled_qty: u64,
        remaining_qty: u64,
    },
    /// Order fully filled
    Filled,
    /// Order cancelled by user or system
    Cancelled,
    /// Order rejected by broker or risk system
    Rejected {
        reason: RejectReason,
        details: String,
    },
    /// Order expired (GTD reached, or session end for DAY orders)
    Expired,
    /// Cancel request sent to broker, awaiting confirmation
    PendingCancel,
    /// Modification request sent to broker, awaiting confirmation
    PendingModify,
}
```

---

## Transitions

```
                    ┌─────────┐
                    │   New   │
                    └────┬────┘
                         │ submit (→ risk validate → broker send)
                         ▼
                  ┌──────────────┐
                  │ PendingSubmit │
                  └──────┬───────┘
                         │ broker ack
                    ┌────▼────┐
                    │ Submitted│
                    └────┬────┘
                         │
              ┌──────────┼──────────┬──────────────┐
              ▼          ▼          ▼              ▼
        ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌──────────┐
        │ Working │ │ Filled  │ │Rejected  │ │Expired   │
        │ (part.) │ │ (full)  │ │          │ │          │
        └────┬────┘ └─────────┘ └──────────┘ └──────────┘
             │ more fills
             ▼
        ┌─────────┐
        │ Filled  │
        └─────────┘

    ┌──────┐     cancel      ┌───────────────┐
    │Working├───────────────►│ PendingCancel  │
    └──┬───┘                 └───────┬───────┘
       │                             │ broker confirms
       │                             ▼
       │                      ┌───────────┐
       │                      │ Cancelled │
       │                      └───────────┘
       │ modify
       ▼
    ┌──────────────┐
    │PendingModify │
    └──────┬───────┘
           │ broker confirms
           ▼
        ┌─────────┐
        │ Working  │  (back to working with new params)
        └─────────┘
```

### Transition Rules (compile-time enforced)

| From | To | Trigger | Validation |
|------|----|---------|------------|
| New | PendingSubmit | `submit()` | Account has margin, position limits OK, not in circuit breaker |
| PendingSubmit | Submitted | `on_ack()` | Broker ClOrdID match |
| PendingSubmit | Rejected | `on_reject()` | Must include reason |
| Submitted | Working | `on_partial_fill()` | fill_qty <= order_qty - cum_fill |
| Submitted | Filled | `on_fill()` | cum_qty == order_qty |
| Submitted | Rejected | `on_reject()` | Must include reason |
| Submitted | PendingCancel | `cancel()` | Only if user initiated |
| Working | Working | `on_partial_fill()` | fill > 0, remaining > 0 |
| Working | Filled | `on_fill()` | remaining == 0 |
| Working | PendingCancel | `cancel()` | Only if user initiated |
| Working | PendingModify | `modify()` | Price/quantity change |
| PendingModify | Working | `on_modify_ack()` | Broker confirms new params |
| PendingModify | Rejected | `on_modify_reject()` | Modification refused |
| PendingCancel | Cancelled | `on_cancel_ack()` | Broker confirms cancel |
| PendingCancel | Filled | `on_fill()` | Fill BEFORE cancel processed |
| PendingCancel | Rejected | `on_cancel_reject()` | Cancel refused (order still alive) |
| Any | Expired | `on_expire()` | Time-in-force reached |

---

## Fills Parciales

Los fills parciales se manejan con:

```rust
struct Fill {
    order_id: OrderId,
    fill_price: f64,
    fill_qty: u64,
    fill_time: Timestamp,
    fill_id: FillId,       // unique per fill, idempotent
    is_buyer_initiated: bool,
}

struct OrderProgress {
    cum_fill_qty: u64,     // total filled quantity
    cum_fill_value: f64,   // total filled notional value
    avg_fill_price: f64,   // volume-weighted average
    last_fill_time: Option<Timestamp>,
}
```

**Idempotencia**: Cada `FillId` se registra en un HashSet. Si un fill duplicado llega, se ignora silenciosamente.

## Compile-time Safety

Los estados se modelan como enums de Rust para que las transiciones inválidas sean errores de compilación:

```rust
impl OrderState {
    fn apply_fill(self, fill: Fill) -> Result<Self, OrderError> {
        match self {
            OrderState::Submitted { qty } => {
                // Valid transition
                if fill.fill_qty == qty {
                    Ok(OrderState::Filled { cum_qty: qty, avg_price: fill.fill_price })
                } else {
                    Ok(OrderState::Working { filled_qty: fill.fill_qty, remaining_qty: qty - fill.fill_qty })
                }
            }
            OrderState::Filled { .. } => {
                // Invalid — already filled
                Err(OrderError::AlreadyFilled)
            }
            // exhaustive match — compiler catches missing arms
            _ => Err(OrderError::InvalidTransition {
                from: self.to_string(),
                to: "Fill",
            }),
        }
    }
}
```

## Reject Reasons

```rust
enum RejectReason {
    RiskCheckFailed { rule: String, limit: f64, actual: f64 },
    BrokerRejected { code: String, message: String },
    InvalidSymbol,
    InvalidPrice,
    InvalidQuantity,
    OrderTooLarge,
    InsufficientMargin,
    CircuitBreakerActive { breaker: CircuitBreaker },
    DuplicateOrderId,
    Unknown { code: String },
}
```

## Performance Constraints

- Las transiciones de estado deben ser O(1) sin allocation en el hot path
- La validación de idempotencia (FillId check) debe ser O(1) amortizado
- Perfil ReleaseSafe (no ReleaseFast) para todo el crate OMS
