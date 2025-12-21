# Havklo SDK

A production-grade Rust SDK for Kraken's WebSocket API v2, built with a focus on correctness, performance, and cross-platform compatibility.

## Why This SDK

Most exchange SDKs suffer from common problems: floating-point precision errors, missing data validation, and poor reconnection handling. Havklo SDK addresses these directly:

**Financial Precision**: All prices and quantities use `rust_decimal` instead of f64. This prevents the subtle rounding errors that can accumulate in trading applications and cause significant issues over time.

**Data Integrity**: Every orderbook update is validated against Kraken's CRC32 checksum. If the data becomes corrupted or out of sync, the SDK detects it immediately rather than silently processing bad data.

**Reliability**: Network issues happen. The SDK handles disconnections with exponential backoff, automatically restores subscriptions, and maintains orderbook state across reconnects.

**Browser Support**: The orderbook engine compiles to WebAssembly without modification. You can run the same Rust code that processes orderbooks on your server in a browser-based trading interface.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kraken-sdk = { git = "https://github.com/havklo/havklo-sdk" }
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use kraken_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Kraken and subscribe to BTC/USD orderbook
    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .connect()
        .await?;

    // Process incoming events
    let mut events = client.events();

    while let Some(event) = events.recv().await {
        if let Event::Market(MarketEvent::OrderbookUpdate { symbol, .. }) = event {
            println!(
                "{}: bid={} ask={} spread={}",
                symbol,
                client.best_bid(&symbol).unwrap_or_default(),
                client.best_ask(&symbol).unwrap_or_default(),
                client.spread(&symbol).unwrap_or_default()
            );
        }
    }

    Ok(())
}
```

## Architecture

The SDK is organized into five crates, each with a specific responsibility:

```
kraken-types     Core types, enums, and error definitions
     |
kraken-book      Orderbook engine (WASM-compatible, no async runtime)
     |
     +---> kraken-wasm    Browser bindings via wasm-bindgen
     |
kraken-ws        WebSocket client with reconnection logic
     |
kraken-sdk       High-level API that ties everything together
```

### Crate Details

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `kraken-types` | Shared types for symbols, price levels, channels, and errors | serde, rust_decimal, thiserror |
| `kraken-book` | Orderbook state machine with checksum validation | kraken-types, crc32fast |
| `kraken-ws` | Native WebSocket client with auto-reconnect | kraken-types, kraken-book, tokio-tungstenite |
| `kraken-sdk` | Builder API and convenience methods | All of the above |
| `kraken-wasm` | Browser bindings for the orderbook engine | kraken-book, wasm-bindgen |

## Features

### Orderbook Management

The orderbook engine maintains a sorted, validated view of the market:

```rust
// Access current market state
let bid = client.best_bid("BTC/USD");
let ask = client.best_ask("BTC/USD");
let spread = client.spread("BTC/USD");
let mid = client.mid_price("BTC/USD");

// Get full orderbook depth
if let Some(book) = client.orderbook("BTC/USD") {
    for level in book.bids_vec().iter().take(10) {
        println!("Bid: {} @ {}", level.qty, level.price);
    }
}
```

### Checksum Validation

Every update from Kraken includes a CRC32 checksum. The SDK validates this automatically:

```rust
while let Some(event) = events.recv().await {
    match event {
        Event::Market(MarketEvent::ChecksumMismatch { symbol, expected, computed }) => {
            // Data integrity issue detected
            eprintln!("Checksum failed for {}: {} vs {}", symbol, expected, computed);
        }
        _ => {}
    }
}
```

The checksum algorithm handles precision correctly by fetching decimal place information from Kraken's instrument channel. This was a non-trivial implementation detail that many SDKs get wrong.

### Automatic Reconnection

Connection drops are handled transparently:

```rust
let client = KrakenClient::builder(["BTC/USD", "ETH/USD"])
    .with_reconnect(ReconnectConfig {
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(30),
        multiplier: 2.0,
        jitter: 0.2,
        max_attempts: None,  // Retry forever
    })
    .connect()
    .await?;
```

When reconnected, the SDK automatically resubscribes to all channels and requests fresh snapshots.

### Multiple Symbols

Subscribe to multiple trading pairs in a single connection:

```rust
let client = KrakenClient::builder(["BTC/USD", "ETH/USD", "SOL/USD"])
    .with_depth(Depth::D10)
    .connect()
    .await?;

// Each symbol maintains its own orderbook
let btc_spread = client.spread("BTC/USD");
let eth_spread = client.spread("ETH/USD");
```

### Configurable Depth

Kraken supports different orderbook depths. Choose based on your needs:

```rust
// Minimal depth for spread monitoring
.with_depth(Depth::D10)

// Standard depth for most applications
.with_depth(Depth::D25)

// Deep orderbook for market making
.with_depth(Depth::D100)

// Full depth (high bandwidth)
.with_depth(Depth::D1000)
```

## WebAssembly Support

The orderbook engine runs in browsers. This is useful for trading interfaces that need client-side orderbook processing.

### Building the WASM Package

```bash
cd crates/kraken-wasm
wasm-pack build --target web --out-dir ../../pkg
```

### Browser Usage

```javascript
import init, { WasmOrderbook } from './pkg/kraken_wasm.js';

async function main() {
    await init();

    const book = new WasmOrderbook('BTC/USD');
    book.enable_history(100);  // Keep last 100 snapshots

    const ws = new WebSocket('wss://ws.kraken.com/v2');

    ws.onopen = () => {
        ws.send(JSON.stringify({
            method: 'subscribe',
            params: { channel: 'book', symbol: ['BTC/USD'], depth: 10 }
        }));
    };

    ws.onmessage = (event) => {
        try {
            book.apply_message(event.data);

            if (book.is_synced()) {
                console.log('Spread:', book.get_spread());
                console.log('Mid:', book.get_mid_price());
            }
        } catch (e) {
            console.error('Failed to process message:', e);
        }
    };
}
```

### WASM API Reference

| Method | Returns | Description |
|--------|---------|-------------|
| `new(symbol)` | `WasmOrderbook` | Create orderbook for a trading pair |
| `apply_message(json)` | - | Process raw WebSocket message |
| `get_bids()` | `Array` | All bid levels (price, qty) |
| `get_asks()` | `Array` | All ask levels (price, qty) |
| `get_spread()` | `number` | Current bid-ask spread |
| `get_mid_price()` | `number` | Current mid price |
| `get_checksum()` | `number` | Last validated checksum |
| `is_synced()` | `boolean` | True if orderbook is valid |
| `enable_history(n)` | - | Keep last n snapshots |
| `get_snapshot_at(i)` | `Object` | Historical snapshot at index |

## Error Handling

The SDK uses typed errors that indicate whether an issue is recoverable:

```rust
use kraken_types::KrakenError;

match result {
    Err(e) if e.is_retryable() => {
        // Transient error, can retry
        let delay = e.retry_after().unwrap_or(Duration::from_secs(1));
        tokio::time::sleep(delay).await;
    }
    Err(e) if e.requires_reconnect() => {
        // Connection-level issue
        client.reconnect().await?;
    }
    Err(e) => {
        // Unrecoverable error
        return Err(e.into());
    }
    Ok(_) => {}
}
```

### Error Categories

| Category | Examples | Recovery |
|----------|----------|----------|
| Connection | `ConnectionFailed`, `ConnectionTimeout` | Retry with backoff |
| Protocol | `ChecksumMismatch`, `InvalidJson` | Request new snapshot |
| Subscription | `SubscriptionRejected`, `SymbolNotFound` | Check symbol format |
| Rate Limit | `RateLimited`, `CloudflareLimit` | Wait before retry |

## Examples

The repository includes working examples:

```bash
# Basic connection and spread monitoring
cargo run --example simple_ticker

# Detailed orderbook streaming with depth
cargo run --example orderbook_stream

# Multiple trading pairs simultaneously
cargo run --example multi_symbol
```

## Testing

Run the test suite:

```bash
# All tests
cargo test --workspace

# With output
cargo test --workspace -- --nocapture

# Specific crate
cargo test -p kraken-book
```

The SDK includes 74 tests covering message parsing, orderbook operations, checksum validation, and integration scenarios.

## Benchmarks

Performance benchmarks using Criterion:

```bash
cargo bench -p kraken-book
```

Measures parsing throughput, orderbook insertion, snapshot application, and checksum computation.

## Project Structure

```
havklo-sdk/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── kraken-types/       # Core types and errors
│   │   └── src/
│   │       ├── symbol.rs   # Trading pair symbols
│   │       ├── level.rs    # Price levels with Decimal
│   │       ├── enums.rs    # Channel, Side, Depth, etc.
│   │       ├── messages.rs # WebSocket message types
│   │       └── error.rs    # Error types with recovery hints
│   ├── kraken-book/        # Orderbook engine
│   │   └── src/
│   │       ├── storage.rs  # BTreeMap-based storage
│   │       ├── checksum.rs # CRC32 validation
│   │       ├── orderbook.rs # State machine
│   │       └── history.rs  # Snapshot ring buffer
│   ├── kraken-ws/          # WebSocket client
│   │   └── src/
│   │       ├── connection.rs   # Connection lifecycle
│   │       ├── reconnect.rs    # Backoff configuration
│   │       ├── subscription.rs # Channel management
│   │       └── events.rs       # Event types
│   ├── kraken-sdk/         # High-level API
│   │   ├── src/
│   │   │   ├── client.rs   # KrakenClient
│   │   │   ├── builder.rs  # Builder pattern
│   │   │   └── prelude.rs  # Convenient imports
│   │   └── examples/       # Working examples
│   └── kraken-wasm/        # Browser bindings
│       └── src/lib.rs      # wasm-bindgen exports
└── tests/
    └── integration.rs      # End-to-end tests
```

## Kraken API Compatibility

This SDK targets Kraken's WebSocket API v2. Key differences from v1:

- Symbols use `BTC/USD` format (not `XBT/USD`)
- Timestamps are RFC3339 format
- Prices and quantities are JSON numbers (handled with Decimal parsing)
- Checksum requires precision from instrument channel

### Supported Channels

| Channel | Status | Description |
|---------|--------|-------------|
| `book` | Full support | Orderbook snapshots and updates |
| `ticker` | Events received | Best bid/ask, 24h stats |
| `trade` | Events received | Executed trades |
| `instrument` | Internal | Precision metadata for checksum |
| `status` | Handled | System status updates |
| `heartbeat` | Handled | Connection keepalive |

## Requirements

- Rust 1.70 or later
- For WASM: wasm-pack and wasm32-unknown-unknown target

## License

MIT License. See [LICENSE](LICENSE) for details.

## Author

Hitakshi Arora
