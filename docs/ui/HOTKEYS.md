# Hotkeys — velox-terminal

Sistema de hotkeys configurables de punta a punta.

---

## Design

Los hotkeys se definen en un archivo de configuración (TOML/JSON) y se cargan al inicio:

```toml
[hotkeys]
"Buy Market" = "F1"
"Sell Market" = "F2"
"Buy Limit" = "Shift+F1"
"Sell Limit" = "Shift+F2"
"Cancel All" = "Escape"
"Flatten Position" = "Ctrl+F"
"Toggle Chart Timeframe" = "T"
"Focus Order Entry" = "Space"
"Toggle DOM" = "D"
"Increment Quantity" = "Ctrl+Up"
"Decrement Quantity" = "Ctrl+Down"
"Toggle Workspace 1" = "Ctrl+1"
"Toggle Workspace 2" = "Ctrl+2"
```

## Default Hotkeys

| Action | Default | Scope |
|--------|---------|-------|
| Buy Market | `F1` | Global (works even if chart focused) |
| Sell Market | `F2` | Global |
| Buy Limit | `Shift+F1` | Global |
| Sell Limit | `Shift+F2` | Global |
| Buy Stop | `Ctrl+F1` | Global |
| Sell Stop | `Ctrl+F2` | Global |
| Cancel All | `Escape` | Global |
| Cancel Last Order | `Ctrl+Z` | Global |
| Flatten Position | `` Ctrl+` `` | Global |
| Increase Qty | `Ctrl+Up` | When order entry focused |
| Decrease Qty | `Ctrl+Down` | When order entry focused |
| Toggle Chart Timeframe | `T` | Chart focused |
| Focus Order Entry | `Space` | Global |
| Toggle DOM | `D` | Global |
| Toggle Chart | `C` | Global |
| Next Symbol | `Ctrl+Tab` | Global |
| Previous Symbol | `Ctrl+Shift+Tab` | Global |
| Save Workspace | `Ctrl+S` | Global |
| Load Workspace | `Ctrl+O` | Global |

## Implementation

```rust
struct HotkeyConfig {
    bindings: HashMap<HotkeyAction, KeyCombo>,
    // Loaded from config file at startup
}

enum HotkeyAction {
    BuyMarket,
    SellMarket,
    BuyLimit,
    SellLimit,
    CancelAll,
    FlattenPosition,
    IncreaseQuantity,
    DecreaseQuantity,
    // ... etc
}

struct KeyCombo {
    key: VirtualKey,
    modifiers: Modifiers,
}

fn dispatch_hotkey(action: &HotkeyAction) {
    match action {
        HotkeyAction::BuyMarket => oms::submit_order(OrderType::Market, Side::Buy),
        HotkeyAction::CancelAll => oms::cancel_all_orders(),
        // ...
    }
}
```

## Rules

- Los hotkeys deben funcionar incluso cuando el foco no está en la ventana de trading (configurable)
- No puede haber conflictos entre hotkeys (validación al cargar configuración)
- Los defaults deben ser razonables para un trader que migra desde NinjaTrader/ATAS
- Cada acción debe tener feedback visual inmediato (flash en pantalla) y sonido opcional
