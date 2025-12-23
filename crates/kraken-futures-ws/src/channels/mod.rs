//! Channel handlers for different data feeds

pub mod book;
pub mod ticker;
pub mod trade;
pub mod position;

pub use book::BookChannel;
pub use ticker::TickerChannel;
pub use trade::TradeChannel;
pub use position::PositionChannel;

/// Kraken Futures channel names
pub mod channels {
    /// Ticker feed with price data
    pub const TICKER: &str = "ticker";
    /// Ticker lite (reduced data)
    pub const TICKER_LITE: &str = "ticker_lite";
    /// Orderbook feed
    pub const BOOK: &str = "book";
    /// Trade feed
    pub const TRADE: &str = "trade";
    /// Heartbeat
    pub const HEARTBEAT: &str = "heartbeat";

    // Private channels
    /// Open positions
    pub const OPEN_POSITIONS: &str = "open_positions";
    /// Open orders
    pub const OPEN_ORDERS: &str = "open_orders";
    /// Fills (executions)
    pub const FILLS: &str = "fills";
    /// Account balances
    pub const ACCOUNT_BALANCES_AND_MARGINS: &str = "account_balances_and_margins";
    /// Notifications
    pub const NOTIFICATIONS: &str = "notifications";
}

/// Subscription request builder
#[derive(Debug, Clone)]
pub struct SubscriptionRequest {
    /// Event type
    pub event: String,
    /// Feed name
    pub feed: String,
    /// Product IDs
    pub product_ids: Vec<String>,
}

impl SubscriptionRequest {
    /// Create a new subscription request
    pub fn new(feed: impl Into<String>, product_ids: Vec<String>) -> Self {
        Self {
            event: "subscribe".to_string(),
            feed: feed.into(),
            product_ids,
        }
    }

    /// Create an unsubscribe request
    pub fn unsubscribe(feed: impl Into<String>, product_ids: Vec<String>) -> Self {
        Self {
            event: "unsubscribe".to_string(),
            feed: feed.into(),
            product_ids,
        }
    }

    /// Convert to JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "event": self.event,
            "feed": self.feed,
            "product_ids": self.product_ids
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_request() {
        let req = SubscriptionRequest::new("ticker", vec!["PI_XBTUSD".to_string()]);
        let json = req.to_json();

        assert_eq!(json["event"], "subscribe");
        assert_eq!(json["feed"], "ticker");
        assert_eq!(json["product_ids"][0], "PI_XBTUSD");
    }
}
