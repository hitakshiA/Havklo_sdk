//! Demo 1: Real-time Spread Monitor
//!
//! Showcases: Sub-microsecond orderbook operations, real-time streaming
//!
//! Run: cargo run --bin spread_monitor

use colored::*;
use kraken_sdk::prelude::*;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(60).cyan());
    println!("{}", "  REAL-TIME SPREAD MONITOR".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Sub-microsecond Performance".cyan());
    println!("{}", "═".repeat(60).cyan());
    println!();

    let client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D10)
        .with_book(true)
        .connect()
        .await?;

    println!("{} Connected to Kraken WebSocket", "✓".green());
    println!("{} Streaming BTC/USD orderbook...\n", "✓".green());

    let mut update_count = 0u64;
    let start = Instant::now();

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let measure_start = Instant::now();
        let spread = client.spread("BTC/USD");
        let spread_time = measure_start.elapsed();

        let measure_start = Instant::now();
        let best_bid = client.best_bid("BTC/USD");
        let bid_time = measure_start.elapsed();

        let measure_start = Instant::now();
        let best_ask = client.best_ask("BTC/USD");
        let ask_time = measure_start.elapsed();

        if let (Some(spread), Some(bid), Some(ask)) = (spread, best_bid, best_ask) {
            update_count += 1;
            let elapsed = start.elapsed().as_secs();

            print!("\r\x1B[K");
            print!(
                "  {} ${:.2}  {} ${:.2}  {} ${:.2}  ",
                "BID:".yellow(),
                bid,
                "ASK:".yellow(),
                ask,
                "SPREAD:".green(),
                spread
            );
            print!(
                "│ {} {:.0}ns {:.0}ns {:.0}ns  ",
                "Latency:".dimmed(),
                bid_time.as_nanos(),
                ask_time.as_nanos(),
                spread_time.as_nanos()
            );
            print!(
                "│ {} {}/s",
                "Rate:".dimmed(),
                if elapsed > 0 { update_count / elapsed } else { 0 }
            );

            use std::io::Write;
            std::io::stdout().flush()?;
        }

        if start.elapsed() > Duration::from_secs(30) {
            break;
        }
    }

    println!("\n\n{} Demo complete. {} updates processed.", "✓".green(), update_count);
    Ok(())
}
