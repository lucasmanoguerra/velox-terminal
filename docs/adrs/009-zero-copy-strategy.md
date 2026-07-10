# ADR-009: Zero-Copy Data Strategy

| | |
|---|---|
| **ADR** | 009 |
| **Title** | Zero-copy data transformation on hot paths |
| **Status** | Accepted |
| **Date** | 2026-07-09 |

## Context

Market data arrives as raw bytes from WebSocket connections and must be transformed into domain types (Tick, Quote, Candle) for processing and display. Each transformation step is a potential source of latency and allocation:

- `serde_json` deserialization allocates Strings, maps, and intermediate values
- Heap allocations in the hot path cause cache misses and GC-like pauses
- Copying data between threads multiplies the overhead

With tick rates up to 1000/sec per symbol, every allocation matters.

## Decision

Use zero-copy techniques on all hot paths. Specifically:

1. **Network → Tick**: `#[repr(C)]` structs with `bytemuck::Pod` for direct cast from wire bytes
2. **IPC (RingBuffer)**: Pass `&[u8]` slices of `Pod` structs — no serde
3. **GPU upload**: `bytemuck::cast_slice` from SoA arrays to GPU vertex buffers
4. **Persistence load**: `rkyv` for zero-copy deserialization from disk
5. **String sharing**: `bytes::Bytes` for zero-copy borrow from network buffers

### Approved Techniques by Path

```rust
// Path 1: Network bytes → Tick (bytemuck Pod)
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
struct TickRaw {
    symbol: [u8; 8],
    price: f64,
    volume: u64,
    timestamp_ns: u64,
}

fn parse_tick(bytes: &[u8]) -> Result<&TickRaw, Error> {
    bytemuck::try_from_bytes(bytes)
        .map_err(|_| Error::InvalidSize)
}

// Path 2: GPU upload (bytemuck cast_slice)
fn upload_candles(gpu: &GpuDevice, candles: &[Candle]) -> Buffer {
    let bytes: &[u8] = bytemuck::cast_slice(candles);
    gpu.create_buffer_init(&wgpu::BufferDescriptor {
        contents: bytes,
        usage: wgpu::BufferUsages::VERTEX,
    })
}

// Path 3: Backtest load (rkyv zero-copy)
use rkyv::check_archived_root;
fn load_tick(bytes: &[u8]) -> Result<&ArchivedTick, Error> {
    check_archived_root::<Tick>(bytes)
        .map_err(|_| Error::CorruptData)
}
```

### When Zero-Copy Is Optional

- Config files: `serde_json` / `serde_toml` (loaded once at startup)
- Exchange info responses: `serde_json` (low frequency)
- User input forms: `serde` (human timescale)

## Consequences

### Positive
- Tick parsing < 100ns (vs 500ns-2μs with serde_json)
- GPU upload via `cast_slice` — no per-element copy
- Predictable, allocation-free hot path
- `#[repr(C)]` structs enable FFI and debugger inspection

### Negative
- Fixed-width fields (symbols padded to 8 bytes)
- Endianness must match wire format (native-endian from modern exchanges)
- `#[repr(C)]` structs require manual alignment management
- Changes to struct layout break all consumers (must reindex)

### Trade-offs
- `bytemuck` chosen over `zerocopy` crate for maturity and ergonomics
- `rkyv` chosen over `bincode` for zero-copy deserialization on load
- Fixed-width strings acceptable because symbol lengths are known (max 8 chars)

## Compliance

- Hot path Cargo.toml files MUST NOT depend on `serde_json` or `serde`
- grep for `serde_json::from_str` in hot path crates (exchange, md, chart) — must be absent
- New hot path structs must derive `bytemuck::Pod` + `bytemuck::Zeroable`
- Verify with `criterion` that tick parsing is < 100ns

## Notes

### Related ADRs
- ADR-008: Allocator Strategy (zero-copy reduces allocation pressure)

### References
- `docs/architecture/DATA_PIPELINE.md` — Latency Budget with zero-copy column
- `docs/quality/CODING_STANDARDS.md` — Zero-Copy Guidelines
- `bytemuck` crate docs
- `rkyv` crate docs

### Change History
- 2026-07-09: Initial draft
