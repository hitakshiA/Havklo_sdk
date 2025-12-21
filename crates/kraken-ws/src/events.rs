//! Connection and subscription events

use kraken_book::OrderbookSnapshot;
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

/// Combined event type for event streams
#[derive(Debug, Clone)]
pub enum Event {
    /// Connection-related event
    Connection(ConnectionEvent),
    /// Subscription-related event
    Subscription(SubscriptionEvent),
    /// Market data event
    Market(MarketEvent),
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
