# Backtesting — velox-terminal

Motor de simulación histórica.

---

## Design Principles

1. **Reusar lógica en vivo**: El backtester ejecuta el mismo código de indicadores y estrategia que corre en producción. Nunca duplicar lógica.
2. **Realismo**: Slippage, comisiones y latencia son configurables y obligatorios.
3. **Determinismo**: Misma entrada → misma salida. Resultados reproducibles.
4. **Parallelismo**: Corridas multi-símbolo y walk-forward analysis en paralelo vía rayon.

---

## Architecture

```
[crates/storage] ── batch read historical data
         │
         ▼
[Loop: for each tick in chronological order]
         │
         ├── [crates/indicators] update(streaming mode)
         │
         ├── [Strategy Logic] evaluate(signal) → order?
         │
         ├── [Simulated OMS] apply(order) → fill
         │     └── slippage + commission model
         │
         └── [Metrics] accumulate(P&L, trades)
```

## Slippage Model

```rust
enum SlippageModel {
    /// Fixed per-lot slippage (e.g., $0.01 per share)
    Fixed(f64),

    /// Percentage of entry price (e.g., 0.05%)
    Percentage(f64),

    /// Based on volume vs average volume at that price level
    VolumeBased {
        avg_volume_per_level: f64,
        slippage_per_std_dev: f64,
    },

    /// No slippage (WARNING: for testing only, not for serious backtesting)
    None,
}
```

Default: `VolumeBased` con parámetros realistas de mercado.

## Commission Model

```rust
struct CommissionModel {
    per_trade: f64,           // Fixed commission per trade (e.g., $2.50)
    per_share: f64,           // Per-share commission (e.g., $0.0035)
    percentage: f64,          // Percentage of notional (e.g., 0.001 for 0.1%)
    min_commission: f64,      // Minimum per trade
    max_commission: f64,      // Maximum per trade
}
```

## Performance Metrics

| Metric | Formula | Purpose |
|--------|---------|---------|
| Sharpe Ratio | (R_p - R_f) / σ_p | Risk-adjusted return |
| Sortino Ratio | (R_p - R_f) / σ_downside | Downside risk-adjusted return |
| Max Drawdown | max(peak - trough) | Maximum peak-to-trough decline |
| Win Rate | wins / total_trades | Percentage of profitable trades |
| Profit Factor | gross_profit / gross_loss | Ratio of winning to losing |
| Calmar Ratio | annual_return / max_drawdown | Return vs risk |
| Average Trade | total_pnl / total_trades | Average P&L per trade |
| Expectancy | (win_rate * avg_win) - (loss_rate * avg_loss) | Expected value per trade |

## Walk-Forward Analysis

```
┌─────────────────────────────────────────────────────────┐
│                     Full Dataset                          │
├─────────────────────┬───────────────────┬───────────────┤
│    In-Sample 1      │   Out-of-Sample 1 │                │
│    (optimize params)│   (validate)      │                │
├─────────────────────┼───────────────────┤                │
│                     │ In-Sample 2       │ OOS 2          │
├─────────────────────┼───────────────────┼───────────────┤
│                     │                   │ In-Sample 3... │
└─────────────────────┴───────────────────┴───────────────┘
```

- **OBLIGATORIO**: Walk-forward analysis. No aceptar resultados de backtest sin WFA.
- **Anchura típica**: 70% in-sample, 30% out-of-sample.
- **Pasos**: 4-6 ventanas.

## Overfitting Detection

| Signal | Warning | Action |
|--------|---------|--------|
| Sharpe > 5.0 | Probable overfitting | Requerir WFA con datos out-of-sample |
| WFA OOS Sharpe < 0.5 * IS Sharpe | Performance no se mantiene | Rechazar estrategia |
| Parámetros optimizados con < 100 trades | Muestra estadística insuficiente | Requerir más datos |
| Strategy con > 5 parámetros optimizados en grid search | Alta dimensionalidad | Reducir parámetros o usar regularization |
