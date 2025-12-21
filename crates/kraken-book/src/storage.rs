//! BTreeMap-based orderbook storage
//!
//! Provides O(log N) operations for orderbook management.
//! Uses `Reverse<Decimal>` for bids to maintain descending order.

use kraken_types::Level;
use rust_decimal::Decimal;
use std::cmp::Reverse;
use std::collections::BTreeMap;

/// Orderbook storage using BTreeMap for O(log N) operations
///
/// - Bids: Stored with `Reverse<Decimal>` key for descending order (highest first)
/// - Asks: Stored with `Decimal` key for ascending order (lowest first)
#[derive(Debug, Clone, Default)]
pub struct TreeBook {
    /// Bids: highest price first (use Reverse for descending order)
    bids: BTreeMap<Reverse<Decimal>, Level>,
    /// Asks: lowest price first (natural ascending order)
    asks: BTreeMap<Decimal, Level>,
}

impl TreeBook {
    /// Create a new empty orderbook
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// Insert or update a bid level
    /// If qty is zero, the level is removed
    pub fn insert_bid(&mut self, price: Decimal, qty: Decimal) {
        if qty.is_zero() {
            self.bids.remove(&Reverse(price));
        } else {
            self.bids.insert(Reverse(price), Level::new(price, qty));
        }
    }

    /// Insert or update an ask level
    /// If qty is zero, the level is removed
    pub fn insert_ask(&mut self, price: Decimal, qty: Decimal) {
        if qty.is_zero() {
            self.asks.remove(&price);
        } else {
            self.asks.insert(price, Level::new(price, qty));
        }
    }

    /// Remove a bid level by price
    pub fn remove_bid(&mut self, price: &Decimal) {
        self.bids.remove(&Reverse(*price));
    }

    /// Remove an ask level by price
    pub fn remove_ask(&mut self, price: &Decimal) {
        self.asks.remove(price);
    }

    /// Get the best bid (highest price)
    pub fn best_bid(&self) -> Option<&Level> {
        self.bids.values().next()
    }

    /// Get the best ask (lowest price)
    pub fn best_ask(&self) -> Option<&Level> {
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

    /// Iterator over bids (highest to lowest price)
    pub fn bids(&self) -> impl Iterator<Item = &Level> {
        self.bids.values()
    }

    /// Iterator over asks (lowest to highest price)
    pub fn asks(&self) -> impl Iterator<Item = &Level> {
        self.asks.values()
    }

    /// Get bids as a vector (for serialization)
    pub fn bids_vec(&self) -> Vec<Level> {
        self.bids.values().cloned().collect()
    }

    /// Get asks as a vector (for serialization)
    pub fn asks_vec(&self) -> Vec<Level> {
        self.asks.values().cloned().collect()
    }

    /// Get top N bids
    pub fn top_bids(&self, n: usize) -> Vec<Level> {
        self.bids.values().take(n).cloned().collect()
    }

    /// Get top N asks
    pub fn top_asks(&self, n: usize) -> Vec<Level> {
        self.asks.values().take(n).cloned().collect()
    }

    /// Number of bid levels
    pub fn bid_count(&self) -> usize {
        self.bids.len()
    }

    /// Number of ask levels
    pub fn ask_count(&self) -> usize {
        self.asks.len()
    }

    /// Total number of levels
    pub fn level_count(&self) -> usize {
        self.bid_count() + self.ask_count()
    }

    /// Check if the orderbook is empty
    pub fn is_empty(&self) -> bool {
        self.bids.is_empty() && self.asks.is_empty()
    }

    /// Clear all levels
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
    }

    /// Truncate to maximum depth (removes levels beyond the limit)
    pub fn truncate(&mut self, max_depth: usize) {
        // Keep only the top `max_depth` bids
        if self.bids.len() > max_depth {
            let keys_to_remove: Vec<_> = self.bids.keys().skip(max_depth).cloned().collect();
            for key in keys_to_remove {
                self.bids.remove(&key);
            }
        }

        // Keep only the top `max_depth` asks
        if self.asks.len() > max_depth {
            let keys_to_remove: Vec<_> = self.asks.keys().skip(max_depth).cloned().collect();
            for key in keys_to_remove {
                self.asks.remove(&key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bid_order() {
        let mut book = TreeBook::new();
        book.insert_bid(dec!(100), dec!(1));
        book.insert_bid(dec!(101), dec!(2));
        book.insert_bid(dec!(99), dec!(3));

        let bids: Vec<_> = book.bids().collect();
        assert_eq!(bids.len(), 3);
        // Should be in descending order
        assert_eq!(bids[0].price, dec!(101));
        assert_eq!(bids[1].price, dec!(100));
        assert_eq!(bids[2].price, dec!(99));
    }

    #[test]
    fn test_ask_order() {
        let mut book = TreeBook::new();
        book.insert_ask(dec!(100), dec!(1));
        book.insert_ask(dec!(101), dec!(2));
        book.insert_ask(dec!(99), dec!(3));

        let asks: Vec<_> = book.asks().collect();
        assert_eq!(asks.len(), 3);
        // Should be in ascending order
        assert_eq!(asks[0].price, dec!(99));
        assert_eq!(asks[1].price, dec!(100));
        assert_eq!(asks[2].price, dec!(101));
    }

    #[test]
    fn test_zero_qty_removes_level() {
        let mut book = TreeBook::new();
        book.insert_bid(dec!(100), dec!(1));
        assert_eq!(book.bid_count(), 1);

        book.insert_bid(dec!(100), dec!(0));
        assert_eq!(book.bid_count(), 0);
    }

    #[test]
    fn test_best_bid_ask() {
        let mut book = TreeBook::new();
        book.insert_bid(dec!(99), dec!(1));
        book.insert_bid(dec!(100), dec!(1));
        book.insert_ask(dec!(101), dec!(1));
        book.insert_ask(dec!(102), dec!(1));

        assert_eq!(book.best_bid_price(), Some(dec!(100)));
        assert_eq!(book.best_ask_price(), Some(dec!(101)));
    }

    #[test]
    fn test_truncate() {
        let mut book = TreeBook::new();
        for i in 1..=20 {
            book.insert_bid(Decimal::from(i), dec!(1));
            book.insert_ask(Decimal::from(100 + i), dec!(1));
        }

        assert_eq!(book.bid_count(), 20);
        assert_eq!(book.ask_count(), 20);

        book.truncate(10);

        assert_eq!(book.bid_count(), 10);
        assert_eq!(book.ask_count(), 10);

        // Best bid should be 20 (highest)
        assert_eq!(book.best_bid_price(), Some(dec!(20)));
        // Best ask should be 101 (lowest)
        assert_eq!(book.best_ask_price(), Some(dec!(101)));
    }
}
