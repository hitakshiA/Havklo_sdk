# Kraken SDK Usage Guide for Track 2 & Track 3

> **SDK Location:** `/Users/akshmnd/Dev Projects/Havklo_sdk`
>
> This document provides everything needed to integrate the Kraken SDK into Track 2 (Orderbook Visualizer) and Track 3 (Strategy Builder).

---

## Table of Contents

1. [SDK Overview](#sdk-overview)
2. [Track 2: WASM Integration (Browser)](#track-2-wasm-integration-browser)
3. [Track 3: Native Rust Integration](#track-3-native-rust-integration)
4. [API Reference](#api-reference)
5. [WebSocket Message Format](#websocket-message-format)
6. [Common Patterns](#common-patterns)

---

## SDK Overview

### Architecture

```
/Users/akshmnd/Dev Projects/Havklo_sdk/
├── crates/
│   ├── kraken-types/     # Shared types (Decimal, Level, Symbol, Messages)
│   ├── kraken-book/      # Orderbook engine (WASM-compatible, no async)
│   ├── kraken-ws/        # WebSocket client (native only, tokio)
│   ├── kraken-sdk/       # High-level API (native only)
│   └── kraken-wasm/      # WASM bindings for browser
│       └── pkg/          # Built WASM package (npm-ready)
```

### Which Crate to Use

| Track | Crate | Location |
|-------|-------|----------|
| Track 2 (Browser) | `kraken-wasm` | `/Users/akshmnd/Dev Projects/Havklo_sdk/crates/kraken-wasm/pkg` |
| Track 3 (Native Rust) | `kraken-sdk` | `/Users/akshmnd/Dev Projects/Havklo_sdk/crates/kraken-sdk` |

---

## Track 2: WASM Integration (Browser)

### Setup

#### 1. Add dependency to package.json

```json
{
  "dependencies": {
    "@kraken-forge/wasm": "file:/Users/akshmnd/Dev Projects/Havklo_sdk/crates/kraken-wasm/pkg"
  }
}
```

#### 2. Install

```bash
npm install
```

### TypeScript Types Available

```typescript
// From kraken_wasm.d.ts

export class WasmOrderbook {
  // Constructors
  constructor(symbol: string);
  static with_depth(symbol: string, depth: number): WasmOrderbook;

  // Core Methods
  apply_message(json: string): string;  // Returns: "snapshot" | "update" | "ignored"
  reset(): void;

  // Getters - Orderbook Data
  get_bids(): Array<{price: number, qty: number}>;
  get_asks(): Array<{price: number, qty: number}>;
  get_top_bids(n: number): Array<{price: number, qty: number}>;
  get_top_asks(n: number): Array<{price: number, qty: number}>;
  get_bid_count(): number;
  get_ask_count(): number;

  // Getters - Prices
  get_best_bid(): number;
  get_best_ask(): number;
  get_spread(): number;
  get_mid_price(): number;

  // Getters - State
  get_symbol(): string;
  get_state(): string;  // "Uninitialized" | "AwaitingSnapshot" | "Synced" | "Desynchronized"
  get_checksum(): number;
  is_synced(): boolean;

  // History (Time-Travel Feature)
  enable_history(max_snapshots: number): void;
  disable_history(): void;
  is_history_enabled(): boolean;
  get_history_length(): number;
  get_snapshot_at(index: number): {bids: Array, asks: Array, checksum: number} | null;
  get_latest_sequence(): bigint;
  clear_history(): void;

  // Cleanup
  free(): void;
}

export function init(): void;  // Initialize panic hook
```

### Complete Example: Track 2 Visualizer

```typescript
import init, { WasmOrderbook } from '@kraken-forge/wasm';

// State
let orderbook: WasmOrderbook | null = null;
let ws: WebSocket | null = null;

// Initialize
async function initialize(symbol: string = "BTC/USD") {
  // 1. Initialize WASM
  await init();

  // 2. Create orderbook with history for time-travel slider
  orderbook = WasmOrderbook.with_depth(symbol, 25);
  orderbook.enable_history(500);  // Keep 500 snapshots

  // 3. Connect to Kraken
  connectWebSocket(symbol);
}

// WebSocket Connection
function connectWebSocket(symbol: string) {
  ws = new WebSocket('wss://ws.kraken.com/v2');

  ws.onopen = () => {
    console.log('Connected to Kraken');

    // Subscribe to orderbook
    ws.send(JSON.stringify({
      method: "subscribe",
      params: {
        channel: "book",
        symbol: [symbol],
        depth: 25,
        snapshot: true
      }
    }));
  };

  ws.onmessage = (event) => {
    if (!orderbook) return;

    try {
      const msgType = orderbook.apply_message(event.data);

      if (msgType === "snapshot" || msgType === "update") {
        updateVisualization();
      }
    } catch (error) {
      console.error('Failed to process message:', error);
    }
  };

  ws.onclose = () => {
    console.log('Disconnected, reconnecting in 3s...');
    setTimeout(() => connectWebSocket(symbol), 3000);
  };

  ws.onerror = (error) => {
    console.error('WebSocket error:', error);
  };
}

// Update 3D Visualization
function updateVisualization() {
  if (!orderbook || !orderbook.is_synced()) return;

  // Get orderbook data
  const bids = orderbook.get_bids();      // [{price, qty}, ...]
  const asks = orderbook.get_asks();      // [{price, qty}, ...]
  const spread = orderbook.get_spread();
  const midPrice = orderbook.get_mid_price();
  const bestBid = orderbook.get_best_bid();
  const bestAsk = orderbook.get_best_ask();

  // Data for 3D rendering
  const visualData = {
    bids: bids.map(level => ({
      price: level.price,
      quantity: level.qty,
      depth: calculateDepth(level.price, midPrice)
    })),
    asks: asks.map(level => ({
      price: level.price,
      quantity: level.qty,
      depth: calculateDepth(level.price, midPrice)
    })),
    spread,
    midPrice,
    bestBid,
    bestAsk,
    timestamp: Date.now()
  };

  // Send to your Three.js/WebGL renderer
  render3DOrderbook(visualData);
}

// Time-Travel Slider
function scrubHistory(sliderValue: number) {
  if (!orderbook || !orderbook.is_history_enabled()) return;

  const historyLength = orderbook.get_history_length();
  const index = Math.floor((sliderValue / 100) * (historyLength - 1));

  const snapshot = orderbook.get_snapshot_at(index);
  if (snapshot) {
    renderHistoricalSnapshot(snapshot);
  }
}

// Helper
function calculateDepth(price: number, midPrice: number): number {
  return Math.abs(price - midPrice) / midPrice * 100;
}

// Cleanup
function cleanup() {
  if (ws) ws.close();
  if (orderbook) orderbook.free();
}

// Start
initialize("BTC/USD");
```

### Multi-Symbol Support

```typescript
// Create multiple orderbooks
const orderbooks = new Map<string, WasmOrderbook>();

const symbols = ["BTC/USD", "ETH/USD", "SOL/USD"];

symbols.forEach(symbol => {
  const book = WasmOrderbook.with_depth(symbol, 10);
  book.enable_history(100);
  orderbooks.set(symbol, book);
});

// Route messages by symbol
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  if (data.channel === "book" && data.data?.[0]?.symbol) {
    const symbol = data.data[0].symbol;
    const book = orderbooks.get(symbol);

    if (book) {
      book.apply_message(event.data);
    }
  }
};
```

---

## Track 3: Native Rust Integration

### Setup

#### 1. Add to Cargo.toml

```toml
[dependencies]
kraken-sdk = { path = "/Users/akshmnd/Dev Projects/Havklo_sdk/crates/kraken-sdk" }
tokio = { version = "1", features = ["full"] }
rust_decimal = "1.33"
rust_decimal_macros = "1.33"

# Optional: for metrics
# kraken-sdk = { path = "...", features = ["metrics"] }

# Optional: for private channels (auth)
# kraken-sdk = { path = "...", features = ["auth"] }
```

### Types Available

```rust
use kraken_sdk::prelude::*;

// Core Types
pub struct KrakenClient { /* ... */ }
pub struct KrakenClientBuilder { /* ... */ }

// Events
pub enum Event {
    Market(MarketEvent),
    Connection(ConnectionEvent),
    Subscription(SubscriptionEvent),
}

pub enum MarketEvent {
    OrderbookSnapshot { symbol: String, snapshot: OrderbookSnapshot },
    OrderbookUpdate { symbol: String, snapshot: OrderbookSnapshot },
    ChecksumMismatch { symbol: String, expected: u32, computed: u32 },
    Heartbeat,
    Status { system: String, version: String },
}

pub enum ConnectionEvent {
    Connected { api_version: String, connection_id: u64 },
    Disconnected { reason: String },
    Reconnecting { attempt: u32, delay: Duration },
    ReconnectFailed { error: String },
    SubscriptionsRestored { count: usize },
}

pub enum SubscriptionEvent {
    Subscribed { channel: String, symbols: Vec<String> },
    Unsubscribed { channel: String, symbols: Vec<String> },
    Error { channel: String, error: String },
}

// Orderbook Types
pub struct OrderbookSnapshot {
    pub bids: Vec<Level>,
    pub asks: Vec<Level>,
    pub checksum: u32,
    pub sequence: u64,
}

pub struct Level {
    pub price: Decimal,
    pub qty: Decimal,
}

// Enums
pub enum Depth { D10, D25, D100, D500, D1000 }
pub enum Endpoint { Public, PublicBeta }
pub enum Channel { Book, Ticker, Trade, Ohlc, Instrument, Executions, Balances }

// Reconnection
pub struct ReconnectConfig {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: f64,
    pub max_attempts: Option<u32>,
}
```

### Complete Example: Track 3 Strategy Builder

```rust
use kraken_sdk::prelude::*;
use rust_decimal_macros::dec;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Build client with custom config
    let mut client = KrakenClient::builder(["BTC/USD", "ETH/USD"])
        .with_depth(Depth::D25)
        .with_reconnect(true)
        .with_reconnect_config(
            ReconnectConfig::new()
                .with_initial_delay(Duration::from_millis(500))
                .with_max_delay(Duration::from_secs(30))
                .with_max_attempts(10)
        )
        .connect()
        .await?;

    println!("Connected to Kraken!");

    // Get event stream
    let mut events = client.events().expect("events already taken");

    // Strategy state
    let mut btc_mid_price = Decimal::ZERO;
    let mut eth_mid_price = Decimal::ZERO;

    // Event loop
    while let Some(event) = events.recv().await {
        match event {
            Event::Market(MarketEvent::OrderbookSnapshot { symbol, snapshot }) => {
                println!("[SNAPSHOT] {} - {} bids, {} asks",
                    symbol, snapshot.bids.len(), snapshot.asks.len());

                handle_orderbook_update(&symbol, &snapshot, &mut btc_mid_price, &mut eth_mid_price);
            }

            Event::Market(MarketEvent::OrderbookUpdate { symbol, snapshot }) => {
                handle_orderbook_update(&symbol, &snapshot, &mut btc_mid_price, &mut eth_mid_price);

                // Run strategy logic
                run_strategy(&client, &symbol, &snapshot);
            }

            Event::Market(MarketEvent::ChecksumMismatch { symbol, expected, computed }) => {
                eprintln!("[WARN] Checksum mismatch for {}: expected {}, got {}",
                    symbol, expected, computed);
                // Orderbook will automatically resync
            }

            Event::Connection(ConnectionEvent::Disconnected { reason }) => {
                println!("[DISCONNECTED] {}", reason);
            }

            Event::Connection(ConnectionEvent::Reconnecting { attempt, delay }) => {
                println!("[RECONNECTING] Attempt {} in {:?}", attempt, delay);
            }

            Event::Connection(ConnectionEvent::Connected { api_version, .. }) => {
                println!("[CONNECTED] API {}", api_version);
            }

            _ => {}
        }
    }

    client.shutdown();
    Ok(())
}

fn handle_orderbook_update(
    symbol: &str,
    snapshot: &OrderbookSnapshot,
    btc_mid: &mut Decimal,
    eth_mid: &mut Decimal,
) {
    if let Some(mid) = snapshot.mid_price() {
        match symbol {
            "BTC/USD" => *btc_mid = mid,
            "ETH/USD" => *eth_mid = mid,
            _ => {}
        }
    }
}

fn run_strategy(client: &KrakenClient, symbol: &str, snapshot: &OrderbookSnapshot) {
    // Example: Spread-based strategy
    let spread = snapshot.spread().unwrap_or_default();
    let mid = snapshot.mid_price().unwrap_or_default();

    // Strategy logic
    if spread < dec!(0.10) && symbol == "BTC/USD" {
        // Tight spread - potential market making opportunity
        let best_bid = snapshot.bids.first().map(|l| l.price);
        let best_ask = snapshot.asks.first().map(|l| l.price);

        println!("[SIGNAL] {} - Tight spread: ${:.2} | Bid: {:?} Ask: {:?}",
            symbol, spread, best_bid, best_ask);
    }

    // Example: Imbalance detection
    let bid_volume: Decimal = snapshot.bids.iter().take(5).map(|l| l.qty).sum();
    let ask_volume: Decimal = snapshot.asks.iter().take(5).map(|l| l.qty).sum();

    if bid_volume > Decimal::ZERO && ask_volume > Decimal::ZERO {
        let imbalance = (bid_volume - ask_volume) / (bid_volume + ask_volume);

        if imbalance.abs() > dec!(0.3) {
            let direction = if imbalance > Decimal::ZERO { "BUY" } else { "SELL" };
            println!("[SIGNAL] {} - Order imbalance: {:.2}% ({} pressure)",
                symbol, imbalance * dec!(100), direction);
        }
    }
}
```

### Direct Orderbook Access

```rust
// Access orderbook directly (without events)
if let Some(snapshot) = client.orderbook("BTC/USD") {
    println!("BTC/USD Orderbook:");
    println!("  Best Bid: {:?}", snapshot.best_bid());
    println!("  Best Ask: {:?}", snapshot.best_ask());
    println!("  Spread: {:?}", snapshot.spread());
    println!("  Mid Price: {:?}", snapshot.mid_price());
    println!("  Synced: {}", snapshot.is_synced());
}

// Convenience methods
let spread = client.spread("BTC/USD");
let mid = client.mid_price("BTC/USD");
let best_bid = client.best_bid("BTC/USD");
let best_ask = client.best_ask("BTC/USD");
```

### Authentication (Private Channels)

```rust
// Enable auth feature in Cargo.toml:
// kraken-sdk = { path = "...", features = ["auth"] }

use kraken_sdk::auth::TokenManager;

// Set environment variables:
// export KRAKEN_API_KEY="your_api_key"
// export KRAKEN_PRIVATE_KEY="your_private_key"

async fn with_auth() -> Result<(), Box<dyn std::error::Error>> {
    // Get WebSocket token
    let token_manager = TokenManager::from_env()?;
    let token = token_manager.get_token().await?;

    println!("Got WebSocket token: {}...", &token[..20]);

    // Use token for private channel subscriptions
    // (Private channel integration coming soon)

    Ok(())
}
```

---

## API Reference

### OrderbookSnapshot Methods

| Method | Return Type | Description |
|--------|-------------|-------------|
| `best_bid()` | `Option<&Level>` | Highest bid price level |
| `best_ask()` | `Option<&Level>` | Lowest ask price level |
| `spread()` | `Option<Decimal>` | Ask - Bid |
| `mid_price()` | `Option<Decimal>` | (Ask + Bid) / 2 |
| `is_synced()` | `bool` | True if orderbook is valid |

### KrakenClient Methods

| Method | Description |
|--------|-------------|
| `builder(symbols)` | Create new client builder |
| `connect().await` | Connect to Kraken WebSocket |
| `events()` | Get event receiver (can only call once) |
| `orderbook(symbol)` | Get current orderbook snapshot |
| `spread(symbol)` | Get current spread |
| `mid_price(symbol)` | Get current mid price |
| `best_bid(symbol)` | Get best bid price |
| `best_ask(symbol)` | Get best ask price |
| `shutdown()` | Gracefully disconnect |

### KrakenClientBuilder Methods

| Method | Description |
|--------|-------------|
| `with_depth(Depth)` | Set orderbook depth (D10, D25, D100, D500, D1000) |
| `with_endpoint(Endpoint)` | Set endpoint (Public, PublicBeta) |
| `with_reconnect(bool)` | Enable/disable auto-reconnect |
| `with_reconnect_config(ReconnectConfig)` | Custom reconnection policy |
| `with_timeout(Duration)` | Connection timeout |

---

## WebSocket Message Format

### Kraken WebSocket v2 Endpoint

```
wss://ws.kraken.com/v2
```

### Subscribe Request

```json
{
  "method": "subscribe",
  "params": {
    "channel": "book",
    "symbol": ["BTC/USD"],
    "depth": 10,
    "snapshot": true
  }
}
```

### Book Snapshot Message

```json
{
  "channel": "book",
  "type": "snapshot",
  "data": [{
    "symbol": "BTC/USD",
    "bids": [
      {"price": 88000.50, "qty": 1.5},
      {"price": 88000.00, "qty": 2.0}
    ],
    "asks": [
      {"price": 88001.00, "qty": 1.0},
      {"price": 88001.50, "qty": 0.5}
    ],
    "checksum": 1234567890
  }]
}
```

### Book Update Message

```json
{
  "channel": "book",
  "type": "update",
  "data": [{
    "symbol": "BTC/USD",
    "bids": [{"price": 88000.75, "qty": 0.8}],
    "asks": [],
    "checksum": 987654321,
    "timestamp": "2025-12-22T12:00:00.000Z"
  }]
}
```

---

## Common Patterns

### Error Handling

```rust
// Rust
match client.connect().await {
    Ok(client) => { /* connected */ }
    Err(e) => eprintln!("Connection failed: {}", e),
}
```

```typescript
// TypeScript
try {
  const msgType = orderbook.apply_message(data);
} catch (error) {
  console.error('Parse error:', error);
  orderbook.reset();
}
```

### Graceful Shutdown

```rust
// Rust
tokio::select! {
    _ = tokio::signal::ctrl_c() => {
        println!("Shutting down...");
        client.shutdown();
    }
    event = events.recv() => {
        // handle event
    }
}
```

```typescript
// TypeScript
window.addEventListener('beforeunload', () => {
  if (ws) ws.close();
  if (orderbook) orderbook.free();
});
```

### Checksum Validation

The SDK automatically validates CRC32 checksums. If a mismatch occurs:
- **Rust**: You receive `MarketEvent::ChecksumMismatch` event
- **WASM**: `apply_message()` throws an error

The orderbook state becomes `Desynchronized` and will resync on next snapshot.

---

## Quick Reference

### Build Commands

```bash
# SDK Location
cd "/Users/akshmnd/Dev Projects/Havklo_sdk"

# Build everything
cargo build --release --workspace

# Build WASM only
cd crates/kraken-wasm && wasm-pack build --target web --release

# Run tests
cargo test --workspace

# Run example
cargo run --example orderbook_stream -p kraken-sdk
```

### File Paths

| Resource | Path |
|----------|------|
| SDK Root | `/Users/akshmnd/Dev Projects/Havklo_sdk` |
| WASM Package | `/Users/akshmnd/Dev Projects/Havklo_sdk/crates/kraken-wasm/pkg` |
| Rust SDK | `/Users/akshmnd/Dev Projects/Havklo_sdk/crates/kraken-sdk` |
| Examples | `/Users/akshmnd/Dev Projects/Havklo_sdk/crates/kraken-sdk/examples` |
| TypeScript Types | `/Users/akshmnd/Dev Projects/Havklo_sdk/crates/kraken-wasm/pkg/kraken_wasm.d.ts` |

---

*Last updated: December 22, 2025*
