//! Integration tests for Havklo SDK
//!
//! Tests the full SDK flow including message parsing, orderbook management,
//! and checksum validation.

mod common;

use common::*;
use kraken_book::{compute_checksum, Orderbook, OrderbookState};
use kraken_types::{ChannelMessage, Decimal, Level, WsMessage};
use rust_decimal_macros::dec;

// =============================================================================
// Message Parsing Tests
// =============================================================================

#[test]
fn test_parse_status_message() {
    let msg = parse_message(STATUS_MESSAGE);

    if let WsMessage::Status(status) = msg {
        assert_eq!(status.channel, "status");
        assert_eq!(status.msg_type, "update");
        assert!(!status.data.is_empty());

        let data = &status.data[0];
        assert_eq!(data.api_version, "v2");
        assert_eq!(data.system.to_string(), "online");
    } else {
        panic!("Expected Status message");
    }
}

#[test]
fn test_parse_heartbeat() {
    let msg = parse_message(HEARTBEAT_MESSAGE);
    assert!(matches!(msg, WsMessage::Heartbeat));
}

#[test]
fn test_parse_subscribe_response() {
    let msg = parse_message(SUBSCRIBE_RESPONSE);

    if let WsMessage::Method(resp) = msg {
        assert!(resp.success);
        assert_eq!(resp.req_id, Some(1));

        let result = resp.result.expect("Expected result");
        assert_eq!(result.channel, "book");
        assert_eq!(result.symbol, Some("BTC/USD".to_string()));
    } else {
        panic!("Expected Method response");
    }
}

#[test]
fn test_parse_book_snapshot() {
    let msg = parse_message(&book_snapshot_message());

    if let WsMessage::Book(book) = msg {
        assert_eq!(book.channel, "book");
        assert_eq!(book.msg_type, "snapshot");
        assert!(!book.data.is_empty());

        let data = &book.data[0];
        assert_eq!(data.symbol, "BTC/USD");
        assert_eq!(data.bids.len(), 3);
        assert_eq!(data.asks.len(), 3);
        assert!(data.checksum > 0);
    } else {
        panic!("Expected Book message");
    }
}

#[test]
fn test_parse_book_update() {
    let msg = parse_message(&book_update_message());

    if let WsMessage::Book(book) = msg {
        assert_eq!(book.msg_type, "update");
        assert!(!book.data.is_empty());

        let data = &book.data[0];
        assert_eq!(data.bids.len(), 1);
        assert!(data.asks.is_empty());
    } else {
        panic!("Expected Book message");
    }
}

// =============================================================================
// Orderbook Lifecycle Tests
// =============================================================================

#[test]
fn test_full_orderbook_lifecycle() {
    let mut book = create_test_orderbook();
    assert_eq!(book.state(), OrderbookState::Uninitialized);

    // Apply snapshot
    let snapshot_msg = parse_message(&book_snapshot_message());
    if let WsMessage::Book(ChannelMessage { data, msg_type, .. }) = snapshot_msg {
        let is_snapshot = msg_type == "snapshot";
        book.apply_book_data(&data[0], is_snapshot).unwrap();
    }

    assert_eq!(book.state(), OrderbookState::Synced);
    assert!(book.is_synced());
    assert_eq!(book.bid_count(), 3);
    assert_eq!(book.ask_count(), 3);

    // Verify best bid/ask
    assert_eq!(book.best_bid().unwrap().price, dec!(100000.0));
    assert_eq!(book.best_ask().unwrap().price, dec!(100001.0));

    // Verify spread and mid price
    assert_eq!(book.spread(), Some(dec!(1.0)));
    assert_eq!(book.mid_price(), Some(dec!(100000.5)));

    // Apply update
    let update_msg = parse_message(&book_update_message());
    if let WsMessage::Book(ChannelMessage { data, msg_type, .. }) = update_msg {
        let is_snapshot = msg_type == "snapshot";
        book.apply_book_data(&data[0], is_snapshot).unwrap();
    }

    assert!(book.is_synced());
    assert_eq!(book.best_bid().unwrap().qty, dec!(2.0)); // Updated quantity

    // Apply remove level
    let remove_msg = parse_message(&book_remove_level_message());
    if let WsMessage::Book(ChannelMessage { data, msg_type, .. }) = remove_msg {
        let is_snapshot = msg_type == "snapshot";
        book.apply_book_data(&data[0], is_snapshot).unwrap();
    }

    assert!(book.is_synced());
    assert_eq!(book.bid_count(), 2); // One level removed
}

#[test]
fn test_checksum_validation_with_real_data() {
    let mut book = Orderbook::new("BTC/USD");

    // Create data with correct checksum
    let data = create_book_data(
        vec![
            (dec!(100.5), dec!(1.0)),
            (dec!(100.0), dec!(2.0)),
            (dec!(99.5), dec!(3.0)),
        ],
        vec![
            (dec!(101.0), dec!(1.5)),
            (dec!(101.5), dec!(2.5)),
            (dec!(102.0), dec!(3.5)),
        ],
    );

    // Apply should succeed
    let result = book.apply_book_data(&data, true);
    assert!(result.is_ok());
    assert!(book.is_synced());
}

#[test]
fn test_checksum_mismatch_desynchronizes() {
    let mut book = Orderbook::new("BTC/USD");

    // Create data with WRONG checksum
    let mut data = create_book_data(
        vec![(dec!(100), dec!(1))],
        vec![(dec!(101), dec!(1))],
    );
    data.checksum = 12345; // Wrong checksum

    let result = book.apply_book_data(&data, true);
    assert!(result.is_err());
    assert_eq!(book.state(), OrderbookState::Desynchronized);
}

#[test]
fn test_multi_symbol_concurrent_updates() {
    let mut btc_book = Orderbook::new("BTC/USD");
    let mut eth_book = Orderbook::new("ETH/USD");

    // Apply BTC snapshot
    let btc_snapshot = parse_message(&book_snapshot_message());
    if let WsMessage::Book(ChannelMessage { data, msg_type, .. }) = btc_snapshot {
        btc_book.apply_book_data(&data[0], msg_type == "snapshot").unwrap();
    }

    // Apply ETH snapshot
    let eth_snapshot = parse_message(&eth_book_snapshot_message());
    if let WsMessage::Book(ChannelMessage { data, msg_type, .. }) = eth_snapshot {
        eth_book.apply_book_data(&data[0], msg_type == "snapshot").unwrap();
    }

    // Both should be synced
    assert!(btc_book.is_synced());
    assert!(eth_book.is_synced());

    // Verify different prices
    assert_eq!(btc_book.best_bid().unwrap().price, dec!(100000.0));
    assert_eq!(eth_book.best_bid().unwrap().price, dec!(3500.0));
}

// =============================================================================
// Checksum Algorithm Tests
// =============================================================================

#[test]
fn test_checksum_deterministic() {
    let bids = vec![
        Level::new(dec!(100), dec!(1)),
        Level::new(dec!(99), dec!(2)),
    ];
    let asks = vec![
        Level::new(dec!(101), dec!(1)),
        Level::new(dec!(102), dec!(2)),
    ];

    let checksum1 = compute_checksum(&bids, &asks);
    let checksum2 = compute_checksum(&bids, &asks);

    assert_eq!(checksum1, checksum2);
}

#[test]
fn test_checksum_order_sensitive() {
    let bids1 = vec![Level::new(dec!(100), dec!(1))];
    let asks1 = vec![Level::new(dec!(101), dec!(1))];

    let bids2 = vec![Level::new(dec!(101), dec!(1))];
    let asks2 = vec![Level::new(dec!(100), dec!(1))];

    let checksum1 = compute_checksum(&bids1, &asks1);
    let checksum2 = compute_checksum(&bids2, &asks2);

    // Swapped sides should produce different checksum
    assert_ne!(checksum1, checksum2);
}

#[test]
fn test_checksum_uses_top_10_only() {
    // Create more than 10 levels
    let mut bids: Vec<Level> = (0..15)
        .map(|i| Level::new(dec!(100) - Decimal::from(i), dec!(1)))
        .collect();
    let asks: Vec<Level> = (0..15)
        .map(|i| Level::new(dec!(101) + Decimal::from(i), dec!(1)))
        .collect();

    let checksum1 = compute_checksum(&bids, &asks);

    // Add more levels beyond 10
    bids.push(Level::new(dec!(50), dec!(100)));

    let checksum2 = compute_checksum(&bids, &asks);

    // Checksum should be the same (only uses top 10)
    assert_eq!(checksum1, checksum2);
}

// =============================================================================
// Decimal Precision Tests
// =============================================================================

#[test]
fn test_decimal_precision_preserved() {
    // Test that small quantities are handled correctly
    let json = r#"{
        "channel": "book",
        "type": "snapshot",
        "data": [{
            "symbol": "BTC/USD",
            "bids": [{"price": 100000.12345, "qty": 0.00000001}],
            "asks": [{"price": 100001.54321, "qty": 0.12345678}],
            "checksum": 0,
            "timestamp": "2025-12-21T12:00:00Z"
        }]
    }"#;

    let msg = parse_message(json);
    if let WsMessage::Book(book) = msg {
        let data = &book.data[0];

        // Verify precision is preserved
        assert_eq!(data.bids[0].price.to_string(), "100000.12345");
        assert_eq!(data.bids[0].qty.to_string(), "0.00000001");
        assert_eq!(data.asks[0].price.to_string(), "100001.54321");
        assert_eq!(data.asks[0].qty.to_string(), "0.12345678");
    } else {
        panic!("Expected Book message");
    }
}

#[test]
fn test_scientific_notation_handling() {
    // Kraken sometimes sends very small quantities in scientific notation
    let json = r#"{
        "channel": "book",
        "type": "snapshot",
        "data": [{
            "symbol": "BTC/USD",
            "bids": [{"price": 100000, "qty": 5e-6}],
            "asks": [{"price": 100001, "qty": 1.5e-4}],
            "checksum": 0,
            "timestamp": "2025-12-21T12:00:00Z"
        }]
    }"#;

    let msg = parse_message(json);
    if let WsMessage::Book(book) = msg {
        let data = &book.data[0];

        // 5e-6 = 0.000005
        assert_eq!(data.bids[0].qty, dec!(0.000005));
        // 1.5e-4 = 0.00015
        assert_eq!(data.asks[0].qty, dec!(0.00015));
    } else {
        panic!("Expected Book message");
    }
}

// =============================================================================
// Snapshot Tests
// =============================================================================

#[test]
fn test_orderbook_snapshot_capture() {
    let mut book = Orderbook::new("BTC/USD");

    let data = create_book_data(
        vec![(dec!(100), dec!(1)), (dec!(99), dec!(2))],
        vec![(dec!(101), dec!(1)), (dec!(102), dec!(2))],
    );
    book.apply_book_data(&data, true).unwrap();

    let snapshot = book.snapshot();

    assert_eq!(snapshot.symbol, "BTC/USD");
    assert_eq!(snapshot.bids.len(), 2);
    assert_eq!(snapshot.asks.len(), 2);
    assert_eq!(snapshot.spread(), Some(dec!(1)));
    assert_eq!(snapshot.mid_price(), Some(dec!(100.5)));
}

// =============================================================================
// Live API Test (Ignored by default)
// =============================================================================

#[test]
#[ignore]
fn test_live_kraken_connection() {
    // Live connection test - run manually with:
    // cargo test test_live_kraken_connection -- --ignored --nocapture
    //
    // For comprehensive live testing, use the examples:
    // cargo run --example orderbook_stream -p kraken-sdk
    // cargo run --example multi_symbol -p kraken-sdk
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_orderbook() {
    let book = Orderbook::new("BTC/USD");

    assert!(book.best_bid().is_none());
    assert!(book.best_ask().is_none());
    assert!(book.spread().is_none());
    assert!(book.mid_price().is_none());
    assert_eq!(book.bid_count(), 0);
    assert_eq!(book.ask_count(), 0);
}

#[test]
fn test_orderbook_reset() {
    let mut book = Orderbook::new("BTC/USD");

    let data = create_book_data(
        vec![(dec!(100), dec!(1))],
        vec![(dec!(101), dec!(1))],
    );
    book.apply_book_data(&data, true).unwrap();

    assert!(book.is_synced());
    assert!(book.best_bid().is_some());

    book.reset();

    assert_eq!(book.state(), OrderbookState::Uninitialized);
    assert!(book.best_bid().is_none());
    assert_eq!(book.bid_count(), 0);
}

#[test]
fn test_update_before_snapshot_ignored() {
    let mut book = Orderbook::new("BTC/USD");

    // Try to apply update without snapshot first
    let data = create_book_data(
        vec![(dec!(100), dec!(1))],
        vec![(dec!(101), dec!(1))],
    );

    // Should be ignored (not synced yet)
    let result = book.apply_book_data(&data, false);
    assert!(result.is_ok()); // Doesn't error, just ignores

    // Still uninitialized
    assert_eq!(book.state(), OrderbookState::Uninitialized);
}
