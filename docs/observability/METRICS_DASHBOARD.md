# Metrics Dashboard — velox-terminal

Métricas de salud del sistema.

---

## Key Metrics

| Metric | Source | Warning | Critical |
|--------|--------|---------|----------|
| Feed latency (p99) | tracing span | > 1ms | > 5ms |
| Feed connected | heartbeat | Disconnected > 5s | Disconnected > 30s |
| Order success rate | OMS | < 95% | < 80% |
| Orders per second | OMS | > 10/s sustained | > 30/s |
| Frame time (p99) | wgpu | > 12ms (60fps) | > 16ms |
| Memory usage | OS | > 1GB | > 2GB |
| CPU usage (avg) | OS | > 50% | > 80% |
| GPU usage | wgpu | > 70% | > 90% |
| Storage write latency | storage | > 100ms | > 500ms |
| Error rate (broker) | broker | > 5% | > 20% |

## Dashboard Layout

```
┌─────────────────────────────────────────────────────┐
│  CONNECTION STATUS                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │ Feed     │ │ Broker   │ │ Data Sv  │            │
│  │ 🟢 Live  │ │ 🟢 Conn  │ │ 🟢 OK    │            │
│  └──────────┘ └──────────┘ └──────────┘            │
├─────────────────────────────────────────────────────┤
│  LATENCY (p50/p99)                                  │
│  ┌────────────────────────────────────────────────┐ │
│  │ Feed:   120μs / 450μs    ████████░░░░░░░░░░░░  │ │
│  │ Render: 4.2ms / 8.1ms   ███░░░░░░░░░░░░░░░░░  │ │
│  │ OMS:    1.1ms / 3.5ms   ██░░░░░░░░░░░░░░░░░░  │ │
│  └────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────┤
│  ACTIVITY                                           │
│  │ Ticks/s: 2,450  Orders/h: 12  Positions: 3     │
├─────────────────────────────────────────────────────┤
│  ALERTS                                             │
│  │ ✅ No active alerts                              │
└─────────────────────────────────────────────────────┘
```

## Alert Rules

| Condition | Severity | Action |
|-----------|----------|--------|
| Feed disconnected > 5s during active hours | CRITICAL | Notify user, block new orders |
| Feed disconnected > 30s any time | HIGH | Notify user, attempt reconnect |
| Order reject rate > 20% in 5min | CRITICAL | Block orders, notify user |
| Frame time > 30ms for 10 consecutive frames | WARN | Log trace, reduce render quality |
| Memory > 1.5GB | WARN | Log, suggest restart |
| Memory > 2GB | HIGH | Force GC, suggest restart |
