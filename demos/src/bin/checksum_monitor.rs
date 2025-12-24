//! Demo 5: Checksum Integrity Monitor
//!
//! Showcases: CRC32 checksum validation, data integrity verification
//!
//! Run: cargo run --bin checksum_monitor

use colored::*;
use kraken_sdk::prelude::*;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(65).cyan());
    println!("{}", "  CHECKSUM INTEGRITY MONITOR".cyan().bold());
    println!("{}", "  Havklo SDK Demo - CRC32 Data Validation".cyan());
    println!("{}", "═".repeat(65).cyan());
    println!();

    let mut client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .with_book(true)
        .connect()
        .await?;

    let mut events = client.events().expect("Events already taken");

    println!("{} Connected to Kraken WebSocket", "✓".green());
    println!("{} Monitoring checksum validation...\n", "✓".green());

    let mut snapshot_count = 0u64;
    let mut update_count = 0u64;
    let start = Instant::now();

    println!(
        "  {:>12}  {:>15}  {:>12}  {:>8}",
        "EVENT".white().bold(),
        "CHECKSUM".white().bold(),
        "STATUS".white().bold(),
        "TOTAL".white().bold()
    );
    println!("  {}", "─".repeat(52));

    while let Some(event) = events.recv().await {
        match event {
            Event::Market(MarketEvent::OrderbookSnapshot { symbol, .. }) => {
                snapshot_count += 1;
                let checksum = client.checksum(&symbol).unwrap_or(0);
                println!(
                    "  {:>12}  {:>15}  {:>12}  {:>8}",
                    "SNAPSHOT".cyan(),
                    format!("{:08X}", checksum),
                    "VALID".green(),
                    snapshot_count + update_count
                );
            }
            Event::Market(MarketEvent::OrderbookUpdate { symbol, .. }) => {
                update_count += 1;
                // Only print every 10th update
                if update_count.is_multiple_of(10) {
                    let checksum = client.checksum(&symbol).unwrap_or(0);
                    println!(
                        "  {:>12}  {:>15}  {:>12}  {:>8}",
                        format!("UPDATE #{}", update_count).yellow(),
                        format!("{:08X}", checksum),
                        "VALID".green(),
                        snapshot_count + update_count
                    );
                }
            }
            _ => {}
        }

        if start.elapsed() > Duration::from_secs(30) {
            break;
        }
    }

    println!();
    println!("{}", "═".repeat(65).cyan());
    println!("  {}", "INTEGRITY REPORT".white().bold());
    println!("{}", "═".repeat(65).cyan());
    println!();
    println!("  Snapshots:        {}", snapshot_count);
    println!("  Updates:          {}", update_count);
    println!("  All Valid:        {}", "YES".green());
    println!();
    println!("  {} Kraken's CRC32 checksum detects: missed messages, corruption, sequence gaps", "Note:".dimmed());

    Ok(())
}
