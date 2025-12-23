//! Types for Kraken REST API requests and responses

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// API Response Wrapper
// ============================================================================

/// Standard Kraken API response wrapper
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    /// Error messages (empty if successful)
    pub error: Vec<String>,
    /// Result data (present if successful)
    pub result: Option<T>,
}

impl<T> ApiResponse<T> {
    /// Check if the response indicates success
    pub fn is_success(&self) -> bool {
        self.error.is_empty()
    }

    /// Get the result, returning an error if the API returned errors
    pub fn into_result(self) -> Result<T, Vec<String>> {
        if self.error.is_empty() {
            self.result.ok_or_else(|| vec!["No result in response".to_string()])
        } else {
            Err(self.error)
        }
    }
}

// ============================================================================
// Market Data Types
// ============================================================================

/// Ticker information for a trading pair
#[derive(Debug, Clone, Deserialize)]
pub struct TickerInfo {
    /// Ask [price, whole lot volume, lot volume]
    pub a: Vec<String>,
    /// Bid [price, whole lot volume, lot volume]
    pub b: Vec<String>,
    /// Last trade closed [price, lot volume]
    pub c: Vec<String>,
    /// Volume [today, last 24 hours]
    pub v: Vec<String>,
    /// Volume weighted average price [today, last 24 hours]
    pub p: Vec<String>,
    /// Number of trades [today, last 24 hours]
    pub t: Vec<u64>,
    /// Low [today, last 24 hours]
    pub l: Vec<String>,
    /// High [today, last 24 hours]
    pub h: Vec<String>,
    /// Today's opening price
    pub o: String,
}

impl TickerInfo {
    /// Get the current ask price
    pub fn ask_price(&self) -> Option<Decimal> {
        self.a.first().and_then(|s| s.parse().ok())
    }

    /// Get the current bid price
    pub fn bid_price(&self) -> Option<Decimal> {
        self.b.first().and_then(|s| s.parse().ok())
    }

    /// Get the last trade price
    pub fn last_price(&self) -> Option<Decimal> {
        self.c.first().and_then(|s| s.parse().ok())
    }

    /// Get the mid price (average of bid and ask)
    pub fn mid_price(&self) -> Option<Decimal> {
        let ask = self.ask_price()?;
        let bid = self.bid_price()?;
        Some((ask + bid) / Decimal::TWO)
    }

    /// Get spread in basis points
    pub fn spread_bps(&self) -> Option<Decimal> {
        let ask = self.ask_price()?;
        let bid = self.bid_price()?;
        let mid = self.mid_price()?;
        Some((ask - bid) / mid * Decimal::from(10000))
    }
}

/// Asset pair information
#[derive(Debug, Clone, Deserialize)]
pub struct AssetPairInfo {
    /// Alternate pair name
    pub altname: String,
    /// WebSocket pair name
    pub wsname: Option<String>,
    /// Asset class of base
    pub aclass_base: String,
    /// Base asset
    pub base: String,
    /// Asset class of quote
    pub aclass_quote: String,
    /// Quote asset
    pub quote: String,
    /// Volume lot size
    pub lot: String,
    /// Pair decimals
    pub pair_decimals: u32,
    /// Lot decimals
    pub lot_decimals: u32,
    /// Lot multiplier
    pub lot_multiplier: u32,
    /// Fee schedule array [volume, percent fee]
    pub fees: Vec<Vec<serde_json::Value>>,
    /// Maker fee schedule
    pub fees_maker: Option<Vec<Vec<serde_json::Value>>>,
    /// Minimum order size
    pub ordermin: Option<String>,
    /// Cost minimum
    pub costmin: Option<String>,
    /// Margin trading enabled
    pub margin_call: Option<u32>,
    /// Margin stop level
    pub margin_stop: Option<u32>,
}

/// OHLC candle data
#[derive(Debug, Clone, Deserialize)]
pub struct OhlcData {
    /// Candle data [time, open, high, low, close, vwap, volume, count]
    #[serde(flatten)]
    pub data: HashMap<String, Vec<Vec<serde_json::Value>>>,
    /// Last timestamp for pagination
    pub last: Option<u64>,
}

/// Individual OHLC candle
#[derive(Debug, Clone)]
pub struct OhlcCandle {
    /// Unix timestamp
    pub time: u64,
    /// Open price
    pub open: Decimal,
    /// High price
    pub high: Decimal,
    /// Low price
    pub low: Decimal,
    /// Close price
    pub close: Decimal,
    /// Volume weighted average price
    pub vwap: Decimal,
    /// Volume
    pub volume: Decimal,
    /// Number of trades
    pub count: u64,
}

/// Trade data from recent trades
#[derive(Debug, Clone)]
pub struct TradeData {
    /// Price
    pub price: Decimal,
    /// Volume
    pub volume: Decimal,
    /// Time (Unix timestamp with decimal)
    pub time: f64,
    /// Buy/sell indicator
    pub side: OrderSide,
    /// Market/limit indicator
    pub order_type: String,
    /// Miscellaneous
    pub misc: String,
}

/// Orderbook snapshot
#[derive(Debug, Clone, Deserialize)]
pub struct OrderbookData {
    /// Ask levels [price, volume, timestamp]
    pub asks: Vec<Vec<String>>,
    /// Bid levels [price, volume, timestamp]
    pub bids: Vec<Vec<String>>,
}

impl OrderbookData {
    /// Get the best ask price
    pub fn best_ask(&self) -> Option<Decimal> {
        self.asks.first().and_then(|level| level.first()?.parse().ok())
    }

    /// Get the best bid price
    pub fn best_bid(&self) -> Option<Decimal> {
        self.bids.first().and_then(|level| level.first()?.parse().ok())
    }

    /// Get the spread
    pub fn spread(&self) -> Option<Decimal> {
        Some(self.best_ask()? - self.best_bid()?)
    }
}

// ============================================================================
// Account Types
// ============================================================================

/// Account balance information
#[derive(Debug, Clone, Deserialize)]
pub struct BalanceInfo(pub HashMap<String, String>);

impl BalanceInfo {
    /// Get balance for a specific asset
    pub fn get(&self, asset: &str) -> Option<Decimal> {
        self.0.get(asset).and_then(|s| s.parse().ok())
    }

    /// Get all non-zero balances
    pub fn non_zero(&self) -> HashMap<String, Decimal> {
        self.0
            .iter()
            .filter_map(|(k, v)| {
                let balance: Decimal = v.parse().ok()?;
                if balance.is_zero() {
                    None
                } else {
                    Some((k.clone(), balance))
                }
            })
            .collect()
    }

    /// Iterate over all balances
    pub fn iter(&self) -> impl Iterator<Item = (&String, Decimal)> {
        self.0.iter().filter_map(|(k, v)| {
            let balance: Decimal = v.parse().ok()?;
            Some((k, balance))
        })
    }
}

/// Extended balance with hold amounts
#[derive(Debug, Clone, Deserialize)]
pub struct ExtendedBalance {
    /// Available balance
    pub balance: String,
    /// Amount on hold
    pub hold_trade: Option<String>,
}

/// Trade history entry
#[derive(Debug, Clone, Deserialize)]
pub struct TradeHistoryEntry {
    /// Order transaction ID
    pub ordertxid: String,
    /// Pair
    pub pair: String,
    /// Time of trade
    pub time: f64,
    /// Type (buy/sell)
    #[serde(rename = "type")]
    pub side: String,
    /// Order type (market/limit)
    pub ordertype: String,
    /// Price
    pub price: String,
    /// Cost
    pub cost: String,
    /// Fee
    pub fee: String,
    /// Volume
    pub vol: String,
    /// Margin
    pub margin: Option<String>,
    /// Miscellaneous
    pub misc: String,
}

/// Open order information
#[derive(Debug, Clone, Deserialize)]
pub struct OpenOrder {
    /// Order status
    pub status: String,
    /// Open timestamp
    pub opentm: f64,
    /// Start timestamp
    pub starttm: Option<f64>,
    /// Expire timestamp
    pub expiretm: Option<f64>,
    /// Order description
    pub descr: OrderDescription,
    /// Volume
    pub vol: String,
    /// Executed volume
    pub vol_exec: String,
    /// Cost
    pub cost: String,
    /// Fee
    pub fee: String,
    /// Average price
    pub price: String,
    /// Stop price (if applicable)
    pub stopprice: Option<String>,
    /// Limit price (if applicable)
    pub limitprice: Option<String>,
    /// Miscellaneous
    pub misc: String,
    /// Order flags
    pub oflags: String,
}

/// Order description
#[derive(Debug, Clone, Deserialize)]
pub struct OrderDescription {
    /// Asset pair
    pub pair: String,
    /// Type (buy/sell)
    #[serde(rename = "type")]
    pub side: String,
    /// Order type
    pub ordertype: String,
    /// Primary price
    pub price: String,
    /// Secondary price
    pub price2: String,
    /// Leverage
    pub leverage: String,
    /// Order description
    pub order: String,
    /// Close order description
    pub close: String,
}

// ============================================================================
// Trading Types
// ============================================================================

/// Order side (buy or sell)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    /// Buy order
    Buy,
    /// Sell order
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buy => write!(f, "buy"),
            Self::Sell => write!(f, "sell"),
        }
    }
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrderType {
    /// Market order
    Market,
    /// Limit order
    Limit,
    /// Stop loss
    StopLoss,
    /// Take profit
    TakeProfit,
    /// Stop loss limit
    StopLossLimit,
    /// Take profit limit
    TakeProfitLimit,
    /// Trailing stop
    TrailingStop,
    /// Trailing stop limit
    TrailingStopLimit,
    /// Settle position
    SettlePosition,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Market => "market",
            Self::Limit => "limit",
            Self::StopLoss => "stop-loss",
            Self::TakeProfit => "take-profit",
            Self::StopLossLimit => "stop-loss-limit",
            Self::TakeProfitLimit => "take-profit-limit",
            Self::TrailingStop => "trailing-stop",
            Self::TrailingStopLimit => "trailing-stop-limit",
            Self::SettlePosition => "settle-position",
        };
        write!(f, "{}", s)
    }
}

/// Time in force for orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good till cancelled
    #[serde(rename = "GTC")]
    GoodTillCancelled,
    /// Immediate or cancel
    #[serde(rename = "IOC")]
    ImmediateOrCancel,
    /// Good till date
    #[serde(rename = "GTD")]
    GoodTillDate,
}

impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GoodTillCancelled => write!(f, "GTC"),
            Self::ImmediateOrCancel => write!(f, "IOC"),
            Self::GoodTillDate => write!(f, "GTD"),
        }
    }
}

/// Order flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderFlag {
    /// Post-only order (maker only)
    PostOnly,
    /// Fee in base currency
    FeeInBase,
    /// Fee in quote currency
    FeeInQuote,
    /// Disable market price protection
    NoMarketPriceProtection,
    /// Order volume in quote currency
    VolumeInQuote,
}

impl OrderFlag {
    /// Get the API string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PostOnly => "post",
            Self::FeeInBase => "fcib",
            Self::FeeInQuote => "fciq",
            Self::NoMarketPriceProtection => "nompp",
            Self::VolumeInQuote => "viqc",
        }
    }
}

/// Request to place an order
#[derive(Debug, Clone)]
pub struct OrderRequest {
    /// Trading pair
    pub pair: String,
    /// Order side
    pub side: OrderSide,
    /// Order type
    pub order_type: OrderType,
    /// Order volume
    pub volume: Decimal,
    /// Price (for limit orders)
    pub price: Option<Decimal>,
    /// Secondary price (for stop-loss-limit, take-profit-limit)
    pub price2: Option<Decimal>,
    /// Time in force
    pub time_in_force: Option<TimeInForce>,
    /// Leverage (for margin)
    pub leverage: Option<String>,
    /// Order flags
    pub flags: Vec<OrderFlag>,
    /// Start time
    pub starttm: Option<String>,
    /// Expire time
    pub expiretm: Option<String>,
    /// User reference ID
    pub userref: Option<i32>,
    /// Validate only (don't submit)
    pub validate: bool,
    /// Close order type
    pub close_order_type: Option<OrderType>,
    /// Close order price
    pub close_price: Option<Decimal>,
    /// Close order price2
    pub close_price2: Option<Decimal>,
}

impl OrderRequest {
    /// Create a market order
    pub fn market(pair: impl Into<String>, side: OrderSide, volume: Decimal) -> Self {
        Self {
            pair: pair.into(),
            side,
            order_type: OrderType::Market,
            volume,
            price: None,
            price2: None,
            time_in_force: None,
            leverage: None,
            flags: Vec::new(),
            starttm: None,
            expiretm: None,
            userref: None,
            validate: false,
            close_order_type: None,
            close_price: None,
            close_price2: None,
        }
    }

    /// Create a limit order
    pub fn limit(pair: impl Into<String>, side: OrderSide, volume: Decimal, price: Decimal) -> Self {
        Self {
            pair: pair.into(),
            side,
            order_type: OrderType::Limit,
            volume,
            price: Some(price),
            price2: None,
            time_in_force: None,
            leverage: None,
            flags: Vec::new(),
            starttm: None,
            expiretm: None,
            userref: None,
            validate: false,
            close_order_type: None,
            close_price: None,
            close_price2: None,
        }
    }

    /// Create a stop loss order
    pub fn stop_loss(pair: impl Into<String>, side: OrderSide, volume: Decimal, stop_price: Decimal) -> Self {
        Self {
            pair: pair.into(),
            side,
            order_type: OrderType::StopLoss,
            volume,
            price: Some(stop_price),
            price2: None,
            time_in_force: None,
            leverage: None,
            flags: Vec::new(),
            starttm: None,
            expiretm: None,
            userref: None,
            validate: false,
            close_order_type: None,
            close_price: None,
            close_price2: None,
        }
    }

    /// Set time in force
    pub fn with_time_in_force(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = Some(tif);
        self
    }

    /// Add an order flag
    pub fn with_flag(mut self, flag: OrderFlag) -> Self {
        self.flags.push(flag);
        self
    }

    /// Set as post-only (maker only)
    pub fn post_only(self) -> Self {
        self.with_flag(OrderFlag::PostOnly)
    }

    /// Set leverage for margin trading
    pub fn with_leverage(mut self, leverage: impl Into<String>) -> Self {
        self.leverage = Some(leverage.into());
        self
    }

    /// Set user reference ID
    pub fn with_userref(mut self, userref: i32) -> Self {
        self.userref = Some(userref);
        self
    }

    /// Set as validate-only (don't actually submit)
    pub fn validate_only(mut self) -> Self {
        self.validate = true;
        self
    }

    /// Add a close order
    pub fn with_close(mut self, order_type: OrderType, price: Decimal) -> Self {
        self.close_order_type = Some(order_type);
        self.close_price = Some(price);
        self
    }
}

/// Response from placing an order
#[derive(Debug, Clone, Deserialize)]
pub struct OrderResponse {
    /// Order description
    pub descr: OrderResponseDescription,
    /// Transaction IDs
    pub txid: Option<Vec<String>>,
}

/// Order response description
#[derive(Debug, Clone, Deserialize)]
pub struct OrderResponseDescription {
    /// Order description
    pub order: String,
    /// Close order description (if applicable)
    pub close: Option<String>,
}

/// Cancel order result
#[derive(Debug, Clone, Deserialize)]
pub struct CancelOrderResult {
    /// Number of orders cancelled
    pub count: u32,
    /// Whether cancel is pending
    pub pending: Option<bool>,
}

/// Edit order result
#[derive(Debug, Clone, Deserialize)]
pub struct EditOrderResult {
    /// New order status
    pub status: String,
    /// Original order ID
    pub originaltxid: String,
    /// New order ID
    pub txid: Option<String>,
    /// Order description
    pub descr: Option<OrderResponseDescription>,
    /// Volume
    pub volume: Option<String>,
    /// Price
    pub price: Option<String>,
    /// Price2
    pub price2: Option<String>,
    /// Error message (if any)
    pub error_message: Option<String>,
}

// ============================================================================
// Funding Types
// ============================================================================

/// Deposit method
#[derive(Debug, Clone, Deserialize)]
pub struct DepositMethod {
    /// Method name
    pub method: String,
    /// Limit
    pub limit: Option<String>,
    /// Fee
    pub fee: Option<String>,
    /// Minimum deposit
    pub minimum: Option<String>,
    /// Address setup fee
    pub address_setup_fee: Option<String>,
    /// Generation address
    pub gen_address: Option<bool>,
}

/// Deposit address
#[derive(Debug, Clone, Deserialize)]
pub struct DepositAddress {
    /// Address
    pub address: String,
    /// Expiry time
    pub expiretm: Option<String>,
    /// New address generated
    pub new: Option<bool>,
}

/// Withdrawal info
#[derive(Debug, Clone, Deserialize)]
pub struct WithdrawInfo {
    /// Withdrawal method
    pub method: String,
    /// Network
    pub network: Option<String>,
    /// Limit
    pub limit: String,
    /// Fee
    pub fee: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_request_builder() {
        let order = OrderRequest::limit("XBTUSD", OrderSide::Buy, Decimal::ONE, Decimal::from(50000))
            .post_only()
            .with_userref(123)
            .with_time_in_force(TimeInForce::GoodTillCancelled);

        assert_eq!(order.pair, "XBTUSD");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.volume, Decimal::ONE);
        assert_eq!(order.price, Some(Decimal::from(50000)));
        assert!(order.flags.contains(&OrderFlag::PostOnly));
        assert_eq!(order.userref, Some(123));
    }

    #[test]
    fn test_ticker_info_parsing() {
        let ticker = TickerInfo {
            a: vec!["50000.00".to_string(), "1".to_string(), "1.000".to_string()],
            b: vec!["49999.00".to_string(), "1".to_string(), "1.000".to_string()],
            c: vec!["50000.00".to_string(), "0.1".to_string()],
            v: vec!["100.0".to_string(), "1000.0".to_string()],
            p: vec!["50000.0".to_string(), "49500.0".to_string()],
            t: vec![100, 1000],
            l: vec!["49000.0".to_string(), "48000.0".to_string()],
            h: vec!["51000.0".to_string(), "52000.0".to_string()],
            o: "50000.0".to_string(),
        };

        assert_eq!(ticker.ask_price(), Some(Decimal::from(50000)));
        assert_eq!(ticker.bid_price(), Some(Decimal::from(49999)));
        assert!(ticker.spread_bps().is_some());
    }
}
