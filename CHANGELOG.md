# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- GitHub Actions CI/CD workflows
- Issue and PR templates
- ARCHITECTURE.md documentation
- CONTRIBUTING.md guidelines
- Structured logging with tracing spans
- Optional Prometheus metrics support
- WebSocket authentication for private channels

## [0.1.0] - 2024-12-22

### Added
- Initial SDK release
- **kraken-types**: Core type definitions with `rust_decimal` precision
- **kraken-book**: WASM-compatible orderbook engine with CRC32 checksum validation
- **kraken-ws**: WebSocket client with automatic reconnection and exponential backoff
- **kraken-sdk**: High-level API with builder pattern
- **kraken-wasm**: Browser bindings for JavaScript integration
- Support for public channels: `book`, `ticker`, `trade`, `ohlc`, `instrument`
- Multi-symbol concurrent subscriptions
- Orderbook state machine with snapshot/update handling
- Historical snapshot ring buffer for time-travel
- Comprehensive test suite with 74+ tests
- Criterion benchmarks for performance validation
- Three working examples: `simple_ticker`, `orderbook_stream`, `multi_symbol`

### Technical Highlights
- Zero floating-point operations for prices/quantities
- Automatic precision extraction from instrument channel
- Thread-safe orderbook storage with DashMap
- Configurable reconnection with jitter

[Unreleased]: https://github.com/hitakshiA/Havklo_sdk/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/hitakshiA/Havklo_sdk/releases/tag/v0.1.0
