# Project Scope — velox-terminal

Alcance del proyecto velox-terminal.

---

## In Scope

### MVP Funcional
- Terminal de escritorio multiplataforma (Windows/macOS/Linux)
- Conexión a al menos un broker/exchange real
- Charting con velas, volumen, y overlays de indicadores
- Order entry (Market, Limit, Stop)
- DOM ladder (Depth of Market)
- Time & Sales
- Watchlist multi-símbolo
- Paneles de posición y órdenes activas
- Hotkeys configurables

### v1 Competitiva
- Tipos de orden avanzados (OCO, Bracket, Stop-Limit)
- Indicadores técnicos completos (SMA, EMA, RSI, MACD, Bollinger, ATR, VWAP, Volume Profile)
- Backtesting con slippage y comisiones
- Múltiples brokers simultáneos
- Workspace guardable/cargable
- Múltiples monitores

### Long-term Roadmap
- Motor de scripting para estrategias (Lua embebido)
- Trading algorítmico automatizado
- Análisis de cartera avanzado
- Simulación de estrategias en tiempo real
- Integración con fuentes de datos alternativas

---

## Out of Scope

- **No es un framework genérico**: El producto es una terminal de trading, no un SDK ni una plataforma de desarrollo
- **No es un bróker**: No procesamos pagos, no custodiamos fondos, no ejecutamos órdenes directamente
- **No es una red social**: No hay perfiles de usuario, seguimiento de traders, ni contenido generado por la comunidad
- **No es mobile-first**: Enfoque desktop con soporte para alta densidad de información
- **No es una plataforma de datos fundamentales**: El foco es datos de precio y volumen (técnico), no fundamentales
- **No es un reemplazo de Excel/Google Sheets**: No hay hoja de cálculo integrada

---

## Technical Boundaries

| Capa | Tecnología | Scope |
|------|-----------|-------|
| Gráficos | wgpu | Renderizado GPU de charting y overlays |
| UI | egui | Paneles de trading, controles, formularios |
| Texto | glyphon | Etiquetas, ejes, anotaciones |
| Async | tokio | I/O de red (feeds, brokers, APIs) |
| Concurrencia | crossbeam | Canales lock-free para hot paths |
| Persistencia | redb/sled | Time-series embebida |
| Scripting | mlua | Estrategias de usuario sandboxeadas |
| Serialización IPC | bytemuck/rkyv | Zero-copy entre threads |
| Testing | proptest/criterion | Property-based tests y benchmarks |
