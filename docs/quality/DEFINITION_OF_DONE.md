# Definition of Done — velox-terminal

Criterios que todo cambio debe cumplir antes de considerarse terminado.

---

## For All Changes

- [ ] Code compiles without warnings (`cargo build`)
- [ ] Clippy passes with `-D warnings`
- [ ] `cargo test` passes (all tests)
- [ ] `cargo fmt --check` passes
- [ ] Code reviewed by at least one other person/agent
- [ ] No `unsafe` without documented `// SAFETY:` invariant
- [ ] No `unwrap()` or `expect()` in new code (use `Result` or proper error handling)
- [ ] New public items have doc comments
- [ ] Error paths are handled (no silent failures)
- [ ] **Hexagonal compliance**: Domain crates don't import infrastructure (tokio, wgpu, egui, crossbeam)
- [ ] **File size**: New files < 200 lines (excluding imports and module doc comments)
- [ ] **Hexagonal exceptions**: Any hot-path exemption documented with `// HEXAGONAL-EXEMPT: <reason>`
- [ ] **Conventional Commit**: `git log --oneline -1` matches `<type>(<scope>): <description>` format

## For OMS / Risk / P&L Changes

- [ ] Property-based tests (proptest) cover state transitions and invariants
- [ ] Integration test with mock broker covers the new path
- [ ] Manual review of the state machine logic
- [ ] The corresponding agent (`oms`, `risk-management`, `qa-financiero`) has been consulted
- [ ] Audit trail is verified to capture the new behavior

## For UI / Charting Changes

- [ ] Visual behavior verified at 60fps and 144fps
- [ ] Frame time budgets not exceeded (no regressions)
- [ ] Hotkeys work as defined
- [ ] Keyboard navigation works (tab order, focus)
- [ ] High-DPI / Retina displays render correctly
- [ ] Both bullish and bearish states render correct colors

## For Broker Connectivity Changes

- [ ] Mock broker tests cover: connect, disconnect, reconnect, submit, cancel, modify
- [ ] Idempotency verified (duplicate messages handled correctly)
- [ ] Credential handling reviewed (no plaintext secrets in logs/config)
- [ ] Connection timeout and recovery scenarios tested
- [ ] Backoff strategy verified (not hammering broker on reconnect)

## For Documentation Changes

- [ ] `docs/README.md` index updated if new files added
- [ ] Section README table updated (`| File | Purpose | Read when |`)
- [ ] ADRs referenced if architectural decision was made
- [ ] `AI_MEMORY.md` updated with key learnings

## Release Gate

- [ ] All of the above
- [ ] Changelog updated
- [ ] Version bumped according to SemVer
- [ ] CI/CD pipeline green on all 3 platforms
- [ ] cargo-audit passes (no known vulnerabilities)
- [ ] Manual smoke test on target platform
