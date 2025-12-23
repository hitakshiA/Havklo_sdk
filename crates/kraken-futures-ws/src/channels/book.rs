//! Orderbook channel handler

use crate::types::{FuturesBookSnapshot, FuturesBookUpdate, FuturesEvent};
use kraken_book::TreeBook;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Orderbook channel handler
pub struct BookChannel {
    /// Orderbooks by product ID
    books: HashMap<String, TreeBook>,
    /// Sequence numbers by product ID
    sequences: HashMap<String, u64>,
    /// Depth limit (informational only, TreeBook doesn't limit)
    _depth: usize,
}

impl BookChannel {
    /// Create a new book channel handler
    pub fn new(depth: usize) -> Self {
        Self {
            books: HashMap::new(),
            sequences: HashMap::new(),
            _depth: depth,
        }
    }

    /// Process a book snapshot
    pub fn process_snapshot(&mut self, snapshot: FuturesBookSnapshot) -> FuturesEvent {
        let product_id = snapshot.product_id.clone();

        // Create or reset book
        let book = self.books.entry(product_id.clone()).or_default();

        // Clear existing book
        book.clear();

        // Apply bids
        for level in &snapshot.bids {
            book.insert_bid(level.price, level.qty);
        }

        // Apply asks
        for level in &snapshot.asks {
            book.insert_ask(level.price, level.qty);
        }

        // Store sequence
        self.sequences.insert(product_id.clone(), snapshot.seq);

        debug!("Applied book snapshot for {} at seq {}", product_id, snapshot.seq);

        FuturesEvent::BookSnapshot(snapshot)
    }

    /// Process a book update
    pub fn process_update(&mut self, update: FuturesBookUpdate) -> Option<FuturesEvent> {
        let product_id = &update.product_id;

        // Check sequence
        if let Some(&last_seq) = self.sequences.get(product_id) {
            if update.seq <= last_seq {
                debug!("Ignoring stale update {} <= {}", update.seq, last_seq);
                return None;
            }

            if update.seq != last_seq + 1 {
                warn!(
                    "Sequence gap detected for {}: expected {}, got {}",
                    product_id,
                    last_seq + 1,
                    update.seq
                );
                // Request resync needed
                return None;
            }
        } else {
            warn!("Update received before snapshot for {}", product_id);
            return None;
        }

        let book = self.books.get_mut(product_id)?;

        // Apply bid updates
        for level in &update.bids {
            if level.qty.is_zero() {
                book.remove_bid(&level.price);
            } else {
                book.insert_bid(level.price, level.qty);
            }
        }

        // Apply ask updates
        for level in &update.asks {
            if level.qty.is_zero() {
                book.remove_ask(&level.price);
            } else {
                book.insert_ask(level.price, level.qty);
            }
        }

        // Update sequence
        self.sequences.insert(product_id.clone(), update.seq);

        Some(FuturesEvent::BookUpdate(update))
    }

    /// Get best bid for a product (returns qty, price)
    pub fn best_bid(&self, product_id: &str) -> Option<(Decimal, Decimal)> {
        let level = self.books.get(product_id)?.best_bid()?;
        Some((level.qty, level.price))
    }

    /// Get best ask for a product (returns qty, price)
    pub fn best_ask(&self, product_id: &str) -> Option<(Decimal, Decimal)> {
        let level = self.books.get(product_id)?.best_ask()?;
        Some((level.qty, level.price))
    }

    /// Get spread for a product
    pub fn spread(&self, product_id: &str) -> Option<Decimal> {
        let book = self.books.get(product_id)?;
        let bid_price = book.best_bid()?.price;
        let ask_price = book.best_ask()?.price;
        Some(ask_price - bid_price)
    }

    /// Get mid price for a product
    pub fn mid_price(&self, product_id: &str) -> Option<Decimal> {
        let book = self.books.get(product_id)?;
        let bid_price = book.best_bid()?.price;
        let ask_price = book.best_ask()?.price;
        Some((bid_price + ask_price) / Decimal::TWO)
    }

    /// Check if we need a snapshot for this product
    pub fn needs_snapshot(&self, product_id: &str) -> bool {
        !self.sequences.contains_key(product_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::BookLevel;

    #[test]
    fn test_book_channel_snapshot() {
        let mut channel = BookChannel::new(10);

        let snapshot = FuturesBookSnapshot {
            product_id: "PI_XBTUSD".to_string(),
            seq: 1,
            bids: vec![
                BookLevel { price: Decimal::from(50000), qty: Decimal::ONE },
                BookLevel { price: Decimal::from(49999), qty: Decimal::TWO },
            ],
            asks: vec![
                BookLevel { price: Decimal::from(50001), qty: Decimal::ONE },
                BookLevel { price: Decimal::from(50002), qty: Decimal::TWO },
            ],
            timestamp: 1234567890,
        };

        let _event = channel.process_snapshot(snapshot);

        assert_eq!(channel.best_bid("PI_XBTUSD"), Some((Decimal::ONE, Decimal::from(50000))));
        assert_eq!(channel.best_ask("PI_XBTUSD"), Some((Decimal::ONE, Decimal::from(50001))));
    }

    #[test]
    fn test_book_channel_update() {
        let mut channel = BookChannel::new(10);

        // First apply snapshot
        let snapshot = FuturesBookSnapshot {
            product_id: "PI_XBTUSD".to_string(),
            seq: 1,
            bids: vec![
                BookLevel { price: Decimal::from(50000), qty: Decimal::ONE },
            ],
            asks: vec![
                BookLevel { price: Decimal::from(50001), qty: Decimal::ONE },
            ],
            timestamp: 1234567890,
        };
        channel.process_snapshot(snapshot);

        // Then apply update
        let update = FuturesBookUpdate {
            product_id: "PI_XBTUSD".to_string(),
            seq: 2,
            bids: vec![
                BookLevel { price: Decimal::from(50000), qty: Decimal::from(2) }, // Update qty
            ],
            asks: vec![],
            timestamp: 1234567891,
        };
        let result = channel.process_update(update);
        assert!(result.is_some());

        // Qty should be updated
        assert_eq!(channel.best_bid("PI_XBTUSD"), Some((Decimal::from(2), Decimal::from(50000))));
    }
}
