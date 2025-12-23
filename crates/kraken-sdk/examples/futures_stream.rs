//! Example: Kraken Futures WebSocket streaming
//!
//! This example demonstrates how to connect to Kraken Futures and stream:
//! - Ticker data with funding rate and mark price
//! - Orderbook snapshots and updates
//! - Trade stream
//!
//! Run with: cargo run --example futures_stream
//!
//! NOTE: Uses public channels only. For position tracking, authentication is required.

use kraken_futures_ws::{FuturesConnection, FuturesConfig, FuturesEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Kraken Futures WebSocket Example ===\n");

    // Configure connection for BTC perpetual
    let config = FuturesConfig::new()
        .with_products(vec![
            "PI_XBTUSD".to_string(),  // BTC/USD perpetual
            "PI_ETHUSD".to_string(),  // ETH/USD perpetual
        ])
        .with_book_depth(25);

    println!("Connecting to Kraken Futures...");
    println!("Products: PI_XBTUSD, PI_ETHUSD\n");

    // Create connection
    let mut conn = FuturesConnection::new(config);
    let mut events = conn.take_event_receiver().expect("take_event_receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        if let Err(e) = conn.connect_and_run().await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Process events
    let mut event_count = 0;
    let max_events = 100;

    println!("Streaming events (max {})...\n", max_events);

    while let Some(event) = events.recv().await {
        event_count += 1;

        match &event {
            FuturesEvent::Connected { server_time } => {
                println!("[Connected] Server time: {}", server_time);
            }
            FuturesEvent::Subscribed { feed, product_ids } => {
                println!("[Subscribed] Feed: {} for {:?}", feed, product_ids);
            }
            FuturesEvent::Ticker(ticker) => {
                // Print ticker summary
                println!("\n=== {} Ticker ===", ticker.product_id);
                if let Some(last) = ticker.last {
                    println!("  Last Price:    ${}", last);
                }
                if let Some(mark) = ticker.mark_price {
                    println!("  Mark Price:    ${}", mark);
                }
                if let Some(index) = ticker.index_price {
                    println!("  Index Price:   ${}", index);
                }
                if let Some(funding) = ticker.funding_rate {
                    // Funding rate is typically expressed as a percentage per 8 hours
                    let annualized = funding * rust_decimal::Decimal::from(3 * 365);
                    println!("  Funding Rate:  {}% (8h) / {}% APR",
                        funding * rust_decimal::Decimal::from(100),
                        annualized * rust_decimal::Decimal::from(100)
                    );
                }
                if let Some(oi) = ticker.open_interest {
                    println!("  Open Interest: {} contracts", oi);
                }
                if let Some(vol) = ticker.vol24h {
                    println!("  24h Volume:    {} contracts", vol);
                }
                // Calculate premium/discount
                if let (Some(mark), Some(index)) = (ticker.mark_price, ticker.index_price) {
                    if !index.is_zero() {
                        let premium = (mark - index) / index * rust_decimal::Decimal::from(100);
                        let sign = if premium >= rust_decimal::Decimal::ZERO { "+" } else { "" };
                        println!("  Premium:       {}{}%", sign, premium.round_dp(4));
                    }
                }
            }
            FuturesEvent::BookSnapshot(snapshot) => {
                println!("\n=== {} Book Snapshot ===", snapshot.product_id);
                println!("  Seq: {}", snapshot.seq);
                println!("  Top Bids:");
                for (i, level) in snapshot.bids.iter().take(3).enumerate() {
                    println!("    {}. ${} x {}", i + 1, level.price, level.qty);
                }
                println!("  Top Asks:");
                for (i, level) in snapshot.asks.iter().take(3).enumerate() {
                    println!("    {}. ${} x {}", i + 1, level.price, level.qty);
                }
            }
            FuturesEvent::BookUpdate(update) => {
                // Just count updates, don't print every one
                if event_count % 20 == 0 {
                    println!("[Book Update] {} seq={} bids={} asks={}",
                        update.product_id, update.seq,
                        update.bids.len(), update.asks.len()
                    );
                }
            }
            FuturesEvent::Trade(trade) => {
                println!("[Trade] {} {:?} {} @ ${}",
                    trade.product_id,
                    trade.side,
                    trade.qty,
                    trade.price
                );
            }
            FuturesEvent::Heartbeat => {
                // Heartbeat received, connection is alive
            }
            FuturesEvent::Disconnected { reason } => {
                println!("[Disconnected] {}", reason);
                break;
            }
            _ => {}
        }

        if event_count >= max_events {
            println!("\n--- Reached {} events, stopping ---", max_events);
            break;
        }
    }

    // Cleanup
    conn_handle.abort();

    println!("\n=== Summary ===");
    println!("Total events processed: {}", event_count);
    println!("\nDone!");

    Ok(())
}
