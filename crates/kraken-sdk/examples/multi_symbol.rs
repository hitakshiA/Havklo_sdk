//! Example: Monitor multiple trading pairs simultaneously
//!
//! Run with: cargo run --example multi_symbol

use kraken_sdk::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let symbols = ["BTC/USD", "ETH/USD", "SOL/USD"];

    println!("=== Multi-Symbol Orderbook Monitor ===\n");
    println!("Monitoring: {:?}\n", symbols);

    // Connect to all symbols
    let mut client = KrakenClient::builder(symbols)
        .with_depth(Depth::D10)
        .connect()
        .await?;

    let mut events = client.events().expect("events() already called");
    let mut spreads: HashMap<String, Decimal> = HashMap::new();
    let mut snapshot_received = 0;

    // Run for 15 seconds
    let timeout = tokio::time::sleep(Duration::from_secs(15));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => {
                println!("\n=== Final Summary ===");
                print_summary(&client, &symbols);
                break;
            }
            event = events.recv() => {
                match event {
                    Some(Event::Market(MarketEvent::OrderbookSnapshot { symbol, snapshot })) => {
                        snapshot_received += 1;
                        let spread = snapshot.spread().unwrap_or_default();
                        spreads.insert(symbol.clone(), spread);

                        println!(
                            "[SNAPSHOT] {} - Mid: ${:.2} | Spread: ${:.4}",
                            symbol,
                            snapshot.mid_price().unwrap_or_default(),
                            spread
                        );

                        if snapshot_received == symbols.len() {
                            println!("\nAll symbols synced! Monitoring updates...\n");
                        }
                    }
                    Some(Event::Market(MarketEvent::OrderbookUpdate { symbol, snapshot })) => {
                        let spread = snapshot.spread().unwrap_or_default();
                        let prev_spread = spreads.get(&symbol).cloned().unwrap_or_default();

                        // Only print if spread changed significantly
                        let change = (spread - prev_spread).abs();
                        if change > Decimal::new(1, 3) {
                            // > 0.001
                            let direction = if spread > prev_spread { "+" } else { "-" };
                            println!(
                                "{}: Spread {}{:.4} -> {:.4} | Mid: ${:.2}",
                                symbol,
                                direction,
                                change,
                                spread,
                                snapshot.mid_price().unwrap_or_default()
                            );
                        }
                        spreads.insert(symbol, spread);
                    }
                    Some(Event::Connection(ConnectionEvent::Reconnecting { attempt, delay })) => {
                        println!("Reconnecting (attempt {}, delay {:?})...", attempt, delay);
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

fn print_summary(client: &KrakenClient, symbols: &[&str]) {
    for symbol in symbols {
        if let Some(book) = client.orderbook(symbol) {
            let bid = book.best_bid().map(|l| l.price);
            let ask = book.best_ask().map(|l| l.price);
            let spread = book.spread();
            let synced = if book.is_synced() { "SYNCED" } else { "DESYNCED" };

            println!(
                "{}: bid={:?} ask={:?} spread={:?} [{}]",
                symbol, bid, ask, spread, synced
            );
        } else {
            println!("{}: No data", symbol);
        }
    }
}
