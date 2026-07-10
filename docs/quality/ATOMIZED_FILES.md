# Atomized Files & Single Responsibility — velox-terminal

Estándar de atomización de archivos fuente. Cada archivo debe tener una única
responsabilidad y menos de 200 líneas efectivas de producción.

---

## Filosofía

> "UNIX philosophy: cada programa hace una cosa y la hace bien. Cada archivo
> sigue la misma regla: una responsabilidad, un concepto, un testeador."

La **atomización** es la práctica de dividir archivos grandes en archivos más
pequeños por responsabilidad. No es refactor por estética — es una decisión
arquitectónica que:

- Mejora la **testeabilidad** (tests unitarios enfocados por archivo)
- Facilita la **revisión** (PRs más pequeños, cambios localizados)
- Reduce **conflictos de merge** (equipos grandes tocando archivos pequeños)
- Documenta **límites de responsabilidad** (cada archivo dice qué hace)

## Regla de 200 Líneas

> **Cada archivo `.rs` debe tener < 200 líneas efectivas de código de producción**
> (excluyendo imports de crate, doc comments de módulo, líneas en blanco
> y bloques `#[cfg(test)]`).

```rust
// ✅ BUENO (~150 líneas efectivas):
// Una struct, su impl, y helpers relacionados
// Todo en un archivo

// ❌ MALO (~500+ líneas efectivas):
// 5 structs, 3 traits implementados, lógica de negocio mezclada,
// helpers de parsing, y tests inline — TODO en un archivo
```

### ¿Qué cuenta como "línea efectiva"?

| Tipo de línea | ¿Cuenta? |
|---------------|----------|
| `use` / `extern crate` imports | ❌ No |
| `//!` doc comments de módulo | ❌ No |
| Líneas en blanco | ❌ No |
| `#[cfg(test)]` blocks | ❌ No (separados) |
| `///` doc comments en items `pub` | ✅ Sí |
| Código (`fn`, `struct`, `impl`, etc.) | ✅ Sí |
| `//` comentarios inline | ✅ Sí |
| `#[derive(...)]`, `#[repr(...)]` | ✅ Sí |

## Inventario Actual (Deuda Técnica)

Escaneo del código fuente al 2026-07-09. Archivos que exceden 200 líneas
efectivas de producción (sin incluir tests):

| Archivo | Líneas prod | Límite | Exceso | Prioridad |
|---------|-------------|--------|--------|-----------|
| `crates/velox-exchange/src/binance_rest.rs` | 892 | 200 | +692 | 🔴 Alta |
| `crates/velox-chart/src/renderer.rs` | 748 | 200 | +548 | 🔴 Alta |
| `crates/velox-oms/src/paper_trader.rs` | 734 | 200 | +534 | 🔴 Alta |
| `crates/velox-exchange/src/binance_user_data.rs` | 765 | 200 | +565 | 🔴 Alta |
| `crates/velox-ui/src/app_state.rs` | 574 | 200 | +374 | 🟡 Media |
| `crates/velox-ui/src/panels.rs` | 571 | 200 | +371 | 🟡 Media |
| `crates/velox-exchange/src/binance.rs` | 530 | 200 | +330 | 🟡 Media |
| `crates/velox-oms/src/order_manager.rs` | 523 | 200 | +323 | 🟡 Media |
| `crates/velox-terminal/src/app.rs` | 401 | 200 | +201 | 🟡 Media |
| `crates/velox-exchange/src/binance_broker.rs` | 306 | 200 | +106 | 🟢 Baja |
| `crates/velox-chart/src/interaction.rs` | 275 | 200 | +75 | 🟢 Baja |
| `crates/velox-md/src/ring_buffer.rs` | 214 | 200 | +14 | 🟢 Baja |
| `crates/velox-oms/src/state_machine.rs` | 182 | 200 | -18 | ✅ Cumple |

**13 archivos exceden el límite**. Prioridad alta para los 4 archivos > 500 líneas.

## Estrategia de Refactorización

### Paso 1: Identificar responsabilidades mezcladas

Cada archivo grande suele mezclar 2-5 responsabilidades distintas.
Ejemplo de `renderer.rs` (748 líneas, 4 responsabilidades):

```
renderer.rs:
  ├── Structs de datos GPU (CandleGpuData, GridVertex, LineVertex)  →  renderer/types.rs
  ├── Creación de pipelines (candle, grid, volume, line)            →  renderer/pipelines.rs
  ├── Update de buffers GPU (update_lines, update_volume)           →  renderer/update.rs
  └── Render loop (render pass 1-5 wgpu commands)                   →  renderer/render.rs
```

### Paso 2: Extraer por responsabilidad

Para cada archivo grande:

1. **Crear directorio**: `archivo/` en lugar de `archivo.rs`
2. **Mover structs/tipos** a `archivo/types.rs` (o `mod.rs`)
3. **Extraer impl blocks** por funcionalidad en archivos separados
4. **Mantener re-export** en `mod.rs` para no romper callers
5. **Mover tests** a `archivo/tests.rs` o `archivo/tests/mod.rs`
6. **Verificar** que cada nuevo archivo < 200 líneas

### Patrón de División Recomendado

```
# ANTES: un archivo monolítico
src/
├── binance.rs           # 530 líneas — feed, handlers, reconnect, parsing

# DESPUÉS: un directorio con responsabilidades separadas
src/
├── binance/
│   ├── mod.rs           # Re-exports, BinanceFeed struct (~40 líneas)
│   ├── feed.rs          # ExchangeFeed impl, run_loop (~150 líneas)
│   ├── handlers.rs      # handle_message, handle_trade, handle_depth (~120 líneas)
│   ├── reconnect.rs     # Reconexión con backoff + jitter (~100 líneas)
│   ├── types.rs         # Tipos específicos del feed (~80 líneas)
│   └── tests.rs         # Tests unitarios (~100 líneas)
```

### Paso 3: No romper callers

```rust
// ANTES: archivo único
// src/binance.rs
pub struct BinanceFeed { ... }
pub fn parse_trade(json: &str) -> Result<Tick, Error> { ... }

// DESPUÉS: directorio, mod.rs re-exporta todo
// src/binance/mod.rs
mod feed;
mod handlers;
mod types;
pub use feed::BinanceFeed;
pub use types::*;
```

Los callers externos importan `velox_exchange::binance::BinanceFeed` —
el path no cambia.

## Cuándo NO Atomizar

1. **`lib.rs`**: Puede exceder 200 líneas si es principalmente re-exports
   y declaraciones `pub mod`. El límite aplica a código de lógica, no a
   re-exports.
2. **Archivos con datos generados**: Tablas grandes de lookup, datos de
   configuración embebidos. Documentar como `// FILE-EXEMPT: generated data`.
3. **Archivos que serán refactorizados pronto**: Si un archivo > 200 líneas
   tiene un refactor planeado en el sprint actual, no vale la pena atomizarlo
   dos veces.

## Verificación Automática

```bash
# Script de verificación de atomización
find crates -name '*.rs' -not -path '*/target/*' | while read f; do
    prod=$(sed -e '/^#\[cfg(test)\]/,/^[^[:space:]]/d' \
               -e 's/^[[:space:]]*\/\/[^!].*$//' \
               -e '/^[[:space:]]*$/d' "$f" | wc -l)
    if [ "$prod" -gt 200 ]; then
        exempt=$(grep -c "FILE-EXEMPT" "$f")
        if [ "$exempt" -eq 0 ]; then
            echo "VIOLATION: $f ($prod lines)"
        fi
    fi
done
```

Integrar en CI como warning no-bloqueante (bloqueante solo para archivos nuevos).

## Referencia: Divisiones Propuestas

### `binance_rest.rs` (892 → 5 archivos)
```
binance_rest/
├── mod.rs          # Re-exports (~20 líneas)
├── client.rs       # BinanceRestClient struct + impl (~200 líneas)
├── endpoints.rs    # new_order, cancel_order, account, etc. (~250 líneas)
├── signing.rs      # HMAC-SHA256, query building (~100 líneas)
├── types.rs        # BinanceAccountInfo, BinanceOrderResponse, etc. (~150 líneas)
└── tests.rs        # Tests (~150 líneas)
```

### `renderer.rs` (748 → 5 archivos)
```
renderer/
├── mod.rs          # ChartRenderer struct, init (~150 líneas)
├── pipelines.rs    # create_candle_pipeline, create_grid_pipeline, etc. (~180 líneas)
├── update.rs       # update_candles, update_volume, update_grid, update_lines (~200 líneas)
├── render.rs       # render() method, render passes 1-5 (~150 líneas)
└── types.rs        # CandleGpuData, GridVertex, LineVertex (~50 líneas)
```

### `paper_trader.rs` (734 → 5 archivos)
```
paper_trader/
├── mod.rs          # PaperTrader struct + public API (~150 líneas)
├── execution.rs    # should_fill_order, execute_open_orders (~120 líneas)
├── bracket.rs      # BracketConfig, create_bracket_children, etc. (~120 líneas)
├── positions.rs    # positions(), pnl calculations (~100 líneas)
└── tests.rs        # All tests (~200 líneas)
```

### `order_manager.rs` (523 → 4 archivos)
```
order_manager/
├── mod.rs          # OrderManager struct + core (~150 líneas)
├── submit.rs       # submit_order, submit_order_with_parent, replace (~120 líneas)
├── fills.rs        # apply_fill, fill management (~100 líneas)
├── cancel.rs       # cancel_order, child cancellation (~80 líneas)
└── tests.rs        # Tests (~150 líneas)
```

### `app_state.rs` (574 → 5 archivos)
```
app_state/
├── mod.rs          # AppState struct + core fields (~150 líneas)
├── orders.rs       # buy(), sell(), build_order(), bracket_prices (~120 líneas)
├── market.rs       # poll_candles(), sync_scroll_pos(), set_timeframe (~120 líneas)
├── broker.rs       # broker connection, keyring, TradingMode (~100 líneas)
└── tests.rs        # Tests (~150 líneas)
```

### `panels.rs` (571 → 6 archivos)
```
panels/
├── mod.rs          # PanelManager impl, show() dispatch (~100 líneas)
├── top_bar.rs      # Top bar: price, timeframe selector (~80 líneas)
├── order_entry.rs  # Order entry panel: tipo, qty, price, TP/SL, Buy/Sell (~120 líneas)
├── positions.rs    # Positions panel, account summary (~100 líneas)
├── dom_ladder.rs   # DOM ladder: bids, asks, spread (~100 líneas)
└── status_bar.rs   # Status bar: connection, mode, stats (~60 líneas)
```

## Meta-Regla

> No atomices por atomizar. Cada archivo nuevo debe tener una razón clara:
> "este archivo contiene toda la lógica de X, mientras que el archivo padre
> se ocupa de Y". Si la división no es obvia, es mejor esperar a que la
> responsabilidad se manifieste naturalmente.
