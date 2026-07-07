# Risk Register — velox-terminal

Riesgos del proyecto con impacto, probabilidad y mitigaciones.

---

| # | Risk | Impact | Probability | Mitigation | Owner |
|---|------|--------|------------|--------|-------|
| R1 | Latencia de renderizado insuficiente con wgpu para 144fps | Alto | Media | Benchmarks tempranos con criterion, perfil de frame budget, geometría instanciada desde el día 1 | charting-engine |
| R2 | egui no escala a interfaces de trading complejas | Alto | Baja | Prototipo temprano de DOM ladder + order entry; tener alternativa (Druid/Slint) evaluada | frontend-egui |
| R3 | Broker API cambia y rompe conectores | Alto | Alta | Tests de integración que detectan cambios, interfaz trait-based con mock fácil, monitoreo de changelogs | broker-integration, dependency-maint |
| R4 | Bug en OMS causa ejecución incorrecta de órdenes | Crítico | Media | Property-based testing, máquina de estados verificada por compilador, ReleaseSafe, QA financiero obligatorio pre-merge | oms, qa-financiero |
| R5 | Bug en Risk Management no detecta violación de límite | Crítico | Baja | Fail-safe por defecto, validaciones redundantes, tests de invariantes con proptest | risk-management, qa-financiero |
| R6 | Deuda técnica acumulada por priorizar velocidad de desarrollo sobre calidad | Medio | Media | Definition of Done, code review obligatorio, revisiones periódicas de deuda técnica | lead, reviewer |
| R7 | Dependencias con vulnerabilidades de seguridad | Alto | Media | cargo-audit semanal, dependabot, priorización de seguridad sobre features | seguridad, dependency-maint |
| R8 | Performance de backtesting insuficiente para walk-forward analysis | Medio | Media | Rayon para paralelización, particionado de datos, benchmarks de performance | backtesting, performance |
| R9 | Scripting engine (Lua) introduce bugs de seguridad en OMS | Alto | Media | Sandboxing estricto, API acotada, ejecución en hilo separado con timeout | scripting-engine, seguridad |
| R10 | Mercado objetivo muy pequeño (traders que usan Linux) | Medio | Alta | Diferenciación en performance y features; soporte Windows/macOS cubre el mercado principal | product-financiero |
