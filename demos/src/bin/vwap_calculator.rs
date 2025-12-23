//! Demo 3: VWAP Slippage Calculator
//!
//! Showcases: Financial precision with rust_decimal, VWAP calculations
//!
//! Run: cargo run --bin vwap_calculator

use colored::*;
use kraken_book::l3::{L3Book, L3Order, L3Side};
use kraken_sdk::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(65).cyan());
    println!("{}", "  VWAP SLIPPAGE CALCULATOR".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Financial Precision".cyan());
    println!("{}", "═".repeat(65).cyan());
    println!();

    let client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D100)
        .with_book(true)
        .connect()
        .await?;

    println!("{} Connected. Building orderbook...\n", "✓".green());
    tokio::time::sleep(Duration::from_secs(3)).await;

    let sizes = [dec!(0.1), dec!(0.5), dec!(1.0), dec!(5.0), dec!(10.0)];

    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let orderbook = match client.orderbook("BTC/USD") {
            Some(ob) => ob,
            None => continue,
        };

        let bids = orderbook.bids_vec();
        let asks = orderbook.asks_vec();

        if bids.is_empty() || asks.is_empty() {
            continue;
        }

        // Build L3 book from L2 snapshot
        let mut book = L3Book::new("BTC/USD", 100);
        for (i, level) in bids.iter().enumerate() {
            book.add_order(
                L3Order::new(&format!("bid_{}", i), level.price, level.qty),
                L3Side::Bid,
            );
        }
        for (i, level) in asks.iter().enumerate() {
            book.add_order(
                L3Order::new(&format!("ask_{}", i), level.price, level.qty),
                L3Side::Ask,
            );
        }

        let best_bid = bids[0].price;
        let best_ask = asks[0].price;
        let mid = (best_bid + best_ask) / dec!(2);

        print!("\x1B[2J\x1B[H");
        println!("{}", "═".repeat(65).cyan());
        println!("{}", "  VWAP SLIPPAGE CALCULATOR".cyan().bold());
        println!("{}", "═".repeat(65).cyan());
        println!();
        println!(
            "  {} ${:.2}  {} ${:.2}  {} ${:.2}",
            "Best Bid:".yellow(), best_bid,
            "Best Ask:".yellow(), best_ask,
            "Mid:".green(), mid
        );
        println!();

        println!("  {} (Market Buy)", "SLIPPAGE ANALYSIS".white().bold());
        println!("  {}", "─".repeat(55));
        println!(
            "  {:>8}  {:>12}  {:>12}  {:>10}  {:>8}",
            "SIZE".dimmed(), "VWAP".dimmed(), "COST".dimmed(), "SLIPPAGE".dimmed(), "BPS".dimmed()
        );

        for size in &sizes {
            if let Some(vwap) = book.vwap_ask(*size) {
                let cost = vwap * size;
                let slippage = vwap - best_ask;
                let bps = if !best_ask.is_zero() {
                    (slippage / best_ask * dec!(10000)).round()
                } else {
                    Decimal::ZERO
                };

                let bps_color = if bps < dec!(5) {
                    format!("{:.0}", bps).green()
                } else if bps < dec!(20) {
                    format!("{:.0}", bps).yellow()
                } else {
                    format!("{:.0}", bps).red()
                };

                println!(
                    "  {:>8.2} BTC  ${:>10.2}  ${:>10.2}  ${:>8.2}  {:>8}",
                    size, vwap, cost, slippage, bps_color
                );
            } else {
                println!("  {:>8.2} BTC  {:>12}  {:>12}  {:>10}  {:>8}", size, "INSUFFICIENT", "-", "-", "-");
            }
        }

        println!("\n  {} rust_decimal - zero floating point errors", "Precision:".dimmed());
    }
}
