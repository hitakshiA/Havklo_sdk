# REST API Guide

Complete guide to using the Kraken REST API client.

## Overview

The `kraken-rest` crate provides a full-featured REST client for Kraken's trading API.

## Public Endpoints (No Authentication)

### Server Time

```rust
let time = client.market().get_server_time().await?;
println!("Unix time: {}", time.unixtime);
println!("RFC1123: {}", time.rfc1123);
```

### System Status

```rust
let status = client.market().get_system_status().await?;
println!("Status: {}", status.status);  // "online", "maintenance", etc.
```

### Asset Information

```rust
// All assets
let assets = client.market().get_assets(None).await?;

// Specific assets
let assets = client.market().get_assets(Some(&["XBT", "ETH"])).await?;

for (name, info) in &assets {
    println!("{}: {} decimals", name, info.decimals);
}
```

### Trading Pairs

```rust
// All pairs
let pairs = client.market().get_asset_pairs(None).await?;

// Specific pairs
let pairs = client.market().get_asset_pairs(Some(&["XBTUSD", "ETHUSD"])).await?;

for (name, info) in &pairs {
    println!("{}: base={}, quote={}", name, info.base, info.quote);
}
```

### Ticker Information

```rust
// Single pair
let tickers = client.get_ticker("XBTUSD").await?;

// Multiple pairs
let tickers = client.get_tickers(&["XBTUSD", "ETHUSD"]).await?;

for (pair, ticker) in &tickers {
    println!("{}: bid={}, ask={}, last={}",
        pair,
        ticker.bid_price().unwrap(),
        ticker.ask_price().unwrap(),
        ticker.last_price().unwrap()
    );
}
```

### Orderbook

```rust
// Get top 10 levels
let books = client.get_orderbook("XBTUSD", Some(10)).await?;

for (pair, book) in &books {
    println!("Best bid: {}", book.best_bid().unwrap());
    println!("Best ask: {}", book.best_ask().unwrap());
    println!("Spread: {}", book.spread().unwrap());
}
```

### OHLC Data

```rust
// 60-minute candles
let result = client.market().get_ohlc("XBTUSD", Some(60), None).await?;

// With since parameter for pagination
let result = client.market().get_ohlc("XBTUSD", Some(60), Some(1700000000)).await?;
```

### Recent Trades

```rust
let result = client.market().get_recent_trades("XBTUSD", None, None).await?;
println!("Last trade ID: {:?}", result.last);
```

### Spread Data

```rust
let result = client.market().get_recent_spreads("XBTUSD", None).await?;
```

## Private Endpoints (Authentication Required)

### Creating Authenticated Client

```rust
use kraken_rest::{KrakenRestClient, Credentials};

// From values
let creds = Credentials::new("api-key", "api-secret")?;
let client = KrakenRestClient::with_credentials(creds);

// From environment variables
let creds = Credentials::from_env()?;  // Uses KRAKEN_API_KEY, KRAKEN_API_SECRET
let client = KrakenRestClient::with_credentials(creds);
```

### Account Balance

```rust
let balances = client.get_balance().await?;

// Get specific asset
if let Some(btc) = balances.get("XXBT") {
    println!("BTC balance: {}", btc);
}

// Get all non-zero balances
for (asset, balance) in balances.non_zero() {
    println!("{}: {}", asset, balance);
}
```

### Open Orders

```rust
let orders = client.get_open_orders().await?;

for (txid, order) in &orders.open {
    println!("Order {}: {} {} @ {}",
        txid,
        order.descr.side,
        order.vol,
        order.descr.price
    );
}
```

## Trading Operations

### Place Market Order

```rust
use kraken_rest::types::{OrderRequest, OrderSide};
use rust_decimal_macros::dec;

let order = OrderRequest::market("XBTUSD", OrderSide::Buy, dec!(0.001));
let result = client.add_order(&order).await?;
println!("Order ID: {:?}", result.txid);
```

### Place Limit Order

```rust
let order = OrderRequest::limit("XBTUSD", OrderSide::Buy, dec!(0.001), dec!(50000));
let result = client.add_order(&order).await?;
```

### Place Post-Only Order (Maker)

```rust
let order = OrderRequest::limit("XBTUSD", OrderSide::Buy, dec!(0.001), dec!(50000))
    .post_only();
let result = client.add_order(&order).await?;
```

### Place Stop-Loss Order

```rust
let order = OrderRequest::stop_loss("XBTUSD", OrderSide::Sell, dec!(0.001), dec!(45000));
let result = client.add_order(&order).await?;
```

### Validate Order (Don't Submit)

```rust
let order = OrderRequest::limit("XBTUSD", OrderSide::Buy, dec!(0.001), dec!(50000))
    .validate_only();
let result = client.add_order(&order).await?;
// Order is validated but not submitted
```

### Cancel Order

```rust
// Cancel by transaction ID
let result = client.cancel_order("OXXXXX-XXXXX-XXXXXX").await?;
println!("Cancelled {} orders", result.count);
```

### Cancel All Orders

```rust
let result = client.cancel_all_orders().await?;
println!("Cancelled {} orders", result.count);
```

### Edit Order

```rust
let result = client.edit_order(
    "OXXXXX-XXXXX-XXXXXX",  // txid
    "XBTUSD",               // pair
    Some("0.002"),          // new volume
    Some("51000"),          // new price
).await?;
```

## Funding Operations

### Get Deposit Methods

```rust
let methods = client.get_deposit_methods("XBT").await?;
for method in &methods {
    println!("Method: {}, Fee: {:?}", method.method, method.fee);
}
```

## Earn (Staking)

### List Staking Strategies

```rust
let strategies = client.list_earn_strategies(Some("ETH")).await?;
for strategy in &strategies.items {
    println!("Strategy: {}, APR: {:?}", strategy.id, strategy.apr);
}
```

### Allocate to Staking

```rust
use rust_decimal_macros::dec;

let result = client.allocate_earn("strategy-id", dec!(1.0)).await?;
```

### Deallocate from Staking

```rust
let result = client.deallocate_earn("strategy-id", dec!(1.0)).await?;
```

## Rate Limiting

The client includes built-in rate limiting:

```rust
use kraken_rest::{KrakenRestClient, client::ClientConfig};

// Enable rate limiting (default)
let config = ClientConfig::new().with_rate_limiting(true);
let client = KrakenRestClient::with_config(config);

// Check rate limiter
if let Some(limiter) = client.rate_limiter() {
    // Check if we can make a public request
    if limiter.try_acquire_public().is_ok() {
        // Token acquired, make request
    }

    // Or wait for availability
    limiter.acquire_public().await;
}
```

## Error Handling

```rust
use kraken_rest::RestError;

match client.get_balance().await {
    Ok(balances) => { /* handle success */ }
    Err(RestError::AuthRequired) => {
        println!("Need credentials");
    }
    Err(RestError::ApiError(errors)) => {
        for error in &errors {
            println!("API Error: {}", error);
        }
    }
    Err(RestError::RequestFailed(e)) => {
        println!("Network error: {}", e);
    }
    Err(RestError::ParseError(e)) => {
        println!("Parse error: {}", e);
    }
}
```

## Configuration

```rust
use kraken_rest::client::ClientConfig;

let config = ClientConfig::new()
    .with_timeout(60)                    // 60 second timeout
    .with_user_agent("my-trading-bot")   // Custom user agent
    .with_rate_limiting(true)            // Enable rate limiting
    .with_credentials(creds);            // Add credentials

let client = KrakenRestClient::with_config(config);
```

## Best Practices

1. **Use rate limiting**: Always enable rate limiting in production
2. **Handle errors**: Implement retry logic for transient errors
3. **Validate orders**: Use `validate_only()` during development
4. **Secure credentials**: Use environment variables, never hardcode
5. **Use post-only**: For limit orders, use `.post_only()` to ensure maker fees
