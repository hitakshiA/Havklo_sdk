//! Common test utilities and fixtures for integration tests
//!
//! Contains sample JSON messages captured from live Kraken API v2

use kraken_book::{compute_checksum, Orderbook, OrderbookState};
use kraken_types::{BookData, Level, WsMessage};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Sample status message from Kraken on connection
pub const STATUS_MESSAGE: &str = r#"{
    "channel": "status",
    "type": "update",
    "data": [{
        "api_version": "v2",
        "connection_id": 12345678901234567890,
        "system": "online",
        "version": "2.0.10"
    }]
}"#;

/// Sample heartbeat message
pub const HEARTBEAT_MESSAGE: &str = r#"{
    "channel": "heartbeat"
}"#;

/// Sample subscribe response
pub const SUBSCRIBE_RESPONSE: &str = r#"{
    "method": "subscribe",
    "req_id": 1,
    "result": {
        "channel": "book",
        "depth": 10,
        "snapshot": true,
        "symbol": "BTC/USD"
    },
    "success": true,
    "time_in": "2025-12-21T12:28:24.000000Z",
    "time_out": "2025-12-21T12:28:24.001000Z"
}"#;

/// Sample book snapshot message (with computed checksum)
pub fn book_snapshot_message() -> String {
    // Create levels that will produce a known checksum
    let bids = vec![
        Level::new(dec!(100000.0), dec!(1.5)),
        Level::new(dec!(99999.0), dec!(2.0)),
        Level::new(dec!(99998.0), dec!(0.5)),
    ];
    let asks = vec![
        Level::new(dec!(100001.0), dec!(1.0)),
        Level::new(dec!(100002.0), dec!(2.5)),
        Level::new(dec!(100003.0), dec!(0.75)),
    ];

    let checksum = compute_checksum(&bids, &asks);

    format!(
        r#"{{
    "channel": "book",
    "type": "snapshot",
    "data": [{{
        "symbol": "BTC/USD",
        "bids": [
            {{"price": 100000.0, "qty": 1.5}},
            {{"price": 99999.0, "qty": 2.0}},
            {{"price": 99998.0, "qty": 0.5}}
        ],
        "asks": [
            {{"price": 100001.0, "qty": 1.0}},
            {{"price": 100002.0, "qty": 2.5}},
            {{"price": 100003.0, "qty": 0.75}}
        ],
        "checksum": {},
        "timestamp": "2025-12-21T12:28:24.113018Z"
    }}]
}}"#,
        checksum
    )
}

/// Sample book update message (with computed checksum after update)
pub fn book_update_message() -> String {
    // After the update: bid at 100000 changes to qty 2.0
    let bids = vec![
        Level::new(dec!(100000.0), dec!(2.0)), // Updated
        Level::new(dec!(99999.0), dec!(2.0)),
        Level::new(dec!(99998.0), dec!(0.5)),
    ];
    let asks = vec![
        Level::new(dec!(100001.0), dec!(1.0)),
        Level::new(dec!(100002.0), dec!(2.5)),
        Level::new(dec!(100003.0), dec!(0.75)),
    ];

    let checksum = compute_checksum(&bids, &asks);

    format!(
        r#"{{
    "channel": "book",
    "type": "update",
    "data": [{{
        "symbol": "BTC/USD",
        "bids": [
            {{"price": 100000.0, "qty": 2.0}}
        ],
        "asks": [],
        "checksum": {},
        "timestamp": "2025-12-21T12:28:24.321740Z"
    }}]
}}"#,
        checksum
    )
}

/// Sample book update that removes a level (qty = 0)
pub fn book_remove_level_message() -> String {
    // After remove: bid at 99998 is removed
    let bids = vec![
        Level::new(dec!(100000.0), dec!(2.0)),
        Level::new(dec!(99999.0), dec!(2.0)),
        // 99998.0 removed
    ];
    let asks = vec![
        Level::new(dec!(100001.0), dec!(1.0)),
        Level::new(dec!(100002.0), dec!(2.5)),
        Level::new(dec!(100003.0), dec!(0.75)),
    ];

    let checksum = compute_checksum(&bids, &asks);

    format!(
        r#"{{
    "channel": "book",
    "type": "update",
    "data": [{{
        "symbol": "BTC/USD",
        "bids": [
            {{"price": 99998.0, "qty": 0}}
        ],
        "asks": [],
        "checksum": {},
        "timestamp": "2025-12-21T12:28:24.500000Z"
    }}]
}}"#,
        checksum
    )
}

/// Create a test orderbook with known state
pub fn create_test_orderbook() -> Orderbook {
    let book = Orderbook::new("BTC/USD");
    assert_eq!(book.state(), OrderbookState::Uninitialized);
    book
}

/// Create book data for testing
pub fn create_book_data(
    bids: Vec<(Decimal, Decimal)>,
    asks: Vec<(Decimal, Decimal)>,
) -> BookData {
    let bids: Vec<Level> = bids
        .into_iter()
        .map(|(p, q)| Level::new(p, q))
        .collect();
    let asks: Vec<Level> = asks
        .into_iter()
        .map(|(p, q)| Level::new(p, q))
        .collect();

    let checksum = compute_checksum(&bids, &asks);

    BookData {
        symbol: "BTC/USD".to_string(),
        bids,
        asks,
        checksum,
        timestamp: None,
    }
}

/// Parse and verify a message
pub fn parse_message(json: &str) -> WsMessage {
    WsMessage::parse(json).expect("Failed to parse message")
}

/// ETH/USD snapshot for multi-symbol testing
pub fn eth_book_snapshot_message() -> String {
    let bids = vec![
        Level::new(dec!(3500.0), dec!(10.0)),
        Level::new(dec!(3499.0), dec!(20.0)),
    ];
    let asks = vec![
        Level::new(dec!(3501.0), dec!(15.0)),
        Level::new(dec!(3502.0), dec!(25.0)),
    ];

    let checksum = compute_checksum(&bids, &asks);

    format!(
        r#"{{
    "channel": "book",
    "type": "snapshot",
    "data": [{{
        "symbol": "ETH/USD",
        "bids": [
            {{"price": 3500.0, "qty": 10.0}},
            {{"price": 3499.0, "qty": 20.0}}
        ],
        "asks": [
            {{"price": 3501.0, "qty": 15.0}},
            {{"price": 3502.0, "qty": 25.0}}
        ],
        "checksum": {},
        "timestamp": "2025-12-21T12:28:24.113018Z"
    }}]
}}"#,
        checksum
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_parse_correctly() {
        // Status
        let status = parse_message(STATUS_MESSAGE);
        assert!(matches!(status, WsMessage::Status(_)));

        // Heartbeat
        let heartbeat = parse_message(HEARTBEAT_MESSAGE);
        assert!(matches!(heartbeat, WsMessage::Heartbeat));

        // Subscribe response
        let subscribe = parse_message(SUBSCRIBE_RESPONSE);
        assert!(matches!(subscribe, WsMessage::Method(_)));

        // Book snapshot
        let snapshot = parse_message(&book_snapshot_message());
        assert!(matches!(snapshot, WsMessage::Book(_)));

        // Book update
        let update = parse_message(&book_update_message());
        assert!(matches!(update, WsMessage::Book(_)));
    }

    #[test]
    fn test_book_data_helper() {
        let data = create_book_data(
            vec![(dec!(100), dec!(1))],
            vec![(dec!(101), dec!(1))],
        );

        assert_eq!(data.symbol, "BTC/USD");
        assert_eq!(data.bids.len(), 1);
        assert_eq!(data.asks.len(), 1);
        assert!(data.checksum > 0);
    }
}
