//! Demo 4: L3 Queue Position Tracker
//!
//! Showcases: L3 orderbook, queue position tracking for market makers
//!
//! Run: cargo run --bin queue_position

use colored::*;
use kraken_book::l3::{L3Book, L3Order, L3Side};
use kraken_sdk::prelude::*;
use rust_decimal_macros::dec;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(65).cyan());
    println!("{}", "  L3 QUEUE POSITION TRACKER".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Market Maker Analytics".cyan());
    println!("{}", "═".repeat(65).cyan());
    println!();

    let client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D25)
        .with_book(true)
        .connect()
        .await?;

    println!("{} Connected. Building L3 orderbook...\n", "✓".green());
    tokio::time::sleep(Duration::from_secs(3)).await;

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let orderbook = match client.orderbook("BTC/USD") {
            Some(ob) => ob,
            None => continue,
        };

        let bids = orderbook.bids_vec();
        let asks = orderbook.asks_vec();

        if bids.is_empty() || asks.is_empty() {
            continue;
        }

        let mut book = L3Book::new("BTC/USD", 100);

        // Add market orders, split each level into 3 orders
        for (i, level) in bids.iter().enumerate() {
            let qty_per_order = level.qty / dec!(3);
            for j in 0..3 {
                book.add_order(
                    L3Order::new(&format!("bid_{}_{}", i, j), level.price, qty_per_order),
                    L3Side::Bid,
                );
            }
        }
        for (i, level) in asks.iter().enumerate() {
            let qty_per_order = level.qty / dec!(3);
            for j in 0..3 {
                book.add_order(
                    L3Order::new(&format!("ask_{}_{}", i, j), level.price, qty_per_order),
                    L3Side::Ask,
                );
            }
        }

        let best_bid = bids[0].price;
        let best_ask = asks[0].price;

        // Add our hypothetical orders
        let our_orders = [
            ("MY_BID_1", best_bid, dec!(0.5), L3Side::Bid),
            ("MY_BID_2", best_bid - dec!(1), dec!(1.0), L3Side::Bid),
            ("MY_ASK_1", best_ask, dec!(0.5), L3Side::Ask),
            ("MY_ASK_2", best_ask + dec!(1), dec!(1.0), L3Side::Ask),
        ];

        for (id, price, qty, side) in &our_orders {
            book.add_order(L3Order::new(*id, *price, *qty), *side);
        }

        print!("\x1B[2J\x1B[H");
        println!("{}", "═".repeat(65).cyan());
        println!("{}", "  L3 QUEUE POSITION TRACKER".cyan().bold());
        println!("{}", "═".repeat(65).cyan());
        println!();
        println!("  {} ${:.2}  {} ${:.2}", "Best Bid:".yellow(), best_bid, "Best Ask:".yellow(), best_ask);
        println!();

        println!("  {}", "YOUR ORDERS - QUEUE POSITION".white().bold());
        println!("  {}", "─".repeat(55));
        println!(
            "  {:>10}  {:>6}  {:>10}  {:>8}  {:>10}  {:>8}",
            "ORDER".dimmed(), "SIDE".dimmed(), "PRICE".dimmed(), "POS".dimmed(), "QTY AHEAD".dimmed(), "FILL %".dimmed()
        );

        for (id, price, _, side) in &our_orders {
            if let Some(pos) = book.queue_position(id) {
                let fill_pct = pos.fill_probability() * 100.0;
                let fill_color = if fill_pct > 50.0 {
                    format!("{:.1}%", fill_pct).green()
                } else if fill_pct > 20.0 {
                    format!("{:.1}%", fill_pct).yellow()
                } else {
                    format!("{:.1}%", fill_pct).red()
                };

                let side_color = match side {
                    L3Side::Bid => "BID".green(),
                    L3Side::Ask => "ASK".red(),
                };

                println!(
                    "  {:>10}  {:>6}  ${:>9.2}  {:>8}  {:>10.4}  {:>8}",
                    id.cyan(), side_color, price, format!("#{}", pos.position), pos.qty_ahead, fill_color
                );
            }
        }

        // Book imbalance
        if let Some(imbalance) = book.imbalance() {
            let imb_color = if imbalance > 0.2 {
                format!("{:+.2}", imbalance).green()
            } else if imbalance < -0.2 {
                format!("{:+.2}", imbalance).red()
            } else {
                format!("{:+.2}", imbalance).white()
            };

            let pressure = if imbalance > 0.2 {
                "BUY PRESSURE".green()
            } else if imbalance < -0.2 {
                "SELL PRESSURE".red()
            } else {
                "NEUTRAL".white()
            };

            println!();
            println!("  {} {}  {}", "Book Imbalance:".yellow(), imb_color, pressure);
        }

        println!();
        println!("  {} Queue position helps market makers optimize order placement", "Tip:".dimmed());
    }
}
