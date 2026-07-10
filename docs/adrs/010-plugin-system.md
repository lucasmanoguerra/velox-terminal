# ADR-010: Plugin System

| | |
|---|---|
| **ADR** | 010 |
| **Title** | Two-tier plugin system: Lua scripting + dynamic libraries |
| **Status** | Accepted |
| **Date** | 2026-07-09 |

## Context

As a professional trading terminal, users need to extend functionality:

- Custom trading strategies and indicators
- Proprietary connectors to specific brokers
- Custom alert logic and risk rules

Requirements:
- Must not compromise system stability (a crashing plugin must not crash the terminal)
- Must not degrade hot path performance (plugin code runs on its own schedule)
- Must be discoverable (users can install/remove without rebuilding)
- Sandboxed execution (plugins cannot access arbitrary system resources)

## Decision

Implement a two-tier plugin system:

### Tier 1: Lua Scripting (Internal)

For user strategies and indicators. Evaluated via `mlua` (Lua 5.4 VM).

- Runs in a separate thread with forced timeout
- Communicates via Event Bus observation + port trait calls
- Sandboxed: restricted `os`, `io`, `require` libraries
- Feature-gated behind `scripting` feature flag
- `velox-scripting` crate

```rust
// Lua script executed per candle:
function on_candle(candle)
    local sma20 = sma(candle.close, 20)
    local rsi14 = rsi(candle.close, 14)
    if rsi14 < 30 and candle.close > sma20 then
        return { action = "buy", qty = 100 }
    end
    return nil
end
```

### Tier 2: Dynamic Libraries (Future/Enterprise)

For advanced extensions. Loaded at runtime via `libloading` (C ABI).

- Each plugin is a `.so` / `.dylib` / `.dll` compiled separately
- Exports a C-ABI `_plugin_create` function returning `*mut dyn Plugin`
- Host and plugin must share Rust compiler version (ABI instability risk)
- WASM (via `wasmtime`) considered as safer alternative for untrusted code

```rust
// Plugin trait (C ABI compatible)
#[repr(C)]
pub struct PluginAPI {
    pub name: *const c_char,
    pub version: u32,
    pub on_event: Option<extern "C" fn(*const Event, *mut c_void)>,
    pub on_tick: Option<extern "C" fn(*const Tick, *mut c_void)>,
}
```

### Design Principles

1. **Core doesn't know about plugins**: Domain core is pure Rust with zero plugin awareness
2. **Plugins observe via Event Bus**: Plugins subscribe to events they need
3. **Plugins interact via port traits**: Plugin-generated orders go through `BrokerClient` trait
4. **Safety first**: Lua scripts are sandboxed; dynamic plugins run in separate process (future)

## Consequences

### Positive
- Users can extend without modifying core
- Lua scripting accessible to non-Rust developers
- Dynamic plugins enable proprietary/enterprise features
- Event Bus provides natural observation point

### Negative
- Lua VM adds ~1MB to binary size (feature-gated)
- Dynamic plugins require Rust ABI compatibility (fragile)
- Plugin API must be stable — changes break existing plugins
- No hot-reload in v1 (requires restart)

### Trade-offs
- `mlua` chosen over `rlua` for Lua 5.4 support and active maintenance
- C ABI chosen over WASM for v1 (lower latency, simpler toolchain)
- WASM may be added in v2 for untrusted plugin sandboxing

## Compliance

- Domain core crates MUST NOT depend on `mlua` or `libloading`
- Plugin execution must have timeout (panic + abort if exceeded)
- All plugin traits must be documented in `velox-scripting` and `velox-plugin` crates

## Notes

### Related ADRs
- ADR-007: Event Bus (plugin observation mechanism)

### References
- `docs/architecture/SYSTEM_OVERVIEW.md` — Plugin System section
- `mlua` crate docs
- `libloading` crate docs

### Change History
- 2026-07-09: Initial draft
