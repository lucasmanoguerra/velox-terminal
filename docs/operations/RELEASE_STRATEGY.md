# Release Strategy — velox-terminal

Estrategia de releases y rollout.

---

## Versioning (SemVer)

| Component | MAJOR | MINOR | PATCH |
|-----------|-------|-------|-------|
| Breaking API change (public traits, config format) | ✓ | | |
| Breaking data format (storage schema change) | ✓ | | |
| New feature (indicators, order types, brokers) | | ✓ | |
| UI/UX changes (no API break) | | ✓ | |
| Bug fixes | | | ✓ |
| Performance improvements (no API change) | | | ✓ |
| Dependency updates (non-breaking) | | | ✓ |

## Release Cadence

| Release Type | Cadence | Examples |
|-------------|---------|----------|
| PATCH | As needed (bugfixes) | Security fix, critical bug |
| MINOR | Monthly | New indicators, new features |
| MAJOR | Quarterly | Breaking changes, architecture changes |

## Rollout Strategy

```
1. Build → CI/CD Pipeline (all platforms)
2. Internal testing → Dev team + simulado
3. Canary (5-10% users) → 24h observation window
   - Monitor: crash rate, error rate, latency, user feedback
   - If crash rate > 0.1% or error rate > 1% → abort rollout
4. Staged rollout (25% → 50% → 100%) → 6h per stage
5. Full release
```

## Rollback Plan

```
1. Identify issue → classification (CRITICAL/HIGH/MEDIUM/LOW)
2. If CRITICAL or HIGH → immediately:
   a. Revert to previous version via feature flag or binary rollback
   b. Notify affected users
   c. Disable affected feature if rollback not possible
3. Post-mortem → root cause analysis → prevent recurrence
```

## Changelog

- Changelog dual: usuario (features visibles) + técnico (cambios de API, refactors)
- Formato: [Keep a Changelog](https://keepachangelog.com/)
- Secciones: Added, Changed, Deprecated, Removed, Fixed, Security
