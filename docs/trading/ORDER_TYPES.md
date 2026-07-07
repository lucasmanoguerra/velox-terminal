# Order Types — velox-terminal

Tipos de orden soportados y planeados.

---

## MVP

| Type | Description | TIF Options | Status |
|------|-------------|-------------|--------|
| **Market** | Execute immediately at best available price | DAY, IOC, FOK | Planned MVP |
| **Limit** | Execute at specified price or better | DAY, GTC, IOC, FOK | Planned MVP |
| **Stop Market** | Becomes market order when stop price is reached | DAY, GTC | Planned MVP |
| **Stop Limit** | Becomes limit order when stop price is reached | DAY, GTC | Planned MVP |

## v1

| Type | Description | TIF Options | Status |
|------|-------------|-------------|--------|
| **OCO** (One-Cancels-Other) | Two orders; fill of one cancels the other | DAY, GTC | Planned v1 |
| **Bracket** | Entry + stop loss + take profit as a group | DAY, GTC | Planned v1 |
| **Trailing Stop** | Stop price that trails market at fixed distance | GTC | Planned v1 |
| **Iceberg** | Visible portion + hidden quantity | DAY, GTC | Planned v1 |

## Long-term Roadmap

| Type | Description | Status |
|------|-------------|--------|
| **TWAP** | Time-Weighted Average Price algo order | Roadmap |
| **VWAP** | Volume-Weighted Average Price algo order | Roadmap |
| **Pegged** | Price pegged to bid/ask/mid | Roadmap |
| **Adaptive** | Smart order routing based on liquidity | Roadmap |

---

## Time-in-Force Options

| TIF | Meaning | Applicable to |
|-----|---------|---------------|
| DAY | Expires at end of trading session | All |
| GTC | Good-Till-Cancelled (persistent) | Limit, Stop, Stop-Limit |
| IOC | Immediate-Or-Cancel (fill what's available, cancel rest) | Market, Limit |
| FOK | Fill-Or-Kill (all or nothing, or cancel entirely) | Market, Limit |
| GTD | Good-Till-Date (specific expiration) | Limit, Stop |
| EXT | Extended hours trading | Market, Limit |

---

## Order Lifecycle (simplified)

```
User sends → Risk validates → Broker receives → Working
                                                     │
                                          ┌──────────┼──────────┐
                                          ▼          ▼          ▼
                                        Filled   Partial    Rejected
                                                   │
                                                   ▼
                                                Filled
```

See `OMS_STATE_MACHINE.md` for the complete state machine.

---

## Standards Reference

Todos los tipos de orden modelados según el estándar FIX 4.4 (tags 40=OrdType, 59=TimeInForce) y las extensiones comunes de brokers.
