//! Example: Market Maker Skeleton with L3 Orderbook
//!
//! This example demonstrates a basic market making strategy using:
//! - L3 (order-level) orderbook for queue position tracking
//! - Bid/ask imbalance calculation
//! - VWAP computation for slippage estimation
//!
//! Run with: cargo run --example market_maker
//!
//! NOTE: This is a demonstration only. Real market making requires:
//! - Proper risk management
//! - Order management and position limits
//! - Latency optimization
//! - Authentication and trading API integration

use kraken_book::l3::{L3Book, L3Order, L3Side};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::Instant;

fn main() {
    println!("=== Market Maker Skeleton with L3 Orderbook ===\n");

    // Create L3 orderbook with depth of 100 levels
    let mut book = L3Book::new("BTC/USD", 100);

    // Simulate receiving a market snapshot
    println!("--- Simulating Market Snapshot ---\n");
    populate_book(&mut book);

    // Display book state
    display_book_summary(&book);

    // Simulate placing our own orders
    println!("\n--- Placing Market Making Orders ---\n");

    // Strategy: Quote around mid price with configurable spread
    let mid_price = book.mid_price().unwrap_or(dec!(50000));
    let half_spread = dec!(5.0); // $5 half-spread

    // Our bid
    let our_bid_price = mid_price - half_spread;
    let our_bid_id = "our_bid_001";
    book.add_order(
        L3Order::new(our_bid_id, our_bid_price, dec!(0.5)),
        L3Side::Bid,
    );
    println!("Placed BID: {} @ ${}", our_bid_id, our_bid_price);

    // Our ask
    let our_ask_price = mid_price + half_spread;
    let our_ask_id = "our_ask_001";
    book.add_order(
        L3Order::new(our_ask_id, our_ask_price, dec!(0.5)),
        L3Side::Ask,
    );
    println!("Placed ASK: {} @ ${}", our_ask_id, our_ask_price);

    // Check our queue positions
    println!("\n--- Queue Position Analysis ---\n");

    if let Some(pos) = book.queue_position(our_bid_id) {
        println!("Our BID queue position:");
        println!("  Position in queue:  {} of {}", pos.position, pos.total_orders);
        println!("  Quantity ahead:     {}", pos.qty_ahead);
        println!("  Fill probability:   {:.1}%", pos.fill_probability() * 100.0);
    }

    if let Some(pos) = book.queue_position(our_ask_id) {
        println!("\nOur ASK queue position:");
        println!("  Position in queue:  {} of {}", pos.position, pos.total_orders);
        println!("  Quantity ahead:     {}", pos.qty_ahead);
        println!("  Fill probability:   {:.1}%", pos.fill_probability() * 100.0);
    }

    // Market making signals
    println!("\n--- Market Making Signals ---\n");

    // 1. Imbalance signal
    if let Some(imbalance) = book.imbalance() {
        let signal = if imbalance > 0.3 {
            "BUY pressure - consider tightening ask"
        } else if imbalance < -0.3 {
            "SELL pressure - consider tightening bid"
        } else {
            "Balanced market"
        };
        println!("Imbalance: {:.2} → {}", imbalance, signal);
    }

    // 2. VWAP for execution cost estimation
    let trade_size = dec!(2.0);
    if let Some(vwap_buy) = book.vwap_ask(trade_size) {
        let slippage = (vwap_buy - mid_price) / mid_price * dec!(100);
        println!("VWAP to buy {} BTC: ${} (slippage: {:.3}%)",
            trade_size, vwap_buy, slippage
        );
    }

    if let Some(vwap_sell) = book.vwap_bid(trade_size) {
        let slippage = (mid_price - vwap_sell) / mid_price * dec!(100);
        println!("VWAP to sell {} BTC: ${} (slippage: {:.3}%)",
            trade_size, vwap_sell, slippage
        );
    }

    // 3. Spread analysis
    if let Some(spread) = book.spread() {
        let spread_bps = spread / mid_price * dec!(10000);
        println!("\nCurrent spread: ${} ({:.1} bps)", spread, spread_bps);

        // Our spread
        let our_spread = our_ask_price - our_bid_price;
        let our_spread_bps = our_spread / mid_price * dec!(10000);
        println!("Our spread:     ${} ({:.1} bps)", our_spread, our_spread_bps);
    }

    // Simulate an update - order in front of us gets filled
    println!("\n--- Simulating Order Fills ---\n");

    // Remove order ahead of our bid (simulating a fill)
    if let Some(best_bid) = book.best_bid() {
        if let Some(first_order) = best_bid.oldest() {
            let order_id = first_order.order_id.clone();
            if order_id != our_bid_id {
                println!("Order {} got filled, removing from book", order_id);
                book.remove_order(&order_id);

                // Check our new queue position
                if let Some(new_pos) = book.queue_position(our_bid_id) {
                    println!("Our new BID position: {} of {} (was closer!)",
                        new_pos.position, new_pos.total_orders
                    );
                }
            }
        }
    }

    // Performance demonstration
    println!("\n--- Performance Benchmark ---\n");
    benchmark_operations();

    println!("\n=== Market Maker Example Complete ===");
}

fn populate_book(book: &mut L3Book) {
    let mid = dec!(50000);

    // Add bid levels (highest to lowest)
    for i in 1..=20 {
        let price = mid - Decimal::from(i) * dec!(2);
        // Add multiple orders at each level
        for j in 0..3 {
            let order_id = format!("bid_{}_{}", i, j);
            let qty = dec!(0.5) + Decimal::from(j) * dec!(0.25);
            book.add_order(L3Order::new(order_id, price, qty), L3Side::Bid);
        }
    }

    // Add ask levels (lowest to highest)
    for i in 1..=20 {
        let price = mid + Decimal::from(i) * dec!(2);
        for j in 0..3 {
            let order_id = format!("ask_{}_{}", i, j);
            let qty = dec!(0.5) + Decimal::from(j) * dec!(0.25);
            book.add_order(L3Order::new(order_id, price, qty), L3Side::Ask);
        }
    }
}

fn display_book_summary(book: &L3Book) {
    println!("Book Summary for {}:", book.symbol());
    println!("  Bid levels: {}", book.bid_level_count());
    println!("  Ask levels: {}", book.ask_level_count());
    println!("  Total orders: {}", book.order_count());

    if let Some(bid) = book.best_bid_price() {
        println!("  Best bid: ${}", bid);
    }
    if let Some(ask) = book.best_ask_price() {
        println!("  Best ask: ${}", ask);
    }
    if let Some(mid) = book.mid_price() {
        println!("  Mid price: ${}", mid);
    }
    if let Some(spread) = book.spread() {
        println!("  Spread: ${}", spread);
    }

    println!("  Total bid qty: {}", book.total_bid_qty());
    println!("  Total ask qty: {}", book.total_ask_qty());
}

fn benchmark_operations() {
    let mut book = L3Book::new("TEST", 1000);

    // Benchmark order additions
    let start = Instant::now();
    let num_orders = 10000;

    for i in 0..num_orders {
        let price = dec!(50000) + Decimal::from(i % 100);
        let side = if i % 2 == 0 { L3Side::Bid } else { L3Side::Ask };
        book.add_order(
            L3Order::new(format!("order_{}", i), price, dec!(1.0)),
            side,
        );
    }

    let add_duration = start.elapsed();
    println!("Added {} orders in {:?} ({:.2} µs/order)",
        num_orders,
        add_duration,
        add_duration.as_micros() as f64 / num_orders as f64
    );

    // Benchmark queue position lookups
    let start = Instant::now();
    let lookups = 1000;

    for i in 0..lookups {
        let order_id = format!("order_{}", i);
        let _ = book.queue_position(&order_id);
    }

    let lookup_duration = start.elapsed();
    println!("Performed {} queue lookups in {:?} ({:.2} µs/lookup)",
        lookups,
        lookup_duration,
        lookup_duration.as_micros() as f64 / lookups as f64
    );

    // Benchmark VWAP calculation
    let start = Instant::now();
    let vwap_calcs = 1000;

    for _ in 0..vwap_calcs {
        let _ = book.vwap_ask(dec!(10.0));
        let _ = book.vwap_bid(dec!(10.0));
    }

    let vwap_duration = start.elapsed();
    println!("Performed {} VWAP calculations in {:?} ({:.2} µs/calc)",
        vwap_calcs * 2,
        vwap_duration,
        vwap_duration.as_micros() as f64 / (vwap_calcs * 2) as f64
    );
}
