//! Native WebSocket client for Kraken WebSocket API v2
//!
//! This crate provides a production-ready WebSocket client for connecting
//! to Kraken's public and private market data feeds.
//!
//! # Features
//!
//! - Automatic reconnection with exponential backoff
//! - Subscription management with restoration after reconnect
//! - Orderbook state maintenance with checksum validation
//! - Event-driven architecture with async streams
//!
//! # Example
//!
//! ```no_run
//! use kraken_ws::{KrakenConnection, ConnectionConfig, Endpoint};
//! use kraken_types::Depth;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ConnectionConfig::new()
//!         .with_endpoint(Endpoint::Public)
//!         .with_depth(Depth::D10);
//!
//!     let conn = KrakenConnection::new(config);
//!     conn.subscribe_orderbook(vec!["BTC/USD".to_string()]);
//!
//!     let mut events = conn.take_event_receiver().unwrap();
//!
//!     // Spawn connection task
//!     tokio::spawn(async move {
//!         conn.connect_and_run().await
//!     });
//!
//!     // Process events
//!     while let Some(event) = events.recv().await {
//!         println!("{:?}", event);
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod circuit_breaker;
pub mod connection;
pub mod endpoint;
pub mod events;
pub mod hooks;
pub mod order_tracker;
pub mod rate_limiter;
pub mod reconnect;
pub mod subscription;
pub mod trading;
pub mod transport;

// Re-export main types
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState, CircuitBreakerStats};
pub use connection::{ConnectionConfig, ConnectionState, KrakenConnection, BackpressurePolicy, EventReceiver};
pub use endpoint::Endpoint;
pub use events::{
    ConnectionEvent, DisconnectReason, Event, MarketEvent, SubscriptionEvent,
    PrivateEvent, OrderStatus, TrackedOrder, OrderFill, ExecutionType, OrderChange, BalanceInfo,
    L3Event,
};
pub use order_tracker::{OrderTracker, LifecycleOrder, LifecycleState, Fill, TrackerConfig, TrackerStats};
pub use rate_limiter::{KrakenRateLimiter, SharedRateLimiter};
pub use reconnect::ReconnectConfig;
pub use subscription::Subscription;
pub use trading::TradingClient;
pub use transport::{Transport, TransportError, WsTransport};
pub use hooks::{Hooks, ConnectInfo, DisconnectInfo, SubscriptionInfo, ChecksumInfo};

// Re-export MockTransport when test-utils feature is enabled
#[cfg(any(test, feature = "test-utils"))]
pub use transport::MockTransport;
