# Review Checklist — velox-terminal

Checklist de revisión de PRs multi-gate.

---

## Gate 1: Correctness

- [ ] Does the code do what it says?
- [ ] Are edge cases handled (empty data, extreme values, timeouts)?
- [ ] Are error paths tested?
- [ ] Does the OMS state machine handle all transitions correctly?
- [ ] Are idempotency keys respected?

## Gate 2: Safety

- [ ] No `unsafe` without documented invariants
- [ ] No `unwrap()` in production paths
- [ ] Risk validations are fail-safe (not fail-open)
- [ ] Credentials are not logged
- [ ] Thread safety: shared state is properly synchronized

## Gate 3: Performance

- [ ] Hot paths have no unnecessary allocations
- [ ] Channel sizes are bounded (backpressure)
- [ ] GPU: no per-element draw calls (geometry is instanced/batched)
- [ ] Async: blocking operations are not on async runtime

## Gate 4: Maintainability

- [ ] Code is readable and follows project conventions
- [ ] Complex logic has explanatory comments
- [ ] Public API is documented
- [ ] No dead code (unused functions, imports)
- [ ] No TODOs left in code (they should be in issue tracker)

## Gate 5: Testing

- [ ] New features have tests
- [ ] OMS/Risk changes have property-based tests
- [ ] Tests are deterministic (no time-dependent or flaky tests)
- [ ] Test coverage is adequate (especially for financial logic)
- [ ] Mock/real separation is clear (never test with real broker credentials)

## Gate 6: Documentation

- [ ] ADRs updated if architectural decision changed
- [ ] AI_MEMORY.md updated with key learnings
- [ ] Section README tables updated if new files added
- [ ] Changelog updated (if user-facing change)
