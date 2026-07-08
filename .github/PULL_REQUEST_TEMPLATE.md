## Description

Please provide a summary of the changes and the problem they solve. Include relevant motivation and context.

Fixes #(issue) | Closes #(issue) | Refs #(issue)

---

## Type of Change

Select the type(s) that apply:

- [ ] **feat** — New feature (non-breaking)
- [ ] **fix** — Bug fix (non-breaking)
- [ ] **refactor** — Code change without functional impact
- [ ] **perf** — Performance improvement
- [ ] **test** — Adding or updating tests
- [ ] **docs** — Documentation only
- [ ] **style** — Formatting, linting, whitespace
- [ ] **chore** — Maintenance, CI, dependencies, build

---

## Testing

Describe the testing you performed:

- [ ] `cargo build --workspace` — no build errors
- [ ] `cargo test --workspace` — all tests pass
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings
- [ ] `cargo fmt --check` — formatting is correct
- [ ] `cargo deny check advisories` — no known vulnerabilities introduced
- [ ] **New unit tests added** — covering the new/changed functionality
- [ ] **Property-based tests added** — if the change affects OMS, Risk, or financial logic
- [ ] **Manual testing** — describe what you tested manually

If your change affects OMS or Risk: have you run property-based tests with increased iterations?

```bash
PROPTEST_CASES=10000 cargo test -p velox-oms
PROPTEST_CASES=10000 cargo test -p velox-risk
```

---

## Checklist

- [ ] My code follows the project's code standards (`cargo fmt` + `cargo clippy` pass).
- [ ] No file exceeds 200 lines of logic (excluding imports and module docs).
- [ ] I have added doc comments to all new public items.
- [ ] I have updated relevant documentation (README, ADRs, module-level docs).
- [ ] My commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
- [ ] My branch is based on `develop` and is up to date.
- [ ] I have rebased my branch (not merged) to avoid unnecessary merge commits.
- [ ] If this change requires a breaking change, I have noted it in the PR description.

---

## Screenshots (if applicable)

If your change affects the UI, please include screenshots showing the before and after.

---

## Security Implications

If your change touches any of the following areas, please describe the security review performed:

- [ ] Credential handling
- [ ] Unsafe code
- [ ] Network I/O (WebSocket, REST)
- [ ] User input / configuration parsing
- [ ] OMS / Risk financial logic

**Any new `unsafe` blocks must include a `// SAFETY:` comment.** Unsafe blocks without justification will not be merged.

---

## Additional Notes

Any additional information for reviewers. Architecture decisions should be recorded as ADRs in `docs/adrs/`.
