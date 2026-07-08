# Security Policy

## Supported Versions

We currently provide security updates for the following versions:

| Version | Supported          |
|---------|-------------------|
| >= 0.1  | ✅ Yes            |
| < 0.1   | ❌ No (pre-release) |

Once we reach a stable `1.0` release, we will adopt a formal LTS policy. Until then, please always use the latest release.

---

## Reporting a Vulnerability

### Private Disclosure

If you discover a security vulnerability in velox-terminal, please report it privately **before** creating a public GitHub issue. We will acknowledge receipt within **48 hours** and provide a timeline for a fix.

**Contact:** `lucasmanoguerra@example.com`

If you have a PGP key, please include your public key in the initial email so we can communicate securely. If the maintainer's PGP key is requested, one will be provided in response.

### What to Include

To help us triage and fix the issue quickly, please include:

- **Type of vulnerability** (e.g., credential exposure, unsafe code, denial of service, data integrity)
- **Affected component** (which crate or module)
- **Steps to reproduce** — minimal, complete, and reproducible
- **Proof of concept** — code snippet or payload if applicable
- **Impact** — what an attacker could achieve
- **Suggested fix** (optional but appreciated)

### Response Targets

| Event                  | Target  |
|------------------------|---------|
| Initial acknowledgment | 48 hours |
| Triage & assessment    | 5 business days |
| Fix (critical)         | 7 days |
| Fix (high)             | 14 days |
| Fix (medium/low)       | Next release |
| Public disclosure      | After fix is released |

If the issue is confirmed, we will release a security patch and credit the reporter (unless they prefer anonymity).

---

## Scope

The following areas are in scope for security review:

### In Scope

- **Credential handling**: API keys, secrets, authentication tokens. Never stored in plaintext. Never logged.
- **OMS correctness**: Order state machine logic must prevent overfills, double-fills, and invalid state transitions. Financial correctness is a security concern.
- **Risk management**: Position limits, circuit breakers, pre-trade validation. Bypassing risk checks can lead to financial loss.
- **Unsafe code**: Every `unsafe` block must be justified with a `// SAFETY:` comment. Code reviews must verify soundness.
- **Network I/O**: WebSocket connections, TLS configuration, message parsing. Injection attacks or message spoofing.
- **Scripting sandbox** (future): Lua scripting engine boundary — sandbox escape attempts.
- **Data integrity**: Market data, order history, P&L calculations must not be corruptible via external input.

### Out of Scope

- **Dependencies with known CVEs**: Tracked via `cargo-deny` in CI, not through this disclosure process.
- **Feature requests disguised as vulnerabilities**: Please open a regular issue for feature requests.
- **Theoretical attacks without a practical reproduction**: We prioritize actionable reports.

---

## Security in CI

We run automated security checks in our CI pipeline:

| Check                  | Tool          | What it detects                          |
|------------------------|---------------|------------------------------------------|
| Advisory check         | `cargo deny`  | Dependencies with known vulnerabilities  |
| License compliance     | `cargo deny`  | Unallowable license types                |
| Clippy security lints  | `cargo clippy`| Unsafe code, panics, etc. (`-D warnings`)|

All pull requests must pass these checks before merging.

---

## Best Practices for Contributors

1. **Never commit secrets.** API keys, passwords, and tokens should never appear in the repository. Use environment variables, keyring, or a `.env` file (gitignored).
2. **Never log credentials.** No `tracing::info!` or `println!` with credentials at any log level.
3. **Prefer `?` over `unwrap()` / `expect()`.** Unwrapping on user-controlled input is a denial-of-service vector.
4. **Document unsafe code** with `// SAFETY:` justification. All unsafe blocks are flagged in code review.
5. **Use checked arithmetic** in OMS/Risk components. The `release-safe` profile enables overflow checks.
6. **Validate all external input.** Parse, then validate. Do not trust exchange messages, user input, or configuration files.
7. **Keep dependencies minimal.** Each new dependency is a potential vulnerability. Justify non-obvious dependencies in the PR description.

---

## Vulnerability Disclosure Timeline

We follow a **coordinated disclosure** model:

1. Reporter submits vulnerability privately.
2. Maintainer acknowledges within 48 hours.
3. Maintainer triages and develops fix.
4. Fix is released (new version tag).
5. Public disclosure after fix is available.

We aim to disclose vulnerabilities publicly within **30 days** of the fix release, unless delayed for a valid reason (e.g., coordinated disclosure with a dependency maintainer).

---

## Recognition

We maintain a security acknowledgements section in our release notes. Reporters who prefer anonymity will be credited as "a security researcher" unless they specify otherwise.

---

## Questions

For questions about this security policy, please open a **GitHub Discussion** (not an issue) or contact the maintainers.
