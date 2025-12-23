//! Demo 8: Orderbook Imbalance Analyzer
//!
//! Showcases: Real-time market analytics, buy/sell pressure detection
//!
//! Run: cargo run --bin imbalance_analyzer

use colored::*;
use kraken_book::l3::{L3Book, L3Order, L3Side};
use kraken_sdk::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::VecDeque;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(65).cyan());
    println!("{}", "  ORDERBOOK IMBALANCE ANALYZER".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Market Pressure Detection".cyan());
    println!("{}", "═".repeat(65).cyan());
    println!();

    let client = KrakenClient::builder(["BTC/USD"])
        .with_depth(Depth::D25)
        .with_book(true)
        .connect()
        .await?;

    println!("{} Connected. Analyzing orderbook imbalance...\n", "✓".green());
    tokio::time::sleep(Duration::from_secs(2)).await;

    let mut imbalance_history: VecDeque<f64> = VecDeque::with_capacity(60);

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

        // Build L3 book for imbalance calculation
        let mut book = L3Book::new("BTC/USD", 50);
        for (i, level) in bids.iter().enumerate() {
            book.add_order(L3Order::new(&format!("b{}", i), level.price, level.qty), L3Side::Bid);
        }
        for (i, level) in asks.iter().enumerate() {
            book.add_order(L3Order::new(&format!("a{}", i), level.price, level.qty), L3Side::Ask);
        }

        let imbalance = book.imbalance().unwrap_or(0.0);
        imbalance_history.push_back(imbalance);
        if imbalance_history.len() > 60 {
            imbalance_history.pop_front();
        }

        let avg_imbalance: f64 = if !imbalance_history.is_empty() {
            imbalance_history.iter().sum::<f64>() / imbalance_history.len() as f64
        } else {
            0.0
        };

        let bid_volume: Decimal = bids.iter().map(|l| l.qty).sum();
        let ask_volume: Decimal = asks.iter().map(|l| l.qty).sum();
        let best_bid = bids[0].price;
        let best_ask = asks[0].price;

        print!("\x1B[2J\x1B[H");
        println!("{}", "═".repeat(65).cyan());
        println!("{}", "  ORDERBOOK IMBALANCE ANALYZER".cyan().bold());
        println!("{}", "═".repeat(65).cyan());
        println!();
        println!("  {} ${:.2}  {} ${:.2}", "Best Bid:".yellow(), best_bid, "Best Ask:".yellow(), best_ask);
        println!();

        // Imbalance gauge
        println!("  {}", "IMBALANCE GAUGE".white().bold());
        println!("  {}", "─".repeat(55));

        let gauge_width = 50;
        let center = gauge_width / 2;
        let position = ((imbalance + 1.0) / 2.0 * gauge_width as f64) as usize;
        let position = position.min(gauge_width - 1);

        let mut gauge = vec![' '; gauge_width];
        gauge[center] = '│';

        if position > center {
            for i in center..=position.min(gauge_width - 1) {
                gauge[i] = '█';
            }
        } else {
            for i in position..center {
                gauge[i] = '█';
            }
        }

        let gauge_str: String = gauge.iter().collect();
        let colored_gauge = if imbalance > 0.2 {
            gauge_str.green()
        } else if imbalance < -0.2 {
            gauge_str.red()
        } else {
            gauge_str.yellow()
        };

        println!("  SELL {} BUY", colored_gauge);
        println!("  -1.0{:^48}+1.0", "");

        let imb_str = format!("{:+.3}", imbalance);
        let pressure = if imbalance > 0.3 {
            ("STRONG BUY PRESSURE", imb_str.green())
        } else if imbalance > 0.1 {
            ("MODERATE BUY PRESSURE", imb_str.green())
        } else if imbalance < -0.3 {
            ("STRONG SELL PRESSURE", imb_str.red())
        } else if imbalance < -0.1 {
            ("MODERATE SELL PRESSURE", imb_str.red())
        } else {
            ("BALANCED / NEUTRAL", imb_str.white())
        };

        println!();
        println!("  Current:   {} {}", pressure.1, pressure.0);
        println!("  Average:   {:+.3} (last {} readings)", avg_imbalance, imbalance_history.len());

        println!();
        println!("  {}", "VOLUME BREAKDOWN".white().bold());
        println!("  {}", "─".repeat(55));

        let total_vol = bid_volume + ask_volume;
        let bid_pct = if !total_vol.is_zero() { bid_volume / total_vol } else { dec!(0.5) };
        let ask_pct = if !total_vol.is_zero() { ask_volume / total_vol } else { dec!(0.5) };

        let bid_bar_len = (bid_pct * dec!(30)).to_string().parse::<f64>().unwrap_or(15.0) as usize;
        let ask_bar_len = (ask_pct * dec!(30)).to_string().parse::<f64>().unwrap_or(15.0) as usize;

        println!("  Bid Volume:  {:>10.4} BTC  {}", bid_volume, "█".repeat(bid_bar_len.min(30)).green());
        println!("  Ask Volume:  {:>10.4} BTC  {}", ask_volume, "█".repeat(ask_bar_len.min(30)).red());

        println!();
        if imbalance > 0.3 && avg_imbalance > 0.2 {
            println!("  {} Sustained buy pressure - potential upward move", "SIGNAL:".green().bold());
        } else if imbalance < -0.3 && avg_imbalance < -0.2 {
            println!("  {} Sustained sell pressure - potential downward move", "SIGNAL:".red().bold());
        } else {
            println!("  {} No clear directional bias", "SIGNAL:".white().bold());
        }
    }
}
