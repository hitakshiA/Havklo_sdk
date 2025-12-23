// Clippy allows for design decisions that are intentional
#![allow(clippy::result_large_err)]      // Large error types preserve full debugging info
#![allow(clippy::large_enum_variant)]    // Event enums are intentionally not boxed for ergonomics
#![allow(clippy::module_inception)]      // channels::channels is intentional for discoverability
#![allow(clippy::too_many_arguments)]    // Trading APIs naturally have many parameters

//! WebSocket client for Kraken Futures API
//!
//! This crate provides a WebSocket client for connecting to Kraken's
//! Futures trading platform, supporting perpetual swaps and other derivatives.
//!
//! # Features
//!
//! - **Orderbook**: Real-time Level 2 orderbook for futures contracts
//! - **Ticker**: Price updates with mark price, index price, funding rate
//! - **Trades**: Trade stream for futures markets
//! - **Positions**: Real-time position tracking and margin updates
//! - **Funding**: Funding rate updates and payments
//!
//! # Differences from Spot API
//!
//! | Aspect | Spot WS v2 | Futures WS |
//! |--------|------------|------------|
//! | Base URL | `wss://ws.kraken.com/v2` | `wss://futures.kraken.com/ws/v1` |
//! | Auth | WS token from REST | API key challenge-response |
//! | Symbol Format | `BTC/USD` | `PI_XBTUSD` (perpetual) |
//! | Additional Data | - | `funding_rate`, `mark_price`, `index_price` |
//!
//! # Example
//!
//! ```no_run
//! use kraken_futures_ws::{FuturesConnection, FuturesConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = FuturesConfig::new()
//!         .with_symbol("PI_XBTUSD");
//!
//!     let mut conn = FuturesConnection::new(config);
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

pub mod auth;
pub mod connection;
pub mod channels;
pub mod types;
pub mod error;

// Re-export main types
pub use connection::{FuturesConnection, FuturesConfig, ConnectionState};
pub use auth::FuturesCredentials;
pub use error::{FuturesError, FuturesResult};
pub use types::{
    // Ticker
    FuturesTicker, FundingRate, MarkPrice, IndexPrice,
    // Book
    FuturesBookSnapshot, FuturesBookUpdate, BookLevel,
    // Trades
    FuturesTrade, TradeSide, TradeType,
    // Positions
    Position, PositionUpdate, MarginInfo, PositionSide,
    // Open Orders (private)
    OpenOrder, OpenOrdersSnapshot, OrderType, OrderStatus,
    // Fills (private)
    Fill, FillsSnapshot, FillType,
    // Account (private)
    AccountBalance, AccountMarginsUpdate,
    // Notifications (private)
    Notification, NotificationType,
    // Events
    FuturesEvent,
    // Symbol
    FuturesSymbol, ContractType,
};
