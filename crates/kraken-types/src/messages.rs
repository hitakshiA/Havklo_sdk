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

    /// Create executions subscription params (private channel)
    ///
    /// Requires a valid WebSocket token from the TokenManager.
    pub fn executions(token: String) -> Self {
        Self {
            channel: Channel::Executions,
            symbol: vec![],
            depth: None,
            snapshot: Some(true),
            interval: None,
            event_trigger: None,
            token: Some(token),
        }
    }

    /// Create balances subscription params (private channel)
    ///
    /// Requires a valid WebSocket token from the TokenManager.
    pub fn balances(token: String) -> Self {
        Self {
            channel: Channel::Balances,
            symbol: vec![],
            depth: None,
            snapshot: Some(true),
            interval: None,
            event_trigger: None,
            token: Some(token),
        }
    }

    /// Set the authentication token for private channels
    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
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

/// Asset data from the instrument channel
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentAsset {
    /// Asset identifier (e.g., "BTC", "USD")
    pub id: String,
    /// Asset status
    #[serde(default)]
    pub status: Option<String>,
    /// Precision (number of decimal places)
    #[serde(default)]
    pub precision: Option<u8>,
    /// Display precision
    #[serde(default)]
    pub precision_display: Option<u8>,
    /// Whether asset is borrowable
    #[serde(default)]
    pub borrowable: Option<bool>,
    /// Collateral value
    #[serde(default)]
    pub collateral_value: Option<Decimal>,
}

/// Trading pair data from the instrument channel
/// Used to get price and quantity precision for checksum calculation
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentPair {
    /// Trading pair symbol (e.g., "BTC/USD")
    pub symbol: String,
    /// Price precision (number of decimal places)
    pub price_precision: u8,
    /// Quantity precision (number of decimal places)
    pub qty_precision: u8,
    /// Price increment (minimum price step)
    #[serde(default)]
    pub price_increment: Option<Decimal>,
    /// Quantity increment (minimum qty step)
    #[serde(default)]
    pub qty_increment: Option<Decimal>,
    /// Minimum order quantity
    #[serde(default)]
    pub qty_min: Option<Decimal>,
    /// Base asset
    #[serde(default)]
    pub base: Option<String>,
    /// Quote currency
    #[serde(default)]
    pub quote: Option<String>,
    /// Trading status
    #[serde(default)]
    pub status: Option<String>,
}

/// Instrument channel data structure
/// Contains both assets and trading pairs information
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentChannelData {
    /// List of assets
    #[serde(default)]
    pub assets: Vec<InstrumentAsset>,
    /// List of trading pairs
    #[serde(default)]
    pub pairs: Vec<InstrumentPair>,
}

/// Instrument channel message (has different structure than other channels)
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentMessage {
    /// Channel name
    pub channel: String,
    /// Message type: "snapshot" or "update"
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Instrument data containing assets and pairs
    pub data: InstrumentChannelData,
}

/// Alias for backwards compatibility
pub type InstrumentData = InstrumentPair;

// ============================================================================
// Private Channel Data Types
// ============================================================================

/// Execution/trade data from the executions channel (private)
#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionData {
    /// Execution type (e.g., "trade", "settled")
    #[serde(rename = "exec_type")]
    pub exec_type: String,
    /// Order ID
    pub order_id: String,
    /// Execution ID
    #[serde(default)]
    pub exec_id: Option<String>,
    /// Trade ID (if applicable)
    #[serde(default)]
    pub trade_id: Option<u64>,
    /// Trading pair symbol
    pub symbol: String,
    /// Order side
    pub side: Side,
    /// Order type
    pub order_type: String,
    /// Order quantity
    #[serde(default)]
    pub order_qty: Option<Decimal>,
    /// Limit price
    #[serde(default)]
    pub limit_price: Option<Decimal>,
    /// Last executed quantity
    #[serde(default)]
    pub last_qty: Option<Decimal>,
    /// Last executed price
    #[serde(default)]
    pub last_price: Option<Decimal>,
    /// Cumulative quantity filled
    #[serde(default)]
    pub cum_qty: Option<Decimal>,
    /// Average fill price
    #[serde(default)]
    pub avg_price: Option<Decimal>,
    /// Fee paid
    #[serde(default)]
    pub fee_paid: Option<Decimal>,
    /// Fee currency
    #[serde(default)]
    pub fee_currency: Option<String>,
    /// Order status
    #[serde(default)]
    pub order_status: Option<String>,
    /// Timestamp
    pub timestamp: String,
}

/// Balance data from the balances channel (private)
#[derive(Debug, Clone, Deserialize)]
pub struct BalanceData {
    /// Asset identifier (e.g., "BTC", "USD")
    pub asset: String,
    /// Available balance (for trading/withdrawal)
    pub balance: Decimal,
    /// Balance on hold (in open orders)
    #[serde(default)]
    pub hold_trade: Option<Decimal>,
}

/// Wallet balance snapshot
#[derive(Debug, Clone, Deserialize)]
pub struct WalletData {
    /// List of asset balances
    pub balances: Vec<BalanceData>,
    /// Wallet type
    #[serde(default)]
    pub wallet_type: Option<String>,
    /// Wallet ID
    #[serde(default)]
    pub wallet_id: Option<String>,
}

// ============================================================================
// Level 3 (L3) Orders Channel Data Types
// ============================================================================

/// L3 order event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum L3EventType {
    /// New order added to book
    Add,
    /// Order modified
    Modify,
    /// Order deleted from book
    Delete,
}

/// Individual L3 order entry
#[derive(Debug, Clone, Deserialize)]
pub struct L3Order {
    /// Unique order ID
    pub order_id: String,
    /// Limit price
    pub limit_price: Decimal,
    /// Order quantity
    pub order_qty: Decimal,
    /// Timestamp
    pub timestamp: String,
    /// Event type (add, modify, delete)
    #[serde(default)]
    pub event: Option<L3EventType>,
}

/// L3 channel data for a symbol
#[derive(Debug, Clone, Deserialize)]
pub struct L3Data {
    /// Trading pair symbol
    pub symbol: String,
    /// Bid orders
    #[serde(default)]
    pub bids: Vec<L3Order>,
    /// Ask orders
    #[serde(default)]
    pub asks: Vec<L3Order>,
    /// CRC32 checksum (for verification)
    #[serde(default)]
    pub checksum: Option<u32>,
}

/// L3 message type
pub type L3Message = ChannelMessage<L3Data>;

// ============================================================================
// WebSocket Trading Request Types
// ============================================================================

/// Time-in-force for orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum TimeInForce {
    /// Good til canceled
    #[serde(rename = "gtc")]
    GTC,
    /// Immediate or cancel
    #[serde(rename = "ioc")]
    IOC,
    /// Good til date
    #[serde(rename = "gtd")]
    GTD,
}

/// Add order request via WebSocket
#[derive(Debug, Clone, Serialize)]
pub struct AddOrderRequest {
    /// Method name
    pub method: &'static str,
    /// Request parameters
    pub params: AddOrderParams,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

/// Add order parameters
#[derive(Debug, Clone, Serialize)]
pub struct AddOrderParams {
    /// Order type (market, limit, etc.)
    pub order_type: String,
    /// Order side (buy/sell)
    pub side: Side,
    /// Trading pair symbol
    pub symbol: String,
    /// Order quantity
    #[serde(serialize_with = "serialize_decimal")]
    pub order_qty: Decimal,
    /// Limit price (required for limit orders)
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_decimal")]
    pub limit_price: Option<Decimal>,
    /// Time in force
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    /// Stop/trigger price
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_decimal")]
    pub trigger_price: Option<Decimal>,
    /// Client order ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<String>,
    /// Post-only flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,
    /// Reduce-only flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    /// WebSocket authentication token
    pub token: String,
}

impl AddOrderRequest {
    /// Create a new add order request
    pub fn new(params: AddOrderParams) -> Self {
        Self {
            method: "add_order",
            params,
            req_id: None,
        }
    }

    /// Set request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

/// Amend order request
#[derive(Debug, Clone, Serialize)]
pub struct AmendOrderRequest {
    /// Method name
    pub method: &'static str,
    /// Request parameters
    pub params: AmendOrderParams,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

/// Amend order parameters
#[derive(Debug, Clone, Serialize)]
pub struct AmendOrderParams {
    /// Order ID to amend
    pub order_id: String,
    /// New limit price
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_decimal")]
    pub limit_price: Option<Decimal>,
    /// New trigger price
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_decimal")]
    pub trigger_price: Option<Decimal>,
    /// New order quantity
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_decimal")]
    pub order_qty: Option<Decimal>,
    /// Post-only flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_only: Option<bool>,
    /// WebSocket authentication token
    pub token: String,
}

impl AmendOrderRequest {
    /// Create a new amend order request
    pub fn new(params: AmendOrderParams) -> Self {
        Self {
            method: "amend_order",
            params,
            req_id: None,
        }
    }

    /// Set request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

/// Cancel order request
#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderRequest {
    /// Method name
    pub method: &'static str,
    /// Request parameters
    pub params: CancelOrderParams,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

/// Cancel order parameters
#[derive(Debug, Clone, Serialize)]
pub struct CancelOrderParams {
    /// Order IDs to cancel
    pub order_id: Vec<String>,
    /// Client order IDs to cancel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<Vec<String>>,
    /// WebSocket authentication token
    pub token: String,
}

impl CancelOrderRequest {
    /// Create a new cancel order request
    pub fn new(params: CancelOrderParams) -> Self {
        Self {
            method: "cancel_order",
            params,
            req_id: None,
        }
    }

    /// Set request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

/// Cancel all orders request
#[derive(Debug, Clone, Serialize)]
pub struct CancelAllRequest {
    /// Method name
    pub method: &'static str,
    /// Request parameters
    pub params: CancelAllParams,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

/// Cancel all orders parameters
#[derive(Debug, Clone, Serialize)]
pub struct CancelAllParams {
    /// WebSocket authentication token
    pub token: String,
}

impl CancelAllRequest {
    /// Create a new cancel all request
    pub fn new(token: String) -> Self {
        Self {
            method: "cancel_all",
            params: CancelAllParams { token },
            req_id: None,
        }
    }

    /// Set request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

/// Cancel all orders on disconnect request
#[derive(Debug, Clone, Serialize)]
pub struct CancelOnDisconnectRequest {
    /// Method name
    pub method: &'static str,
    /// Request parameters
    pub params: CancelOnDisconnectParams,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

/// Cancel on disconnect parameters
#[derive(Debug, Clone, Serialize)]
pub struct CancelOnDisconnectParams {
    /// Timeout in seconds (0 to disable)
    pub timeout: u32,
    /// WebSocket authentication token
    pub token: String,
}

impl CancelOnDisconnectRequest {
    /// Create a new cancel on disconnect request
    pub fn new(timeout: u32, token: String) -> Self {
        Self {
            method: "cancel_all_orders_after",
            params: CancelOnDisconnectParams { timeout, token },
            req_id: None,
        }
    }

    /// Set request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

/// Batch add orders request
#[derive(Debug, Clone, Serialize)]
pub struct BatchAddRequest {
    /// Method name
    pub method: &'static str,
    /// Request parameters
    pub params: BatchAddParams,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

/// Batch add parameters
#[derive(Debug, Clone, Serialize)]
pub struct BatchAddParams {
    /// List of orders to add
    pub orders: Vec<BatchOrder>,
    /// WebSocket authentication token
    pub token: String,
    /// Deadline for execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    /// Validate only, don't execute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate: Option<bool>,
}

/// Individual order in batch
#[derive(Debug, Clone, Serialize)]
pub struct BatchOrder {
    /// Order type
    pub order_type: String,
    /// Side
    pub side: Side,
    /// Symbol
    pub symbol: String,
    /// Quantity
    #[serde(serialize_with = "serialize_decimal")]
    pub order_qty: Decimal,
    /// Limit price
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "serialize_option_decimal")]
    pub limit_price: Option<Decimal>,
    /// Client order ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<String>,
}

impl BatchAddRequest {
    /// Create a new batch add request
    pub fn new(params: BatchAddParams) -> Self {
        Self {
            method: "batch_add",
            params,
            req_id: None,
        }
    }

    /// Set request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

/// Batch cancel orders request
#[derive(Debug, Clone, Serialize)]
pub struct BatchCancelRequest {
    /// Method name
    pub method: &'static str,
    /// Request parameters
    pub params: BatchCancelParams,
    /// Optional request ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_id: Option<u64>,
}

/// Batch cancel parameters
#[derive(Debug, Clone, Serialize)]
pub struct BatchCancelParams {
    /// Order IDs to cancel
    pub orders: Vec<String>,
    /// Client order IDs to cancel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cl_ord_id: Option<Vec<String>>,
    /// WebSocket authentication token
    pub token: String,
}

impl BatchCancelRequest {
    /// Create a new batch cancel request
    pub fn new(params: BatchCancelParams) -> Self {
        Self {
            method: "batch_cancel",
            params,
            req_id: None,
        }
    }

    /// Set request ID
    pub fn with_req_id(mut self, id: u64) -> Self {
        self.req_id = Some(id);
        self
    }
}

// ============================================================================
// Trading Response Types
// ============================================================================

/// Add order response result
#[derive(Debug, Clone, Deserialize)]
pub struct AddOrderResult {
    /// Assigned order ID
    pub order_id: String,
    /// Client order ID (if provided)
    #[serde(default)]
    pub cl_ord_id: Option<String>,
    /// Order status
    #[serde(default)]
    pub order_status: Option<String>,
    /// Order creation timestamp
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// Cancel order response result
#[derive(Debug, Clone, Deserialize)]
pub struct CancelOrderResult {
    /// Order ID that was canceled
    pub order_id: String,
    /// Client order ID
    #[serde(default)]
    pub cl_ord_id: Option<String>,
}

/// Cancel all response result
#[derive(Debug, Clone, Deserialize)]
pub struct CancelAllResult {
    /// Number of orders canceled
    pub count: u32,
}

/// Cancel on disconnect response result
#[derive(Debug, Clone, Deserialize)]
pub struct CancelOnDisconnectResult {
    /// Current timeout setting
    #[serde(default)]
    pub current_time: Option<String>,
    /// Trigger time (when orders will be canceled)
    #[serde(default)]
    pub trigger_time: Option<String>,
}

/// Batch order response
#[derive(Debug, Clone, Deserialize)]
pub struct BatchOrderResult {
    /// Results for each order in the batch
    #[serde(default)]
    pub orders: Vec<BatchOrderResultItem>,
}

/// Individual batch order result
#[derive(Debug, Clone, Deserialize)]
pub struct BatchOrderResultItem {
    /// Order ID
    #[serde(default)]
    pub order_id: Option<String>,
    /// Client order ID
    #[serde(default)]
    pub cl_ord_id: Option<String>,
    /// Error message if failed
    #[serde(default)]
    pub error: Option<String>,
}

/// Helper function to serialize Decimal as string
fn serialize_decimal<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

/// Helper function to serialize Option<Decimal> as string
fn serialize_option_decimal<S>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(d) => serializer.serialize_str(&d.to_string()),
        None => serializer.serialize_none(),
    }
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

/// Executions message type (private)
pub type ExecutionsMessage = ChannelMessage<ExecutionData>;

/// Balances message type (private)
pub type BalancesMessage = ChannelMessage<WalletData>;

// Note: InstrumentMessage is defined as a separate struct above
// because its data structure differs from other channel messages

// ============================================================================
// Raw Message Parsing
// ============================================================================

/// Parsed message from WebSocket
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum WsMessage {
    /// Method response (subscribe, unsubscribe, pong, trading responses)
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
    /// Instrument channel update (provides precision info)
    Instrument(InstrumentMessage),
    /// Executions channel update (private - requires auth)
    Executions(ExecutionsMessage),
    /// Balances channel update (private - requires auth)
    Balances(BalancesMessage),
    /// Level 3 orders channel update (requires special access)
    Level3(L3Message),
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
            Some("instrument") => {
                let msg: InstrumentMessage = serde_json::from_value(value)?;
                Ok(Self::Instrument(msg))
            }
            Some("executions") => {
                let msg: ExecutionsMessage = serde_json::from_value(value)?;
                Ok(Self::Executions(msg))
            }
            Some("balances") => {
                let msg: BalancesMessage = serde_json::from_value(value)?;
                Ok(Self::Balances(msg))
            }
            Some("level3") => {
                let msg: L3Message = serde_json::from_value(value)?;
                Ok(Self::Level3(msg))
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

    /// Check if this is an L3 snapshot
    pub fn is_l3_snapshot(&self) -> bool {
        matches!(self, Self::Level3(msg) if msg.msg_type == "snapshot")
    }

    /// Check if this is an L3 update
    pub fn is_l3_update(&self) -> bool {
        matches!(self, Self::Level3(msg) if msg.msg_type == "update")
    }

    /// Check if this is a trading method response (add_order, cancel_order, etc.)
    pub fn is_trading_response(&self) -> bool {
        matches!(self, Self::Method(resp) if matches!(
            resp.method.as_str(),
            "add_order" | "amend_order" | "edit_order" | "cancel_order" |
            "cancel_all" | "cancel_all_orders_after" | "batch_add" | "batch_cancel"
        ))
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
