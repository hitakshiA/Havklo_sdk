# Architecture

This document describes the high-level architecture of the Havklo SDK.

## Overview

```
                                    ┌─────────────────────────────────────────┐
                                    │              User Application           │
                                    └─────────────────────────────────────────┘
                                                        │
                                                        ▼
┌───────────────────────────────────────────────────────────────────────────────────────┐
│                                     kraken-sdk                                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │   Builder   │  │   Client    │  │   Market    │  │    Auth     │  │   Metrics   │ │
│  │   Pattern   │  │     API     │  │   Data      │  │   (OAuth)   │  │ (Prometheus)│ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘ │
└───────────────────────────────────────────────────────────────────────────────────────┘
         │                   │                │
         ▼                   ▼                ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│   kraken-ws     │  │   kraken-book   │  │  kraken-types   │
│  ─────────────  │  │  ─────────────  │  │  ─────────────  │
│  • Transport    │  │  • Orderbook    │  │  • Messages     │
│  • Reconnection │  │  • Checksum     │  │  • Level        │
│  • Circuit Break│  │  • L3 Book      │  │  • Symbol       │
│  • Trading      │  │  • History      │  │  • Errors       │
└─────────────────┘  └─────────────────┘  └─────────────────┘
         │
         ▼
┌─────────────────┐                      ┌─────────────────┐
│  kraken-auth    │                      │  kraken-wasm    │
│  ─────────────  │                      │  ─────────────  │
│  • Token Mgmt   │                      │  • Browser WASM │
│  • Signatures   │                      │  • JS Bindings  │
└─────────────────┘                      └─────────────────┘
```

## Crate Responsibilities

### kraken-types
**Core type definitions shared across all crates.**

- `Level`: Price/quantity pairs with `rust_decimal` precision
- `Symbol`: Trading pair representations
- `Messages`: WebSocket message structures (subscribe, unsubscribe, etc.)
- `Error`: Error types and codes
- `RateLimit`: Rate limiting structures

**Design decisions:**
- Zero floating-point operations for prices/quantities
- All monetary values use `rust_decimal::Decimal`
- Serde-compatible for JSON serialization

### kraken-ws
**Low-level WebSocket transport layer.**

- `Transport`: Raw WebSocket connection management
- `Reconnection`: Exponential backoff with jitter
- `CircuitBreaker`: Prevents reconnection storms
- `OrderTracker`: Tracks order state for trading

**Design decisions:**
- Uses `tokio-tungstenite` for async WebSocket
- Event-driven architecture with channels
- Configurable reconnection parameters

### kraken-book
**Orderbook management and analysis.**

- `Orderbook`: L2 orderbook with bid/ask management
- `Checksum`: CRC32 checksum validation per Kraken spec
- `L3Book`: Level 3 orderbook with individual orders
- `History`: Ring buffer for orderbook snapshots (time-travel)
- `Storage`: Thread-safe orderbook storage with DashMap

**Design decisions:**
- Checksum validation ensures data integrity
- History buffer enables orderbook replay
- WASM-compatible (no std dependencies where possible)

### kraken-auth
**Authentication for private channels.**

- Token management with automatic refresh
- Request signing for authenticated endpoints
- OAuth 2.0 flow support

### kraken-sdk
**High-level API that ties everything together.**

- `KrakenClientBuilder`: Fluent builder pattern for configuration
- `KrakenClient`: Main entry point for users
- `MarketData`: Market data access (orderbooks, tickers)
- `Metrics`: Optional Prometheus metrics export

**Design decisions:**
- Builder pattern for flexible configuration
- Sensible defaults for all options
- Optional features (metrics, auth) behind feature flags

### kraken-wasm
**WebAssembly bindings for browser usage.**

- `WasmOrderbook`: Browser-compatible orderbook
- JavaScript/TypeScript bindings via `wasm-bindgen`
- Web-optimized message handling

## Data Flow

### Subscription Flow
```
User calls client.subscribe()
         │
         ▼
┌─────────────────────────────┐
│  KrakenClient::subscribe()  │
└─────────────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│  Build subscription message │
│  (kraken-types::Message)    │
└─────────────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│  Send via WebSocket         │
│  (kraken-ws::Transport)     │
└─────────────────────────────┘
         │
         ▼
    Kraken Server
```

### Orderbook Update Flow
```
WebSocket receives message
         │
         ▼
┌─────────────────────────────┐
│  Parse JSON message         │
│  (kraken-types)             │
└─────────────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│  Apply to orderbook         │
│  (kraken-book::Orderbook)   │
└─────────────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│  Validate checksum          │
│  (kraken-book::Checksum)    │
└─────────────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│  Store in DashMap           │
│  (kraken-book::Storage)     │
└─────────────────────────────┘
         │
         ▼
┌─────────────────────────────┐
│  Emit event to user         │
│  (mpsc channel)             │
└─────────────────────────────┘
```

## Feature Flags

| Feature | Crate | Description |
|---------|-------|-------------|
| `metrics` | kraken-sdk | Prometheus metrics export |
| `auth` | kraken-sdk | Private channel authentication |
| `full` | kraken-sdk | All features enabled |

## Error Handling

All crates use a unified error type hierarchy:

```rust
pub enum KrakenError {
    // Connection errors
    ConnectionFailed(String),
    ConnectionClosed,

    // Protocol errors
    InvalidMessage(String),
    ChecksumMismatch { expected: u32, actual: u32 },

    // Subscription errors
    SubscriptionFailed { symbol: String, reason: String },

    // Rate limiting
    RateLimited { retry_after: Duration },
}
```

## Thread Safety

- `KrakenClient` is `Send + Sync`
- Orderbooks are stored in `DashMap` for concurrent access
- Event channels use `tokio::sync::mpsc`

## Performance Considerations

1. **Zero-copy parsing**: Messages parsed directly from bytes where possible
2. **Decimal arithmetic**: No floating-point rounding errors
3. **Connection pooling**: Single connection per client
4. **Backpressure**: Bounded channels prevent memory exhaustion

## Testing Strategy

- **Unit tests**: Per-crate functionality
- **Integration tests**: Full client flow with mock server
- **Doc tests**: Example code in documentation
- **Benchmarks**: Performance regression tracking

## WASM Considerations

The `kraken-wasm` crate provides browser compatibility:

1. No `std` networking (uses browser WebSocket)
2. No threads (single-threaded execution)
3. `wasm-bindgen` for JS interop
4. Smaller binary size (optimized for web)

## Security

- No secrets stored in memory longer than needed
- Token refresh before expiration
- TLS-only WebSocket connections
- Input validation on all public APIs
