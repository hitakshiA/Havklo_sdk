# Integration Guide for Track 2 & Track 3

> This document provides everything needed to integrate the Havklo SDK into Track 2 (Orderbook Visualizer) and Track 3 (Strategy Builder).

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
crates/
├── kraken-types/     # Shared types (Decimal, Level, Symbol, Messages)
├── kraken-book/      # Orderbook engine (WASM-compatible, no async)
├── kraken-ws/        # WebSocket client (native only, tokio)
├── kraken-sdk/       # High-level API (native only)
└── kraken-wasm/      # WASM bindings for browser
    └── pkg/          # Built WASM package (npm-ready)
```

### Which Crate to Use

| Track | Crate | Description |
|-------|-------|-------------|
| Track 2 (Browser) | `kraken-wasm` | WASM bindings for JavaScript |
| Track 3 (Native Rust) | `kraken-sdk` | High-level Rust API |

---

## Track 2: WASM Integration (Browser)

### Setup

#### 1. Build WASM package

```bash
cd crates/kraken-wasm
wasm-pack build --target web
```

#### 2. Add dependency to package.json

```json
{
  "dependencies": {
    "@kraken-forge/wasm": "file:path/to/crates/kraken-wasm/pkg"
  }
}
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

  // Getters - Prices
  get_best_bid(): number;
  get_best_ask(): number;
  get_spread(): number;
  get_mid_price(): number;

  // Getters - State
  get_symbol(): string;
  get_state(): string;  // "Uninitialized" | "AwaitingSnapshot" | "Synced" | "Desynchronized"
  is_synced(): boolean;

  // History (Time-Travel Feature)
  enable_history(max_snapshots: number): void;
  get_history_length(): number;
  get_snapshot_at(index: number): {bids: Array, asks: Array, checksum: number} | null;

  // Cleanup
  free(): void;
}

export function init(): void;
```

### Complete Example: Track 2 Visualizer

```typescript
import init, { WasmOrderbook } from '@kraken-forge/wasm';

let orderbook: WasmOrderbook | null = null;
let ws: WebSocket | null = null;

async function initialize(symbol: string = "BTC/USD") {
  await init();
  orderbook = WasmOrderbook.with_depth(symbol, 25);
  orderbook.enable_history(500);
  connectWebSocket(symbol);
}

function connectWebSocket(symbol: string) {
  ws = new WebSocket('wss://ws.kraken.com/v2');

  ws.onopen = () => {
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
    setTimeout(() => connectWebSocket(symbol), 3000);
  };
}

function updateVisualization() {
  if (!orderbook || !orderbook.is_synced()) return;

  const bids = orderbook.get_bids();
  const asks = orderbook.get_asks();
  const spread = orderbook.get_spread();
  const midPrice = orderbook.get_mid_price();

  // Send to your Three.js/WebGL renderer
  render3DOrderbook({ bids, asks, spread, midPrice });
}

function cleanup() {
  if (ws) ws.close();
  if (orderbook) orderbook.free();
}

initialize("BTC/USD");
```

---

## Track 3: Native Rust Integration

### Setup

```toml
[dependencies]
kraken-sdk = { git = "https://github.com/havklo/havklo-sdk" }
tokio = { version = "1", features = ["full"] }
rust_decimal = "1.33"
rust_decimal_macros = "1.33"
```

### Types Available

```rust
use kraken_sdk::prelude::*;

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
}

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

pub enum Depth { D10, D25, D100, D500, D1000 }
```

### Complete Example: Track 3 Strategy Builder

```rust
use kraken_sdk::prelude::*;
use rust_decimal_macros::dec;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = KrakenClient::builder(["BTC/USD", "ETH/USD"])
        .with_depth(Depth::D25)
        .with_reconnect(true)
        .connect()
        .await?;

    let mut events = client.events().expect("events already taken");

    while let Some(event) = events.recv().await {
        match event {
            Event::Market(MarketEvent::OrderbookUpdate { symbol, snapshot }) => {
                let spread = snapshot.spread().unwrap_or_default();
                let bid_vol: Decimal = snapshot.bids.iter().take(5).map(|l| l.qty).sum();
                let ask_vol: Decimal = snapshot.asks.iter().take(5).map(|l| l.qty).sum();

                if bid_vol + ask_vol > Decimal::ZERO {
                    let imbalance = (bid_vol - ask_vol) / (bid_vol + ask_vol);
                    if imbalance.abs() > dec!(0.3) {
                        println!("[SIGNAL] {} imbalance: {:.2}%", symbol, imbalance * dec!(100));
                    }
                }
            }
            Event::Connection(ConnectionEvent::Disconnected { reason }) => {
                println!("[DISCONNECTED] {}", reason);
            }
            _ => {}
        }
    }

    client.shutdown();
    Ok(())
}
```

### Direct Orderbook Access

```rust
// Access orderbook directly (without events)
if let Some(snapshot) = client.orderbook("BTC/USD") {
    println!("Best Bid: {:?}", snapshot.best_bid());
    println!("Spread: {:?}", snapshot.spread());
}

// Convenience methods
let spread = client.spread("BTC/USD");
let mid = client.mid_price("BTC/USD");
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
| `shutdown()` | Gracefully disconnect |

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
    "bids": [{"price": 88000.50, "qty": 1.5}],
    "asks": [{"price": 88001.00, "qty": 1.0}],
    "checksum": 1234567890
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
  orderbook.apply_message(data);
} catch (error) {
  console.error('Parse error:', error);
  orderbook.reset();
}
```

### Graceful Shutdown

```rust
tokio::select! {
    _ = tokio::signal::ctrl_c() => {
        client.shutdown();
    }
    event = events.recv() => { /* handle */ }
}
```

```typescript
window.addEventListener('beforeunload', () => {
  if (ws) ws.close();
  if (orderbook) orderbook.free();
});
```

### Checksum Validation

The SDK automatically validates CRC32 checksums. If a mismatch occurs:
- **Rust**: You receive `MarketEvent::ChecksumMismatch` event
- **WASM**: `apply_message()` throws an error

---

## Build Commands

```bash
# Build everything
cargo build --release --workspace

# Build WASM only
cd crates/kraken-wasm && wasm-pack build --target web --release

# Run tests
cargo test --workspace

# Run example
cargo run --example orderbook_stream -p kraken-sdk
```
