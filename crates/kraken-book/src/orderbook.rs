//! Orderbook state machine
//!
//! Manages the orderbook lifecycle and checksum validation.
//!
//! # State Machine
//!
//! ```text
//! Uninitialized → AwaitingSnapshot → Synced ↔ Desynchronized
//! ```

use crate::{
    checksum::{compute_checksum_with_precision, DEFAULT_PRICE_PRECISION, DEFAULT_QTY_PRECISION},
    storage::TreeBook,
};
use kraken_types::{BookData, Level};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Orderbook synchronization state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OrderbookState {
    /// No subscription, no data
    #[default]
    Uninitialized,
    /// Subscribed, waiting for snapshot
    AwaitingSnapshot,
    /// Processing deltas normally
    Synced,
    /// Checksum failed, needs recovery
    Desynchronized,
}


/// Checksum mismatch error
#[derive(Debug, Clone)]
pub struct ChecksumMismatch {
    /// Symbol that had the mismatch
    pub symbol: String,
    /// Expected checksum from server
    pub expected: u32,
    /// Computed checksum locally
    pub computed: u32,
}

impl std::fmt::Display for ChecksumMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Checksum mismatch for {}: expected {}, computed {}",
            self.symbol, self.expected, self.computed
        )
    }
}

impl std::error::Error for ChecksumMismatch {}

/// Result of applying a message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyResult {
    /// Snapshot was applied
    Snapshot,
    /// Delta update was applied
    Update,
    /// Message was not a book message
    Ignored,
}

/// Managed orderbook with state tracking and checksum validation
pub struct Orderbook {
    /// Symbol for this orderbook
    symbol: String,
    /// Price level storage
    storage: TreeBook,
    /// Last validated checksum
    last_checksum: u32,
    /// Current synchronization state
    state: OrderbookState,
    /// Subscribed depth
    depth: u32,
    /// Price precision (decimal places) for checksum calculation
    price_precision: u8,
    /// Quantity precision (decimal places) for checksum calculation
    qty_precision: u8,
}

impl Orderbook {
    /// Create a new orderbook for a symbol
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            storage: TreeBook::new(),
            last_checksum: 0,
            state: OrderbookState::Uninitialized,
            depth: 10, // Default depth
            price_precision: DEFAULT_PRICE_PRECISION,
            qty_precision: DEFAULT_QTY_PRECISION,
        }
    }

    /// Create with specific depth
    pub fn with_depth(symbol: impl Into<String>, depth: u32) -> Self {
        Self {
            symbol: symbol.into(),
            storage: TreeBook::new(),
            last_checksum: 0,
            state: OrderbookState::Uninitialized,
            depth,
            price_precision: DEFAULT_PRICE_PRECISION,
            qty_precision: DEFAULT_QTY_PRECISION,
        }
    }

    /// Set the precision values (from instrument channel)
    ///
    /// This should be called before applying any book data to ensure
    /// correct checksum validation.
    pub fn set_precision(&mut self, price_precision: u8, qty_precision: u8) {
        self.price_precision = price_precision;
        self.qty_precision = qty_precision;
    }

    /// Get the current price precision
    pub fn price_precision(&self) -> u8 {
        self.price_precision
    }

    /// Get the current quantity precision
    pub fn qty_precision(&self) -> u8 {
        self.qty_precision
    }

    /// Get the symbol
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Get the current state
    pub fn state(&self) -> OrderbookState {
        self.state
    }

    /// Check if the orderbook is synchronized
    pub fn is_synced(&self) -> bool {
        self.state == OrderbookState::Synced
    }

    /// Get the last validated checksum
    pub fn last_checksum(&self) -> u32 {
        self.last_checksum
    }

    /// Get the subscribed depth
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Get the best bid
    pub fn best_bid(&self) -> Option<&Level> {
        self.storage.best_bid()
    }

    /// Get the best ask
    pub fn best_ask(&self) -> Option<&Level> {
        self.storage.best_ask()
    }

    /// Get the spread (ask - bid)
    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask.price - bid.price),
            _ => None,
        }
    }

    /// Get the mid price ((ask + bid) / 2)
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some((ask.price + bid.price) / Decimal::TWO),
            _ => None,
        }
    }

    /// Get bids as a vector (for serialization/WASM)
    pub fn bids_vec(&self) -> Vec<Level> {
        self.storage.bids_vec()
    }

    /// Get asks as a vector (for serialization/WASM)
    pub fn asks_vec(&self) -> Vec<Level> {
        self.storage.asks_vec()
    }

    /// Get top N bids
    pub fn top_bids(&self, n: usize) -> Vec<Level> {
        self.storage.top_bids(n)
    }

    /// Get top N asks
    pub fn top_asks(&self, n: usize) -> Vec<Level> {
        self.storage.top_asks(n)
    }

    /// Number of bid levels
    pub fn bid_count(&self) -> usize {
        self.storage.bid_count()
    }

    /// Number of ask levels
    pub fn ask_count(&self) -> usize {
        self.storage.ask_count()
    }

    /// Mark as awaiting snapshot (call when subscribing)
    pub fn set_awaiting_snapshot(&mut self) {
        self.state = OrderbookState::AwaitingSnapshot;
    }

    /// Apply book data from a channel message
    pub fn apply_book_data(
        &mut self,
        data: &BookData,
        is_snapshot: bool,
    ) -> Result<ApplyResult, ChecksumMismatch> {
        if is_snapshot {
            self.apply_snapshot_data(data)
        } else {
            self.apply_delta_data(data)
        }
    }

    /// Apply a snapshot (full orderbook state)
    fn apply_snapshot_data(&mut self, data: &BookData) -> Result<ApplyResult, ChecksumMismatch> {
        // Clear existing state
        self.storage.clear();

        // Load all levels
        for level in &data.bids {
            self.storage.insert_bid(level.price, level.qty);
        }
        for level in &data.asks {
            self.storage.insert_ask(level.price, level.qty);
        }

        // Truncate to subscribed depth
        self.storage.truncate(self.depth as usize);

        // Validate checksum
        self.validate_checksum(data.checksum)?;

        self.state = OrderbookState::Synced;
        Ok(ApplyResult::Snapshot)
    }

    /// Apply a delta update
    fn apply_delta_data(&mut self, data: &BookData) -> Result<ApplyResult, ChecksumMismatch> {
        // Skip if not synced
        if self.state != OrderbookState::Synced {
            return Ok(ApplyResult::Ignored);
        }

        // Apply bid updates (qty == 0 means remove)
        for level in &data.bids {
            if level.qty.is_zero() {
                self.storage.remove_bid(&level.price);
            } else {
                self.storage.insert_bid(level.price, level.qty);
            }
        }

        // Apply ask updates
        for level in &data.asks {
            if level.qty.is_zero() {
                self.storage.remove_ask(&level.price);
            } else {
                self.storage.insert_ask(level.price, level.qty);
            }
        }

        // Truncate to subscribed depth
        self.storage.truncate(self.depth as usize);

        // Validate checksum
        self.validate_checksum(data.checksum)?;

        Ok(ApplyResult::Update)
    }

    /// Validate the current state against expected checksum
    fn validate_checksum(&mut self, expected: u32) -> Result<(), ChecksumMismatch> {
        let bids = self.storage.bids_vec();
        let asks = self.storage.asks_vec();
        let computed = compute_checksum_with_precision(
            &bids,
            &asks,
            self.price_precision,
            self.qty_precision,
        );

        if computed != expected {
            self.state = OrderbookState::Desynchronized;
            return Err(ChecksumMismatch {
                symbol: self.symbol.clone(),
                expected,
                computed,
            });
        }

        self.last_checksum = expected;
        Ok(())
    }

    /// Clear and reset the orderbook
    pub fn reset(&mut self) {
        self.storage.clear();
        self.last_checksum = 0;
        self.state = OrderbookState::Uninitialized;
    }

    /// Capture current state as a snapshot
    pub fn snapshot(&self) -> OrderbookSnapshot {
        OrderbookSnapshot {
            symbol: self.symbol.clone(),
            bids: self.bids_vec(),
            asks: self.asks_vec(),
            checksum: self.last_checksum,
            state: self.state,
        }
    }
}

/// Immutable snapshot of orderbook state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    /// Trading pair symbol
    pub symbol: String,
    /// Bid levels
    pub bids: Vec<Level>,
    /// Ask levels
    pub asks: Vec<Level>,
    /// Checksum at time of snapshot
    pub checksum: u32,
    /// State at time of snapshot
    #[serde(skip)]
    pub state: OrderbookState,
}

impl Default for OrderbookSnapshot {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            bids: Vec::new(),
            asks: Vec::new(),
            checksum: 0,
            state: OrderbookState::Uninitialized,
        }
    }
}

impl OrderbookSnapshot {
    /// Get the best bid price
    pub fn best_bid_price(&self) -> Option<Decimal> {
        self.bids.first().map(|l| l.price)
    }

    /// Get the best ask price
    pub fn best_ask_price(&self) -> Option<Decimal> {
        self.asks.first().map(|l| l.price)
    }

    /// Get the spread
    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_ask_price(), self.best_bid_price()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get the mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_ask_price(), self.best_bid_price()) {
            (Some(ask), Some(bid)) => Some((ask + bid) / Decimal::TWO),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::compute_checksum;
    use rust_decimal_macros::dec;

    fn make_book_data(bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> BookData {
        let bids: Vec<Level> = bids
            .into_iter()
            .map(|(p, q)| Level::from_f64(p, q))
            .collect();
        let asks: Vec<Level> = asks
            .into_iter()
            .map(|(p, q)| Level::from_f64(p, q))
            .collect();

        // Compute the checksum with default precision
        let checksum = compute_checksum(&bids, &asks);

        BookData {
            symbol: "BTC/USD".to_string(),
            bids,
            asks,
            checksum,
            timestamp: None,
        }
    }

    #[test]
    fn test_orderbook_snapshot() {
        let mut book = Orderbook::new("BTC/USD");
        assert_eq!(book.state(), OrderbookState::Uninitialized);

        let data = make_book_data(vec![(100.0, 1.0), (99.0, 2.0)], vec![(101.0, 1.0), (102.0, 2.0)]);

        book.apply_book_data(&data, true).unwrap();
        assert_eq!(book.state(), OrderbookState::Synced);
        assert!(book.is_synced());
    }

    #[test]
    fn test_orderbook_delta() {
        let mut book = Orderbook::new("BTC/USD");

        // Apply snapshot first
        let snapshot = make_book_data(vec![(100.0, 1.0)], vec![(101.0, 1.0)]);
        book.apply_book_data(&snapshot, true).unwrap();

        // Apply delta
        let delta = make_book_data(vec![(100.0, 2.0)], vec![(101.0, 2.0)]);
        book.apply_book_data(&delta, false).unwrap();

        assert_eq!(book.best_bid().unwrap().qty, dec!(2));
        assert_eq!(book.best_ask().unwrap().qty, dec!(2));
    }

    #[test]
    fn test_spread_and_mid() {
        let mut book = Orderbook::new("BTC/USD");
        let data = make_book_data(vec![(100.0, 1.0)], vec![(102.0, 1.0)]);
        book.apply_book_data(&data, true).unwrap();

        assert_eq!(book.spread(), Some(dec!(2)));
        assert_eq!(book.mid_price(), Some(dec!(101)));
    }

    #[test]
    fn test_checksum_mismatch() {
        let mut book = Orderbook::new("BTC/USD");

        // Create data with wrong checksum
        let mut data = make_book_data(vec![(100.0, 1.0)], vec![(101.0, 1.0)]);
        data.checksum = 12345; // Wrong checksum

        let result = book.apply_book_data(&data, true);
        assert!(result.is_err());
        assert_eq!(book.state(), OrderbookState::Desynchronized);
    }

    #[test]
    fn test_reset() {
        let mut book = Orderbook::new("BTC/USD");
        let data = make_book_data(vec![(100.0, 1.0)], vec![(101.0, 1.0)]);
        book.apply_book_data(&data, true).unwrap();

        book.reset();
        assert_eq!(book.state(), OrderbookState::Uninitialized);
        assert_eq!(book.bid_count(), 0);
        assert_eq!(book.ask_count(), 0);
    }
}
