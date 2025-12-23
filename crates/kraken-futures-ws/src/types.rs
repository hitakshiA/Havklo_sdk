//! Types for Kraken Futures WebSocket messages

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ============================================================================
// Symbol Types
// ============================================================================

/// Futures contract type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractType {
    /// Perpetual swap (no expiry)
    #[serde(rename = "perpetual")]
    Perpetual,
    /// Fixed maturity future
    #[serde(rename = "fixed_maturity")]
    FixedMaturity,
}

/// Futures symbol with parsed components
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuturesSymbol {
    /// Full symbol string (e.g., "PI_XBTUSD")
    pub raw: String,
    /// Base asset (e.g., "XBT")
    pub base: String,
    /// Quote asset (e.g., "USD")
    pub quote: String,
    /// Contract type
    pub contract_type: ContractType,
}

impl FuturesSymbol {
    /// Parse a futures symbol
    pub fn parse(symbol: &str) -> Option<Self> {
        let symbol_upper = symbol.to_uppercase();

        // Perpetual: PI_XBTUSD
        if symbol_upper.starts_with("PI_") {
            let pair = &symbol_upper[3..];
            // Common pairs: XBTUSD, ETHUSD, etc.
            let (base, quote) = if pair.ends_with("USD") {
                (pair[..pair.len() - 3].to_string(), "USD".to_string())
            } else if pair.ends_with("EUR") {
                (pair[..pair.len() - 3].to_string(), "EUR".to_string())
            } else if pair.ends_with("GBP") {
                (pair[..pair.len() - 3].to_string(), "GBP".to_string())
            } else {
                return None;
            };

            return Some(Self {
                raw: symbol_upper,
                base,
                quote,
                contract_type: ContractType::Perpetual,
            });
        }

        // Fixed maturity: FI_XBTUSD_YYMMDD
        if symbol_upper.starts_with("FI_") {
            // FI_XBTUSD_240628
            let rest = &symbol_upper[3..];
            let parts: Vec<&str> = rest.split('_').collect();
            if !parts.is_empty() {
                let pair = parts[0];
                let (base, quote) = if pair.ends_with("USD") {
                    (pair[..pair.len() - 3].to_string(), "USD".to_string())
                } else {
                    return None;
                };

                return Some(Self {
                    raw: symbol_upper,
                    base,
                    quote,
                    contract_type: ContractType::FixedMaturity,
                });
            }
        }

        None
    }

    /// Check if this is a perpetual contract
    pub fn is_perpetual(&self) -> bool {
        self.contract_type == ContractType::Perpetual
    }
}

// ============================================================================
// Ticker Types
// ============================================================================

/// Complete futures ticker with all price data
#[derive(Debug, Clone, Deserialize)]
pub struct FuturesTicker {
    /// Product ID (symbol)
    pub product_id: String,
    /// Last trade price
    pub last: Option<Decimal>,
    /// Last trade quantity
    pub last_qty: Option<Decimal>,
    /// Last trade time
    pub last_time: Option<String>,
    /// Best bid price
    pub bid: Option<Decimal>,
    /// Best bid quantity
    pub bid_size: Option<Decimal>,
    /// Best ask price
    pub ask: Option<Decimal>,
    /// Best ask quantity
    pub ask_size: Option<Decimal>,
    /// 24h volume
    pub vol24h: Option<Decimal>,
    /// 24h volume in quote currency
    pub volume_quote: Option<Decimal>,
    /// Open interest
    pub open_interest: Option<Decimal>,
    /// Mark price (fair price)
    pub mark_price: Option<Decimal>,
    /// Index price (spot price)
    pub index_price: Option<Decimal>,
    /// Current funding rate
    pub funding_rate: Option<Decimal>,
    /// Next funding time
    pub next_funding_rate_time: Option<String>,
    /// Open price (24h)
    pub open24h: Option<Decimal>,
    /// High price (24h)
    pub high24h: Option<Decimal>,
    /// Low price (24h)
    pub low24h: Option<Decimal>,
    /// Price change (24h)
    pub change24h: Option<Decimal>,
    /// Price change percentage (24h)
    pub change_pct24h: Option<Decimal>,
    /// Premium percentage
    pub premium: Option<Decimal>,
    /// Suspended flag
    pub suspended: Option<bool>,
    /// Post-only mode
    pub post_only: Option<bool>,
}

impl FuturesTicker {
    /// Get the spread
    pub fn spread(&self) -> Option<Decimal> {
        Some(self.ask? - self.bid?)
    }

    /// Get the mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        let ask = self.ask?;
        let bid = self.bid?;
        Some((ask + bid) / Decimal::TWO)
    }

    /// Get the premium/discount to spot
    pub fn premium_pct(&self) -> Option<Decimal> {
        let mark = self.mark_price?;
        let index = self.index_price?;
        if index.is_zero() {
            return None;
        }
        Some((mark - index) / index * Decimal::from(100))
    }
}

/// Funding rate information
#[derive(Debug, Clone, Deserialize)]
pub struct FundingRate {
    /// Product ID
    pub product_id: String,
    /// Current funding rate (per 8 hours)
    pub funding_rate: Decimal,
    /// Relative funding rate
    pub relative_funding_rate: Option<Decimal>,
    /// Next funding time
    pub next_funding_rate_time: String,
}

impl FundingRate {
    /// Get annualized funding rate
    pub fn annualized(&self) -> Decimal {
        // 3 funding periods per day * 365 days
        self.funding_rate * Decimal::from(3 * 365)
    }
}

/// Mark price update
#[derive(Debug, Clone, Deserialize)]
pub struct MarkPrice {
    /// Product ID
    pub product_id: String,
    /// Mark price
    pub mark_price: Decimal,
    /// Time
    pub time: String,
}

/// Index price update
#[derive(Debug, Clone, Deserialize)]
pub struct IndexPrice {
    /// Product ID
    pub product_id: String,
    /// Index price (spot)
    pub index_price: Decimal,
    /// Time
    pub time: String,
}

// ============================================================================
// Orderbook Types
// ============================================================================

/// Orderbook snapshot for futures
#[derive(Debug, Clone, Deserialize)]
pub struct FuturesBookSnapshot {
    /// Product ID
    pub product_id: String,
    /// Sequence number
    pub seq: u64,
    /// Bid levels [price, qty]
    pub bids: Vec<BookLevel>,
    /// Ask levels [price, qty]
    pub asks: Vec<BookLevel>,
    /// Timestamp
    pub timestamp: u64,
}

/// Orderbook update for futures
#[derive(Debug, Clone, Deserialize)]
pub struct FuturesBookUpdate {
    /// Product ID
    pub product_id: String,
    /// Sequence number
    pub seq: u64,
    /// Bid updates
    pub bids: Vec<BookLevel>,
    /// Ask updates
    pub asks: Vec<BookLevel>,
    /// Timestamp
    pub timestamp: u64,
}

/// Single orderbook level
#[derive(Debug, Clone, Deserialize)]
pub struct BookLevel {
    /// Price
    pub price: Decimal,
    /// Quantity (0 = remove level)
    pub qty: Decimal,
}

// ============================================================================
// Trade Types
// ============================================================================

/// Futures trade
#[derive(Debug, Clone, Deserialize)]
pub struct FuturesTrade {
    /// Product ID
    pub product_id: String,
    /// Trade ID
    pub uid: String,
    /// Trade side (buy/sell)
    pub side: TradeSide,
    /// Trade type (fill, liquidation, etc.)
    #[serde(rename = "type")]
    pub trade_type: TradeType,
    /// Price
    pub price: Decimal,
    /// Quantity
    pub qty: Decimal,
    /// Time
    pub time: String,
    /// Sequence
    pub seq: Option<u64>,
}

/// Trade side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeSide {
    Buy,
    Sell,
}

/// Trade type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeType {
    /// Normal fill
    Fill,
    /// Liquidation
    Liquidation,
    /// Assignment
    Assignment,
    /// Termination
    Termination,
    /// Block trade
    Block,
}

// ============================================================================
// Position Types
// ============================================================================

/// Position information
#[derive(Debug, Clone, Deserialize)]
pub struct Position {
    /// Product ID
    pub product_id: String,
    /// Position side (long/short)
    pub side: PositionSide,
    /// Position size
    pub size: Decimal,
    /// Entry price
    pub entry_price: Decimal,
    /// Mark price
    pub mark_price: Decimal,
    /// Liquidation price
    pub liq_price: Option<Decimal>,
    /// Unrealized PnL
    pub unrealized_pnl: Decimal,
    /// Realized PnL
    pub realized_pnl: Decimal,
    /// Margin used
    pub margin: Decimal,
    /// Effective leverage
    pub leverage: Decimal,
}

impl Position {
    /// Calculate ROE (Return on Equity)
    pub fn roe(&self) -> Option<Decimal> {
        if self.margin.is_zero() {
            return None;
        }
        Some(self.unrealized_pnl / self.margin * Decimal::from(100))
    }

    /// Calculate position value
    pub fn value(&self) -> Decimal {
        self.size * self.mark_price
    }
}

/// Position side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Long,
    Short,
}

/// Position update event
#[derive(Debug, Clone, Deserialize)]
pub struct PositionUpdate {
    /// Updated positions
    pub positions: Vec<Position>,
    /// Account info
    pub account: Option<String>,
    /// Timestamp
    pub timestamp: String,
}

/// Margin information
#[derive(Debug, Clone, Deserialize)]
pub struct MarginInfo {
    /// Available margin
    pub available_margin: Decimal,
    /// Initial margin
    pub initial_margin: Decimal,
    /// Maintenance margin
    pub maintenance_margin: Decimal,
    /// Portfolio value
    pub portfolio_value: Decimal,
    /// Unrealized PnL
    pub unrealized_pnl: Decimal,
    /// Margin level percentage
    pub margin_level: Decimal,
}

// ============================================================================
// Event Types
// ============================================================================

/// Combined event type for futures streams
#[derive(Debug, Clone)]
pub enum FuturesEvent {
    /// Connected to server
    Connected {
        server_time: String,
    },
    /// Disconnected from server
    Disconnected {
        reason: String,
    },
    /// Ticker update
    Ticker(FuturesTicker),
    /// Funding rate update
    Funding(FundingRate),
    /// Mark price update
    MarkPrice(MarkPrice),
    /// Index price update
    IndexPrice(IndexPrice),
    /// Book snapshot
    BookSnapshot(FuturesBookSnapshot),
    /// Book update
    BookUpdate(FuturesBookUpdate),
    /// Trade
    Trade(FuturesTrade),
    /// Position update
    Position(PositionUpdate),
    /// Margin update
    Margin(MarginInfo),
    /// Heartbeat
    Heartbeat,
    /// Subscription confirmed
    Subscribed {
        feed: String,
        product_ids: Vec<String>,
    },
    /// Subscription rejected
    SubscriptionError {
        feed: String,
        error: String,
    },
}

// ============================================================================
// WebSocket Message Types
// ============================================================================

/// Incoming WebSocket message
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FuturesMessage {
    /// Challenge for authentication
    Challenge { event: String, message: String },
    /// Subscription response
    Subscribed { event: String, feed: String, product_ids: Option<Vec<String>> },
    /// Error message
    Error { event: String, message: String },
    /// Info message
    Info { event: String, version: Option<u32> },
    /// Heartbeat
    Heartbeat { feed: String, product_id: Option<String>, time: u64 },
    /// Ticker
    Ticker { feed: String, #[serde(flatten)] data: FuturesTicker },
    /// Book snapshot
    BookSnapshot { feed: String, #[serde(flatten)] data: FuturesBookSnapshot },
    /// Trade
    Trade { feed: String, #[serde(flatten)] data: FuturesTrade },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_perpetual_symbol() {
        let sym = FuturesSymbol::parse("PI_XBTUSD").unwrap();
        assert_eq!(sym.base, "XBT");
        assert_eq!(sym.quote, "USD");
        assert!(sym.is_perpetual());
    }

    #[test]
    fn test_parse_fixed_maturity_symbol() {
        let sym = FuturesSymbol::parse("FI_XBTUSD_240628").unwrap();
        assert_eq!(sym.base, "XBT");
        assert_eq!(sym.quote, "USD");
        assert!(!sym.is_perpetual());
    }

    #[test]
    fn test_funding_rate_annualized() {
        let rate = FundingRate {
            product_id: "PI_XBTUSD".to_string(),
            funding_rate: Decimal::new(1, 4), // 0.0001 = 0.01%
            relative_funding_rate: None,
            next_funding_rate_time: "2024-01-01T00:00:00Z".to_string(),
        };

        // 0.0001 * 3 * 365 = 0.1095 = 10.95%
        let annualized = rate.annualized();
        assert_eq!(annualized, Decimal::new(1095, 4));
    }

    #[test]
    fn test_position_roe() {
        let pos = Position {
            product_id: "PI_XBTUSD".to_string(),
            side: PositionSide::Long,
            size: Decimal::from(1),
            entry_price: Decimal::from(50000),
            mark_price: Decimal::from(51000),
            liq_price: Some(Decimal::from(40000)),
            unrealized_pnl: Decimal::from(100),
            realized_pnl: Decimal::ZERO,
            margin: Decimal::from(500),
            leverage: Decimal::from(10),
        };

        let roe = pos.roe().unwrap();
        assert_eq!(roe, Decimal::from(20)); // 100/500 * 100 = 20%
    }
}
