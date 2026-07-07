# Market Data Model — velox-terminal

Estructuras de datos de mercado: tick, quote, trade, OHLCV.

---

## Core Structures

### Tick (Last Trade)

```rust
#[repr(C)]  // Ensures predictable layout for bytemuck zero-copy
struct Tick {
    symbol: [u8; 8],        // Symbol padded to 8 bytes (e.g., "ES      ", "AAPL   ")
    price: f64,              // Last traded price
    volume: u64,             // Volume of this tick
    timestamp_ns: u64,       // Nanoseconds since epoch (UTC)
    exchange: [u8; 4],       // Exchange code (e.g., "CME ", "XNAS")
    conditions: u8,          // Tick condition flags (see below)
    side: u8,                // 0 = unknown, 1 = buy, 2 = sell
}
// Total: 8 + 8 + 8 + 8 + 4 + 1 + 1 = 38 bytes → padded to 40 bytes
```

### Quote (Bid/Ask)

```rust
#[repr(C)]
struct Quote {
    symbol: [u8; 8],
    bid_price: f64,
    ask_price: f64,
    bid_size: u64,
    ask_size: u64,
    timestamp_ns: u64,
    exchange: [u8; 4],
    quote_condition: u8,
}
// Total: 8 + 8 + 8 + 8 + 8 + 8 + 4 + 1 = 53 bytes → padded to 56 bytes
```

### Candle (OHLCV)

```rust
#[repr(C)]
struct Candle {
    symbol: [u8; 8],
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: u64,
    timestamp_ns: u64,       // Start of candle period
    timeframe_seconds: u32,  // 60 (1m), 300 (5m), 3600 (1h), 86400 (1d)
    tick_count: u32,         // Number of ticks aggregated
    vwap: f64,               // Volume-weighted average price
}
// Total: 8 + 8*5 + 8 + 4 + 4 + 8 = 72 bytes
```

---

## Memory Layout Strategy: SoA vs AoS

| Consumer | Pattern | Layout | Reason |
|----------|---------|--------|--------|
| Charting (render) | Sequential iteration over prices | SoA (arrays of f64 for open/high/low/close) | Cache-friendly iteration, SIMD-friendly |
| OMS (last price) | Random lookup by symbol | AoS (`Tick` struct) | Single struct fetch per lookup |
| Storage (write) | Batched append | SoA blocks | Better compression ratio |
| Backtesting (scan) | Sequential range scan | SoA (via columnar storage) | Only load needed columns |

### SoA for Hot Path

For charting rendering, use Structure of Arrays:

```rust
struct CandleBatch {
    symbols: Vec<u64>,         // Symbol as hash
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<u64>,
    timestamps: Vec<u64>,
    // metadata
    len: usize,
    timeframe: u32,
}
```

---

## Aggregation Pipeline

```
Tick arrives
    │
    ▼
Update active candle for each timeframe:
    ┌──────────────┐
    │ 1m candle    │ ← O(1) update: close=price, high=max(high, price), low=min(low, price), volume+=tick_volume
    ├──────────────┤
    │ 5m candle    │ ← same logic, different bucket
    ├──────────────┤
    │ 15m candle   │
    ├──────────────┤
    │ 1h candle    │
    ├──────────────┤
    │ 1d candle    │
    └──────────────┘
    │
    ▼
When bucket boundary crosses:
    │
    ├── Emit completed candle (crossbeam channel → storage)
    ├── Start new candle
    └── Update derived timeframes (1m → 5m, etc.) via running state
```

**Key design**: No recomputar desde cero. Cada timeframe mantiene un running state:

```rust
struct RunningCandle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: u64,
    tick_count: u32,
    vwap_sum: f64,       // sum of (price * volume) for VWAP calculation
    last_timestamp_ns: u64,
}
```

---

## Serialization

### IPC (Between Threads)

- **Formato**: bytemuck (`Pod` structs) → zero-copy cast to `&[u8]`
- **Transporte**: crossbeam channel carrying `&[u8]` slices from ring buffer
- **Sin heap alloc** en hot path: pre-allocar buffers en ring buffer

### Persistence (Disk)

- **Formato**: rkyv (zero-copy deserialization) para carga rápida en backtesting
- **Alternativa**: bincode para escritura más rápida en ingesta de datos
- **Compresión**: Zstd (ratio ~5:1 para tick data) o LZ4 (más rápido, ratio ~3:1)

---

## Performance Targets

| Operation | Target | Method |
|-----------|--------|--------|
| Parse raw tick → `Tick` struct | < 100ns | bytemuck cast |
| Update 5 timeframes from 1 tick | < 500ns | O(1) per timeframe |
| Serialize `Candle` → bytes | < 50ns | bytemuck cast |
| Batch emit 1000 candles to channel | < 10μs | crossbench batch send |
