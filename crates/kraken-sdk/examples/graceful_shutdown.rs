//! Example: Graceful Shutdown with Signal Handling
//!
//! This example demonstrates proper shutdown handling:
//! - Listening for Ctrl+C (SIGINT)
//! - Clean disconnection from WebSocket
//! - Flushing pending operations
//!
//! Run with: cargo run --example graceful_shutdown
//!
//! Press Ctrl+C to trigger graceful shutdown

use kraken_sdk::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Graceful Shutdown Example ===\n");
    println!("Press Ctrl+C to trigger graceful shutdown\n");

    // Shared state
    let update_count = Arc::new(AtomicU64::new(0));
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    // Clone for signal handler
    let shutdown_flag_signal = shutdown_flag.clone();

    // Set up Ctrl+C handler
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        println!("\n[SIGNAL] Received Ctrl+C, initiating graceful shutdown...");
        shutdown_flag_signal.store(true, Ordering::SeqCst);
    });

    // Connect to Kraken
    println!("Connecting to Kraken...");
    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .connect()
        .await?;

    println!("Connected! Processing events...\n");

    let mut events = client.events().expect("events() already called");
    let update_count_clone = update_count.clone();

    // Main event loop with shutdown check
    while !shutdown_flag.load(Ordering::SeqCst) {
        // Use timeout to periodically check shutdown flag
        match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            events.recv()
        ).await {
            Ok(Some(event)) => {
                match event {
                    Event::Market(MarketEvent::OrderbookUpdate { symbol, .. }) => {
                        let count = update_count_clone.fetch_add(1, Ordering::Relaxed) + 1;
                        if count % 50 == 0 {
                            println!("[{}] Processed {} updates", symbol, count);
                        }
                    }
                    Event::Market(MarketEvent::OrderbookSnapshot { symbol, .. }) => {
                        println!("[{}] Received initial snapshot", symbol);
                    }
                    Event::Connection(ConnectionEvent::Connected { .. }) => {
                        println!("[CONNECTION] Connected successfully");
                    }
                    _ => {}
                }
            }
            Ok(None) => {
                // Channel closed
                println!("[EVENT] Event channel closed");
                break;
            }
            Err(_) => {
                // Timeout, check shutdown flag
                continue;
            }
        }
    }

    // Graceful shutdown sequence
    println!("\n--- Shutdown Sequence ---\n");

    // 1. Stop accepting new work
    println!("1. Stopping event processing...");

    // 2. Report final statistics
    let final_count = update_count.load(Ordering::Relaxed);
    println!("2. Final statistics:");
    println!("   - Total updates processed: {}", final_count);

    // 3. Clean disconnect
    println!("3. Disconnecting from Kraken...");
    client.shutdown();

    // 4. Cleanup complete
    println!("4. Shutdown complete!");

    println!("\n=== Graceful Shutdown Successful ===");
    Ok(())
}
