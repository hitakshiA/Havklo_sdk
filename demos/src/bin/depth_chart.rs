//! Demo 10: ASCII Depth Chart
//!
//! Showcases: Orderbook visualization, real-time depth display
//!
//! Run: cargo run --bin depth_chart

use colored::*;
use kraken_sdk::prelude::*;
use kraken_types::Level;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

fn draw_depth_chart(bids: &[Level], asks: &[Level]) {
    let levels_to_show = 10;
    let half_width = 25;

    // Calculate max volume for scaling
    let max_bid_vol: Decimal = bids.iter().take(levels_to_show).map(|l| l.qty).max().unwrap_or(dec!(1));
    let max_ask_vol: Decimal = asks.iter().take(levels_to_show).map(|l| l.qty).max().unwrap_or(dec!(1));
    let max_vol = max_bid_vol.max(max_ask_vol);

    // Print asks (reversed - highest first)
    let asks_to_show: Vec<_> = asks.iter().take(levels_to_show).collect();
    for level in asks_to_show.iter().rev() {
        let bar_len = if !max_vol.is_zero() {
            ((level.qty / max_vol) * Decimal::from(half_width))
                .to_string()
                .parse::<usize>()
                .unwrap_or(0)
                .min(half_width)
        } else {
            0
        };

        let bar = "█".repeat(bar_len);
        let padding = " ".repeat(half_width - bar_len);

        println!(
            "  {:>10.4} │{}{}│ ${:<10.2}",
            level.qty,
            padding,
            bar.red(),
            level.price
        );
    }

    // Spread line
    let spread = if !bids.is_empty() && !asks.is_empty() {
        asks[0].price - bids[0].price
    } else {
        Decimal::ZERO
    };
    let mid = if !bids.is_empty() && !asks.is_empty() {
        (asks[0].price + bids[0].price) / dec!(2)
    } else {
        Decimal::ZERO
    };

    println!(
        "  {:>10} ├{}┤ {}",
        "",
        "─".repeat(half_width * 2),
        format!("SPREAD: ${:.2}", spread).yellow()
    );

    // Print bids
    for level in bids.iter().take(levels_to_show) {
        let bar_len = if !max_vol.is_zero() {
            ((level.qty / max_vol) * Decimal::from(half_width))
                .to_string()
                .parse::<usize>()
                .unwrap_or(0)
                .min(half_width)
        } else {
            0
        };

        let bar = "█".repeat(bar_len);
        let padding = " ".repeat(half_width - bar_len);

        println!(
            "  {:>10.4} │{}{}│ ${:<10.2}",
            level.qty,
            bar.green(),
            padding,
            level.price
        );
    }

    println!();
    println!(
        "  {} ${:.2}  {} {:.4} BTC  {} {:.4} BTC",
        "Mid:".white(),
        mid,
        "Bid Vol:".green(),
        bids.iter().take(levels_to_show).map(|l| l.qty).sum::<Decimal>(),
        "Ask Vol:".red(),
        asks.iter().take(levels_to_show).map(|l| l.qty).sum::<Decimal>()
    );
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(70).cyan());
    println!("{}", "  ASCII DEPTH CHART".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Real-time Orderbook Visualization".cyan());
    println!("{}", "═".repeat(70).cyan());
    println!();

    let client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D25)
        .with_book(true)
        .connect()
        .await?;

    println!("{} Connected. Rendering depth chart...\n", "✓".green());
    tokio::time::sleep(Duration::from_secs(2)).await;

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let orderbook = match client.orderbook("BTC/USD") {
            Some(ob) => ob,
            None => continue,
        };

        let bids = orderbook.bids_vec();
        let asks = orderbook.asks_vec();

        if bids.is_empty() || asks.is_empty() {
            continue;
        }

        // Clear screen
        print!("\x1B[2J\x1B[H");

        println!("{}", "═".repeat(70).cyan());
        println!("{}{}", "  BTC/USD DEPTH CHART".cyan().bold(), "  (Live)".dimmed());
        println!("{}", "═".repeat(70).cyan());
        println!();

        println!(
            "  {:>10} {:^50} {:<10}",
            "VOLUME".white().bold(),
            "ORDERBOOK".white().bold(),
            "PRICE".white().bold()
        );
        println!("  {}", "─".repeat(66));

        println!(
            "  {} = Asks (Sell)    {} = Bids (Buy)",
            "███".red(),
            "███".green()
        );
        println!();

        draw_depth_chart(&bids, &asks);

        println!();
        println!(
            "  {} {}",
            "Updated:".dimmed(),
            chrono::Local::now().format("%H:%M:%S%.3f")
        );
    }
}
