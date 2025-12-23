//! Level 3 orderbook implementation
//!
//! This module provides order-level (L3) orderbook tracking, which maintains
//! individual orders rather than just aggregated price levels like L2.
//!
//! # Key Features
//!
//! - **Order-level tracking**: Each order is tracked with its unique ID
//! - **FIFO queue semantics**: Orders at the same price are queued in FIFO order
//! - **Queue position**: Calculate your position in the queue and quantity ahead
//! - **O(1) order lookup**: HashMap index for fast order operations
//! - **L2 compatibility**: Can generate aggregated L2 levels for checksum validation
//!
//! # Use Cases
//!
//! - **Market making**: Track your order's queue position to estimate fills
//! - **Order flow analysis**: See individual orders entering/leaving the book
//! - **Fill prediction**: Estimate probability of being filled based on queue position
//!
//! # Example
//!
//! ```
//! use kraken_book::l3::{L3Book, L3Order, L3Side};
//! use rust_decimal_macros::dec;
//!
//! // Create an L3 orderbook
//! let mut book = L3Book::new("BTC/USD", 10);
//!
//! // Add orders
//! book.add_order(L3Order::new("order1", dec!(100), dec!(1)), L3Side::Bid);
//! book.add_order(L3Order::new("order2", dec!(100), dec!(2)), L3Side::Bid);
//! book.add_order(L3Order::new("order3", dec!(100), dec!(3)), L3Side::Bid);
//!
//! // Check queue position for order2
//! if let Some(pos) = book.queue_position("order2") {
//!     println!("Position: {}", pos.position);  // 1 (0-indexed)
//!     println!("Orders ahead: {}", pos.orders_ahead);  // 1
//!     println!("Qty ahead: {}", pos.qty_ahead);  // 1.0
//!     println!("Fill probability: {:.2}", pos.fill_probability());  // 0.67
//! }
//!
//! // Get aggregated L2 view
//! let bids = book.aggregated_bids();
//! assert_eq!(bids[0].qty, dec!(6));  // 1 + 2 + 3 = 6 at price 100
//! ```
//!
//! # Kraken L3 Subscription
//!
//! L3 data requires authentication and has rate limits based on depth:
//!
//! | Depth | Rate Counter | Use Case |
//! |-------|--------------|----------|
//! | 10    | 5            | Low latency scalping |
//! | 100   | 25           | Day trading |
//! | 1000  | 100          | Market making, full depth |

pub mod book;
pub mod order;

// Re-export main types
pub use book::{L3Book, L3BookSnapshot, L3ChecksumMismatch};
pub use order::{L3Order, L3PriceLevel, L3Side, OrderLocation, QueuePosition};
