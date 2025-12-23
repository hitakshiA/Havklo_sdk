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
git clone https://github.com/your-username/havklo-sdk.git
cd havklo-sdk

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

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
