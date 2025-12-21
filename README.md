# Havklo SDK

Production-grade Rust SDK for Kraken's WebSocket API v2 with WASM support.

## Features

- Real-time market data feeds (orderbook, trades, ticker)
- Orderbook state management with CRC32 checksum validation
- Automatic reconnection with exponential backoff
- WASM-compatible orderbook engine for browser applications
- High-level, ergonomic API

## Crate Structure

| Crate | Description |
|-------|-------------|
| `kraken-types` | Zero-dependency shared types |
| `kraken-book` | WASM-compatible orderbook engine |
| `kraken-ws` | Native WebSocket client (tokio) |
| `kraken-sdk` | High-level unified API |
| `kraken-wasm` | Browser WASM bindings |

## Quick Start

```rust
use kraken_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KrakenClient::builder()
        .symbols(["BTC/USD", "ETH/USD"])
        .depth(Depth::D10)
        .build()
        .await?;

    while let Some(event) = client.events().next().await {
        match event? {
            Event::OrderbookUpdate { symbol, .. } => {
                println!("{}: spread = {:?}", symbol, client.spread(&symbol));
            }
            _ => {}
        }
    }

    Ok(())
}
```

## WASM Usage

```javascript
import init, { WasmOrderbook } from 'kraken-wasm';

await init();

const book = new WasmOrderbook('BTC/USD');
book.enable_history(100);

ws.onmessage = (event) => {
    book.apply_message(event.data);
    console.log('Spread:', book.get_spread());
};
```

## Building

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build WASM package
cd crates/kraken-wasm
wasm-pack build --target web --out-dir ../../pkg
```

## License

MIT License - see [LICENSE](LICENSE) for details.
