# Panel System — velox-terminal

Sistema de paneles dockables y workspace layout.

---

## Design

El workspace se compone de paneles que el usuario puede reorganizar libremente:

```
┌─────────────────────────────────────────────────────────┐
│  Menu Bar  │  Symbol Selector  │  Timeframe  │  ▢ ╳    │
├──────────┬──────────────────────────────────────┬────────┤
│ Watchlist│                                      │ DOM    │
│          │                                      │ Ladder │
│ ES       │        Chart Area                    │        │
│ NQ       │    (candles + indicators)            │Bid Ask │
│ CL       │                                      │ 450 452│
│ GC       │                                      │ 449 453│
│          │                                      │ 448    │
├──────────┴──────────────────────────────────────┴────────┤
│ Positions  │  Orders  │  Time & Sales  │  Account        │
│ ES +2 450  │ Buy 1 ES │  451.25 100 +  │  Cash: $50,234  │
│ NQ -1      │ Sell Lmt  │  451.50 200 - │  Mgn: $12,000   │
└─────────────────────────────────────────────────────────┘
```

## Panel Types

| Panel | Type | Default Position | Purpose |
|-------|------|-----------------|---------|
| Chart | Main | Center | Candlestick chart with indicators |
| Watchlist | Side | Left | List of symbols with last price/change |
| DOM Ladder | Side | Right | Depth of Market |
| Order Entry | Float | Bottom-right | Order submission form |
| Positions | Dock | Bottom-left | Open positions |
| Orders | Dock | Bottom | Active and pending orders |
| Time & Sales | Dock | Bottom-right | Trade tape |
| Account | Dock | Bottom | Account summary |
| Order Book | Float | Right (with DOM) | Full order book |

## Docking System

```rust
enum DockPosition {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Float { x: f32, y: f32, width: f32, height: f32 },
    Tab { group_id: u32 },
}
```

- Panels can be dragged by their title bar
- Dropping on another panel → tab group
- Dropping on edge → split
- Floating panels can be positioned on secondary monitors
- Workspace layouts can be saved/loaded as JSON

## State Management

El layout del workspace se persiste en `~/.config/velox-terminal/workspace.json`:

```json
{
  "version": 1,
  "panels": [
    { "type": "chart", "dock": "center", "symbol": "ES", "timeframe": "1m" },
    { "type": "watchlist", "dock": "left", "width": 200 },
    { "type": "dom", "dock": "right", "width": 300, "symbol": "ES" }
  ],
  "active_tab_groups": [],
  "monitor_layout": { "monitors": 2, "main": 0 }
}
```
