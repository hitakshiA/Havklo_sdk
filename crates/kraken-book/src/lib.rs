//! WASM-compatible orderbook engine for Kraken WebSocket API v2
//!
//! This crate provides the core orderbook data structures and checksum validation.
//! It is designed to compile to both native and WASM targets.
//!
//! # Critical WASM Constraints
//!
//! - NO `std::time::Instant` (panics in WASM)
//! - NO `tokio` (doesn't compile to WASM)
//! - NO networking code
//!
//! # Example
//!
//! ```
//! use kraken_book::{Orderbook, OrderbookState};
//! use kraken_types::BookData;
//!
//! let mut book = Orderbook::new("BTC/USD");
//! assert_eq!(book.state(), OrderbookState::Uninitialized);
//! ```

pub mod checksum;
pub mod history;
pub mod orderbook;
pub mod storage;

// Re-export main types
pub use checksum::{
    compute_checksum, compute_checksum_with_precision, ChecksumResult,
    DEFAULT_PRICE_PRECISION, DEFAULT_QTY_PRECISION,
};
pub use history::{HistoryBuffer, TimestampedSnapshot};
pub use orderbook::{ApplyResult, ChecksumMismatch, Orderbook, OrderbookSnapshot, OrderbookState};
pub use storage::TreeBook;
