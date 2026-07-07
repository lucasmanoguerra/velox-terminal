# Data Retention — velox-terminal

Políticas de retención de datos.

---

## Retention Schedule

| Data Type | Retention | Storage Tier | Purge Strategy | Legal Basis |
|-----------|-----------|-------------|----------------|-------------|
| Audit log (trading) | 5 years | Cold (compressed) | Purge oldest partition | MiFID II Art. 16 |
| Audit log (auth) | 2 years | Cold | Purge after 2 years | GDPR |
| Tick data | 6 months | Hot (fast access) | Archive to cold, then purge | Operational |
| Daily OHLCV | 5 years | Cold | Purge oldest file | Backtesting + Compliance |
| Account snapshots | 2 years | Cold | Purge after 2 years | Compliance |
| Logs (system) | 90 days | Hot | Rotated daily, purge after 90d | Operational |
| Cache data | Session | Memory | Deleted on close | None |

## Implementation

- Particionado por fecha (`storage/symbol/YYYY/MM/DD/`)
- Compresión Zstd para datos cold (ratio ~5:1)
- Purga automática vía job diario que verifica fecha de partición
- Antes de purgar: checksum de integridad, backup final

## Data Export

Los usuarios deben poder exportar sus datos antes de la purga:

```rust
enum ExportFormat {
    CSV,
    Parquet,
    JSON,
}

fn export_trading_data(
    symbol: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    format: ExportFormat,
) -> Result<PathBuf, StorageError>;
```
