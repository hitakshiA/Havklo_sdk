//! Demo 6: Auto-Reconnect Demo
//!
//! Showcases: Automatic reconnection, exponential backoff, circuit breaker
//!
//! Run: cargo run --bin reconnect_demo

use colored::*;
use kraken_sdk::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(65).cyan());
    println!("{}", "  AUTO-RECONNECT DEMO".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Resilient Connection Handling".cyan());
    println!("{}", "═".repeat(65).cyan());
    println!();

    println!("{}", "  RECONNECTION CONFIGURATION".white().bold());
    println!("  {}", "─".repeat(50));
    println!("  Initial Delay:     {} ms", "100".cyan());
    println!("  Max Delay:         {} seconds", "30".cyan());
    println!("  Backoff Multiplier: {}", "2.0x".cyan());
    println!("  Circuit Breaker:   {} failures to open", "5".cyan());
    println!();

    // Show backoff progression
    println!("{}", "  EXPONENTIAL BACKOFF PROGRESSION".white().bold());
    println!("  {}", "─".repeat(50));

    let mut delay = 100u64;
    let max_delay = 30000u64;

    for attempt in 1..=8 {
        let bar_len = (delay / 400).min(40) as usize;
        println!(
            "  Attempt {:>2}: {:>6} ms  {}",
            attempt,
            delay,
            "█".repeat(bar_len).yellow()
        );
        delay = (delay * 2).min(max_delay);
    }

    println!();
    println!("{}", "  LIVE CONNECTION TEST".white().bold());
    println!("  {}", "─".repeat(50));

    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .with_book(true)
        .connect()
        .await?;

    let mut events = client.events().expect("Events already taken");

    let start = std::time::Instant::now();

    while let Some(event) = events.recv().await {
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");

        match &event {
            Event::Connection(ConnectionEvent::Connected { .. }) => {
                println!(
                    "  {} {} {}",
                    format!("[{}]", timestamp).dimmed(),
                    "●".green(),
                    "Connected to Kraken".green()
                );
            }
            Event::Market(MarketEvent::OrderbookSnapshot { symbol, .. }) => {
                println!(
                    "  {} {} Orderbook snapshot for {}",
                    format!("[{}]", timestamp).dimmed(),
                    "●".blue(),
                    symbol.cyan()
                );
            }
            _ => {}
        }

        if start.elapsed() > Duration::from_secs(10) {
            break;
        }
    }

    println!();
    println!("{}", "  CIRCUIT BREAKER STATES".white().bold());
    println!("  {}", "─".repeat(50));
    println!(
        "  {} → {} → {} → {}",
        "Closed".green(),
        "Open (5 failures)".red(),
        "Half-Open".yellow(),
        "Closed".green()
    );
    println!();
    println!("  {} SDK handles connection drops and API errors automatically", "Note:".dimmed());

    Ok(())
}
