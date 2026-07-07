# Trading Documentation — velox-terminal

Documentación específica del dominio de trading: órdenes, riesgo, datos de mercado, backtesting.

## Documents

| File | Purpose | Read when |
|------|---------|-----------|
| `OMS_STATE_MACHINE.md` | Máquina de estados de órdenes, transiciones, fills parciales | Implementing OMS, debugging order state issues |
| `RISK_RULES.md` | Validaciones pre-trade, límites, circuit breakers | Implementing risk validations, configuring limits |
| `MARKET_DATA_MODEL.md` | Estructuras de tick/quote/OHLCV, SoA vs AoS, agregación | Working with market data structures, serialization |
| `ORDER_TYPES.md` | Tipos de orden soportados (Market, Limit, Stop, OCO, Bracket) | Adding new order types, configuring order entry UI |
| `BACKTESTING.md` | Motor de backtesting, slippage, métricas | Running/implementing backtests |

## Recommended Loading Order

1. `MARKET_DATA_MODEL.md` — understanding the data
2. `ORDER_TYPES.md` — what orders we support
3. `OMS_STATE_MACHINE.md` — how orders live
4. `RISK_RULES.md` — what guards them
5. `BACKTESTING.md` — how we validate strategies (if needed)
