# Contributing to Havklo SDK

Thank you for your interest in contributing to the Havklo SDK! This document provides guidelines and information for contributors.

## Development Setup

### Prerequisites

- Rust 1.70 or later
- wasm-pack (for WASM builds)
- Git

### Getting Started

```bash
# Clone the repository
git clone https://github.com/hitakshiA/Havklo_sdk.git
cd Havklo_sdk

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run clippy
cargo clippy --workspace --all-targets

# Format code
cargo fmt --all
```

### Running Examples

```bash
# Simple ticker
cargo run --example simple_ticker

# Orderbook streaming
cargo run --example orderbook_stream

# Multi-symbol monitoring
cargo run --example multi_symbol
```

### Building WASM

```bash
cd crates/kraken-wasm
wasm-pack build --target web
```

## Code Style

### Formatting

We use `rustfmt` with the configuration in `rustfmt.toml`:
- Max line width: 100 characters
- Edition: 2021

Always run `cargo fmt --all` before committing.

### Linting

We use `clippy` with strict settings. All warnings are treated as errors:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Documentation

- All public items must have documentation comments (`///`)
- Include code examples in doc comments where helpful
- Run `cargo test --doc` to verify examples compile

## Commit Messages

We follow conventional commit format:

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, no code change
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(kraken-ws): add automatic reconnection with exponential backoff
fix(kraken-book): correct checksum calculation for high-precision decimals
docs(readme): add WASM usage examples
```

## Pull Request Process

1. **Fork** the repository and create your branch from `main`
2. **Write tests** for any new functionality
3. **Update documentation** if you're changing public APIs
4. **Run the full test suite** locally:
   ```bash
   cargo test --workspace
   cargo clippy --workspace --all-targets
   cargo fmt --all -- --check
   ```
5. **Update CHANGELOG.md** if your change is user-facing
6. **Submit a PR** with a clear description of changes

### PR Checklist

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Clippy reports no issues
- [ ] Code is formatted with rustfmt
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (if applicable)

## Testing

### Unit Tests

Each crate has unit tests in `src/` files:

```bash
cargo test -p kraken-types
cargo test -p kraken-book
cargo test -p kraken-ws
cargo test -p kraken-sdk
```

### Integration Tests

Integration tests are in `crates/kraken-sdk/tests/`:

```bash
cargo test -p kraken-sdk --test integration
```

### Benchmarks

Performance benchmarks use Criterion:

```bash
cargo bench --bench parsing
cargo bench --bench orderbook
```

## Architecture Overview

The SDK is organized as a Cargo workspace with 5 crates:

```
kraken-types     # Core types, minimal dependencies
    ↓
kraken-book      # Orderbook engine (WASM-compatible)
    ↓
kraken-ws        # WebSocket client
    ↓
kraken-sdk       # High-level API

kraken-wasm      # Browser bindings (wraps kraken-book)
```

See the main [README](README.md) for architecture details and advanced usage.

## Getting Help

- Open an issue for bugs or feature requests
- Check existing issues before creating new ones
- For questions, use GitHub Discussions

## Stability Policy

### Stable APIs

The following are considered stable and follow strict SemVer:

- `KrakenClient` builder and public methods
- `Event` enum variants and their fields
- `KrakenError` enum variants
- All types in `kraken_types` crate
- `Orderbook` and `L3Book` public methods

### Unstable/Internal APIs

The following may change without major version bump:

- Anything marked `#[doc(hidden)]`
- Internal modules (not re-exported in `prelude`)
- Benchmark utilities
- Test helpers

### What We Consider Breaking Changes

- Removing public API items
- Changing function signatures
- Adding required fields to structs (without defaults)
- Changing error variants in ways that break matching
- MSRV increases

### What We Do NOT Consider Breaking

- Adding new `#[non_exhaustive]` enum variants
- Adding optional fields to structs
- Performance improvements
- Bug fixes (even if code relied on buggy behavior)
- Dependency updates (unless they change our public API)

## SemVer Discipline

We follow [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** (1.0.0 → 2.0.0): Breaking API changes
- **MINOR** (1.0.0 → 1.1.0): New features, backward compatible
- **PATCH** (1.0.0 → 1.0.1): Bug fixes, backward compatible

### Pre-1.0 Guarantees

While we're pre-1.0:
- MINOR bumps may include breaking changes
- PATCH bumps are always backward compatible
- We'll clearly document breaking changes in CHANGELOG.md

### Protocol Changes

Kraken may change their WebSocket API. We handle this as:
- **Additive changes** (new fields): PATCH or MINOR
- **Breaking changes** (removed fields): MAJOR (or MINOR pre-1.0)
- **New channels**: MINOR

## Yank Policy

We will yank a published version if:

1. **Security vulnerability**: Critical security issues
2. **Data corruption**: Bugs that could corrupt orderbook state
3. **Compilation failure**: Version doesn't compile on advertised MSRV
4. **Dependency issue**: Yanked or broken dependency

We will **NOT** yank for:
- Minor bugs with workarounds
- Performance regressions
- Feature requests

### Yank Process

1. Open a GitHub issue explaining the yank reason
2. Publish a patched version first (if possible)
3. Yank the affected version(s)
4. Update CHANGELOG.md with yank notice

## Issue Triage

Issues will be closed immediately if:
- Duplicate of existing issue
- Not using latest stable version
- No reproduction steps provided
- Feature requests for out-of-scope items (see README non-goals)
- Questions that belong in Discussions

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
