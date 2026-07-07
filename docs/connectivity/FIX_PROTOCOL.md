# FIX Protocol — velox-terminal

Implementación del protocolo FIX (Financial Information eXchange) para conectividad con brokers institucionales.

---

## Overview

FIX es el estándar de facto para comunicación con brokers y exchanges institucionales. velox-terminal soporta FIX 4.2, 4.4 y 5.0 SP2.

## Components

```
crates/broker-fix/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── session.rs          # FIX session lifecycle
│   ├── messages/           # Message definitions & encoding
│   │   ├── mod.rs
│   │   ├── admin.rs        # Logon, Heartbeat, TestRequest, ResendRequest, SequenceReset
│   │   ├── application.rs  # NewOrderSingle, ExecutionReport, OrderCancelRequest, etc.
│   │   └── decoder.rs      # Low-level tag=value parser
│   ├── transport/          # TCP transport layer
│   │   ├── mod.rs
│   │   ├── connection.rs   # TCP connection with keepalive
│   │   └── reconnection.rs # Exponential backoff reconnection
│   └── dictionary/         # FIX dictionary per version
│       ├── mod.rs
│       └── spec/
```

## Session Lifecycle

```
┌─────────┐    Logon     ┌─────────┐    Heartbeat    ┌──────────┐
│  Idle   │ ──────────▶  │ Active  │ ──────────────▶ │ Pending  │
│         │ ◀──────────  │         │ ◀────────────── │  Logout  │
└─────────┘   Logout     └─────────┘  Timeout        └──────────┘
                              │  Logout Request          │
                              ├─────────────────────────▶│
                              │◀─────────────────────────│
                              │     Logout               │
```

## Message Flow (New Order)

```
Client (velox)                           Server (Broker)
     │                                        │
     │  NewOrderSingle (35=D)                 │
     │───────────────────────────────────────▶│
     │                                        │
     │  ExecutionReport (35=8)                │
     │  ExecType=New (0)                      │
     │◀───────────────────────────────────────│
     │                                        │
     │  ExecutionReport (35=8)                │
     │  ExecType=PartialFill (1)              │
     │◀───────────────────────────────────────│
     │                                        │
     │  ExecutionReport (35=8)                │
     │  ExecType=Filled (2)                   │
     │◀───────────────────────────────────────│
```

## Connection Strategy

| Aspect | Strategy |
|--------|----------|
| **Transport** | Persistent TCP with TLS 1.3 |
| **Heartbeat** | Every 30s configurable |
| **Reconnection** | Exponential backoff: 1s, 2s, 4s, 8s, 16s, max 60s |
| **Seq Numbers** | Persistent in SQLite, reset on gap fill |
| **Resend** | Gap fill request on seq number gap |
| **Logout** | Graceful: send logout, wait for Logout, close |
| **Testing** | FixSimulator for integration tests |

## Reliability

```rust
/// FIX session configuration
struct FixSessionConfig {
    sender_comp_id: String,         // e.g., "VELOX"
    target_comp_id: String,         // e.g., "IBKR"
    host: String,                   // broker FIX endpoint
    port: u16,
    ssl: bool,
    heartbeat_interval: u64,        // seconds
    max_retry_attempts: u32,
    persistent_seq_num_db: PathBuf, // SQLite path
    version: FixVersion,            // FIX42, FIX44, FIX50SP2
}

/// Possible session states
enum FixSessionState {
    Disconnected,
    Connecting,
    LoggingOn,
    Active,
    LoggingOut,
    PendingLogout,
    Failed(FixError),
}
```

## Performance Targets

| Metric | Target |
|--------|--------|
| Message encode | < 1µs |
| Message decode | < 1µs |
| TCP send + receive | < 100µs (local) |
| Session recovery | < 5s after disconnect |
| Memory per session | < 5MB |
