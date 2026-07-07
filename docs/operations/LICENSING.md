# Licensing — velox-terminal

Sistema de licencias para distribución comercial.

---

## Edition Tiers

| Feature | Community | Pro | Enterprise |
|---------|-----------|-----|------------|
| Charting (1 chart) | ✓ | ✓ | ✓ |
| Charting (unlimited) | ✗ | ✓ | ✓ |
| Indicators (basic) | ✓ | ✓ | ✓ |
| Indicators (advanced) | ✗ | ✓ | ✓ |
| DOM Ladder | ✗ | ✓ | ✓ |
| Order Entry | ✓ | ✓ | ✓ |
| OMS (real trading) | ✗ | ✓ | ✓ |
| OMS (paper trading) | ✓ | ✓ | ✓ |
| Multi-broker | ✗ | 1 broker | Unlimited |
| Algo Scripting | ✗ | ✓ | ✓ |
| Backtesting | ✗ | ✓ | ✓ |
| Multi-monitor | ✗ | ✗ | ✓ |
| API Access | ✗ | ✗ | ✓ |
| Priority Support | ✗ | Email | Phone + Slack |

## License File

El archivo de licencia se almacena en `~/.config/velox-terminal/license.key`, cifrado con clave específica del hardware:

```rust
struct License {
    edition: Edition,            // Community | Pro | Enterprise
    licensee: String,            // Name or company
    email: String,
    hw_fingerprint: [u8; 32],   // SHA-256 of hardware signature
    issued_at: chrono::DateTime<Utc>,
    expires_at: Option<chrono::DateTime<Utc>>,
    features: Vec<Feature>,
    signature: Vec<u8>,          // RSA-4096 signature
}
```

## Hardware Fingerprint

```rust
fn generate_hw_fingerprint() -> [u8; 32] {
    let mut hasher = Sha256::new();
    // Components (no PII)
    hasher.update(get_mac_address());    // primary NIC
    hasher.update(get_cpu_id());         // CPU serial
    hasher.update(get_machine_id());     // /etc/machine-id or equivalent
    hasher.finalize().into()
}
```

## Validation Flow

```
1. Launch → check license.key exists
2. Decrypt + verify RSA signature
3. Compare hw_fingerprint against current hardware
4. Check expiration date
5. Check feature flags against requested features
6. If valid → proceed
7. If invalid → show activation dialog
```

## Activation Dialog

Para usuarios sin licencia válida, mostrar diálogo de activación:

```
┌─ Activate velox-terminal ─────────────────┐
│                                            │
│  Enter License Key:                        │
│  ┌──────────────────────────────────────┐  │
│  │ XXXXXX-XXXXXX-XXXXXX-XXXXXX         │  │
│  └──────────────────────────────────────┘  │
│                                            │
│  [ACTIVATE]  [START TRIAL (14 days)]       │
│                                            │
│  Or purchase at https://velox-terminal.com │
└────────────────────────────────────────────┘
```

## Non-Blocking Enforcement

- Nunca interrumpir una sesión de trading activa por expiración de licencia
- Si la licencia expira durante una sesión, mostrar warning y permitir continuar
- Al cerrar la sesión, pedir reactivación antes de la próxima apertura
- Grace period de 24 horas para renovación

## Trial Mode

- 14 días desde el primer uso
- Full Pro features durante el trial
- HW-fingerprinted para evitar renewals
- Cuenta regresiva visible en status bar

## Security

| Aspect | Implementation |
|--------|----------------|
| License file | AES-256-GCM encrypted |
| Signature | RSA-4096, server-side private key |
| Validation | On launch + every 24h while running |
| Offline | Works offline, checks cache expiry |
| Tampering | HW fingerprint mismatch → invalid license |
| Revocation | Local revocation list updated hourly |
