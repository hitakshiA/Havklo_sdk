//! High-level SDK for Kraken WebSocket API v2
//!
//! This crate provides an ergonomic, high-level API for connecting to Kraken's
//! real-time market data feeds. It handles connection management, automatic
//! reconnection, and orderbook state maintenance.
//!
//! # Quick Start
//!
//! ```no_run
//! use kraken_sdk::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to Kraken
//!     let mut client = KrakenClient::builder(["BTC/USD", "ETH/USD"])
//!         .with_depth(Depth::D10)
//!         .connect()
//!         .await?;
//!
//!     // Get market data
//!     if let Some(spread) = client.spread("BTC/USD") {
//!         println!("BTC/USD spread: {}", spread);
//!     }
//!
//!     // Process events
//!     let mut events = client.events().unwrap();
//!     while let Some(event) = events.recv().await {
//!         match event {
//!             Event::Market(MarketEvent::OrderbookUpdate { symbol, snapshot }) => {
//!                 println!("{}: mid = {:?}", symbol, snapshot.mid_price());
//!             }
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - **Simple API**: Builder pattern for configuration
//! - **Automatic Reconnection**: Exponential backoff with jitter
//! - **Orderbook Management**: State tracking with checksum validation
//! - **Event-Driven**: Async event stream for all updates
//! - **Type-Safe**: Full type safety with Rust's type system

pub mod builder;
pub mod client;
pub mod prelude;

// Re-export main types
pub use builder::KrakenClientBuilder;
pub use client::KrakenClient;

// Re-export commonly used types from dependencies
pub use kraken_book::{Orderbook, OrderbookSnapshot, OrderbookState};
pub use kraken_types::{Depth, KrakenError, Level, Symbol};
pub use kraken_ws::{ConnectionState, Endpoint, Event, ReconnectConfig};
