//! Example: Advanced reconnection configuration
//!
//! This example demonstrates how to configure custom reconnection behavior
//! with exponential backoff, jitter, and maximum retry limits.
//!
//! Run with: cargo run --example advanced_reconnect

use kraken_sdk::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Advanced Reconnection Example ===\n");

    // Configure custom reconnection policy with exponential backoff
    let reconnect_config = ReconnectConfig::new()
        // Start with 500ms delay
        .with_initial_delay(Duration::from_millis(500))
        // Cap at 60 seconds max delay
        .with_max_delay(Duration::from_secs(60))
        // Double delay each attempt (exponential backoff)
        .with_multiplier(2.0)
        // Add 20% jitter to prevent thundering herd
        .with_jitter(0.2)
        // Give up after 10 attempts
        .with_max_attempts(10);

    println!("Reconnection Policy:");
    println!("  Initial delay: 500ms");
    println!("  Max delay: 60s");
    println!("  Multiplier: 2.0x");
    println!("  Jitter: 20%");
    println!("  Max attempts: 10\n");

    // Show calculated delays for each attempt
    println!("Expected delays (without jitter):");
    for attempt in 1..=10 {
        let delay = reconnect_config.delay_for_attempt(attempt);
        println!("  Attempt {}: {:?}", attempt, delay);
    }
    println!();

    // Build client with custom reconnection
    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .with_reconnect(true)
        .with_reconnect_config(reconnect_config)
        .with_timeout(Duration::from_secs(15))
        .connect()
        .await?;

    println!("Connected! Monitoring for 30 seconds...\n");

    let mut events = client.events().expect("events() already called");
    let timeout = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(timeout);

    let mut update_count = 0;
    let mut reconnect_count = 0;

    loop {
        tokio::select! {
            _ = &mut timeout => {
                println!("\n=== Session Summary ===");
                println!("  Updates received: {}", update_count);
                println!("  Reconnections: {}", reconnect_count);
                break;
            }
            event = events.recv() => {
                match event {
                    Some(Event::Market(MarketEvent::OrderbookSnapshot { symbol, .. })) => {
                        println!("[SNAPSHOT] {}", symbol);
                    }
                    Some(Event::Market(MarketEvent::OrderbookUpdate { symbol, snapshot })) => {
                        update_count += 1;
                        if update_count % 20 == 0 {
                            println!(
                                "[UPDATE #{}] {} | Mid: ${:.2}",
                                update_count,
                                symbol,
                                snapshot.mid_price().unwrap_or_default()
                            );
                        }
                    }
                    Some(Event::Connection(ConnectionEvent::Connected { api_version, .. })) => {
                        println!("[CONNECTED] API version: {}", api_version);
                    }
                    Some(Event::Connection(ConnectionEvent::Disconnected { reason })) => {
                        println!("[DISCONNECTED] Reason: {:?}", reason);
                    }
                    Some(Event::Connection(ConnectionEvent::Reconnecting { attempt, delay })) => {
                        reconnect_count += 1;
                        println!(
                            "[RECONNECTING] Attempt {} in {:?}...",
                            attempt, delay
                        );
                    }
                    Some(Event::Connection(ConnectionEvent::ReconnectFailed { error })) => {
                        println!("[RECONNECT FAILED] {}", error);
                        break;
                    }
                    None => {
                        println!("[STREAM CLOSED]");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    client.shutdown();
    println!("\nDone!");
    Ok(())
}
