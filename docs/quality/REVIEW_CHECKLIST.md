# Review Checklist — velox-terminal

Checklist de revisión de PRs multi-gate.

---

## Gate 1: Correctness

- [ ] Does the code do what it says?
- [ ] Are edge cases handled (empty data, extreme values, timeouts)?
- [ ] Are error paths tested?
- [ ] Does the OMS state machine handle all transitions correctly?
- [ ] Are idempotency keys respected?
- [ ] Is backpressure correctly configured (bounded channels, explicit drop policy)?

## Gate 2: Safety

- [ ] No `unsafe` without documented invariants
- [ ] No `unwrap()` in production paths
- [ ] Risk validations are fail-safe (not fail-open)
- [ ] Credentials are not logged
- [ ] Thread safety: shared state is properly synchronized

## Gate 3: Performance

- [ ] Hot paths have no unnecessary allocations
- [ ] Channel sizes are bounded (backpressure policy documented)
- [ ] GPU: no per-element draw calls (geometry is instanced/batched)
- [ ] Async: blocking operations are not on async runtime
- [ ] **Zero-copy**: Tick/quote paths use `bytemuck::Pod` or `rkyv`, not `serde_json`
- [ ] **Profiling**: Performance changes include before/after benchmark data
- [ ] **Batching**: Hot loops use batch operations (e.g. `pop_n`) not element-at-a-time

## Gate 4: Maintainability

- [ ] Code is readable and follows project conventions
- [ ] Complex logic has explanatory comments
- [ ] Public API is documented
- [ ] No dead code (unused functions, imports)
- [ ] No TODOs left in code (they should be in issue tracker)
- [ ] Allocator: domain core uses `System`, adapters may use `mimalloc` with feature flag
- [ ] **Atomization**: New files < 200 lines. If modifying an existing file > 200 lines, consider atomizing it into smaller files by responsibility (see `docs/quality/ATOMIZED_FILES.md`)

## Gate 5: Testing

- [ ] New features have tests
- [ ] OMS/Risk changes have property-based tests
- [ ] Tests are deterministic (no time-dependent or flaky tests)
- [ ] Test coverage is adequate (especially for financial logic)
- [ ] Mock/real separation is clear (never test with real broker credentials)
- [ ] **Fuzzing**: Parser/protocol changes have fuzz targets passing 30s+ CI run
- [ ] **Benchmarks**: Hot path changes include criterion benchmark results

## Gate 6: Documentation

- [ ] ADRs updated if architectural decision changed
- [ ] AI_MEMORY.md updated with key learnings
- [ ] Section README tables updated if new files added
- [ ] Changelog updated (if user-facing change)
- [ ] Backpressure policy documented in relevant pipeline docs
