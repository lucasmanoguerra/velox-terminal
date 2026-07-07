# DOM Ladder — velox-terminal

Depth of Market ladder — visualización del libro de órdenes.

---

## Layout

```
Symbol: ES    Last: 450.25    Volume: 125,430

───────────────────────────────────────────────
        Bid                    Ask
  Size    Price  Δ    Δ  Price    Size
───────────────────────────────────────────────
                    ▲  ▲  450.50  1,234
                    │  │  450.25  3,456
   1,234  450.00  ──┘  │  450.00  5,678  ◄ Last
   5,678  449.75      │  449.75  2,345
   2,345  449.50      ▼  449.50  1,234
───────────────────────────────────────────────
     Bid Vol: 9,257              Ask Vol: 13,947
     Imbalance: 33.4% Ask
───────────────────────────────────────────────
```

## Features

| Feature | Description |
|---------|-------------|
| **Cumulative volume** | Volume bars next to each price level |
| **Imbalance indicator** | Visual cue when bid/ask volume is skewed |
| **Last price marker** | Highlighted row at current last trade |
| **One-click trading** | Click bid → sell, click ask → buy |
| **Flashing** | Levels flash on trade (green for bid, red for ask) |
| **Delta** | Cumulative bid-ask delta over time |
| **Heat map** | Color intensity by volume at level |
| **Auto-scroll** | Center on last price or lock to current view |

## Performance

- El DOM debe renderizar en < 1ms (es uno de los paneles más críticos)
- Las actualizaciones de precio/flash deben ser < 16ms
- Los datos de profundidad se reciben en cada tick de quote (no cada trade)
