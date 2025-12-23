//! Shared types for Kraken WebSocket API v2
//!
//! This crate provides the core type definitions used across the Havklo SDK.
//! It has minimal dependencies and can be used independently.
//!
//! # Key Types
//!
//! - [`Symbol`] - Trading pair symbols (e.g., "BTC/USD")
//! - [`Level`] - Orderbook price level with decimal precision
//! - [`Channel`], [`Depth`], [`Side`] - Subscription enums
//! - [`WsMessage`] - Parsed WebSocket message
//! - [`KrakenError`] - Error types
//! - [`KrakenApiError`], [`KrakenErrorCode`] - Comprehensive Kraken API error mapping
//! - [`TokenBucket`], [`RateLimitConfig`] - Client-side rate limiting

pub mod enums;
pub mod error;
pub mod error_codes;
pub mod level;
pub mod messages;
pub mod rate_limit;
pub mod symbol;

// Re-export commonly used types
pub use enums::*;
pub use error::*;
pub use error_codes::*;
pub use level::*;
pub use messages::*;
pub use rate_limit::*;
pub use symbol::*;

// Re-export rust_decimal for users
pub use rust_decimal::Decimal;
