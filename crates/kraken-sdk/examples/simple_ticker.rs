//! Simple example: Connect and print orderbook spread
//!
//! Run with: cargo run --example simple_ticker

use kraken_sdk::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Connecting to Kraken WebSocket API...");

    // Create and connect client
    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .connect()
        .await?;

    println!("Connected! Waiting for orderbook data...");

    // Get event stream
    let mut events = client.events();

    // Process events for 10 seconds
    let timeout = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                println!("\nTimeout reached. Shutting down...");
                break;
            }
            event = events.recv() => {
                match event {
                    Some(Event::Market(MarketEvent::OrderbookSnapshot { symbol, .. })) => {
                        println!("\nReceived snapshot for {}", symbol);
                        print_orderbook_info(&client, &symbol);
                    }
                    Some(Event::Market(MarketEvent::OrderbookUpdate { symbol, .. })) => {
                        print_orderbook_info(&client, &symbol);
                    }
                    Some(Event::Connection(ConnectionEvent::Connected { api_version, .. })) => {
                        println!("Connected to Kraken API {}", api_version);
                    }
                    Some(Event::Subscription(SubscriptionEvent::Subscribed { channel, symbols })) => {
                        println!("Subscribed to {} for {:?}", channel, symbols);
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }

    client.shutdown();
    Ok(())
}

fn print_orderbook_info(client: &KrakenClient, symbol: &str) {
    if let (Some(bid), Some(ask)) = (client.best_bid(symbol), client.best_ask(symbol)) {
        let spread = client.spread(symbol).unwrap_or_default();
        let mid = client.mid_price(symbol).unwrap_or_default();
        println!(
            "{}: bid={:.2} ask={:.2} spread={:.2} mid={:.2}",
            symbol, bid, ask, spread, mid
        );
    }
}
