# Broker Interface — velox-terminal

Interfaz común `BrokerClient` para todos los brokers/exchanges.

---

## Trait Definition

```rust
#[async_trait]
pub trait BrokerClient: Send + Sync {
    /// Connect to the broker with given credentials
    async fn connect(&self, config: BrokerConfig) -> Result<ConnectionHandle, BrokerError>;

    /// Disconnect gracefully
    async fn disconnect(&self, handle: &ConnectionHandle) -> Result<(), BrokerError>;

    /// Submit a new order
    async fn submit_order(&self, order: NewOrder) -> Result<OrderId, BrokerError>;

    /// Cancel an existing order
    async fn cancel_order(&self, order_id: OrderId) -> Result<(), BrokerError>;

    /// Modify an existing order
    async fn modify_order(&self, order_id: OrderId, modify: OrderModification) -> Result<(), BrokerError>;

    /// Request current positions
    async fn get_positions(&self) -> Result<Vec<Position>, BrokerError>;

    /// Request current account info
    async fn get_account_info(&self) -> Result<AccountInfo, BrokerError>;

    /// Subscribe to market data for a symbol
    async fn subscribe_market_data(&self, symbol: &str) -> Result<FeedToken, BrokerError>;

    /// Unsubscribe from market data
    async fn unsubscribe_market_data(&self, token: FeedToken) -> Result<(), BrokerError>;

    /// Subscribe to order updates (fills, rejections, etc.)
    async fn subscribe_order_updates(&self) -> Result<Receiver<OrderUpdate>, BrokerError>;
}
```

## Authentication

```rust
struct BrokerCredentials {
    api_key: EncryptedString,
    api_secret: EncryptedString,
    passphrase: Option<EncryptedString>,
    // Stored via keyring, never logged or serialized in plaintext
}
```

## Error Handling

```rust
enum BrokerError {
    ConnectionFailed { reason: String, retryable: bool },
    AuthenticationFailed { reason: String },
    RateLimited { retry_after_secs: u64 },
    OrderRejected { order_id: OrderId, reason: String, code: String },
    Timeout { operation: String },
    Disconnected { will_reconnect: bool },
    ProtocolError { message: String },
    NotImplemented { feature: String },
}
```

## Implementation Pattern

Cada broker implementa el trait en su propio módulo:

```
crates/broker/src/
├── mod.rs
├── traits.rs              # BrokerClient definition
├── interactive_brokers/   # IBKR implementation
│   ├── mod.rs
│   ├── client.rs
│   ├── connection.rs
│   └── messages.rs
├── alpaca/               # Alpaca implementation
│   ├── mod.rs
│   ├── rest.rs
│   ├── websocket.rs
│   └── auth.rs
├── simulator/            # Paper trading implementation
│   ├── mod.rs
│   ├── market_sim.rs
│   └── order_sim.rs
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

    mock! {
        pub BrokerClient {}
        impl BrokerClient for BrokerClient {
            // auto-generated mock methods
        }
    }
}
```

Todos los tests de OMS usan un `MockBrokerClient` — nunca un broker real en tests unitarios.
