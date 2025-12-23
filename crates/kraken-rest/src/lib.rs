//! REST API client for Kraken cryptocurrency exchange
//!
//! This crate provides a complete REST API client for trading on Kraken,
//! including market data, account management, and order execution.
//!
//! # Features
//!
//! - **Market Data**: Ticker, orderbook, OHLC, recent trades
//! - **Account**: Balances, trade history, open orders
//! - **Trading**: Place, cancel, and edit orders
//! - **Funding**: Deposit/withdraw operations
//!
//! # Authentication
//!
//! Private endpoints require API credentials. The client uses HMAC-SHA512
//! signing as specified by Kraken's API documentation.
//!
//! # Example
//!
//! ```no_run
//! use kraken_rest::{KrakenRestClient, Credentials};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Public endpoints (no auth required)
//!     let client = KrakenRestClient::new();
//!     let ticker = client.get_ticker("XBTUSD").await?;
//!     println!("BTC/USD: {:?}", ticker);
//!
//!     // Private endpoints (auth required)
//!     let creds = Credentials::from_env()?;
//!     let auth_client = KrakenRestClient::with_credentials(creds);
//!     let balance = auth_client.get_balance().await?;
//!     println!("Balances: {:?}", balance);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Rate Limiting
//!
//! The client respects Kraken's rate limits:
//! - Public endpoints: 1 request/second
//! - Private endpoints: Tier-based limits (starter: 15 calls/minute)
//!
//! Use `kraken-types::RateLimitConfig` for client-side rate limiting.

pub mod client;
pub mod auth;
pub mod error;
pub mod endpoints;
pub mod types;

// Re-export main types
pub use client::KrakenRestClient;
pub use auth::Credentials;
pub use error::RestError;

// Re-export endpoint-specific types
pub use types::{
    // Market data
    TickerInfo, AssetPairInfo, OhlcData, TradeData, OrderbookData,
    // Account
    BalanceInfo, TradeHistoryEntry, OpenOrder,
    // Trading
    OrderRequest, OrderResponse, OrderType, OrderSide, TimeInForce,
    // Responses
    ApiResponse,
};
