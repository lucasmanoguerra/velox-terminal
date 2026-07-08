# Contributing to velox-terminal

First off, thank you for considering contributing to velox-terminal. We welcome contributions from the community — whether it's a bug report, a feature request, a documentation fix, or a pull request.

Please read and follow our guidelines to make the process smooth and effective.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Project Architecture](#project-architecture)
- [Code Standards](#code-standards)
- [Commit Conventions](#commit-conventions)
- [Branch Workflow](#branch-workflow)
- [Pull Request Process](#pull-request-process)
- [Testing](#testing)
- [Security](#security)
- [Adding a New Exchange Adapter](#adding-a-new-exchange-adapter)
- [Questions & Support](#questions--support)

---

## Code of Conduct

All contributors must abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Be respectful, inclusive, and constructive.

---

## Getting Started

1. **Fork the repository** on GitHub.
2. **Clone your fork:**
   ```bash
   git clone https://github.com/<your-username>/velox-terminal.git
   cd velox-terminal
   ```
3. **Add the upstream remote:**
   ```bash
   git remote add upstream https://github.com/lucasmanoguerra/velox-terminal.git
   ```
4. **Create a branch** from `develop`:
   ```bash
   git fetch upstream
   git checkout -b feat/my-feature upstream/develop
   ```
5. **Make your changes**, following the standards below.
6. **Open a pull request** via `gh`:
   ```bash
   gh pr create --repo lucasmanoguerra/velox-terminal --base develop
   ```

---

## Development Environment

### Prerequisites

| Tool  | Version | Notes |
|-------|---------|-------|
| Rust  | 2024 edition | MSRV defined by workspace `rust-version`; use `rustup update` |
| wgpu  | 24       | Requires Vulkan (Linux), DirectX 12 (Windows), or Metal (macOS) |
| egui  | 0.31     | Immediate-mode UI framework |
| Node.js | (optional) | Only if contributing to documentation tooling |

### Setup

```bash
# Install Rust toolchain
rustup default stable
rustup component add clippy rustfmt

# Build the workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Install security auditing tool
cargo install cargo-deny
```

### GPU Requirements

wgpu 24 requires a GPU with Vulkan (Linux), DirectX 12 (Windows 10+), or Metal (macOS 10.15+). There is no software fallback. If you encounter GPU-related errors, ensure your graphics drivers are up to date.

### Editor Configuration

We recommend `rust-analyzer` for IDE support. A `.vscode/settings.json` with the following is provided in the repository:

```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.cargo.features": "all"
}
```

---

## Project Architecture

velox-terminal follows a **hexagonal (ports & adapters) architecture** organized as a Cargo workspace:

```
velox-terminal/
├── crates/
│   ├── velox-core/         # Domain primitives (Order, Tick, Quote, Candle, Side)
│   ├── velox-md/           # Market data: ring buffer, candle aggregation
│   ├── velox-indicators/   # Technical indicators (SMA, EMA, RSI, MACD, etc.)
│   ├── velox-oms/          # Order management system & state machine
│   ├── velox-risk/         # Pre-trade risk validation, circuit breaker
│   ├── velox-broker/       # Broker abstraction (BrokerClient trait)
│   ├── velox-broker-fix/   # FIX protocol adapter (not yet implemented)
│   ├── velox-exchange/     # Exchange feed connectors (ports + Binance adapter)
│   ├── velox-storage/      # Local persistence (time-series DB)
│   ├── velox-backtest/     # Backtesting engine
│   ├── velox-scripting/    # Lua scripting sandbox
│   ├── velox-gpu/          # WGPU shader modules and GPU abstractions
│   ├── velox-chart/        # Chart rendering engine with wgpu
│   ├── velox-ui/           # egui panels, theme, AppState
│   └── velox-terminal/     # Binary entry point, event loop orchestration
└── docs/
    ├── adrs/               # Architecture Decision Records
    ├── architecture/       # System design docs
    ├── trading/            # Trading domain docs
    ├── gpu/                # Rendering pipeline docs
    └── ...
```

### Hexagonal Architecture Pattern

- **Ports** are Rust `trait`s defined at the boundary of a domain crate.
  - Example: `velox-exchange::ExchangeFeed` — the port for market data feeds.
  - Example: `velox-broker::BrokerClient` — the port for order execution.
- **Adapters** are concrete implementations of those traits.
  - Example: `velox-exchange::binance::BinanceFeed` implements `ExchangeFeed`.
  - Example: `velox-broker::mock::MockBroker` implements `BrokerClient`.

To add a new adapter (e.g., Kraken feed), implement the relevant trait without modifying the port. This keeps domain logic decoupled from infrastructure.

---

## Code Standards

### Formatting & Linting

All code must pass these checks before committing:

```bash
cargo fmt --check        # No formatting diffs
cargo clippy --workspace --all-targets -- -D warnings  # Zero warnings
git diff --check         # No whitespace errors
```

We enforce `-D warnings` in CI (`RUSTFLAGS: "-D warnings"`). Clippy warnings are treated as errors — do not use `#[allow(...)]` unless explicitly justified with a comment.

### File Structure

- **Source files should not exceed 200 lines** (excluding imports and module-level doc comments).
- If a file exceeds 200 lines of logic, extract the functionality into a submodule.
- Each file should have a single responsibility.

### Unsafe Code

- Every `unsafe` block must be preceded by a `// SAFETY:` comment explaining why it is safe.
- Unsafe blocks without justification will be rejected in code review.
- See `docs/security/UNSAFE_REVIEW.md` for our unsafe code review checklist.

### Documentation

- All public items must have doc comments (`///` or `//!`).
- Use `//` for internal comments only.
- Architecture decisions must be recorded in `docs/adrs/` as markdown ADR files.

### Error Handling

- Use `thiserror` for library error types.
- Use `anyhow` for binary/application-level error handling.
- Avoid `unwrap()` and `expect()` in production code. Prefer `?` operator with proper error propagation.
- The only exception is in test code and in `main()` where panics are acceptable on unrecoverable errors.

### Concurrency

- UI runs on a single thread (winit event loop).
- Network I/O runs on the tokio runtime.
- Hot paths (market data) use `crossbeam` lock-free SPSC channels via `RingBuffer`.
- Shared state between threads must use `Arc<AtomicBool>`, `Arc<Mutex>`, or `dashmap` where appropriate.

---

## Commit Conventions

We use [Conventional Commits](https://www.conventionalcommits.org/) for all commits. This enables automatic changelog generation and semantic versioning.

### Format

```
<type>(<scope>): <short description>

<optional body explaining what and why, not how>

<optional footer>
```

### Types

| Type       | When to use                           | Example                                       |
|------------|---------------------------------------|-----------------------------------------------|
| `feat`     | New feature                           | `feat(oms): add bracket order support`        |
| `fix`      | Bug fix                               | `fix(md): prevent ring buffer overwrap`       |
| `docs`     | Documentation changes                 | `docs: add FIX protocol architecture doc`     |
| `refactor` | Code change without functional impact | `refactor(chart): extract axis renderer`      |
| `test`     | Adding or updating tests              | `test(risk): add proptest for circuit breaker`|
| `chore`    | Maintenance, deps, CI, tooling        | `chore: update wgpu to v24`                   |
| `perf`     | Performance improvement               | `perf(core): replace Vec with SegQueue`       |
| `style`    | Formatting only (`cargo fmt`)         | `style: cargo fmt`                             |

### Scope

The scope should be the crate or module name:
- `(oms)` — Order management
- `(md)` — Market data
- `(chart)` — Charting engine
- `(ui)` — egui panels
- `(core)` — Domain primitives
- `(risk)` — Risk management
- `(exchange)` — Exchange connectors
- `(broker)` — Broker interface
- `(ci)` — CI/CD only

### Body Guidelines

- Explain **what** changed and **why**, not **how** (the code documents the how).
- Use bullet points for lists of changes.
- Reference GitHub issues with `Closes #123` or `Refs #456`.

### Atomic Commits

- **One logical change per commit.** Do not mix formatting with logic, or refactors with features.
- Small commits enable easier code review, bisect, and rollback.

---

## Branch Workflow

| Branch    | Purpose                                      | CI required |
|-----------|----------------------------------------------|-------------|
| `main`    | Production-ready. Always stable, CI green.   | Yes         |
| `develop` | Integration branch for features.             | Yes         |
| `feat/*`  | New features. Branch from `develop`.         | Yes         |
| `fix/*`   | Bug fixes. Branch from `develop`.            | Yes         |
| `docs/*`  | Documentation-only changes.                  | Yes         |

**Rules:**
- All feature and fix branches must be based on `develop`, not `main`.
- Direct pushes to `main` are prohibited — only merge via PR.
- Hotfixes for critical production issues may go directly to `main` with expedited review.
- Keep branches short-lived. Large features should be broken into smaller PRs.

---

## Pull Request Process

### Before Opening a PR

1. Ensure your branch is up to date with `develop`:
   ```bash
   git fetch upstream
   git rebase upstream/develop
   ```
2. Make sure the following pass locally:
   ```bash
   cargo build --workspace
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo fmt --check
   cargo deny check advisories
   ```
3. Write a clear PR description using the [PR template](.github/PULL_REQUEST_TEMPLATE.md).
4. Open the PR using the `gh` CLI:
   ```bash
   gh pr create --repo lucasmanoguerra/velox-terminal --base develop
   ```

### Review Requirements

- **At least one approval** from a maintainer is required before merging.
- All CI checks must pass (build, lint, test, security).
- Reviewers may request changes. Please address them promptly.
- Once approved, the author merges using **Squash and Merge** (for feature branches) or **Rebase and Merge** (for small cleanups).

### What Reviewers Look For

- Correctness: does the code do what it claims?
- Safety: are there any unsafe blocks without justification?
- Performance: are there obvious inefficiencies (e.g., allocations in hot paths)?
- Test coverage: are there tests for new functionality? Are OMS/Risk changes backed by property-based tests?
- Architecture: does the change respect the hexagonal architecture? Are new traits well-designed?
- Documentation: are public items documented? Are ADRs updated if needed?

---

## Testing

### General Requirements

- All new code must include tests.
- Bug fixes must include a regression test that fails before the fix.
- Tests must be deterministic. Avoid time-dependent tests.

### Property-Based Tests

- **OMS (Order Management System)** and **Risk Management** changes **require** property-based tests using [`proptest`](https://docs.rs/proptest/).
- Examples of properties to test:
  - Fill quantities never exceed order quantities.
  - State transitions follow the defined state machine.
  - Duplicate fills are idempotent.
  - Order replacement never changes filled quantity.
- See existing tests in `crates/velox-oms/tests/` and `crates/velox-risk/tests/` for reference.

### Test Commands

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p velox-oms

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name

# Run proptest with more iterations
PROPTEST_CASES=10000 cargo test -p velox-oms
```

### Benchmarks

Performance-sensitive code should include benchmarks using `criterion`. Benchmarks live in `benches/` directories within each crate.

```bash
cargo bench --workspace
```

---

## Security

### Reporting Vulnerabilities

Please report security vulnerabilities to the maintainers via the process described in [SECURITY.md](SECURITY.md). **Do not report security issues in public GitHub issues.**

### CI Security Checks

We run `cargo deny check advisories` in CI to detect known vulnerabilities in dependencies. If your PR introduces a new dependency with a known advisory, the CI will fail.

### Security-Sensitive Areas

The following areas require extra scrutiny and must be reviewed by at least one maintainer:

- **Credential handling**: API keys, secrets, token storage. Never log credentials at any log level.
- **OMS correctness**: Order state machine logic, fill management, risk validation.
- **Unsafe code**: Every `unsafe` block must be justified and reviewed.
- **Network I/O**: WebSocket reconnection logic, TLS configuration, message parsing.
- **Scripting sandbox**: Lua scripting engine boundaries (future).

---

## Adding a New Exchange Adapter

Adding support for a new exchange (e.g., Kraken, Coinbase, Bybit) follows a consistent pattern:

### 1. Implement the `ExchangeFeed` Trait

Create a new module under `crates/velox-exchange/src/` (e.g., `kraken.rs`):

```rust
//! Kraken exchange connector.

use std::sync::Arc;
use velox_core::CoreError;
use velox_md::ring_buffer::RingBuffer;
use crate::ExchangeFeed;

pub struct KrakenFeed {
    // Connection state, API credentials, etc.
}

impl ExchangeFeed for KrakenFeed {
    fn start(&self, ring: Arc<RingBuffer>) -> Result<(), CoreError> {
        // Connect to Kraken WebSocket, spawn reader task
    }

    fn stop(&self) -> Result<(), CoreError> {
        // Graceful disconnect
    }

    fn subscribe(&self, symbol: &str) -> Result<(), CoreError> {
        // Normalize symbol (e.g., "XBT/USD" -> "XBT/USD")
        // Send subscribe message over WebSocket
    }

    fn unsubscribe(&self, symbol: &str) -> Result<(), CoreError> {
        // Send unsubscribe message
    }
}
```

### 2. Register the Adapter

Add a constructor function or builder to `crates/velox-exchange/src/lib.rs`.

### 3. Add Tests

- At minimum: a stream path test, symbol normalization test, and a message parsing test.
- If the exchange has a testnet/sandbox, add integration tests.

### 4. See Existing Reference

- `crates/velox-exchange/src/binance.rs` — full implementation with auto-reconnect.
- `crates/velox-exchange/src/trait.rs` — the `ExchangeFeed` trait definition.

---

## Questions & Support

- **GitHub Issues**: Use for bug reports and feature requests. Choose the appropriate template.
- **Discussions**: For questions, ideas, and community conversation.
- **Security**: See [SECURITY.md](SECURITY.md) for reporting vulnerabilities.

---

## Attribution

This CONTRIBUTING guide is adapted from best practices across the Rust ecosystem and tailored for velox-terminal's specific architecture and quality standards.
