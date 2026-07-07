# Connectivity Documentation — velox-terminal

Conectores a brokers/exchanges: FIX, WebSocket, REST. Reconexión resiliente, idempotencia.

## Documents

| File | Purpose | Read when |
|------|---------|-----------|
| `BROKER_INTERFACE.md` | Trait BrokerClient, interfaz común para todos los brokers | Adding a new broker connector |
| `FIX_PROTOCOL.md` | Integración FIX 4.4, sesiones, mensajes | Debugging FIX connectivity |
| `WEBSOCKET_FEED.md` | Conexión WebSocket para market data | Implementing real-time data feed |
| `RECONNECTION_STRATEGY.md` | Backoff exponencial, idempotencia, session recovery | Building resilient connections |

## Recommended Loading Order

1. `BROKER_INTERFACE.md` — understand the abstraction
2. `RECONNECTION_STRATEGY.md` — understand reliability
3. Individual protocol docs as needed
