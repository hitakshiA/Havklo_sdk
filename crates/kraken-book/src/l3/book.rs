//! L3 orderbook storage with order-level tracking
//!
//! This module provides the main L3 orderbook implementation that tracks
//! individual orders with FIFO queue semantics at each price level.

use crate::checksum::{compute_checksum_with_precision, DEFAULT_PRICE_PRECISION, DEFAULT_QTY_PRECISION};
use crate::l3::order::{L3Order, L3PriceLevel, L3Side, OrderLocation, QueuePosition};
use kraken_types::Level;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};

/// L3 orderbook with individual order tracking
///
/// This is a hybrid data structure that uses:
/// - BTreeMap for price levels (sorted order for efficient iteration)
/// - HashMap for order index (O(1) lookup by order ID)
///
/// # Example
///
/// ```
/// use kraken_book::l3::{L3Book, L3Order, L3Side};
/// use rust_decimal_macros::dec;
///
/// let mut book = L3Book::new("BTC/USD", 10);
///
/// // Add orders
/// book.add_order(L3Order::new("order1", dec!(100), dec!(1)), L3Side::Bid);
/// book.add_order(L3Order::new("order2", dec!(101), dec!(2)), L3Side::Ask);
///
/// // Check queue position
/// if let Some(pos) = book.queue_position("order1") {
///     println!("Position: {}, Qty ahead: {}", pos.position, pos.qty_ahead);
/// }
/// ```
#[derive(Debug)]
pub struct L3Book {
    /// Symbol for this orderbook
    symbol: String,
    /// Bid levels (highest first - uses Reverse for descending order)
    bids: BTreeMap<Reverse<Decimal>, L3PriceLevel>,
    /// Ask levels (lowest first - natural ascending order)
    asks: BTreeMap<Decimal, L3PriceLevel>,
    /// Order index: order_id -> (price, side) for O(1) lookup
    order_index: HashMap<String, OrderLocation>,
    /// Maximum depth to maintain
    depth: u32,
    /// Last sequence number processed
    last_sequence: u64,
    /// Price precision for checksum
    price_precision: u8,
    /// Quantity precision for checksum
    qty_precision: u8,
}

impl L3Book {
    /// Create a new L3 orderbook
    pub fn new(symbol: impl Into<String>, depth: u32) -> Self {
        Self {
            symbol: symbol.into(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            order_index: HashMap::new(),
            depth,
            last_sequence: 0,
            price_precision: DEFAULT_PRICE_PRECISION,
            qty_precision: DEFAULT_QTY_PRECISION,
        }
    }

    /// Set precision for checksum calculation
    pub fn set_precision(&mut self, price_precision: u8, qty_precision: u8) {
        self.price_precision = price_precision;
        self.qty_precision = qty_precision;
    }

    /// Get the symbol
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Get the maximum depth
    pub fn depth(&self) -> u32 {
        self.depth
    }

    /// Get last processed sequence number
    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    /// Update the last sequence number
    pub fn set_last_sequence(&mut self, seq: u64) {
        self.last_sequence = seq;
    }

    // ========================================================================
    // Order Operations
    // ========================================================================

    /// Add a new order to the book
    ///
    /// Returns true if the order was added, false if it already exists
    pub fn add_order(&mut self, order: L3Order, side: L3Side) -> bool {
        // Check if order already exists
        if self.order_index.contains_key(&order.order_id) {
            return false;
        }

        let price = order.price;
        let order_id = order.order_id.clone();

        match side {
            L3Side::Bid => {
                let level = self.bids
                    .entry(Reverse(price))
                    .or_insert_with(|| L3PriceLevel::new(price));
                level.add_order(order);
            }
            L3Side::Ask => {
                let level = self.asks
                    .entry(price)
                    .or_insert_with(|| L3PriceLevel::new(price));
                level.add_order(order);
            }
        }

        // Update index
        self.order_index.insert(order_id, OrderLocation { price, side });
        true
    }

    /// Remove an order from the book
    ///
    /// Returns the removed order if found
    pub fn remove_order(&mut self, order_id: &str) -> Option<L3Order> {
        // Look up order location
        let location = self.order_index.remove(order_id)?;

        let removed = match location.side {
            L3Side::Bid => {
                let level = self.bids.get_mut(&Reverse(location.price))?;
                let order = level.remove_order(order_id)?;
                // Clean up empty level
                if level.is_empty() {
                    self.bids.remove(&Reverse(location.price));
                }
                order
            }
            L3Side::Ask => {
                let level = self.asks.get_mut(&location.price)?;
                let order = level.remove_order(order_id)?;
                // Clean up empty level
                if level.is_empty() {
                    self.asks.remove(&location.price);
                }
                order
            }
        };

        Some(removed)
    }

    /// Modify an order's quantity
    ///
    /// Returns true if the order was found and modified
    pub fn modify_order(&mut self, order_id: &str, new_qty: Decimal) -> bool {
        let location = match self.order_index.get(order_id) {
            Some(loc) => loc.clone(),
            None => return false,
        };

        match location.side {
            L3Side::Bid => {
                if let Some(level) = self.bids.get_mut(&Reverse(location.price)) {
                    return level.modify_order(order_id, new_qty);
                }
            }
            L3Side::Ask => {
                if let Some(level) = self.asks.get_mut(&location.price) {
                    return level.modify_order(order_id, new_qty);
                }
            }
        }
        false
    }

    /// Get an order by ID
    pub fn get_order(&self, order_id: &str) -> Option<&L3Order> {
        let location = self.order_index.get(order_id)?;

        match location.side {
            L3Side::Bid => {
                self.bids.get(&Reverse(location.price))?.get_order(order_id)
            }
            L3Side::Ask => {
                self.asks.get(&location.price)?.get_order(order_id)
            }
        }
    }

    /// Get the queue position for an order
    pub fn queue_position(&self, order_id: &str) -> Option<QueuePosition> {
        let location = self.order_index.get(order_id)?;

        match location.side {
            L3Side::Bid => {
                self.bids.get(&Reverse(location.price))?.queue_position(order_id)
            }
            L3Side::Ask => {
                self.asks.get(&location.price)?.queue_position(order_id)
            }
        }
    }

    /// Get the side of an order
    pub fn order_side(&self, order_id: &str) -> Option<L3Side> {
        self.order_index.get(order_id).map(|loc| loc.side)
    }

    /// Check if an order exists
    pub fn has_order(&self, order_id: &str) -> bool {
        self.order_index.contains_key(order_id)
    }

    // ========================================================================
    // Book Operations
    // ========================================================================

    /// Clear all orders and levels
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.order_index.clear();
        self.last_sequence = 0;
    }

    /// Get the best bid (highest price level)
    pub fn best_bid(&self) -> Option<&L3PriceLevel> {
        self.bids.values().next()
    }

    /// Get the best ask (lowest price level)
    pub fn best_ask(&self) -> Option<&L3PriceLevel> {
        self.asks.values().next()
    }

    /// Get the best bid price
    pub fn best_bid_price(&self) -> Option<Decimal> {
        self.best_bid().map(|l| l.price)
    }

    /// Get the best ask price
    pub fn best_ask_price(&self) -> Option<Decimal> {
        self.best_ask().map(|l| l.price)
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

    /// Iterate over bid levels (highest to lowest)
    pub fn bid_levels(&self) -> impl Iterator<Item = &L3PriceLevel> {
        self.bids.values()
    }

    /// Iterate over ask levels (lowest to highest)
    pub fn ask_levels(&self) -> impl Iterator<Item = &L3PriceLevel> {
        self.asks.values()
    }

    /// Get top N bid levels
    pub fn top_bids(&self, n: usize) -> Vec<&L3PriceLevel> {
        self.bids.values().take(n).collect()
    }

    /// Get top N ask levels
    pub fn top_asks(&self, n: usize) -> Vec<&L3PriceLevel> {
        self.asks.values().take(n).collect()
    }

    /// Number of bid levels
    pub fn bid_level_count(&self) -> usize {
        self.bids.len()
    }

    /// Number of ask levels
    pub fn ask_level_count(&self) -> usize {
        self.asks.len()
    }

    /// Total number of orders in the book
    pub fn order_count(&self) -> usize {
        self.order_index.len()
    }

    /// Check if book is empty
    pub fn is_empty(&self) -> bool {
        self.order_index.is_empty()
    }

    // ========================================================================
    // Aggregation (for L2 compatibility)
    // ========================================================================

    /// Get aggregated bid levels as L2 format
    pub fn aggregated_bids(&self) -> Vec<Level> {
        self.bids
            .values()
            .map(|level| Level::new(level.price, level.total_qty()))
            .collect()
    }

    /// Get aggregated ask levels as L2 format
    pub fn aggregated_asks(&self) -> Vec<Level> {
        self.asks
            .values()
            .map(|level| Level::new(level.price, level.total_qty()))
            .collect()
    }

    /// Get top N aggregated bids
    pub fn top_aggregated_bids(&self, n: usize) -> Vec<Level> {
        self.bids
            .values()
            .take(n)
            .map(|level| Level::new(level.price, level.total_qty()))
            .collect()
    }

    /// Get top N aggregated asks
    pub fn top_aggregated_asks(&self, n: usize) -> Vec<Level> {
        self.asks
            .values()
            .take(n)
            .map(|level| Level::new(level.price, level.total_qty()))
            .collect()
    }

    // ========================================================================
    // Checksum Validation
    // ========================================================================

    /// Compute checksum for the current book state
    ///
    /// Uses the same algorithm as L2 but with aggregated levels
    pub fn compute_checksum(&self) -> u32 {
        let bids = self.top_aggregated_bids(10);
        let asks = self.top_aggregated_asks(10);
        compute_checksum_with_precision(&bids, &asks, self.price_precision, self.qty_precision)
    }

    /// Validate against expected checksum
    pub fn validate_checksum(&self, expected: u32) -> Result<(), L3ChecksumMismatch> {
        let computed = self.compute_checksum();
        if computed != expected {
            Err(L3ChecksumMismatch {
                symbol: self.symbol.clone(),
                expected,
                computed,
            })
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // Depth Management
    // ========================================================================

    /// Truncate to maximum depth (removes levels beyond the limit)
    ///
    /// This also removes orders from the index for truncated levels.
    pub fn truncate(&mut self) {
        let depth = self.depth as usize;

        // Truncate bids
        if self.bids.len() > depth {
            let keys_to_remove: Vec<_> = self.bids.keys().skip(depth).cloned().collect();
            for key in keys_to_remove {
                if let Some(level) = self.bids.remove(&key) {
                    for order in level.orders() {
                        self.order_index.remove(&order.order_id);
                    }
                }
            }
        }

        // Truncate asks
        if self.asks.len() > depth {
            let keys_to_remove: Vec<_> = self.asks.keys().skip(depth).cloned().collect();
            for key in keys_to_remove {
                if let Some(level) = self.asks.remove(&key) {
                    for order in level.orders() {
                        self.order_index.remove(&order.order_id);
                    }
                }
            }
        }
    }

    // ========================================================================
    // Analytics
    // ========================================================================

    /// Get total bid quantity across all levels
    pub fn total_bid_qty(&self) -> Decimal {
        self.bids.values().map(|l| l.total_qty()).sum()
    }

    /// Get total ask quantity across all levels
    pub fn total_ask_qty(&self) -> Decimal {
        self.asks.values().map(|l| l.total_qty()).sum()
    }

    /// Get the bid/ask imbalance ratio
    ///
    /// Returns (bid_qty - ask_qty) / (bid_qty + ask_qty)
    /// Range: -1.0 (all asks) to 1.0 (all bids)
    pub fn imbalance(&self) -> Option<f64> {
        let bid_qty = self.total_bid_qty();
        let ask_qty = self.total_ask_qty();
        let total = bid_qty + ask_qty;

        if total.is_zero() {
            return None;
        }

        let diff = bid_qty - ask_qty;
        Some((diff / total).to_string().parse::<f64>().unwrap_or(0.0))
    }

    /// Get VWAP (Volume Weighted Average Price) for bids up to a quantity
    pub fn vwap_bid(&self, target_qty: Decimal) -> Option<Decimal> {
        let mut remaining = target_qty;
        let mut total_value = Decimal::ZERO;
        let mut total_qty = Decimal::ZERO;

        for level in self.bids.values() {
            if remaining.is_zero() {
                break;
            }

            let fill_qty = remaining.min(level.total_qty());
            total_value += level.price * fill_qty;
            total_qty += fill_qty;
            remaining -= fill_qty;
        }

        if total_qty.is_zero() {
            None
        } else {
            Some(total_value / total_qty)
        }
    }

    /// Get VWAP (Volume Weighted Average Price) for asks up to a quantity
    pub fn vwap_ask(&self, target_qty: Decimal) -> Option<Decimal> {
        let mut remaining = target_qty;
        let mut total_value = Decimal::ZERO;
        let mut total_qty = Decimal::ZERO;

        for level in self.asks.values() {
            if remaining.is_zero() {
                break;
            }

            let fill_qty = remaining.min(level.total_qty());
            total_value += level.price * fill_qty;
            total_qty += fill_qty;
            remaining -= fill_qty;
        }

        if total_qty.is_zero() {
            None
        } else {
            Some(total_value / total_qty)
        }
    }

    /// Take a snapshot of the current book state
    pub fn snapshot(&self) -> L3BookSnapshot {
        L3BookSnapshot {
            symbol: self.symbol.clone(),
            bids: self.aggregated_bids(),
            asks: self.aggregated_asks(),
            bid_orders: self.bids.values().flat_map(|l| l.orders()).cloned().collect(),
            ask_orders: self.asks.values().flat_map(|l| l.orders()).cloned().collect(),
            checksum: self.compute_checksum(),
            sequence: self.last_sequence,
        }
    }
}

/// Checksum mismatch error for L3 book
#[derive(Debug, Clone)]
pub struct L3ChecksumMismatch {
    pub symbol: String,
    pub expected: u32,
    pub computed: u32,
}

impl std::fmt::Display for L3ChecksumMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "L3 checksum mismatch for {}: expected {}, computed {}",
            self.symbol, self.expected, self.computed
        )
    }
}

impl std::error::Error for L3ChecksumMismatch {}

/// Immutable snapshot of L3 book state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L3BookSnapshot {
    /// Symbol
    pub symbol: String,
    /// Aggregated bid levels
    pub bids: Vec<Level>,
    /// Aggregated ask levels
    pub asks: Vec<Level>,
    /// Individual bid orders
    pub bid_orders: Vec<L3Order>,
    /// Individual ask orders
    pub ask_orders: Vec<L3Order>,
    /// Checksum
    pub checksum: u32,
    /// Sequence number
    pub sequence: u64,
}

impl L3BookSnapshot {
    /// Get total order count
    pub fn order_count(&self) -> usize {
        self.bid_orders.len() + self.ask_orders.len()
    }

    /// Get best bid price
    pub fn best_bid_price(&self) -> Option<Decimal> {
        self.bids.first().map(|l| l.price)
    }

    /// Get best ask price
    pub fn best_ask_price(&self) -> Option<Decimal> {
        self.asks.first().map(|l| l.price)
    }

    /// Get spread
    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_ask_price(), self.best_bid_price()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    /// Get mid price
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
    use rust_decimal_macros::dec;

    #[test]
    fn test_l3_book_creation() {
        let book = L3Book::new("BTC/USD", 10);
        assert_eq!(book.symbol(), "BTC/USD");
        assert_eq!(book.depth(), 10);
        assert!(book.is_empty());
    }

    #[test]
    fn test_add_orders() {
        let mut book = L3Book::new("BTC/USD", 10);

        // Add bids
        assert!(book.add_order(L3Order::new("b1", dec!(100), dec!(1)), L3Side::Bid));
        assert!(book.add_order(L3Order::new("b2", dec!(100), dec!(2)), L3Side::Bid));
        assert!(book.add_order(L3Order::new("b3", dec!(99), dec!(3)), L3Side::Bid));

        // Add asks
        assert!(book.add_order(L3Order::new("a1", dec!(101), dec!(1)), L3Side::Ask));
        assert!(book.add_order(L3Order::new("a2", dec!(102), dec!(2)), L3Side::Ask));

        assert_eq!(book.order_count(), 5);
        assert_eq!(book.bid_level_count(), 2);
        assert_eq!(book.ask_level_count(), 2);
    }

    #[test]
    fn test_duplicate_order_rejected() {
        let mut book = L3Book::new("BTC/USD", 10);

        assert!(book.add_order(L3Order::new("o1", dec!(100), dec!(1)), L3Side::Bid));
        assert!(!book.add_order(L3Order::new("o1", dec!(100), dec!(2)), L3Side::Bid));

        assert_eq!(book.order_count(), 1);
    }

    #[test]
    fn test_remove_order() {
        let mut book = L3Book::new("BTC/USD", 10);

        book.add_order(L3Order::new("o1", dec!(100), dec!(1)), L3Side::Bid);
        book.add_order(L3Order::new("o2", dec!(100), dec!(2)), L3Side::Bid);

        let removed = book.remove_order("o1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().order_id, "o1");
        assert_eq!(book.order_count(), 1);

        // Level should still exist with one order
        assert_eq!(book.bid_level_count(), 1);
    }

    #[test]
    fn test_remove_last_order_removes_level() {
        let mut book = L3Book::new("BTC/USD", 10);

        book.add_order(L3Order::new("o1", dec!(100), dec!(1)), L3Side::Bid);
        book.remove_order("o1");

        assert_eq!(book.order_count(), 0);
        assert_eq!(book.bid_level_count(), 0);
    }

    #[test]
    fn test_modify_order() {
        let mut book = L3Book::new("BTC/USD", 10);

        book.add_order(L3Order::new("o1", dec!(100), dec!(5)), L3Side::Bid);
        assert!(book.modify_order("o1", dec!(3)));

        let order = book.get_order("o1").unwrap();
        assert_eq!(order.qty, dec!(3));
    }

    #[test]
    fn test_best_bid_ask() {
        let mut book = L3Book::new("BTC/USD", 10);

        book.add_order(L3Order::new("b1", dec!(99), dec!(1)), L3Side::Bid);
        book.add_order(L3Order::new("b2", dec!(100), dec!(1)), L3Side::Bid);
        book.add_order(L3Order::new("a1", dec!(101), dec!(1)), L3Side::Ask);
        book.add_order(L3Order::new("a2", dec!(102), dec!(1)), L3Side::Ask);

        assert_eq!(book.best_bid_price(), Some(dec!(100)));
        assert_eq!(book.best_ask_price(), Some(dec!(101)));
        assert_eq!(book.spread(), Some(dec!(1)));
        assert_eq!(book.mid_price(), Some(dec!(100.5)));
    }

    #[test]
    fn test_queue_position() {
        let mut book = L3Book::new("BTC/USD", 10);

        book.add_order(L3Order::new("o1", dec!(100), dec!(1)), L3Side::Bid);
        book.add_order(L3Order::new("o2", dec!(100), dec!(2)), L3Side::Bid);
        book.add_order(L3Order::new("o3", dec!(100), dec!(3)), L3Side::Bid);

        let pos = book.queue_position("o2").unwrap();
        assert_eq!(pos.position, 1);
        assert_eq!(pos.qty_ahead, dec!(1));
        assert_eq!(pos.total_orders, 3);
    }

    #[test]
    fn test_aggregated_levels() {
        let mut book = L3Book::new("BTC/USD", 10);

        book.add_order(L3Order::new("b1", dec!(100), dec!(1)), L3Side::Bid);
        book.add_order(L3Order::new("b2", dec!(100), dec!(2)), L3Side::Bid);
        book.add_order(L3Order::new("b3", dec!(99), dec!(3)), L3Side::Bid);

        let bids = book.aggregated_bids();
        assert_eq!(bids.len(), 2);
        // Best bid should be 100 with qty 3 (1 + 2)
        assert_eq!(bids[0].price, dec!(100));
        assert_eq!(bids[0].qty, dec!(3));
        // Second bid should be 99 with qty 3
        assert_eq!(bids[1].price, dec!(99));
        assert_eq!(bids[1].qty, dec!(3));
    }

    #[test]
    fn test_truncate() {
        let mut book = L3Book::new("BTC/USD", 2);

        // Add 3 bid levels
        book.add_order(L3Order::new("b1", dec!(100), dec!(1)), L3Side::Bid);
        book.add_order(L3Order::new("b2", dec!(99), dec!(1)), L3Side::Bid);
        book.add_order(L3Order::new("b3", dec!(98), dec!(1)), L3Side::Bid);

        // Add 3 ask levels
        book.add_order(L3Order::new("a1", dec!(101), dec!(1)), L3Side::Ask);
        book.add_order(L3Order::new("a2", dec!(102), dec!(1)), L3Side::Ask);
        book.add_order(L3Order::new("a3", dec!(103), dec!(1)), L3Side::Ask);

        book.truncate();

        assert_eq!(book.bid_level_count(), 2);
        assert_eq!(book.ask_level_count(), 2);
        assert_eq!(book.order_count(), 4);

        // Should have removed b3 and a3
        assert!(!book.has_order("b3"));
        assert!(!book.has_order("a3"));
    }

    #[test]
    fn test_vwap() {
        let mut book = L3Book::new("BTC/USD", 10);

        book.add_order(L3Order::new("a1", dec!(100), dec!(1)), L3Side::Ask);
        book.add_order(L3Order::new("a2", dec!(101), dec!(2)), L3Side::Ask);
        book.add_order(L3Order::new("a3", dec!(102), dec!(3)), L3Side::Ask);

        // VWAP for buying 3 units: (100*1 + 101*2) / 3 = 302/3 = ~100.67
        let vwap = book.vwap_ask(dec!(3)).unwrap();
        assert!(vwap > dec!(100.66) && vwap < dec!(100.68));
    }

    #[test]
    fn test_snapshot() {
        let mut book = L3Book::new("BTC/USD", 10);

        book.add_order(L3Order::new("b1", dec!(100), dec!(1)), L3Side::Bid);
        book.add_order(L3Order::new("a1", dec!(101), dec!(2)), L3Side::Ask);

        let snapshot = book.snapshot();
        assert_eq!(snapshot.symbol, "BTC/USD");
        assert_eq!(snapshot.order_count(), 2);
        assert_eq!(snapshot.best_bid_price(), Some(dec!(100)));
        assert_eq!(snapshot.best_ask_price(), Some(dec!(101)));
    }
}
