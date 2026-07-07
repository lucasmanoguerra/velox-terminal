# Dependency Map — velox-terminal

Mapa de dependencias críticas entre subsistemas. Útil para planificar cambios cross-cutting y entender impacto.

---

## Critical Paths

```
systems-architect ──┬──> market-data-feed ──> charting-engine
                    ├──> oms ──> risk-management
                    └──> time-series-storage ──> backtesting

ui-ux-trading ──> frontend-egui ──> charting-engine (comparten wgpu)

broker-integration ──> oms
                  └──> market-data-feed
```

## Dependencias de Datos

| Dato | Producido por | Consumido por | Formato |
|------|-------------|-------------|---------|
| Tick (last price) | `feed` | `oms`, `charting`, `indicators`, `storage`, `risk` | `core::Tick` (bytemuck) |
| Candle OHLCV | `feed` (agregación) | `charting`, `indicators`, `storage` | `core::Candle` (bytemuck) |
| Order | `oms`, `gui` (user) | `risk`, `broker` (send), `gui` (display) | `core::Order` |
| Position | `oms` (derivado) | `gui`, `risk` | `core::Position` |
| Indicator value | `indicators` | `charting` (overlay) | Genérico T |
| Historical ticks | `storage` | `backtest` | `core::Tick` (rkyv) |
| User command | `gui` | `oms` | crossbeam channel enum |

## Dependencias de Compilación

```
core ──(no deps)──> foundation types
  │
  ├── feed ──> core
  ├── oms ──> core, risk
  ├── risk ──> core
  ├── broker ──> core
  ├── storage ──> core
  ├── indicators ──> core
  ├── charting ──> core, indicators
  ├── gui ──> core, charting, feed
  ├── backtest ──> core, indicators, storage
  └── scripting ──> core, indicators, oms
```

## Reglas de Dependencia

1. **No cyclical dependencies**: El grafo de dependencias entre crates debe ser un DAG. Verificado por `cargo-deny` o script CI.
2. **core es la base**: Todos los crates dependen de `core`, pero `core` no depende de nadie.
3. **risk es puro**: `risk` solo depende de `core`. Sin I/O, sin estado externo. Puramente funcional.
4. **charting y gui comparten wgpu**: Comparten el contexto de wgpu pero no se conocen entre sí a nivel de tipos. La integración es via `egui-wgpu`.
5. **indicators es independiente**: No depende del feed ni del charting. Opera sobre arrays de números.
6. **backtest reusa lógica en vivo**: Depende de `indicators` y `oms` para reusar la misma lógica de estrategia.

## Impact Analysis Quick Reference

| Si cambias... | Revisa... | Notifica a... |
|---------------|-----------|---------------|
| core::Tick | Todos los consumidores (feed, oms, charting, storage, indicators) | market-data-arch, feed, charting-engine |
| core::Order | oms, broker, gui, risk | oms, broker-integration |
| BrokerClient trait | broker (implementaciones), oms (llamante) | broker-integration, oms |
| RiskValidator trait | risk (implementación), oms (llamante) | risk-management, oms |
| OHLCV aggregation logic | feed (agregación), charting (consumo) | market-data-arch, charting-engine |
| wgpu pipeline | charting, gui (comparten contexto) | charting-engine, frontend-egui |
| Storage schema | storage (escritura), backtest (lectura) | time-series-storage, backtesting |
