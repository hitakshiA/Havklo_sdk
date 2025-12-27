# kraken-sdk

High-level Rust SDK for Kraken WebSocket API v2 with sub-microsecond orderbook operations.

## Quick Start

Add to Cargo.toml:
```toml
[dependencies]
kraken-sdk = "0.1"
tokio = { version = "1", features = ["full"] }
```

Example code:
```rust
use kraken_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KrakenClient::builder(vec!["BTC/USD".into()])
        .with_depth(Depth::D10)
        .with_book(true)
        .connect()
        .await?;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        if let Some(spread) = client.spread("BTC/USD") {
            println!("Spread: {}", spread);
        }
    }
}
```

## Features

* **Sub-microsecond orderbook** — 3.5ns spread calculation, 100ns delta updates
* **L2 + L3 orderbooks** — Price levels and individual order tracking
* **CRC32 validation** — Automatic checksum verification on every update
* **Auto-reconnect** — Exponential backoff with circuit breaker
* **Financial precision** — rust_decimal throughout (no floating point errors)

## Optional Features
```toml
# Prometheus metrics
kraken-sdk = { version = "0.1", features = ["metrics"] }

# Authenticated trading
kraken-sdk = { version = "0.1", features = ["auth"] }
```

## Documentation & Examples

* **Full Documentation:** [https://miny.mintlify.app](https://miny.mintlify.app)
* **Example Integrations:** [https://github.com/hitakshiA/Havklo_sdk](https://github.com/hitakshiA/Havklo_sdk)

## Part of Havklo

This crate is part of the [Havklo SDK](https://github.com/hitakshiA/Havklo_sdk) workspace.

## License

MIT
