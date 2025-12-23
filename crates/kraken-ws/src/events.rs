//! Connection and subscription events
//!
//! This module provides event types for both public market data and
//! private account data (executions, balances).

use kraken_book::OrderbookSnapshot;
use kraken_types::{BalanceData, Decimal, ExecutionData, Side};
use std::collections::HashMap;
use std::time::Duration;

/// Reason for disconnection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Server closed the connection
    ServerClosed,
    /// Network error occurred
    NetworkError(String),
    /// Connection timed out
    Timeout,
    /// Client requested shutdown
    Shutdown,
    /// Authentication failed
    AuthFailed,
}

/// Connection lifecycle events
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// Successfully connected to the endpoint
    Connected {
        /// API version reported by server
        api_version: String,
        /// Connection ID from server
        connection_id: u64,
    },
    /// Connection was lost
    Disconnected {
        /// Reason for disconnection
        reason: DisconnectReason,
    },
    /// Attempting to reconnect
    Reconnecting {
        /// Current attempt number (1-indexed)
        attempt: u32,
        /// Delay before this attempt
        delay: Duration,
    },
    /// Reconnection attempts exhausted
    ReconnectFailed {
        /// Final error
        error: String,
    },
    /// Subscriptions restored after reconnect
    SubscriptionsRestored {
        /// Number of subscriptions restored
        count: usize,
    },
}

/// Subscription-specific events
#[derive(Debug, Clone)]
pub enum SubscriptionEvent {
    /// Subscription confirmed by server
    Subscribed {
        /// Channel name
        channel: String,
        /// Symbol(s)
        symbols: Vec<String>,
    },
    /// Subscription rejected
    Rejected {
        /// Channel name
        channel: String,
        /// Rejection reason
        reason: String,
    },
    /// Unsubscribed from channel
    Unsubscribed {
        /// Channel name
        channel: String,
        /// Symbol(s)
        symbols: Vec<String>,
    },
}

/// Market data events
#[derive(Debug, Clone)]
pub enum MarketEvent {
    /// Orderbook snapshot received
    OrderbookSnapshot {
        /// Trading pair symbol
        symbol: String,
        /// Full orderbook state
        snapshot: OrderbookSnapshot,
    },
    /// Orderbook updated
    OrderbookUpdate {
        /// Trading pair symbol
        symbol: String,
        /// Updated orderbook state
        snapshot: OrderbookSnapshot,
    },
    /// Checksum validation failed
    ChecksumMismatch {
        /// Symbol that failed
        symbol: String,
        /// Expected checksum
        expected: u32,
        /// Computed checksum
        computed: u32,
    },
    /// Status message from server
    Status {
        /// System status (online, maintenance, etc.)
        system: String,
        /// API version
        version: String,
    },
    /// Heartbeat received
    Heartbeat,
}

// ============================================================================
// Private Channel Events
// ============================================================================

/// Order status in the lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderStatus {
    /// Order is pending (not yet acknowledged)
    Pending,
    /// Order is new (acknowledged, waiting in book)
    New,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is completely filled
    Filled,
    /// Order was canceled
    Canceled,
    /// Order expired
    Expired,
    /// Order was rejected
    Rejected,
}

impl OrderStatus {
    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => Self::Pending,
            "new" | "open" => Self::New,
            "partially_filled" | "partiallyfilled" => Self::PartiallyFilled,
            "filled" | "closed" => Self::Filled,
            "canceled" | "cancelled" => Self::Canceled,
            "expired" => Self::Expired,
            _ => Self::Rejected,
        }
    }

    /// Check if order is still active (can be filled)
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::New | Self::PartiallyFilled)
    }

    /// Check if order is terminal (no more changes)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Filled | Self::Canceled | Self::Expired | Self::Rejected)
    }
}

/// Tracked order with full state
#[derive(Debug, Clone)]
pub struct TrackedOrder {
    /// Order ID
    pub order_id: String,
    /// Trading pair symbol
    pub symbol: String,
    /// Order side
    pub side: Side,
    /// Order type (limit, market, etc.)
    pub order_type: String,
    /// Original order quantity
    pub order_qty: Decimal,
    /// Limit price (if limit order)
    pub limit_price: Option<Decimal>,
    /// Cumulative filled quantity
    pub filled_qty: Decimal,
    /// Average fill price
    pub avg_price: Option<Decimal>,
    /// Current order status
    pub status: OrderStatus,
    /// Total fees paid
    pub total_fees: Decimal,
    /// Fee currency
    pub fee_currency: Option<String>,
    /// List of fills for this order
    pub fills: Vec<OrderFill>,
    /// Timestamp of last update
    pub last_update: String,
}

impl TrackedOrder {
    /// Create a new tracked order from execution data
    pub fn from_execution(exec: &ExecutionData) -> Self {
        Self {
            order_id: exec.order_id.clone(),
            symbol: exec.symbol.clone(),
            side: exec.side,
            order_type: exec.order_type.clone(),
            order_qty: exec.order_qty.unwrap_or(Decimal::ZERO),
            limit_price: exec.limit_price,
            filled_qty: exec.cum_qty.unwrap_or(Decimal::ZERO),
            avg_price: exec.avg_price,
            status: exec.order_status.as_ref()
                .map(|s| OrderStatus::from_str(s))
                .unwrap_or(OrderStatus::Pending),
            total_fees: exec.fee_paid.unwrap_or(Decimal::ZERO),
            fee_currency: exec.fee_currency.clone(),
            fills: Vec::new(),
            last_update: exec.timestamp.clone(),
        }
    }

    /// Update order from new execution data
    pub fn update(&mut self, exec: &ExecutionData) {
        if let Some(cum_qty) = exec.cum_qty {
            self.filled_qty = cum_qty;
        }
        if let Some(avg_price) = exec.avg_price {
            self.avg_price = Some(avg_price);
        }
        if let Some(ref status) = exec.order_status {
            self.status = OrderStatus::from_str(status);
        }
        if let Some(fee) = exec.fee_paid {
            self.total_fees = fee;
        }
        if exec.fee_currency.is_some() {
            self.fee_currency = exec.fee_currency.clone();
        }
        self.last_update = exec.timestamp.clone();
    }

    /// Add a fill to this order
    pub fn add_fill(&mut self, fill: OrderFill) {
        self.fills.push(fill);
    }

    /// Remaining quantity to be filled
    pub fn remaining_qty(&self) -> Decimal {
        self.order_qty - self.filled_qty
    }

    /// Fill percentage (0.0 to 1.0)
    pub fn fill_percentage(&self) -> f64 {
        if self.order_qty.is_zero() {
            return 0.0;
        }
        (self.filled_qty / self.order_qty)
            .to_string()
            .parse()
            .unwrap_or(0.0)
    }
}

/// Individual fill (partial execution)
#[derive(Debug, Clone)]
pub struct OrderFill {
    /// Execution ID
    pub exec_id: Option<String>,
    /// Trade ID
    pub trade_id: Option<u64>,
    /// Fill quantity
    pub qty: Decimal,
    /// Fill price
    pub price: Decimal,
    /// Fee for this fill
    pub fee: Decimal,
    /// Fee currency
    pub fee_currency: Option<String>,
    /// Timestamp
    pub timestamp: String,
}

impl OrderFill {
    /// Create from execution data (if it's a trade)
    pub fn from_execution(exec: &ExecutionData) -> Option<Self> {
        // Only create fill if there's a last_qty (indicates actual trade)
        let qty = exec.last_qty?;
        let price = exec.last_price?;

        Some(Self {
            exec_id: exec.exec_id.clone(),
            trade_id: exec.trade_id,
            qty,
            price,
            fee: exec.fee_paid.unwrap_or(Decimal::ZERO),
            fee_currency: exec.fee_currency.clone(),
            timestamp: exec.timestamp.clone(),
        })
    }
}

/// Private channel events (requires authentication)
#[derive(Debug, Clone)]
pub enum PrivateEvent {
    /// Order execution event
    Execution {
        /// Raw execution data from Kraken
        data: ExecutionData,
        /// Parsed execution type
        exec_type: ExecutionType,
    },
    /// Order state changed
    OrderUpdate {
        /// Updated order state
        order: TrackedOrder,
        /// What changed
        change: OrderChange,
    },
    /// Balance update event
    BalanceUpdate {
        /// Updated balances
        balances: Vec<BalanceData>,
        /// Whether this is a snapshot
        is_snapshot: bool,
    },
    /// Full balance snapshot
    BalanceSnapshot {
        /// All balances keyed by asset
        balances: HashMap<String, BalanceInfo>,
    },
}

/// Type of execution event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionType {
    /// New order created
    New,
    /// Order was filled (partially or fully)
    Trade,
    /// Order was canceled
    Canceled,
    /// Order expired
    Expired,
    /// Order was amended
    Amended,
    /// Order pending
    Pending,
    /// Unknown type
    Unknown,
}

impl ExecutionType {
    /// Parse from exec_type string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "new" => Self::New,
            "trade" | "filled" => Self::Trade,
            "canceled" | "cancelled" => Self::Canceled,
            "expired" => Self::Expired,
            "amended" | "modified" => Self::Amended,
            "pending" => Self::Pending,
            _ => Self::Unknown,
        }
    }
}

/// What changed in an order update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderChange {
    /// Order was created
    Created,
    /// Order was partially filled
    PartialFill,
    /// Order was completely filled
    FullFill,
    /// Order was canceled
    Canceled,
    /// Order expired
    Expired,
    /// Order quantity or price was modified
    Modified,
}

/// Enhanced balance information
#[derive(Debug, Clone)]
pub struct BalanceInfo {
    /// Asset identifier
    pub asset: String,
    /// Available balance (for trading/withdrawal)
    pub available: Decimal,
    /// Balance on hold (in open orders)
    pub hold: Decimal,
    /// Total balance (available + hold)
    pub total: Decimal,
}

impl BalanceInfo {
    /// Create from balance data
    pub fn from_data(data: &BalanceData) -> Self {
        let hold = data.hold_trade.unwrap_or(Decimal::ZERO);
        Self {
            asset: data.asset.clone(),
            available: data.balance,
            hold,
            total: data.balance + hold,
        }
    }
}

/// Combined event type for event streams
#[derive(Debug, Clone)]
pub enum Event {
    /// Connection-related event
    Connection(ConnectionEvent),
    /// Subscription-related event
    Subscription(SubscriptionEvent),
    /// Market data event
    Market(MarketEvent),
    /// Private channel event (executions, balances)
    Private(PrivateEvent),
}

impl From<ConnectionEvent> for Event {
    fn from(event: ConnectionEvent) -> Self {
        Event::Connection(event)
    }
}

impl From<SubscriptionEvent> for Event {
    fn from(event: SubscriptionEvent) -> Self {
        Event::Subscription(event)
    }
}

impl From<MarketEvent> for Event {
    fn from(event: MarketEvent) -> Self {
        Event::Market(event)
    }
}

impl From<PrivateEvent> for Event {
    fn from(event: PrivateEvent) -> Self {
        Event::Private(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_status_parsing() {
        assert_eq!(OrderStatus::from_str("pending"), OrderStatus::Pending);
        assert_eq!(OrderStatus::from_str("new"), OrderStatus::New);
        assert_eq!(OrderStatus::from_str("open"), OrderStatus::New);
        assert_eq!(OrderStatus::from_str("filled"), OrderStatus::Filled);
        assert_eq!(OrderStatus::from_str("closed"), OrderStatus::Filled);
        assert_eq!(OrderStatus::from_str("canceled"), OrderStatus::Canceled);
        assert_eq!(OrderStatus::from_str("cancelled"), OrderStatus::Canceled);
    }

    #[test]
    fn test_order_status_states() {
        assert!(OrderStatus::Pending.is_active());
        assert!(OrderStatus::New.is_active());
        assert!(OrderStatus::PartiallyFilled.is_active());
        assert!(!OrderStatus::Filled.is_active());
        assert!(!OrderStatus::Canceled.is_active());

        assert!(OrderStatus::Filled.is_terminal());
        assert!(OrderStatus::Canceled.is_terminal());
        assert!(OrderStatus::Expired.is_terminal());
        assert!(!OrderStatus::New.is_terminal());
    }

    #[test]
    fn test_execution_type_parsing() {
        assert_eq!(ExecutionType::from_str("new"), ExecutionType::New);
        assert_eq!(ExecutionType::from_str("trade"), ExecutionType::Trade);
        assert_eq!(ExecutionType::from_str("filled"), ExecutionType::Trade);
        assert_eq!(ExecutionType::from_str("canceled"), ExecutionType::Canceled);
        assert_eq!(ExecutionType::from_str("cancelled"), ExecutionType::Canceled);
    }

    #[test]
    fn test_balance_info_creation() {
        let data = BalanceData {
            asset: "BTC".to_string(),
            balance: Decimal::new(100, 2), // 1.00
            hold_trade: Some(Decimal::new(25, 2)), // 0.25
        };

        let info = BalanceInfo::from_data(&data);
        assert_eq!(info.asset, "BTC");
        assert_eq!(info.available, Decimal::new(100, 2));
        assert_eq!(info.hold, Decimal::new(25, 2));
        assert_eq!(info.total, Decimal::new(125, 2));
    }
}
