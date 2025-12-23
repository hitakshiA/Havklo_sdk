//! WASM-compatible orderbook engine for Kraken WebSocket API v2
//!
//! This crate provides the core orderbook data structures and checksum validation.
//! It is designed to compile to both native and WASM targets.
//!
//! # Features
//!
//! - **Level 2 (L2)**: Aggregated price levels with checksum validation
//! - **Level 3 (L3)**: Order-level tracking with FIFO queue semantics
//!
//! # Critical WASM Constraints
//!
//! - NO `std::time::Instant` (panics in WASM)
//! - NO `tokio` (doesn't compile to WASM)
//! - NO networking code
//!
//! # L2 Example
//!
//! ```
//! use kraken_book::{Orderbook, OrderbookState};
//! use kraken_types::BookData;
//!
//! let mut book = Orderbook::new("BTC/USD");
//! assert_eq!(book.state(), OrderbookState::Uninitialized);
//! ```
//!
//! # L3 Example
//!
//! ```
//! use kraken_book::l3::{L3Book, L3Order, L3Side};
//! use rust_decimal_macros::dec;
//!
//! let mut book = L3Book::new("BTC/USD", 10);
//! book.add_order(L3Order::new("order1", dec!(100), dec!(1)), L3Side::Bid);
//!
//! if let Some(pos) = book.queue_position("order1") {
//!     println!("Position: {}, Qty ahead: {}", pos.position, pos.qty_ahead);
//! }
//! ```

pub mod checksum;
pub mod history;
pub mod l3;
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

// Re-export L3 types at crate root for convenience
pub use l3::{L3Book, L3BookSnapshot, L3ChecksumMismatch, L3Order, L3PriceLevel, L3Side, QueuePosition};
