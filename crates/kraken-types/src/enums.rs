//! Channel, Side, Depth, and OrderType enums

use serde::{Deserialize, Serialize};

/// WebSocket channel types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Channel {
    /// Ticker channel - price and volume updates
    Ticker,
    /// Book channel - Level 2 orderbook
    Book,
    /// Trade channel - executed trades
    Trade,
    /// OHLC channel - candlestick data
    #[serde(rename = "ohlc")]
    Ohlc,
    /// Instrument channel - reference data
    Instrument,
    /// Executions channel - private order/trade events
    Executions,
    /// Balances channel - private balance updates
    Balances,
    /// Status channel - system status
    Status,
    /// Level 3 orders channel - individual orders (requires special access)
    #[serde(rename = "level3")]
    Level3,
}

impl Channel {
    /// Returns the channel name as used in API messages
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ticker => "ticker",
            Self::Book => "book",
            Self::Trade => "trade",
            Self::Ohlc => "ohlc",
            Self::Instrument => "instrument",
            Self::Executions => "executions",
            Self::Balances => "balances",
            Self::Status => "status",
            Self::Level3 => "level3",
        }
    }

    /// Returns true if this is a private (authenticated) channel
    pub fn is_private(&self) -> bool {
        matches!(self, Self::Executions | Self::Balances)
    }

    /// Returns true if this is the L3 channel (requires special endpoint)
    pub fn is_l3(&self) -> bool {
        matches!(self, Self::Level3)
    }
}

/// Trade side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Buy order
    Buy,
    /// Sell order
    Sell,
}

impl Side {
    /// Returns the opposite side
    pub fn opposite(&self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

/// Orderbook depth levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Depth {
    /// 10 price levels per side
    #[serde(rename = "10")]
    #[default]
    D10 = 10,
    /// 25 price levels per side
    #[serde(rename = "25")]
    D25 = 25,
    /// 100 price levels per side
    #[serde(rename = "100")]
    D100 = 100,
    /// 500 price levels per side
    #[serde(rename = "500")]
    D500 = 500,
    /// 1000 price levels per side
    #[serde(rename = "1000")]
    D1000 = 1000,
}

impl Depth {
    /// Returns the depth as a u32
    pub fn as_u32(&self) -> u32 {
        *self as u32
    }
}


/// Order types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    /// Market order - executes immediately at best available price
    #[serde(rename = "market")]
    Market,
    /// Limit order - executes at specified price or better
    #[serde(rename = "limit")]
    Limit,
    /// Stop-loss order
    #[serde(rename = "stop-loss")]
    StopLoss,
    /// Take-profit order
    #[serde(rename = "take-profit")]
    TakeProfit,
    /// Stop-loss limit order
    #[serde(rename = "stop-loss-limit")]
    StopLossLimit,
    /// Take-profit limit order
    #[serde(rename = "take-profit-limit")]
    TakeProfitLimit,
}

/// OHLC interval in minutes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OhlcInterval {
    /// 1 minute
    #[serde(rename = "1")]
    M1 = 1,
    /// 5 minutes
    #[serde(rename = "5")]
    M5 = 5,
    /// 15 minutes
    #[serde(rename = "15")]
    M15 = 15,
    /// 30 minutes
    #[serde(rename = "30")]
    M30 = 30,
    /// 1 hour (60 minutes)
    #[serde(rename = "60")]
    H1 = 60,
    /// 4 hours (240 minutes)
    #[serde(rename = "240")]
    H4 = 240,
    /// 1 day (1440 minutes)
    #[serde(rename = "1440")]
    D1 = 1440,
    /// 1 week (10080 minutes)
    #[serde(rename = "10080")]
    W1 = 10080,
    /// 15 days (21600 minutes)
    #[serde(rename = "21600")]
    D15 = 21600,
}

/// System status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemStatus {
    /// Normal operation
    Online,
    /// Cancel-only mode
    CancelOnly,
    /// Post-only mode
    PostOnly,
    /// Limit-only mode
    LimitOnly,
    /// Reduce-only mode
    ReduceOnly,
    /// Maintenance mode
    Maintenance,
}

impl std::fmt::Display for SystemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "online"),
            Self::CancelOnly => write!(f, "cancel_only"),
            Self::PostOnly => write!(f, "post_only"),
            Self::LimitOnly => write!(f, "limit_only"),
            Self::ReduceOnly => write!(f, "reduce_only"),
            Self::Maintenance => write!(f, "maintenance"),
        }
    }
}

/// Ticker event trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TickerTrigger {
    /// Trigger on trades
    #[default]
    Trades,
    /// Trigger on best bid/offer changes
    Bbo,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_serde() {
        assert_eq!(serde_json::to_string(&Channel::Book).unwrap(), "\"book\"");
        assert_eq!(serde_json::to_string(&Channel::Ohlc).unwrap(), "\"ohlc\"");

        let parsed: Channel = serde_json::from_str("\"ticker\"").unwrap();
        assert_eq!(parsed, Channel::Ticker);
    }

    #[test]
    fn test_depth_serde() {
        // Depth serializes as number
        let depth = Depth::D10;
        assert_eq!(depth.as_u32(), 10);
    }

    #[test]
    fn test_side_opposite() {
        assert_eq!(Side::Buy.opposite(), Side::Sell);
        assert_eq!(Side::Sell.opposite(), Side::Buy);
    }
}
