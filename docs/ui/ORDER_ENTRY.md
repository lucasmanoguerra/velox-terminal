# Order Entry — velox-terminal

Panel de entrada de órdenes.

---

## Layout

```
┌───────────────────────────────────────┐
│  Order Entry                          │
├───────────────────────────────────────┤
│  Symbol: [ES      ]  Qty: [1]        │
│  Side:   ● Buy  ○ Sell               │
│  Type:   [Market ▼]                   │
│  Price:  [450.25         ]            │
│  Stop:   [               ]            │
│  TIF:    [DAY ▼]                      │
│                                       │
│  [PREVIEW]  [  BUY  ]  [ SELL ]      │
│                                       │
│  /!\ Risk: Max loss $500              │
│       Limits: OK ✓                    │
└───────────────────────────────────────┘
```

## Quick Buttons

Los quick buttons permiten envío con un clic:

```
┌───────────────────────────────────────┐
│  [Buy 1] [Buy 2] [Buy 5] [Buy 10]    │
│  [Sell 1] [Sell 2] [Sell 5] [Sell 10]│
│                                       │
│  [Cancel All] [Flatten]  [Reverse]    │
└───────────────────────────────────────┘
```

## Order Confirmation

Antes de enviar, mostrar confirmación visual:

```
┌─ Confirm Order ───────────────────────┐
│  Buy 1 ES at MARKET                   │
│  Est. Value: $45,025                  │
│  Risk: Max $500                       │
│                                       │
│        [CANCEL]  [CONFIRM]            │
└───────────────────────────────────────┘
```

La confirmación se puede deshabilitar para one-click trading vía hotkeys.

## Advanced Features (v1)

- **Order templates**: Guardar configuraciones de orden para reúso
- **Scale in/out**: Entrar/salir en múltiples niveles predefinidos
- **Bracket builder**: Configurar stop loss + take profit visualmente
- **OCO builder**: Configurar pares de órdenes One-Cancels-Other
