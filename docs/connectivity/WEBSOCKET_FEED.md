# WebSocket Feed — velox-terminal

Conexión WebSocket para market data y órdenes en brokers modernos (Alpaca, Tradier, Polygon, etc.).

---

## Architecture

```
┌──────────┐     WebSocket      ┌──────────┐
│ velox    │ ◄────────────────► │  Broker  │
│ terminal │                    │  Server  │
└────┬─────┘                    └──────────┘
     │
     │ Channels (crossbeam)
     ├──▶ Market Data Ring Buffer
     ├──▶ Order Updates
     └──▶ Account Events
```

## Connection Lifecycle

```rust
struct WebSocketConfig {
    url: String,                    // e.g., wss://paper-api.alpaca.markets/stream
    api_key: EncryptedString,
    api_secret: EncryptedString,
    heartbeat_interval: Duration,
    reconnect_backoff: BackoffStrategy,
}

enum WsConnectionState {
    Disconnected,
    Connecting,
    Authenticating,
    Connected { session_id: String },
    Reconnecting { attempt: u32, next_attempt_at: Instant },
    Failed { reason: String, permanent: bool },
}
```

## Message Protocol

### Authentication

```json
{
  "action": "auth",
  "key": "AK123...",
  "secret": "..."
}
```

### Subscription

```json
{
  "action": "subscribe",
  "trades": ["ES", "NQ", "CL"],
  "quotes": ["ES", "NQ"],
  "bars": ["ES", "1Min"],
  "status": ["ES"]
}
```

### Data Messages

```json
// Trade
{
  "T": "t",
  "S": "ES",
  "p": 4502.25,
  "s": 100,
  "t": "2026-07-06T14:30:00.123456Z",
  "c": ["@", "F"]
}

// Quote
{
  "T": "q",
  "S": "ES",
  "bp": 4502.00,
  "bs": 500,
  "ap": 4502.25,
  "as": 1200,
  "t": "2026-07-06T14:30:00.123456Z"
}

// Bar (OHLCV)
{
  "T": "b",
  "S": "ES",
  "o": 4500.00,
  "h": 4510.50,
  "l": 4495.75,
  "c": 4505.25,
  "v": 125430,
  "t": "2026-07-06T14:30:00Z"
}
```

## Feed Implementation

```rust
/// Generic WebSocket feed for market data
struct WsMarketFeed {
    connection: WsConnection,
    subscriptions: HashMap<String, SubscriptionSpec>,
    ring_buffer: Arc<MpmcRingBuffer<MarketDataEvent>>,
    last_heartbeat: Instant,
}

impl WsMarketFeed {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&mut self, symbol: &str, channels: &[ChannelType]) -> Result<()>;
    async fn unsubscribe(&mut self, symbol: &str) -> Result<()>;
    async fn reconnect(&mut self) -> Result<()>;
    fn next_event(&self) -> Option<MarketDataEvent>;
}
```

## Ring Buffer Integration

Los eventos de WebSocket se insertan en un ring buffer lock-free para que el hilo de procesamiento de mercado los consuma sin contención:

```rust
const RING_BUFFER_CAPACITY: usize = 1 << 16; // 65,536 events

enum MarketDataEvent {
    Trade(TradeTick),
    Quote(QuoteTick),
    Bar(Ohlcv),
    Status(TradingStatus),
    Error(WsError),
    ConnectionStateChange(WsConnectionState),
}
```

## Auto-Reconnection

```
[Connected] ── disconnect ──▶ [Disconnected]
    ▲                              │
    │                              ▼
    │                       [Waiting 1s]
    │                              │
    │                              ▼
    │                       [Connecting] ── fail ──▶ [Waiting 2s]
    │                              │                      │
    │                              ▼                      │
    │                       [Authenticating]              │
    │                              │                      │
    │                              ▼                      │
    └──────────────────────── [Connected]                 │
                                                          │
                                        [Max attempts (10)]│
                                                          ▼
                                                  [Failed: permanent]
```

## Performance Targets

| Metric | Target |
|--------|--------|
| Connection establishment | < 500ms |
| Reconnection | < 5s (after network recovery) |
| Message parse | < 500ns |
| Ring buffer insert | < 100ns |
| Memory per connection | < 2MB |

## Supported Brokers

| Broker | WebSocket | Data Types | Auth |
|--------|-----------|------------|------|
| Alpaca | ✓ | trades, quotes, bars, account | API Key + Secret |
| Polygon | ✓ | trades, quotes, bars | API Key |
| Tradier | ✓ | quotes, trades, stream | Bearer Token |
| Schwab | ✓ | quotes, trades | OAuth 2.0 |
| Tradovate | ✓ | quotes, trades, DOM | JWT |
