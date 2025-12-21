//! Re-exports for convenience
//!
//! Import everything you need with:
//! ```
//! use kraken_sdk::prelude::*;
//! ```

// Client
pub use crate::client::KrakenClient;
pub use crate::builder::KrakenClientBuilder;

// Types from kraken-types
pub use kraken_types::{
    Channel, Depth, KrakenError, Level, Side, Symbol,
    BookData, SubscribeParams, SubscribeRequest,
};

// WebSocket types
pub use kraken_ws::{
    ConnectionConfig, ConnectionState, Endpoint, Event,
    ConnectionEvent, MarketEvent, SubscriptionEvent,
    ReconnectConfig,
};

// Orderbook types
pub use kraken_book::{
    Orderbook, OrderbookSnapshot, OrderbookState,
    ApplyResult, ChecksumMismatch,
};

// Decimal for prices/quantities
pub use rust_decimal::Decimal;
