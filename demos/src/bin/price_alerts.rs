//! Demo 9: Price Alert System
//!
//! Showcases: Practical trading use case, event-driven architecture
//!
//! Run: cargo run --bin price_alerts

use colored::*;
use kraken_sdk::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Duration;

struct PriceAlert {
    symbol: String,
    condition: AlertCondition,
    triggered: bool,
}

enum AlertCondition {
    Above(Decimal),
    Below(Decimal),
    SpreadAbove(Decimal),
}

impl PriceAlert {
    fn check(&mut self, bid: Decimal, ask: Decimal) -> Option<String> {
        if self.triggered {
            return None;
        }

        let mid = (bid + ask) / dec!(2);
        let spread = ask - bid;

        let triggered = match self.condition {
            AlertCondition::Above(price) => mid > price,
            AlertCondition::Below(price) => mid < price,
            AlertCondition::SpreadAbove(threshold) => spread > threshold,
        };

        if triggered {
            self.triggered = true;
            let msg = match self.condition {
                AlertCondition::Above(price) => {
                    format!("{} above ${:.2} (current: ${:.2})", self.symbol, price, mid)
                }
                AlertCondition::Below(price) => {
                    format!("{} below ${:.2} (current: ${:.2})", self.symbol, price, mid)
                }
                AlertCondition::SpreadAbove(threshold) => {
                    format!("{} spread > ${:.2} (current: ${:.2})", self.symbol, threshold, spread)
                }
            };
            Some(msg)
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═".repeat(65).cyan());
    println!("{}", "  PRICE ALERT SYSTEM".cyan().bold());
    println!("{}", "  Havklo SDK Demo - Event-Driven Trading".cyan());
    println!("{}", "═".repeat(65).cyan());
    println!();

    let client = KrakenClient::builder(["BTC/USD", "ETH/USD"])
        .with_depth(Depth::D10)
        .with_book(true)
        .connect()
        .await?;

    println!("{} Connected to Kraken WebSocket\n", "✓".green());
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get current prices to set relative alerts
    let btc_mid = client.best_bid("BTC/USD")
        .and_then(|b| client.best_ask("BTC/USD").map(|a| (b + a) / dec!(2)))
        .unwrap_or(dec!(100000));

    let eth_mid = client.best_bid("ETH/USD")
        .and_then(|b| client.best_ask("ETH/USD").map(|a| (b + a) / dec!(2)))
        .unwrap_or(dec!(3500));

    // Create alerts relative to current price
    let mut alerts = vec![
        PriceAlert { symbol: "BTC/USD".into(), condition: AlertCondition::Above(btc_mid + dec!(50)), triggered: false },
        PriceAlert { symbol: "BTC/USD".into(), condition: AlertCondition::Below(btc_mid - dec!(50)), triggered: false },
        PriceAlert { symbol: "BTC/USD".into(), condition: AlertCondition::SpreadAbove(dec!(10)), triggered: false },
        PriceAlert { symbol: "ETH/USD".into(), condition: AlertCondition::Above(eth_mid + dec!(10)), triggered: false },
        PriceAlert { symbol: "ETH/USD".into(), condition: AlertCondition::Below(eth_mid - dec!(10)), triggered: false },
    ];

    println!("{}", "  CONFIGURED ALERTS".white().bold());
    println!("  {}", "─".repeat(55));
    println!("  {} BTC/USD above ${:.2}", "•".yellow(), btc_mid + dec!(50));
    println!("  {} BTC/USD below ${:.2}", "•".yellow(), btc_mid - dec!(50));
    println!("  {} BTC/USD spread > $10.00", "•".yellow());
    println!("  {} ETH/USD above ${:.2}", "•".yellow(), eth_mid + dec!(10));
    println!("  {} ETH/USD below ${:.2}", "•".yellow(), eth_mid - dec!(10));
    println!();

    println!("{}", "  MONITORING...".white().bold());
    println!("  {}", "─".repeat(55));

    let mut alert_count = 0;
    let start = std::time::Instant::now();

    for _ in 0..120 {
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Check BTC alerts
        if let (Some(bid), Some(ask)) = (client.best_bid("BTC/USD"), client.best_ask("BTC/USD")) {
            for alert in alerts.iter_mut().filter(|a| a.symbol == "BTC/USD") {
                if let Some(msg) = alert.check(bid, ask) {
                    alert_count += 1;
                    let timestamp = chrono::Local::now().format("%H:%M:%S");
                    println!(
                        "  {} {} {} {}",
                        format!("[{}]", timestamp).dimmed(),
                        "🔔",
                        "ALERT:".red().bold(),
                        msg.yellow()
                    );
                }
            }
        }

        // Check ETH alerts
        if let (Some(bid), Some(ask)) = (client.best_bid("ETH/USD"), client.best_ask("ETH/USD")) {
            for alert in alerts.iter_mut().filter(|a| a.symbol == "ETH/USD") {
                if let Some(msg) = alert.check(bid, ask) {
                    alert_count += 1;
                    let timestamp = chrono::Local::now().format("%H:%M:%S");
                    println!(
                        "  {} {} {} {}",
                        format!("[{}]", timestamp).dimmed(),
                        "🔔",
                        "ALERT:".red().bold(),
                        msg.yellow()
                    );
                }
            }
        }

        // Show current prices
        let btc = client.best_bid("BTC/USD")
            .and_then(|b| client.best_ask("BTC/USD").map(|a| (b + a) / dec!(2)));
        let eth = client.best_bid("ETH/USD")
            .and_then(|b| client.best_ask("ETH/USD").map(|a| (b + a) / dec!(2)));

        print!(
            "\r  Watching: BTC ${:.2}  ETH ${:.2}  Alerts: {}     ",
            btc.unwrap_or_default(),
            eth.unwrap_or_default(),
            alert_count
        );
        use std::io::Write;
        std::io::stdout().flush()?;

        if start.elapsed() > Duration::from_secs(60) {
            break;
        }
    }

    println!();
    println!();
    println!("{}", "═".repeat(65).cyan());
    println!("  {} Demo complete. {} alerts triggered.", "✓".green(), alert_count);
    println!("{}", "═".repeat(65).cyan());

    Ok(())
}
