//! Example: Using the Stream API with tokio::select!
//!
//! This example demonstrates using the `futures::Stream` trait implementation
//! for event processing with `tokio::select!` and other async patterns.
//!
//! Run with: cargo run --example stream_api

use kraken_sdk::prelude::*;
use std::time::Duration;
use std::pin::pin;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Stream API Example ===\n");
    println!("Demonstrating futures::Stream with tokio::select!\n");

    // Create and connect client
    let mut client = KrakenClient::builder(["BTC/USD", "ETH/USD"])
        .with_depth(Depth::D10)
        .connect()
        .await?;

    println!("Connected! Streaming events...\n");

    // Get event receiver as a Stream
    let events = client.events().expect("events() already called");

    // Example 1: Using StreamExt::next() with pin!
    println!("--- Processing with Stream::next() ---\n");

    // Pin the stream for use with combinators
    let mut pinned_events = pin!(events);

    let mut update_count = 0;
    let timeout = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                println!("\nTimeout reached after {} updates", update_count);
                break;
            }
            // Using Stream::next() instead of recv()
            event = pinned_events.next() => {
                match event {
                    Some(Event::Market(MarketEvent::OrderbookUpdate { symbol, snapshot })) => {
                        update_count += 1;
                        if update_count % 10 == 0 {
                            let spread = snapshot.spread().unwrap_or_default();
                            println!(
                                "[{:3}] {} | Spread: ${:.4}",
                                update_count, symbol, spread
                            );
                        }
                    }
                    Some(Event::Market(MarketEvent::OrderbookSnapshot { symbol, .. })) => {
                        println!("[SNAPSHOT] {}", symbol);
                    }
                    Some(Event::Connection(ConnectionEvent::Connected { api_version, .. })) => {
                        println!("[CONNECTED] API {}", api_version);
                    }
                    Some(Event::Subscription(SubscriptionEvent::Subscribed { channel, symbols })) => {
                        println!("[SUBSCRIBED] {} for {:?}", channel, symbols);
                    }
                    None => {
                        println!("Stream ended");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    println!("\n--- Stream Benefits ---\n");
    println!("The Stream trait enables:");
    println!("  - Integration with tokio::select!");
    println!("  - Compatibility with async ecosystem");
    println!("  - Flexible event processing patterns");
    println!("  - Standard Rust async idioms");

    println!("\n--- Example complete ---");

    client.shutdown();
    Ok(())
}
