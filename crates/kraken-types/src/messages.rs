//! Request and response message types for Kraken WebSocket API v2

use crate::{Channel, Depth, Level, OhlcInterval, Side, SystemStatus, TickerTrigger};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ============================================================================
// Request Types
// ============================================================================

/// Subscribe request message
#[derive(Debug, Clone, Serialize)]
pub struct SubscribeRequest {
    /// Always "subscribe"
    pub method: &'static str,
    /// Subscription parameters
    pub params: SubscribeParams,
    /// Optional request ID (echoed in response)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

impl SubscribeRequest {
    /// Create a new subscribe request
    pub fn new(params: SubscribeParams) -> Self {
        Self {
            method: "subscribe",
            params,
            req_id: None,
        }
    }

    /// Add a request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

/// Subscription parameters
#[derive(Debug, Clone, Serialize)]
pub struct SubscribeParams {
    /// Channel to subscribe to
    pub channel: Channel,
    /// Symbols to subscribe to
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub symbol: Vec<String>,
    /// Orderbook depth (book channel only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
    /// Whether to receive initial snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<bool>,
    /// OHLC interval (ohlc channel only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
    /// Ticker event trigger
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_trigger: Option<TickerTrigger>,
    /// Authentication token (private channels)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl SubscribeParams {
    /// Create book subscription params
    pub fn book(symbols: Vec<String>, depth: Depth) -> Self {
        Self {
            channel: Channel::Book,
            symbol: symbols,
            depth: Some(depth.as_u32()),
            snapshot: Some(true),
            interval: None,
            event_trigger: None,
            token: None,
        }
    }

    /// Create ticker subscription params
    pub fn ticker(symbols: Vec<String>) -> Self {
        Self {
            channel: Channel::Ticker,
            symbol: symbols,
            depth: None,
            snapshot: None,
            interval: None,
            event_trigger: Some(TickerTrigger::Trades),
            token: None,
        }
    }

    /// Create trade subscription params
    pub fn trade(symbols: Vec<String>) -> Self {
        Self {
            channel: Channel::Trade,
            symbol: symbols,
            depth: None,
            snapshot: Some(true),
            interval: None,
            event_trigger: None,
            token: None,
        }
    }

    /// Create OHLC subscription params
    pub fn ohlc(symbols: Vec<String>, interval: OhlcInterval) -> Self {
        Self {
            channel: Channel::Ohlc,
            symbol: symbols,
            depth: None,
            snapshot: None,
            interval: Some(interval as u32),
            event_trigger: None,
            token: None,
        }
    }
}

/// Unsubscribe request message
#[derive(Debug, Clone, Serialize)]
pub struct UnsubscribeRequest {
    /// Always "unsubscribe"
    pub method: &'static str,
    /// Unsubscription parameters
    pub params: SubscribeParams,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

impl UnsubscribeRequest {
    /// Create a new unsubscribe request
    pub fn new(params: SubscribeParams) -> Self {
        Self {
            method: "unsubscribe",
            params,
            req_id: None,
        }
    }
}

/// Ping request for keepalive
#[derive(Debug, Clone, Serialize)]
pub struct PingRequest {
    /// Always "ping"
    pub method: &'static str,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

impl PingRequest {
    /// Create a new ping request
    pub fn new() -> Self {
        Self {
            method: "ping",
            req_id: None,
        }
    }

    /// Add a request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

impl Default for PingRequest {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// Subscribe/Unsubscribe response
#[derive(Debug, Clone, Deserialize)]
pub struct MethodResponse {
    /// Method name (subscribe, unsubscribe, pong)
    pub method: String,
    /// Result details
    pub result: Option<SubscribeResult>,
    /// Whether the operation succeeded
    pub success: bool,
    /// Request processing start time
    pub time_in: String,
    /// Request processing end time
    pub time_out: String,
    /// Echoed request ID
    #[serde(default)]
    pub req_id: Option<u64>,
    /// Error message if failed
    #[serde(default)]
    pub error: Option<String>,
}

/// Subscription result details
#[derive(Debug, Clone, Deserialize)]
pub struct SubscribeResult {
    /// Channel name
    pub channel: String,
    /// Symbol (for symbol-specific subscriptions)
    #[serde(default)]
    pub symbol: Option<String>,
    /// Depth (for book subscriptions)
    #[serde(default)]
    pub depth: Option<u32>,
    /// Snapshot flag
    #[serde(default)]
    pub snapshot: Option<bool>,
}

// ============================================================================
// Channel Data Types
// ============================================================================

/// Generic channel message wrapper
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelMessage<T> {
    /// Channel name
    pub channel: String,
    /// Message type: "snapshot" or "update"
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Channel-specific data
    pub data: Vec<T>,
}

/// Status channel data (sent on connection)
#[derive(Debug, Clone, Deserialize)]
pub struct StatusData {
    /// API version
    pub api_version: String,
    /// Unique connection ID
    pub connection_id: u64,
    /// System status
    pub system: SystemStatus,
    /// Server version
    pub version: String,
}

/// Book snapshot/update data
#[derive(Debug, Clone, Deserialize)]
pub struct BookData {
    /// Trading pair symbol
    pub symbol: String,
    /// Bid levels
    pub bids: Vec<Level>,
    /// Ask levels
    pub asks: Vec<Level>,
    /// CRC32 checksum
    pub checksum: u32,
    /// Update timestamp (updates only)
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// Ticker data
#[derive(Debug, Clone, Deserialize)]
pub struct TickerData {
    /// Trading pair symbol
    pub symbol: String,
    /// Best bid price
    pub bid: Decimal,
    /// Best bid quantity
    pub bid_qty: Decimal,
    /// Best ask price
    pub ask: Decimal,
    /// Best ask quantity
    pub ask_qty: Decimal,
    /// Last trade price
    pub last: Decimal,
    /// 24h volume
    pub volume: Decimal,
    /// 24h VWAP
    pub vwap: Decimal,
    /// 24h low
    pub low: Decimal,
    /// 24h high
    pub high: Decimal,
    /// 24h price change
    pub change: Decimal,
    /// 24h price change percentage
    pub change_pct: Decimal,
}

/// Trade data
#[derive(Debug, Clone, Deserialize)]
pub struct TradeData {
    /// Trading pair symbol
    pub symbol: String,
    /// Trade side
    pub side: Side,
    /// Trade price
    pub price: Decimal,
    /// Trade quantity
    pub qty: Decimal,
    /// Order type
    pub ord_type: String,
    /// Unique trade ID
    pub trade_id: u64,
    /// Trade timestamp
    pub timestamp: String,
}

/// OHLC candle data
#[derive(Debug, Clone, Deserialize)]
pub struct OhlcData {
    /// Trading pair symbol
    pub symbol: String,
    /// Open price
    pub open: Decimal,
    /// High price
    pub high: Decimal,
    /// Low price
    pub low: Decimal,
    /// Close price
    pub close: Decimal,
    /// VWAP
    pub vwap: Decimal,
    /// Volume
    pub volume: Decimal,
    /// Number of trades
    pub trades: u64,
    /// Interval start time
    pub interval_begin: String,
    /// Interval in minutes
    pub interval: u32,
}

// ============================================================================
// Convenience Type Aliases
// ============================================================================

/// Status message type
pub type StatusMessage = ChannelMessage<StatusData>;

/// Book message type
pub type BookMessage = ChannelMessage<BookData>;

/// Ticker message type
pub type TickerMessage = ChannelMessage<TickerData>;

/// Trade message type
pub type TradeMessage = ChannelMessage<TradeData>;

/// OHLC message type
pub type OhlcMessage = ChannelMessage<OhlcData>;

// ============================================================================
// Raw Message Parsing
// ============================================================================

/// Parsed message from WebSocket
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// Method response (subscribe, unsubscribe, pong)
    Method(MethodResponse),
    /// Status channel update
    Status(StatusMessage),
    /// Book channel update
    Book(BookMessage),
    /// Ticker channel update
    Ticker(TickerMessage),
    /// Trade channel update
    Trade(TradeMessage),
    /// OHLC channel update
    Ohlc(OhlcMessage),
    /// Heartbeat message
    Heartbeat,
    /// Unknown/unsupported message
    Unknown(serde_json::Value),
}

/// Type alias for backwards compatibility and clarity
pub type SubscribeResponse = MethodResponse;

impl WsMessage {
    /// Parse a raw JSON message
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;

        // Check if it's a method response
        if value.get("method").is_some() {
            let response: MethodResponse = serde_json::from_value(value)?;
            return Ok(Self::Method(response));
        }

        // Check channel type
        let channel = value.get("channel").and_then(|v| v.as_str());

        match channel {
            Some("status") => {
                let msg: StatusMessage = serde_json::from_value(value)?;
                Ok(Self::Status(msg))
            }
            Some("book") => {
                let msg: BookMessage = serde_json::from_value(value)?;
                Ok(Self::Book(msg))
            }
            Some("ticker") => {
                let msg: TickerMessage = serde_json::from_value(value)?;
                Ok(Self::Ticker(msg))
            }
            Some("trade") => {
                let msg: TradeMessage = serde_json::from_value(value)?;
                Ok(Self::Trade(msg))
            }
            Some("ohlc") => {
                let msg: OhlcMessage = serde_json::from_value(value)?;
                Ok(Self::Ohlc(msg))
            }
            Some("heartbeat") => Ok(Self::Heartbeat),
            _ => Ok(Self::Unknown(value)),
        }
    }

    /// Check if this is a book snapshot
    pub fn is_book_snapshot(&self) -> bool {
        matches!(self, Self::Book(msg) if msg.msg_type == "snapshot")
    }

    /// Check if this is a book update
    pub fn is_book_update(&self) -> bool {
        matches!(self, Self::Book(msg) if msg.msg_type == "update")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_message() {
        let json = r#"{
            "channel": "status",
            "type": "update",
            "data": [{
                "api_version": "v2",
                "connection_id": 5688663770896937000,
                "system": "online",
                "version": "2.0.10"
            }]
        }"#;

        let msg = WsMessage::parse(json).unwrap();
        match msg {
            WsMessage::Status(status) => {
                assert_eq!(status.data[0].api_version, "v2");
                assert_eq!(status.data[0].system, SystemStatus::Online);
            }
            _ => panic!("Expected Status message"),
        }
    }

    #[test]
    fn test_parse_book_snapshot() {
        let json = r#"{
            "channel": "book",
            "type": "snapshot",
            "data": [{
                "symbol": "BTC/USD",
                "bids": [{"price": 88813.5, "qty": 0.00460208}],
                "asks": [{"price": 88813.6, "qty": 2.85806499}],
                "checksum": 2919786898,
                "timestamp": "2025-12-21T12:28:24.113018Z"
            }]
        }"#;

        let msg = WsMessage::parse(json).unwrap();
        assert!(msg.is_book_snapshot());

        match msg {
            WsMessage::Book(book) => {
                assert_eq!(book.data[0].symbol, "BTC/USD");
                assert_eq!(book.data[0].checksum, 2919786898);
                assert!(!book.data[0].bids.is_empty());
                assert!(!book.data[0].asks.is_empty());
            }
            _ => panic!("Expected Book message"),
        }
    }

    #[test]
    fn test_parse_subscribe_response() {
        let json = r#"{
            "method": "subscribe",
            "result": {
                "channel": "book",
                "depth": 10,
                "snapshot": true,
                "symbol": "BTC/USD"
            },
            "success": true,
            "time_in": "2025-12-21T12:28:05.362191Z",
            "time_out": "2025-12-21T12:28:05.362232Z"
        }"#;

        let msg = WsMessage::parse(json).unwrap();
        match msg {
            WsMessage::Method(resp) => {
                assert!(resp.success);
                assert_eq!(resp.method, "subscribe");
                assert!(resp.result.is_some());
            }
            _ => panic!("Expected Method response"),
        }
    }

    #[test]
    fn test_subscribe_request_serialization() {
        let params = SubscribeParams::book(vec!["BTC/USD".to_string()], Depth::D10);
        let req = SubscribeRequest::new(params).with_req_id(1);

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"subscribe\""));
        assert!(json.contains("\"channel\":\"book\""));
        assert!(json.contains("\"depth\":10"));
    }
}
