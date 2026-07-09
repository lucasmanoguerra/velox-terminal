# Referencia de Inspiración — velox-terminal

Proyectos open source y plataformas comerciales que inspiran el diseño,
arquitectura y hoja de ruta de velox-terminal.

---

## Proyectos Open Source

### 1. Fincept Terminal

| Atributo | Detalle |
|----------|---------|
| **URL** | [GitHub](https://github.com/fincept/fincept-terminal) |
| **Stack** | C++20, Qt6, Python embebido |
| **Licencia** | Open source |

**Características destacadas**:
- Terminal financiera todo-en-uno inspirada en Bloomberg Terminal
- 100+ conectores de datos financieros
- IA integrada + soporte para LLMs locales (Ollama, etc.)
- Automatización mediante sistema de nodos visuales
- Análisis financiero avanzado
- Broker integration

**Qué aprender**: Arquitectura de terminal financiera integral. Sistema de conectores
de datos extensible. Integración de IA local en flujos de trading.

---

### 2. OpenTerminalUI

| Atributo | Detalle |
|----------|---------|
| **URL** | [GitHub](https://github.com/OpenTerminalUI/openterminalui) |
| **Stack** | React, TypeScript, FastAPI, Python, Docker |
| **Licencia** | Open source |

**Características destacadas**:
- Múltiples gráficos sincronizados
- Stock screener con filtros avanzados
- Portfolio management con P&L tracking
- Gestión de riesgo integrada
- Backtesting engine
- Command palette (acceso rápido tipo VS Code)
- Watchlists personalizables

**Qué aprender**: UX/UI moderna para terminal financiera. Command palette pattern.
Diseño de screener y portfolio dashboard.

---

### 3. Nautilus Trader

| Atributo | Detalle |
|----------|---------|
| **URL** | [GitHub](https://github.com/nautechsystems/nautilus_trader) |
| **Stack** | Rust (core), Python (binding), arquitectura por eventos |
| **Licencia** | LGPL |

**Características destacadas**:
- Motor de trading cuantitativo de alto rendimiento
- Arquitectura event-driven con actor model
- Backtesting con slippage y comisiones realistas
- Live trading multi-broker
- Estrategias en Python sobre core Rust
- Serialización zero-copy (similar a nuestro stack)

**Qué aprender**: Arquitectura de sistema de trading profesional en Rust.
Integración Rust ↔ Python. Event sourcing aplicado a trading. Modelo de
órdenes y fills.

---

### 4. OpenAlgo

| Atributo | Detalle |
|----------|---------|
| **URL** | [GitHub](https://github.com/OpenAlgo/OpenAlgo) |
| **Stack** | Python, Web |
| **Licencia** | Open source |

**Características destacadas**:
- Integración con TradingView (webhooks)
- Conexión MetaTrader
- APIs REST para automatización
- Dashboard web de monitoreo
- Estrategias algorítmicas
- Multi-broker support

**Qué aprender**: Patrón de integración TradingView → broker. Automatización
vía webhooks. Dashboard de monitoreo de estrategias.

---

### 5. ProfitMaker

| Atributo | Detalle |
|----------|---------|
| **URL** | [GitHub](https://github.com/profitmaker/profitmaker) |
| **Stack** | React, Node.js, CCXT |
| **Licencia** | Open source |

**Características destacadas**:
- Terminal crypto multiplataforma
- Gráficos en tiempo real
- Gestión de órdenes (market, limit, stop)
- Balances y portfolio
- Historial de trades
- 130+ exchanges via CCXT

**Qué aprender**: Integración con CCXT para multi-exchange. UI de trading
crypto. Manejo de balances multi-divisa.

---

### 6. Freqtrade

| Atributo | Detalle |
|----------|---------|
| **URL** | [GitHub](https://github.com/freqtrade/freqtrade) |
| **Stack** | Python, Web (Flask/Vue.js) |
| **Licencia** | GPL |

**Características destacadas**:
- Bot de trading algorítmico
- Web UI para monitoreo y control
- Gestión de órdenes avanzada (DCA, stop-loss, trailing)
- Backtesting con hiperparameter optimization
- Estrategias personalizables en Python
- Riesgo: max drawdown, position sizing, stake management
- Signal providers (TradingView, MQTT, etc.)

**Qué aprender**: Bots de trading en producción. Estrategias de risk management
probadas. Arquitectura de backtesting + live trading. Optimización con
machine learning.

---

### 7. OS Engine (Order Storm Engine)

| Atributo | Detalle |
|----------|---------|
| **URL** | [GitHub](https://github.com/OrderStorm/os-engine) |
| **Stack** | C#, .NET, Windows Forms |
| **Licencia** | Open source |

**Características destacadas**:
- Terminal de trading de escritorio profesional
- Múltiples brokers (Interactive Brokers, etc.)
- Trading manual y automático
- Análisis técnico avanzado
- Gestión de riesgo integrada

**Qué aprender**: Arquitectura de terminal .NET para trading. Estrategias de
conexión multi-broker. UX para trading manual profesional.

---

## Plataformas Comerciales (Referencia de UX/features)

### NinjaTrader
- **Tipo**: Terminal de escritorio profesional (C#, .NET)
- **Puntos fuertes**: DOM superiores, chart trading, Market Analyzer,
  Strategy Builder visual, ATM Strategies, multi-broker
- **Inspiración**: DOM ladder, chart trading, estrategias visuales

### MetaTrader 4/5
- **Tipo**: Terminal de trading retail más usada del mundo (C++)
- **Puntos fuertes**: MQL4/MQL5 scripting, indicadores personalizables,
  EA (Expert Advisors), backtesting integrado
- **Inspiración**: Motor de scripting, community marketplace, backtesting

### TradingView
- **Tipo**: Plataforma web de charting
- **Puntos fuertes**: Pine Script, alertas, screeners, watchlists,
  calendario económico, community ideas, multi-timeframe
- **Inspiración**: UX de charting, Pine Script-like DSL, alertas,
  calendario económico, social trading features

### ATAS (Advanced Trading Analytical Software)
- **Tipo**: Terminal profesional para Order Flow (C++)
- **Puntos fuertes**: Footprint charts, cluster analysis, DOM premium,
  Volume Profile, Market Profile, Time & Sales, Smart DOM
- **Inspiración**: Footprint, Volume Profile, Order Flow, DOM avanzado

### DeepChart
- **Tipo**: Plataforma de análisis técnico
- **Puntos fuertes**: Herramientas de dibujo avanzadas, detección de
  patrones, IA para reconocimiento de figuras
- **Inspiración**: Drawing tools, pattern recognition, AI analysis

---

## Extracción de Features Clave por Categoría

### Charting & Visualización
| Feature | Fuente | Prioridad |
|---------|--------|-----------|
| Multi-timeframe sincronizado | TradingView, NinjaTrader | ★★★★★ |
| Footprint charts | ATAS | ★★★★★ |
| Volume Profile | ATAS, NinjaTrader | ★★★★★ |
| DOM ladder profesional | NinjaTrader, ATAS | ★★★★★ |
| Order Flow | ATAS | ★★★★☆ |
| Time & Sales | ATAS, NinjaTrader | ★★★★☆ |
| Multi-panel charts | TradingView | ★★★★☆ |
| Chart trading (clickear en chart) | NinjaTrader | ★★★★★ |
| Screener | TradingView | ★★★☆☆ |
| Heatmap de liquidez | ATAS, DeepChart | ★★★☆☆ |

### Análisis Técnico
| Feature | Fuente | Prioridad |
|---------|--------|-----------|
| Indicadores O(1) (SMA, EMA, RSI, etc.) | Todas | ★★★★★ |
| Order Blocks / FVG / iFVG | ATAS, ICT | ★★★★★ |
| Market Structure (BOS/CHoCH) | ICT, Smart Money | ★★★★★ |
| Fibonacci completo | TradingView | ★★★★☆ |
| Drawing tools | TradingView, NinjaTrader | ★★★★★ |
| Pattern recognition IA | DeepChart | ★★★☆☆ |
| Custom indicators engine | TradingView, MT5 | ★★★★☆ |

### Trading & Órdenes
| Feature | Fuente | Prioridad |
|---------|--------|-----------|
| One-click trading | NinjaTrader | ★★★★★ |
| Bracket orders (SL/TP) | NinjaTrader | ★★★★★ |
| OCO / OTO | NinjaTrader | ★★★★☆ |
| Trailing stop | Todas | ★★★★☆ |
| ATM Strategies | NinjaTrader | ★★★★☆ |
| Auto break-even | NinjaTrader | ★★★☆☆ |
| Partial close / Scale out | NinjaTrader | ★★★★☆ |
| Reverse position | NinjaTrader | ★★★☆☆ |
| Pyramid scaling | NinjaTrader | ★★★☆☆ |
| Position size calculator | Todas | ★★★★☆ |

### Automatización & Algoritmia
| Feature | Fuente | Prioridad |
|---------|--------|-----------|
| Motor de scripting | MT5 (MQL), TradingView (Pine) | ★★★★★ |
| Backtesting realista | MT5, NinjaTrader, Freqtrade | ★★★★★ |
| Walk-forward / Optimization | MT5, Freqtrade | ★★★★☆ |
| Estrategias visuales | NinjaTrader | ★★★★☆ |
| Señales externas (webhooks/TradingView) | OpenAlgo, Freqtrade | ★★★☆☆ |
| Hiperparameter optimization | Freqtrade | ★★★☆☆ |

### Gestión de Riesgo
| Feature | Fuente | Prioridad |
|---------|--------|-----------|
| Risk calculator (SL/ATR/% acct) | Freqtrade, NinjaTrader | ★★★★★ |
| Max drawdown limit | Freqtrade | ★★★★☆ |
| Kill switch | NinjaTrader | ★★★★★ |
| Close all positions | Todas | ★★★★★ |
| Position limits | OMS propio | ★★★★☆ |

### UX/Productividad
| Feature | Fuente | Prioridad |
|---------|--------|-----------|
| Multi-window / pop-out charts | NinjaTrader, ATAS | ★★★★★ |
| Multi-monitor soporte | NinjaTrader, ATAS | ★★★★★ |
| Command palette | OpenTerminalUI | ★★★★☆ |
| Hotkeys configurables | NinjaTrader, ATAS | ★★★★★ |
| Watchlists | TradingView | ★★★★☆ |
| Symbol search + favorites | TradingView | ★★★★☆ |
| Price alerts + notifications | TradingView | ★★★★★ |
| Economic calendar | TradingView | ★★★★☆ |
| Session replay | ATAS, NinjaTrader | ★★★★★ |
| Trading journal / diary | TradingView, ATAS | ★★★★☆ |

### Innovación
| Feature | Fuente | Prioridad |
|---------|--------|-----------|
| IA local (Ollama, LLM) | Fincept Terminal | ★★★★☆ |
| Agentes MCP para automatización | Velox (propio) | ★★★★☆ |
| Social features (community trades) | TradingView | ★★☆☆☆ |
| Screenshot compartible | TradingView | ★★★☆☆ |
| Nodos visuales de automatización | Fincept Terminal | ★★★☆☆ |

---

## Stack tecnológico comparado

| Proyecto | Lenguaje | UI | GPU | Scripting | Broker API |
|----------|----------|----|-----|-----------|------------|
| **velox-terminal** | Rust | egui/wgpu | wgpu | Lua (mlua) | FIX + REST |
| Fincept Terminal | C++20 | Qt6 | — | Python | Propietaria |
| Nautilus Trader | Rust + Python | — | — | Python | Multi-broker |
| NinjaTrader | C#/.NET | WPF | DirectX | C# | Multi-broker |
| TradingView | TS/JS | Canvas/WebGL | WebGL | Pine Script | — |
| ATAS | C++ | WPF | DirectX | — | Multi-broker |
| Freqtrade | Python | Vue.js | — | Python | CCXT |

---

*Última actualización: 2026-07-09*
