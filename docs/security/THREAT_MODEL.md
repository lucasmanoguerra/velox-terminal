# Threat Model — velox-terminal

Identificación de vectores de ataque y mitigaciones.

---

## Threat Matrix

| # | Threat | Risk | Mitigation |
|---|--------|------|------------|
| T1 | Credential exfiltration via log | CRITICAL | Nunca loggear credenciales, SecretString con zeroize |
| T2 | Credential exfiltration via crash dump | HIGH | SecretString con zeroize, Sanitize crash reports |
| T3 | Reverse engineering of binary | MEDIUM | Rust compilation obfusicates, pero asumir que es posible |
| T4 | Manipulation of market data feed | MEDIUM | Validar secuencia de timestamps, detectar gaps anómalos |
| T5 | Injection via scripting engine | HIGH | Sandboxing estricto (mlua), sin acceso a red/disco, timeout forzoso |
| T6 | Replay attack on broker connection | MEDIUM | ClOrdID incrementales, timestamp en mensajes, seq numbers FIX |
| T7 | Unauthorized order modification | MEDIUM | Risk Management fail-safe, confirmación de orden antes de enviar |
| T8 | Man-in-the-middle on broker comms | HIGH | rustls/TLS obligatorio en todas las conexiones |
| T9 | Local privilege escalation via IPC | LOW | IPC solo entre threads del mismo proceso, no sockets |
| T10 | Dependency vulnerability (supply chain) | MEDIUM | cargo-audit semanal, dependabot, revisión de breaking changes |

## Attack Surface

```
┌──────────────────────────────────────────┐
│              velox-terminal               │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │  Input Surface                     │  │
│  │  • Hotkeys (keyboard events)       │  │ ← T9
│  │  • Mouse events (chart clicking)   │  │
│  │  • Scripting engine (user code)    │  │ ← T5
│  │  • Config files                    │  │ ← T1
│  └────────────────────────────────────┘  │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │  Network Surface                    │  │
│  │  • Broker FIX/TLS connection        │  │ ← T6, T7, T8
│  │  • Market data WebSocket/WSS       │  │ ← T4, T8
│  │  • License validation server       │  │
│  └────────────────────────────────────┘  │
│                                          │
│  ┌────────────────────────────────────┐  │
│  │  Storage Surface                    │  │
│  │  • Time-series DB (disk)           │  │
│  │  • Keyring (OS credential store)   │  │ ← T1, T2
│  │  • Config files                    │  │
│  └────────────────────────────────────┘  │
└──────────────────────────────────────────┘
```

## Security Review Cadence

| Review | Frequency | Owner |
|--------|-----------|-------|
| cargo-audit | Weekly (automated) | dependency-maint |
| Dependency review | Monthly | dependency-maint |
| Unsafe block audit | Per-release | seguridad |
| Scripting sandbox review | Per-release | scripting-engine |
| Full threat model review | Quarterly | lead |
