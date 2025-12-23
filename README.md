# Havklo SDK

[![CI](https://github.com/havklo/havklo-sdk/workflows/CI/badge.svg)](https://github.com/havklo/havklo-sdk/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

A production-grade Rust SDK for Kraken's APIs, featuring WebSocket streaming (Spot + Futures), REST trading, L2/L3 orderbooks, and browser support via WebAssembly.

## Why This SDK

Most exchange SDKs suffer from common problems: floating-point precision errors, missing data validation, and poor reconnection handling. Havklo SDK addresses these directly:

**Financial Precision**: All prices and quantities use `rust_decimal` instead of f64. This prevents the subtle rounding errors that can accumulate in trading applications.

**Data Integrity**: Every orderbook update is validated against Kraken's CRC32 checksum. If data becomes corrupted or out of sync, the SDK detects it immediately.

**Complete Platform Coverage**: Spot WebSocket, Futures WebSocket, and REST API - all in one SDK. Trade perpetual swaps, check balances, place orders, and stream real-time data.

**L3 Orderbook**: Order-level depth with queue position tracking. Know exactly where your order sits in the book and estimate fill probability.

**Production Ready**: Client-side rate limiting, automatic token refresh, comprehensive error handling, and Prometheus-ready metrics.

**Browser Support**: The orderbook engine compiles to WebAssembly. Run the same Rust code in browsers for trading interfaces.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kraken-sdk = { git = "https://github.com/havklo/havklo-sdk" }
tokio = { version = "1", features = ["full"] }

# For REST API trading
kraken-rest = { git = "https://github.com/havklo/havklo-sdk" }

# For Futures trading
kraken-futures-ws = { git = "https://github.com/havklo/havklo-sdk" }
```

## Quick Start

### WebSocket Streaming

```rust
use kraken_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .connect()
        .await?;

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

### REST API Trading

```rust
use kraken_rest::{KrakenRestClient, Credentials};
use kraken_rest::types::{OrderRequest, OrderSide};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Public endpoints (no auth required)
    let client = KrakenRestClient::new();

    let tickers = client.get_ticker("XBTUSD").await?;
    println!("BTC price: ${}", tickers["XXBTZUSD"].last_price().unwrap());

    // Private endpoints (auth required)
    let auth_client = KrakenRestClient::with_credentials(
        Credentials::new(api_key, api_secret)?
    );

    let balances = auth_client.get_balance().await?;
    let order = OrderRequest::limit("XBTUSD", OrderSide::Buy, dec!(0.001), dec!(50000));
    let result = auth_client.add_order(&order).await?;

    Ok(())
}
```

### Futures Streaming

```rust
use kraken_futures_ws::{FuturesConnection, FuturesConfig, FuturesEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = FuturesConfig::new()
        .with_products(vec!["PI_XBTUSD".to_string()])
        .with_book_depth(25);

    let mut conn = FuturesConnection::new(config);
    let mut events = conn.take_event_receiver().unwrap();

    tokio::spawn(async move { conn.connect_and_run().await });

    while let Some(event) = events.recv().await {
        match event {
            FuturesEvent::Ticker(t) => {
                println!("Mark: ${}, Funding: {}%",
                    t.mark_price.unwrap_or_default(),
                    t.funding_rate.unwrap_or_default() * dec!(100)
                );
            }
            _ => {}
        }
    }
    Ok(())
}
```

## Architecture

The SDK is organized into seven modular crates:

```
                    ┌─────────────────────────────────────┐
                    │           kraken-sdk               │
                    │    (High-level unified API)        │
                    └─────────────────────────────────────┘
                           │           │           │
           ┌───────────────┼───────────┼───────────┼───────────────┐
           ▼               ▼           ▼           ▼               ▼
    ┌───────────┐   ┌───────────┐ ┌─────────┐ ┌──────────────┐ ┌─────────┐
    │ kraken-ws │   │kraken-rest│ │kraken-  │ │kraken-futures│ │ kraken- │
    │ (Spot WS) │   │  (REST)   │ │  book   │ │    -ws       │ │  wasm   │
    └───────────┘   └───────────┘ │(L2+L3)  │ │ (Futures WS) │ │(Browser)│
                                  └─────────┘ └──────────────┘ └─────────┘
                                       │
                              ┌────────┴────────┐
                              ▼                 ▼
                       ┌───────────┐     ┌───────────┐
                       │kraken-types│     │ L3 Order │
                       │(Core types)│     │  Book    │
                       └───────────┘     └───────────┘
```

### Crate Details

| Crate | Purpose | Key Features |
|-------|---------|--------------|
| `kraken-types` | Core types and errors | Error codes, rate limits, symbols |
| `kraken-book` | Orderbook engine | L2 + L3, checksum validation, WASM-compatible |
| `kraken-ws` | Spot WebSocket client | Auto-reconnect, private channels |
| `kraken-rest` | REST API client | HMAC-SHA512 auth, all trading endpoints |
| `kraken-futures-ws` | Futures WebSocket | Perpetuals, funding rates, positions |
| `kraken-sdk` | High-level API | Builder pattern, unified interface |
| `kraken-wasm` | Browser bindings | L2 + L3 orderbook in JavaScript |

## Features

### L3 Orderbook (Order-Level Depth)

Track individual orders and queue position - essential for market making:

```rust
use kraken_book::l3::{L3Book, L3Order, L3Side};

let mut book = L3Book::new("BTC/USD", 100);

// Add orders
book.add_order(L3Order::new("order_123", dec!(50000), dec!(1.5)), L3Side::Bid);

// Check queue position
if let Some(pos) = book.queue_position("order_123") {
    println!("Position: {} of {}", pos.position, pos.total_orders);
    println!("Quantity ahead: {}", pos.qty_ahead);
    println!("Fill probability: {:.1}%", pos.fill_probability() * 100.0);
}

// Market making analytics
let imbalance = book.imbalance();  // Buy/sell pressure
let vwap = book.vwap_ask(dec!(10.0));  // Cost to buy 10 BTC
```

### REST API Endpoints

Complete trading capability:

| Category | Endpoints |
|----------|-----------|
| **Market Data** | Ticker, Orderbook, OHLC, Trades, Assets, Pairs |
| **Account** | Balance, Trade History, Ledgers, Open Orders |
| **Trading** | Add Order, Cancel Order, Edit Order, Cancel All |
| **Funding** | Deposit Methods, Deposit Addresses, Withdrawals |
| **Earn** | Staking, Unstaking, Pending Rewards |

### Futures API

Full perpetual swap support:

```rust
// Futures-specific data
FuturesEvent::Ticker(ticker) => {
    ticker.mark_price      // Fair price for liquidations
    ticker.index_price     // Spot index
    ticker.funding_rate    // 8-hourly funding
    ticker.open_interest   // Total open contracts
}

FuturesEvent::Position(pos) => {
    pos.pnl                // Unrealized P&L
    pos.leverage           // Current leverage
    pos.liquidation_price  // Liquidation threshold
}
```

### Private Channels

Real-time execution and balance updates:

```rust
// Subscribe to private channels (requires auth token)
Event::Private(PrivateEvent::Execution(exec)) => {
    println!("Fill: {} {} @ {}", exec.side, exec.qty, exec.price);
}

Event::Private(PrivateEvent::BalanceUpdate(balances)) => {
    for (asset, balance) in balances.iter() {
        println!("{}: {}", asset, balance);
    }
}
```

### Client-Side Rate Limiting

Production-safe by default:

```rust
use kraken_types::rate_limit::{TokenBucket, RateLimitCategory};

// Automatic rate limiting with token bucket
let limiter = KrakenRateLimiter::new();

// Check before making requests
if limiter.try_acquire(RateLimitCategory::RestPrivate) {
    client.get_balance().await?;
}

// Or wait for capacity
limiter.acquire(RateLimitCategory::WsOrder).await;
```

### Prometheus Metrics

Production observability (enable with `metrics` feature):

```rust
// Counters
kraken_messages_total{channel="book", symbol="BTC/USD"}
kraken_orders_total{side="buy", type="limit"}
kraken_rate_limit_rejections_total{category="rest_private"}

// Gauges
kraken_orderbook_spread{symbol="BTC/USD"}
kraken_orderbook_imbalance{symbol="BTC/USD"}
kraken_rate_limit_tokens{category="ws_order"}

// Histograms
kraken_rest_request_duration_seconds{endpoint="AddOrder"}
kraken_orderbook_update_duration_seconds
kraken_order_roundtrip_seconds
```

### Checksum Validation

Every update validated against Kraken's CRC32:

```rust
Event::Market(MarketEvent::ChecksumMismatch { symbol, expected, computed }) => {
    eprintln!("Checksum failed for {}: {} vs {}", symbol, expected, computed);
    // SDK automatically requests fresh snapshot
}
```

### Automatic Reconnection

```rust
let client = KrakenClient::builder(["BTC/USD"])
    .with_reconnect(ReconnectConfig {
        initial_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(30),
        multiplier: 2.0,
        max_attempts: None,  // Retry forever
    })
    .connect()
    .await?;
```

## WebAssembly Support

### Building WASM Package

```bash
cd crates/kraken-wasm
wasm-pack build --target web
```

### Browser Usage (L2 Orderbook)

```javascript
import init, { WasmOrderbook } from './pkg/kraken_wasm.js';

await init();
const book = new WasmOrderbook('BTC/USD');

ws.onmessage = (event) => {
    book.apply_message(event.data);
    if (book.is_synced()) {
        console.log('Spread:', book.get_spread());
        console.log('Mid:', book.get_mid_price());
    }
};
```

### Browser Usage (L3 Orderbook)

```javascript
import init, { WasmL3Book } from './pkg/kraken_wasm.js';

await init();
const book = new WasmL3Book('BTC/USD', 100);

// Add order and check queue position
book.add_order('my_order', 'bid', '50000.00', '1.5');
const pos = book.get_queue_position('my_order');
console.log('Fill probability:', pos.fill_probability);

// Analytics
console.log('Imbalance:', book.get_imbalance());
console.log('VWAP to buy 10:', book.get_vwap_ask('10.0'));
```

### WASM API Reference

| Method | Returns | Description |
|--------|---------|-------------|
| `WasmOrderbook.new(symbol)` | L2 book | Create L2 orderbook |
| `WasmL3Book.new(symbol, depth)` | L3 book | Create L3 orderbook |
| `apply_message(json)` | - | Process WebSocket message |
| `get_bids()` / `get_asks()` | Array | Price levels |
| `get_spread()` | number | Bid-ask spread |
| `get_queue_position(id)` | Object | Queue position info |
| `get_imbalance()` | number | Buy/sell pressure (-1 to 1) |
| `get_vwap_ask(qty)` | number | VWAP to buy quantity |

## Examples

```bash
# Basic ticker streaming
cargo run --example simple_ticker

# L2 orderbook streaming
cargo run --example orderbook_stream

# Multiple trading pairs
cargo run --example multi_symbol

# Advanced reconnection handling
cargo run --example advanced_reconnect

# REST API trading demo
cargo run --example rest_trading

# Futures WebSocket streaming
cargo run --example futures_stream

# Market maker with L3 orderbook
cargo run --example market_maker
```

## Error Handling

Comprehensive error codes with recovery strategies:

```rust
use kraken_types::error_codes::KrakenApiError;

match error {
    KrakenApiError::RateLimitExceeded => {
        // Backoff and retry
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    KrakenApiError::InsufficientFunds => {
        // User action required
        eprintln!("Not enough balance");
    }
    KrakenApiError::InvalidNonce => {
        // Retry with new nonce
    }
    KrakenApiError::ServiceUnavailable => {
        // Retry after delay
    }
}
```

## Testing

```bash
# All tests (175 total)
cargo test --workspace

# Specific crate
cargo test -p kraken-book
cargo test -p kraken-rest

# With output
cargo test --workspace -- --nocapture
```

## Benchmarks

```bash
cargo bench -p kraken-book
```

Typical performance:
- Orderbook update: <10µs
- L3 queue position lookup: <5µs
- Checksum validation: <1µs

## Project Structure

```
havklo-sdk/
├── crates/
│   ├── kraken-types/       # Core types, errors, rate limits
│   ├── kraken-book/        # L2 + L3 orderbook engine
│   ├── kraken-ws/          # Spot WebSocket client
│   ├── kraken-rest/        # REST API client
│   ├── kraken-futures-ws/  # Futures WebSocket client
│   ├── kraken-sdk/         # High-level unified API
│   │   └── examples/       # 7 working examples
│   └── kraken-wasm/        # Browser bindings
├── .github/workflows/      # CI/CD
└── docs/                   # Additional documentation
```

## API Compatibility

| API | Version | Status |
|-----|---------|--------|
| Spot WebSocket | v2 | Full support |
| Futures WebSocket | v1 | Full support |
| REST API | v0 | Full support |

## Requirements

- Rust 1.70+
- For WASM: `wasm-pack` and `wasm32-unknown-unknown` target

## License

MIT License. See [LICENSE](LICENSE) for details.

## Author

Hitakshi Arora
