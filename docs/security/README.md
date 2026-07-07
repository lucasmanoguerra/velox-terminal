# Security Documentation — velox-terminal

Seguridad de credenciales, threat model, auditoría de dependencias.

## Documents

| File | Purpose | Read when |
|------|---------|-----------|
| `CREDENTIAL_MANAGEMENT.md` | Almacenamiento seguro de API keys y tokens | Setting up broker connections |
| `THREAT_MODEL.md` | Identificación de vectores de ataque | Security review, threat modeling |
| `AUDIT_LOG.md` | Log de auditoría de operaciones | Compliance, security investigation |

## Recommended Loading Order

1. `CREDENTIAL_MANAGEMENT.md` — how to store secrets
2. `THREAT_MODEL.md` — what we're protecting against
3. `AUDIT_LOG.md` — how we track operations
