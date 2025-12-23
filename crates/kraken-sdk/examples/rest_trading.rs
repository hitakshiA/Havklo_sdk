//! Example: REST API trading operations
//!
//! This example demonstrates how to use the REST API for:
//! - Fetching market data (ticker, orderbook)
//! - Checking account balances
//! - Placing and managing orders
//!
//! Run with: cargo run --example rest_trading
//!
//! NOTE: For actual trading, set KRAKEN_API_KEY and KRAKEN_API_SECRET environment variables.

use kraken_rest::{Credentials, KrakenRestClient};
use kraken_rest::types::{OrderRequest, OrderSide};
use rust_decimal_macros::dec;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Kraken REST API Example ===\n");

    // Create client (no credentials for public endpoints)
    let client = KrakenRestClient::new();

    // ========================================================================
    // PUBLIC ENDPOINTS - No authentication required
    // ========================================================================

    println!("--- Public Market Data ---\n");

    // Get ticker for BTC/USD
    println!("Fetching BTC/USD ticker...");
    match client.get_ticker("XBTUSD").await {
        Ok(tickers) => {
            if let Some(ticker) = tickers.get("XXBTZUSD") {
                if let Some(last) = ticker.last_price() {
                    println!("  Last Price: ${}", last);
                }
                if let Some(bid) = ticker.bid_price() {
                    println!("  Best Bid:   ${}", bid);
                }
                if let Some(ask) = ticker.ask_price() {
                    println!("  Best Ask:   ${}", ask);
                }
                if let Some(mid) = ticker.mid_price() {
                    println!("  Mid Price:  ${}", mid);
                }
                if let Some(spread_bps) = ticker.spread_bps() {
                    println!("  Spread:     {:.1} bps", spread_bps);
                }
            } else {
                println!("  No ticker data for XBTUSD");
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Get multiple tickers
    println!("Fetching multiple tickers...");
    match client.get_tickers(&["XBTUSD", "ETHUSD"]).await {
        Ok(tickers) => {
            for (symbol, ticker) in tickers {
                if let Some(last) = ticker.last_price() {
                    println!("  {}: ${}", symbol, last);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Get orderbook
    println!("Fetching BTC/USD orderbook (depth=10)...");
    match client.get_orderbook("XBTUSD", Some(10)).await {
        Ok(books) => {
            if let Some(book) = books.values().next() {
                println!("  Top Bids:");
                for (i, level) in book.bids.iter().take(3).enumerate() {
                    // level is Vec<String> with [price, volume, timestamp]
                    if level.len() >= 2 {
                        println!("    {}. ${} x {}", i + 1, level[0], level[1]);
                    }
                }
                println!("  Top Asks:");
                for (i, level) in book.asks.iter().take(3).enumerate() {
                    if level.len() >= 2 {
                        println!("    {}. ${} x {}", i + 1, level[0], level[1]);
                    }
                }
                if let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask()) {
                    println!("  Spread: ${}", ask - bid);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // ========================================================================
    // PRIVATE ENDPOINTS - Authentication required
    // ========================================================================

    // Check for API credentials
    let api_key = env::var("KRAKEN_API_KEY").ok();
    let api_secret = env::var("KRAKEN_API_SECRET").ok();

    if let (Some(key), Some(secret)) = (api_key, api_secret) {
        println!("--- Private Account Data ---\n");

        // Create authenticated client
        let auth_client = KrakenRestClient::with_credentials(
            Credentials::new(key, secret)?
        );

        // Get account balance
        println!("Fetching account balances...");
        match auth_client.get_balance().await {
            Ok(balances) => {
                println!("  Balances:");
                // Use non_zero() to get only balances > 0
                for (asset, balance) in balances.non_zero().iter().take(10) {
                    println!("    {}: {}", asset, balance);
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();

        // Get open orders
        println!("Fetching open orders...");
        match auth_client.get_open_orders().await {
            Ok(result) => {
                if result.open.is_empty() {
                    println!("  No open orders");
                } else {
                    for (txid, order) in result.open.iter().take(5) {
                        println!("  {} - {} {} {}",
                            txid,
                            order.descr.side,
                            order.descr.pair,
                            order.descr.price
                        );
                    }
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
        println!();

        // Example: Place a limit order (validate only - won't execute)
        println!("--- Order Example (validate only) ---\n");

        let order = OrderRequest::limit("XBTUSD", OrderSide::Buy, dec!(0.001), dec!(30000))
            .validate_only();

        println!("Order request (validate only):");
        println!("  Pair: {}", order.pair);
        println!("  Type: {:?}", order.order_type);
        println!("  Side: {:?}", order.side);
        println!("  Volume: {}", order.volume);
        println!("  Price: ${}", order.price.unwrap_or_default());

        match auth_client.add_order(&order).await {
            Ok(response) => {
                println!("\n  Validation passed!");
                println!("  Description: {}", response.descr.order);
            }
            Err(e) => println!("\n  Validation error: {}", e),
        }

    } else {
        println!("--- Private Endpoints Skipped ---");
        println!("Set KRAKEN_API_KEY and KRAKEN_API_SECRET to test private endpoints.");
        println!();
        println!("Example:");
        println!("  export KRAKEN_API_KEY='your-api-key'");
        println!("  export KRAKEN_API_SECRET='your-api-secret'");
    }

    println!("\nDone!");
    Ok(())
}
