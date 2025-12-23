# Havklo SDK Quick Start Guide

This guide will get you up and running with Havklo SDK in minutes.

## Installation

Add the SDK to your `Cargo.toml`:

```toml
[dependencies]
# Core SDK with WebSocket support
kraken-sdk = { git = "https://github.com/havklo/havklo-sdk" }
tokio = { version = "1", features = ["full"] }

# Optional: REST API client
kraken-rest = { git = "https://github.com/havklo/havklo-sdk" }

# Optional: Futures WebSocket
kraken-futures-ws = { git = "https://github.com/havklo/havklo-sdk" }
```

## Basic WebSocket Streaming

Stream real-time orderbook data:

```rust
use kraken_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client for BTC/USD with depth 10
    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .connect()
        .await?;

    let mut events = client.events();

    // Process events
    while let Some(event) = events.recv().await {
        match event {
            Event::Market(MarketEvent::OrderbookUpdate { symbol, .. }) => {
                println!(
                    "{}: bid={} ask={} spread={}",
                    symbol,
                    client.best_bid(&symbol).unwrap_or_default(),
                    client.best_ask(&symbol).unwrap_or_default(),
                    client.spread(&symbol).unwrap_or_default()
                );
            }
            Event::Connection(ConnectionEvent::Connected { .. }) => {
                println!("Connected to Kraken!");
            }
            _ => {}
        }
    }

    Ok(())
}
```

## REST API Trading

Get market data and place orders:

```rust
use kraken_rest::{KrakenRestClient, Credentials};
use kraken_rest::types::{OrderRequest, OrderSide};
use rust_decimal_macros::dec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Public endpoints (no auth needed)
    let client = KrakenRestClient::new();

    // Get BTC price
    let tickers = client.get_ticker("XBTUSD").await?;
    if let Some(ticker) = tickers.values().next() {
        println!("BTC price: ${}", ticker.last_price().unwrap());
    }

    // Get orderbook
    let orderbooks = client.get_orderbook("XBTUSD", Some(10)).await?;

    // For trading, you need credentials
    let auth_client = KrakenRestClient::with_credentials(
        Credentials::new("your-api-key", "your-api-secret")?
    );

    // Get account balance
    let balances = auth_client.get_balance().await?;
    for (asset, balance) in balances.iter() {
        if !balance.is_zero() {
            println!("{}: {}", asset, balance);
        }
    }

    // Place a limit order
    let order = OrderRequest::limit("XBTUSD", OrderSide::Buy, dec!(0.001), dec!(50000))
        .post_only();  // Maker only

    let result = auth_client.add_order(&order).await?;
    println!("Order placed: {:?}", result.txid);

    Ok(())
}
```

## Using Environment Variables

Store credentials safely:

```bash
export KRAKEN_API_KEY="your-key"
export KRAKEN_API_SECRET="your-secret"
```

```rust
use kraken_rest::{KrakenRestClient, Credentials};

let creds = Credentials::from_env()?;
let client = KrakenRestClient::with_credentials(creds);
```

## L3 Orderbook (Market Making)

Track individual orders and queue position:

```rust
use kraken_book::l3::{L3Book, L3Order, L3Side};
use rust_decimal_macros::dec;

let mut book = L3Book::new("BTC/USD", 100);

// Add your order
book.add_order(
    L3Order::new("my_order_123", dec!(50000), dec!(1.5)),
    L3Side::Bid
);

// Check queue position
if let Some(pos) = book.queue_position("my_order_123") {
    println!("Position in queue: {}", pos.position);
    println!("Quantity ahead: {}", pos.qty_ahead);
    println!("Fill probability: {:.1}%", pos.fill_probability() * 100.0);
}
```

## Futures Trading

Stream perpetual swap data:

```rust
use kraken_futures_ws::{FuturesConnection, FuturesConfig, FuturesEvent};
use rust_decimal_macros::dec;

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
                println!(
                    "Mark: ${}, Funding: {}%",
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

## Rate Limiting

The SDK includes built-in rate limiting:

```rust
use kraken_rest::{KrakenRestClient, client::ClientConfig};

// Rate limiting enabled by default
let client = KrakenRestClient::new();

// Or disable it
let config = ClientConfig::new().with_rate_limiting(false);
let client = KrakenRestClient::with_config(config);

// Check rate limiter status
if let Some(limiter) = client.rate_limiter() {
    println!("Public tokens available: {}", limiter.available_public());
    println!("Private tokens available: {}", limiter.available_private());
}
```

## Error Handling

Handle Kraken API errors:

```rust
use kraken_rest::RestError;
use kraken_types::error_codes::KrakenApiError;

match client.get_balance().await {
    Ok(balances) => println!("Balance: {:?}", balances),
    Err(RestError::AuthRequired) => {
        println!("Need to provide API credentials");
    }
    Err(RestError::ApiError(errors)) => {
        for error in &errors {
            let api_error = KrakenApiError::parse(error);
            match api_error.recovery_strategy() {
                RecoveryStrategy::Backoff { .. } => {
                    println!("Rate limited, backing off...");
                }
                RecoveryStrategy::Fatal => {
                    println!("Fatal error: {}", error);
                }
                _ => {}
            }
        }
    }
    Err(e) => println!("Error: {:?}", e),
}
```

## Next Steps

- See [REST_API.md](./REST_API.md) for complete REST endpoint documentation
- See [WASM.md](./WASM.md) for browser integration (includes L2/L3 orderbook and WebSocket examples)
- See the main [README](../README.md) for architecture overview and advanced usage
- Run examples: `cargo run --example simple_ticker`
