//! Demo 2: Multi-Symbol Dashboard
//!
//! Showcases: Concurrent multi-symbol streaming, independent orderbook state
//!
//! Run: cargo run --bin multi_symbol

use colored::*;
use kraken_sdk::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

const SYMBOLS: [&str; 5] = ["BTC/USD", "ETH/USD", "SOL/USD", "XRP/USD", "DOGE/USD"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(70).cyan());
    println!("{}", "  MULTI-SYMBOL DASHBOARD".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Concurrent Symbol Streaming".cyan());
    println!("{}", "═".repeat(70).cyan());
    println!();

    let symbols: Vec<String> = SYMBOLS.iter().map(|s| s.to_string()).collect();

    let client = KrakenClient::builder(symbols)
        .with_depth(Depth::D10)
        .with_book(true)
        .connect()
        .await?;

    println!("{} Connected. Tracking {} symbols.\n", "✓".green(), SYMBOLS.len());

    println!(
        "  {:<10} {:>12} {:>12} {:>10} {:>10}",
        "SYMBOL".white().bold(),
        "BID".white().bold(),
        "ASK".white().bold(),
        "SPREAD".white().bold(),
        "BPS".white().bold()
    );
    println!("  {}", "─".repeat(58));

    for _ in 0..60 {
        tokio::time::sleep(Duration::from_secs(1)).await;

        print!("\x1B[{}A", SYMBOLS.len());

        for symbol in SYMBOLS {
            let bid = client.best_bid(symbol);
            let ask = client.best_ask(symbol);
            let spread = client.spread(symbol);

            if let (Some(bid), Some(ask), Some(spread)) = (bid, ask, spread) {
                let mid = (bid + ask) / Decimal::TWO;
                let bps = if !mid.is_zero() {
                    (spread / mid * dec!(10000)).round()
                } else {
                    Decimal::ZERO
                };

                let spread_color = if bps < dec!(5) {
                    spread.to_string().green()
                } else if bps < dec!(20) {
                    spread.to_string().yellow()
                } else {
                    spread.to_string().red()
                };

                println!(
                    "  {:<10} {:>12.2} {:>12.2} {:>10} {:>10.1}",
                    symbol.cyan(),
                    bid,
                    ask,
                    spread_color,
                    bps
                );
            } else {
                println!("  {:<10} {:>12} {:>12} {:>10} {:>10}", symbol.cyan(), "-", "-", "-", "-");
            }
        }
    }

    println!("\n{} Demo complete.", "✓".green());
    Ok(())
}
