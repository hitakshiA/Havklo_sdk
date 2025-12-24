# Havklo SDK

[![CI](https://github.com/hitakshiA/Havklo_sdk/workflows/CI/badge.svg)](https://github.com/hitakshiA/Havklo_sdk/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

A high-performance Rust SDK for Kraken's WebSocket APIs. Built for algorithmic trading, market making, and real-time data streaming with sub-microsecond orderbook operations.

## Features

- **Zero-copy parsing** with sub-microsecond orderbook updates
- **L2 + L3 orderbooks** with queue position tracking
- **CRC32 checksum validation** on every update
- **WebSocket trading** - place, amend, cancel orders without REST
- **Automatic reconnection** with exponential backoff and circuit breaker
- **Browser support** via WebAssembly
- **Financial precision** using `rust_decimal` (no floating point errors)

## Performance

All benchmarks run on Apple M1 Pro. Numbers represent median times.

### Orderbook Operations

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Best bid/ask lookup | **1.0 ns** | 1,000,000,000/s |
| Spread calculation | **3.5 ns** | 285,000,000/s |
| Mid-price calculation | **22.7 ns** | 44,000,000/s |
| Snapshot capture | **81.6 ns** | 12,250,000/s |
| Apply delta update | **~100 ns** | 10,000,000/s |
| Apply full snapshot (100 levels) | **10.0 µs** | 100,000/s |
| Checksum validation | **5.9 µs** | 169,000/s |

### Message Parsing

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Parse subscription response | **538 ns** | 1,860,000/s |
| Parse orderbook update | **1.1 µs** | 920,000/s |
| Parse orderbook snapshot | **2.7 µs** | 370,000/s |
| Parse scientific notation | **3.5 µs** | 286,000/s |

### L3 Orderbook (Order-Level)

| Operation | Latency | Throughput |
|-----------|---------|------------|
| Best bid/ask | **1.0 ns** | 1,000,000,000/s |
| VWAP (1 BTC) | **28 ns** | 35,700,000/s |
| VWAP (10 BTC) | **155 ns** | 6,450,000/s |
| Add order | **~150 ns** | 6,600,000/s |
| Remove order | **~200 ns** | 5,000,000/s |
| Queue position | **~30 ns** | 33,000,000/s |
| Full snapshot | **23.7 µs** | 42,000/s |

## Quick Start

Add to `Cargo.toml`:

```toml
[dependencies]
kraken-sdk = { git = "https://github.com/hitakshiA/Havklo_sdk" }
tokio = { version = "1", features = ["full"] }
```

Stream BTC/USD orderbook:

```rust
use kraken_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KrakenClient::builder(vec!["BTC/USD".into()])
        .with_depth(Depth::D10)
        .with_book(true)
        .connect()
        .await?;

    // Access real-time market data
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        if let Some(spread) = client.spread("BTC/USD") {
            println!("BTC/USD spread: {}", spread);
        }

        if let Some(bid) = client.best_bid("BTC/USD") {
            println!("Best bid: {}", bid);
        }
    }
}
```

Process events:

```rust
use kraken_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KrakenClient::builder(vec!["BTC/USD".into()])
        .with_book(true)
        .connect()
        .await?;

    let mut events = client.events();
    while let Some(event) = events.recv().await {
        match event {
            Event::Market(MarketEvent::OrderbookSnapshot { symbol, .. }) => {
                println!("{}: Snapshot received", symbol);
            }
            Event::Market(MarketEvent::OrderbookUpdate { symbol, .. }) => {
                println!("{}: Update received", symbol);
            }
            _ => {}
        }
    }
    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      kraken-sdk                              │
│         High-level API with builder pattern                  │
├─────────────────┬─────────────────┬─────────────────────────┤
│   kraken-ws     │ kraken-futures  │      kraken-auth        │
│  Spot WS v2     │   Futures WS    │   Token management      │
├─────────────────┴─────────────────┴─────────────────────────┤
│                       kraken-book                            │
│         L2/L3 orderbook engine (WASM-compatible)            │
├─────────────────────────────────────────────────────────────┤
│                      kraken-types                            │
│              Shared types, error codes, enums                │
└─────────────────────────────────────────────────────────────┘
```

### Crate Responsibilities

| Crate | Purpose | WASM |
|-------|---------|------|
| `kraken-sdk` | Unified high-level API | No |
| `kraken-ws` | Spot WebSocket v2 with trading | No |
| `kraken-futures-ws` | Futures perpetuals streaming | No |
| `kraken-auth` | API authentication, token refresh | No |
| `kraken-book` | L2/L3 orderbook engine | **Yes** |
| `kraken-types` | Core types, error handling | **Yes** |
| `kraken-wasm` | JavaScript bindings | **Yes** |

## Design Decisions

### Financial Precision with rust_decimal

Floating point arithmetic causes rounding errors that compound in trading systems. We use `rust_decimal` throughout:

```rust
// This is wrong:
let price: f64 = 0.1 + 0.2;  // = 0.30000000000000004

// We do this:
let price: Decimal = dec!(0.1) + dec!(0.2);  // = 0.3 exactly
```

### CRC32 Checksum Validation

Kraken sends a checksum with each orderbook message. We validate every update to detect:
- Missed messages
- Corrupted data
- Sequence gaps

On mismatch, the SDK automatically requests a fresh snapshot.

### Lock-Free Orderbook Access

The orderbook uses `DashMap` for concurrent read access without blocking writers:

```rust
// Multiple threads can read simultaneously
let spread = client.spread("BTC/USD");  // No mutex, no blocking
```

Writers (delta updates) proceed independently. Readers always see a consistent snapshot.

### Circuit Breaker Pattern

After repeated connection failures, the circuit breaker opens to prevent cascade failures:

```
Closed (normal) → Open (after N failures) → Half-Open (test connection) → Closed
```

Configuration:
- 5 failures to open
- 30 second reset timeout
- Exponential backoff: 100ms → 200ms → 400ms → ... → 30s max

### Zero-Copy Event Handling

Events are delivered via `tokio::sync::mpsc` channels. The connection task writes, your handler reads—no intermediate copies.

### WASM Compatibility

`kraken-book` compiles to WebAssembly because:
- No `tokio` dependency (async is in `kraken-ws`)
- No networking code
- Pure computation with `rust_decimal` and standard collections

## Advanced Features

### L3 Orderbook (Order-Level Depth)

Track individual orders, not just aggregated levels:

```rust
use kraken_book::l3::{L3Book, L3Order, L3Side};

let mut book = L3Book::new("BTC/USD", 1000);

// Add individual orders
let order = L3Order::new("order_123", dec!(50000), dec!(1.5));
book.add_order(order, L3Side::Bid);

// Query queue position
if let Some(pos) = book.queue_position("order_123") {
    println!("Position: {} of {} in queue", pos.position, pos.total_orders);
    println!("Volume ahead: {}", pos.volume_ahead);
}

// Aggregate to L2 view
let l2_bids = book.aggregated_bids();
```

### WebSocket Trading

Place orders without REST API latency:

```rust
use kraken_ws::trading::{OrderRequest, OrderSide, OrderType};

// Place limit order
let order = OrderRequest::limit("BTC/USD", OrderSide::Buy, dec!(1.0), dec!(50000));
conn.place_order(order);

// Batch operations
conn.cancel_all_orders("BTC/USD");
```

### Automatic Reconnection

Connection drops are handled automatically:

```rust
use kraken_ws::{ConnectionConfig, ReconnectConfig};

let config = ConnectionConfig::new()
    .with_reconnect(ReconnectConfig {
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(30),
        multiplier: 2.0,
        max_attempts: None,  // Retry forever
    });
```

### Rate Limiting

Built-in rate limiting prevents API throttling:

```rust
// Automatic rate limiting per Kraken's documented limits
// Public: 1 req/sec
// Private: Based on verification tier
```

### Multi-Symbol Streaming

```rust
let client = KrakenClient::builder(vec![
    "BTC/USD".into(),
    "ETH/USD".into(),
    "SOL/USD".into(),
])
.with_depth(Depth::D25)
.connect()
.await?;

// Each symbol maintains independent orderbook state
for symbol in ["BTC/USD", "ETH/USD", "SOL/USD"] {
    if let Some(spread) = client.spread(symbol) {
        println!("{}: {}", symbol, spread);
    }
}
```

### Browser Integration (WASM)

```bash
cd crates/kraken-wasm
wasm-pack build --target web
```

```javascript
import init, { WasmOrderbook } from './pkg/kraken_wasm.js';

await init();
const book = new WasmOrderbook("BTC/USD", 10);
book.apply_snapshot(snapshotJson);
console.log("Spread:", book.spread());
```

## Examples

```bash
# Basic ticker streaming
cargo run --example simple_ticker

# L2 orderbook with checksum validation
cargo run --example orderbook_stream

# Multiple symbols
cargo run --example multi_symbol

# Futures perpetuals
cargo run --example futures_stream

# L3 market making simulation
cargo run --example market_maker

# Reconnection handling
cargo run --example advanced_reconnect

# Full SDK validation against live Kraken
cargo run --example live_validation
```

## Testing

```bash
# Unit tests
cargo test --workspace

# Integration tests (requires network)
cargo test --workspace -- --ignored

# Benchmarks
cargo bench -p kraken-book

# Live validation (connects to real Kraken)
cargo run --example live_validation
```

## API Compatibility

| API | Version | Coverage |
|-----|---------|----------|
| Spot WebSocket | v2 | Full (book, ticker, trade, ohlc) |
| Spot WS Trading | v2 | Full (add, edit, cancel, batch) |
| Futures WebSocket | v1 | Full (all feeds) |
| L3 Orders Channel | v2 | Full |
| REST API | - | Token endpoint only |

## Error Handling

All errors implement structured recovery:

```rust
match error {
    KrakenError::RateLimited { retry_after } => {
        // Wait and retry
        tokio::time::sleep(retry_after).await;
    }
    KrakenError::ChecksumMismatch { .. } => {
        // SDK automatically resubscribes
    }
    KrakenError::AuthenticationFailed { .. } => {
        // Refresh credentials
    }
    _ => {}
}

// Or use built-in recovery
if error.is_retryable() {
    let delay = error.retry_after().unwrap_or(Duration::from_secs(1));
    tokio::time::sleep(delay).await;
}
```

## Dependencies

The SDK is designed with minimal dependencies for each use case:

| Crate | Dependencies | Use Case |
|-------|-------------|----------|
| `kraken-types` | 28 | Shared types only |
| `kraken-book` | 35 | WASM orderbook engine |
| `kraken-sdk` | 268 | Full async SDK |

The full SDK includes async runtime (tokio), networking, and observability. For browser/WASM usage, only `kraken-book` (35 deps) is needed.

## Requirements

- Rust 1.70+
- For WASM: `wasm-pack` and `wasm32-unknown-unknown` target

## License

MIT License. See [LICENSE](LICENSE) for details.

## Author

Hitakshi Arora
