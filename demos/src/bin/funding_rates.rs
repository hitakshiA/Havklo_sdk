//! Demo 7: Futures Funding Rate Monitor
//!
//! Showcases: Futures WebSocket support, funding rate tracking
//!
//! Run: cargo run --bin funding_rates

use colored::*;
use kraken_futures_ws::{FuturesConfig, FuturesConnection, FuturesEvent};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::time::Duration;

const PRODUCTS: [&str; 3] = ["PI_XBTUSD", "PI_ETHUSD", "PF_SOLUSD"];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(70).cyan());
    println!("{}", "  FUTURES FUNDING RATE MONITOR".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Perpetual Swap Analytics".cyan());
    println!("{}", "═".repeat(70).cyan());
    println!();

    let config = FuturesConfig::new()
        .with_products(PRODUCTS.iter().map(|s| s.to_string()).collect());

    let mut conn = FuturesConnection::new(config);
    let mut events = conn.take_event_receiver().expect("Events already taken");

    tokio::spawn(async move {
        if let Err(e) = conn.connect_and_run().await {
            eprintln!("Connection error: {}", e);
        }
    });

    println!("{} Connecting to Kraken Futures...\n", "✓".green());

    let mut funding_data: HashMap<String, (Decimal, Decimal, Decimal)> = HashMap::new();

    println!(
        "  {:<12} {:>12} {:>14} {:>12}",
        "PRODUCT".white().bold(),
        "MARK PRICE".white().bold(),
        "FUNDING RATE".white().bold(),
        "ANNUAL".white().bold()
    );
    println!("  {}", "─".repeat(54));

    // Print initial placeholders
    for product in PRODUCTS {
        println!("  {:<12} {:>12} {:>14} {:>12}", product.cyan(), "-", "-", "-");
    }

    let start = std::time::Instant::now();

    while let Some(event) = events.recv().await {
        if let FuturesEvent::Ticker(ticker) = event {
            let mark_price = ticker.mark_price.unwrap_or_default();
            let funding_rate = ticker.funding_rate.unwrap_or_default();
            let annual = funding_rate * dec!(365) * dec!(3) * dec!(100);

            funding_data.insert(ticker.product_id.clone(), (mark_price, funding_rate, annual));

            // Move cursor up and redraw
            print!("\x1B[{}A", PRODUCTS.len());

            for &product in &PRODUCTS {
                if let Some((mark, rate, annual)) = funding_data.get(product) {
                    let rate_pct = rate * dec!(100);
                    let rate_color = if *rate > Decimal::ZERO {
                        format!("{:+.6}%", rate_pct).green()
                    } else if *rate < Decimal::ZERO {
                        format!("{:+.6}%", rate_pct).red()
                    } else {
                        format!("{:+.6}%", rate_pct).white()
                    };

                    let annual_color = if *annual > dec!(10) {
                        format!("{:+.2}%", annual).green()
                    } else if *annual < dec!(-10) {
                        format!("{:+.2}%", annual).red()
                    } else {
                        format!("{:+.2}%", annual).white()
                    };

                    println!(
                        "  {:<12} ${:>11.2} {:>14} {:>12}",
                        product.cyan(), mark, rate_color, annual_color
                    );
                } else {
                    println!("  {:<12} {:>12} {:>14} {:>12}", product.cyan(), "-", "-", "-");
                }
            }
        }

        if start.elapsed() > Duration::from_secs(30) {
            break;
        }
    }

    println!();
    println!("{}", "═".repeat(70).cyan());
    println!("  {}", "FUNDING RATE EXPLAINED".white().bold());
    println!("{}", "═".repeat(70).cyan());
    println!();
    println!("  {} Positive rate = Longs pay shorts (bullish sentiment)", "•".green());
    println!("  {} Negative rate = Shorts pay longs (bearish sentiment)", "•".red());
    println!("  {} Funding occurs every 8 hours on Kraken Futures", "•".white());

    Ok(())
}
