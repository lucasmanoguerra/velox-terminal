---
name: Bug report
about: Report a reproducible bug in velox-terminal
title: ''
labels: bug
assignees: ''
---

## Description

A clear and concise description of the bug. What is happening, and what should be happening instead?

---

## Reproduction Steps

Minimal, complete, and reproducible steps:

1. Go to '...'
2. Click on '...'
3. Scroll down to '...'
4. See error

Include a minimal code snippet if applicable:

```rust
// Add a code snippet that reproduces the issue
```

---

## Expected Behavior

What did you expect to happen?

---

## Actual Behavior

What actually happened? Include error messages, panics, or unexpected output.

---

## Environment

| Variable          | Value                                      |
|-------------------|--------------------------------------------|
| **OS**            | e.g., Windows 11, macOS 14.5, Ubuntu 24.04 |
| **Rust version**  | `rustc --version` output                   |
| **GPU**           | e.g., NVIDIA RTX 4080, Apple M3, Intel UHD |
| **Backend**       | e.g., Vulkan 1.3, DirectX 12, Metal        |
| **Terminal**      | e.g., wezterm, iTerm2, Windows Terminal    |
| **Version**       | velox-terminal version or commit SHA       |

Get your Rust version with:

```bash
rustc --version
```

---

## Logs

Attach or paste relevant logs. Enable verbose logging with:

```bash
RUST_LOG=info cargo run
```

For debug-level logs:

```bash
RUST_LOG=debug cargo run
```

Filter logs by crate if the issue is in a specific component:

```bash
RUST_LOG=velox_oms=debug,velox_md=info cargo run
```

> **Note:** Review logs for any sensitive information (API keys, tokens, account IDs) before posting.

---

## Severity

Select one:

- [ ] **Critical** — Application crash, data loss, incorrect financial calculation, security vulnerability
- [ ] **High** — Major feature broken, no workaround
- [ ] **Medium** — Feature broken, has a workaround
- [ ] **Low** — Cosmetic issue, minor inconvenience

---

## Additional Context

- Does this happen consistently or intermittently?
- Does it happen with specific symbols, timeframes, or configurations?
- Screenshots or screen recordings are helpful.
- Does the issue reproduce with the latest commit on `develop`?

---

## Possible Fix (optional)

If you have an idea of what might be causing the issue, describe it here. You don't need to have a complete fix — any clues are appreciated.
