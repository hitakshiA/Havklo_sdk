//! Example: Stream orderbook updates with full details
//!
//! Run with: cargo run --example orderbook_stream

use kraken_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Kraken Orderbook Stream ===\n");

    // Create client with deeper orderbook
    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D25)
        .connect()
        .await?;

    println!("Streaming orderbook updates for BTC/USD (depth: 25)...\n");

    let mut events = client.events().expect("events() already called");
    let mut update_count = 0;
    let max_updates = 50;

    while let Some(event) = events.recv().await {
        match event {
            Event::Market(MarketEvent::OrderbookSnapshot { symbol, snapshot }) => {
                println!("=== SNAPSHOT for {} ===", symbol);
                print_snapshot(&snapshot);
                println!();
            }
            Event::Market(MarketEvent::OrderbookUpdate { symbol, snapshot }) => {
                update_count += 1;
                println!(
                    "[Update #{}/{}] {} | Mid: ${:.2} | Spread: ${:.4}",
                    update_count,
                    max_updates,
                    symbol,
                    snapshot.mid_price().unwrap_or_default(),
                    snapshot.spread().unwrap_or_default()
                );

                if update_count >= max_updates {
                    println!("\nReached {} updates. Stopping.", max_updates);
                    break;
                }
            }
            Event::Market(MarketEvent::ChecksumMismatch {
                symbol,
                expected,
                computed,
            }) => {
                println!(
                    "WARNING: Checksum mismatch for {}: expected {}, got {}",
                    symbol, expected, computed
                );
            }
            Event::Connection(ConnectionEvent::Disconnected { reason }) => {
                println!("Disconnected: {:?}", reason);
                break;
            }
            _ => {}
        }
    }

    client.shutdown();
    println!("\nDone!");
    Ok(())
}

fn print_snapshot(snapshot: &OrderbookSnapshot) {
    println!("  Top 5 Bids:");
    for (i, level) in snapshot.bids.iter().take(5).enumerate() {
        println!("    {}. ${:.2} x {:.8}", i + 1, level.price, level.qty);
    }

    println!("  Top 5 Asks:");
    for (i, level) in snapshot.asks.iter().take(5).enumerate() {
        println!("    {}. ${:.2} x {:.8}", i + 1, level.price, level.qty);
    }

    if let (Some(mid), Some(spread)) = (snapshot.mid_price(), snapshot.spread()) {
        println!("  Mid Price: ${:.2}", mid);
        println!("  Spread: ${:.4}", spread);
    }
    println!("  Checksum: {}", snapshot.checksum);
}
