# CI/CD Pipeline — velox-terminal

Pipeline de integración continua y despliegue continuo.

---

## CI Pipeline (per push)

```yaml
name: CI

on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo audit  # Security audit

  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace
      - run: cargo test --workspace --release  # Property-based tests in release

  build:
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - x86_64-pc-windows-msvc
          - x86_64-apple-darwin
          - aarch64-apple-darwin
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo build --release --target ${{ matrix.target }}
```

## Release Pipeline (per tag)

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  release:
    permissions:
      contents: write
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: velox-terminal-x86_64-linux.AppImage
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            artifact: velox-terminal-x86_64-windows.msi
          - target: x86_64-apple-darwin
            os: macos-latest
            artifact: velox-terminal-x86_64-macos.dmg
          - target: aarch64-apple-darwin
            os: macos-latest
            artifact: velox-terminal-aarch64-macos.dmg
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release --target ${{ matrix.target }}
      - run: cargo packager --target ${{ matrix.target }}  # Create installer
      - uses: softprops/action-gh-release@v1
        with:
          files: dist/*.${{ matrix.artifact }}
```

## Caching Strategy

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    workspaces: |
      crates
    key: ${{ matrix.target }}
    cache-on-failure: true
    shared-key: velox-cache-${{ matrix.target }}
```

## Performance Targets

| Stage | Time Target | Details |
|-------|-------------|---------|
| Lint + clippy + audit | < 3 min | Ubuntu only |
| Test (all platforms) | < 10 min | Matrix: ubuntu, windows, macOS |
| Build (release) | < 15 min | Per target with warm cache |
| Full release pipeline | < 30 min | All targets + packaging |
