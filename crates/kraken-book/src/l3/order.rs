//! L3 order types for individual order tracking
//!
//! This module provides types for Level 3 (order-level) orderbook data,
//! which tracks individual orders rather than just aggregated price levels.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Individual order in the L3 orderbook
///
/// Represents a single order with its unique ID, price, quantity, and metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L3Order {
    /// Unique order ID assigned by Kraken
    pub order_id: String,
    /// Order price
    pub price: Decimal,
    /// Order quantity (remaining)
    pub qty: Decimal,
    /// Timestamp when order was placed (microseconds since epoch)
    pub timestamp: u64,
    /// Sequence number for ordering within same timestamp
    pub sequence: u64,
}

impl L3Order {
    /// Create a new L3 order
    pub fn new(order_id: impl Into<String>, price: Decimal, qty: Decimal) -> Self {
        Self {
            order_id: order_id.into(),
            price,
            qty,
            timestamp: 0,
            sequence: 0,
        }
    }

    /// Create with full metadata
    pub fn with_metadata(
        order_id: impl Into<String>,
        price: Decimal,
        qty: Decimal,
        timestamp: u64,
        sequence: u64,
    ) -> Self {
        Self {
            order_id: order_id.into(),
            price,
            qty,
            timestamp,
            sequence,
        }
    }
}

/// Price level containing multiple orders (FIFO queue)
///
/// Orders at the same price are maintained in FIFO order (oldest first).
/// This is critical for queue position calculation.
#[derive(Debug, Clone, Default)]
pub struct L3PriceLevel {
    /// Price for this level
    pub price: Decimal,
    /// Orders at this price level (FIFO order - oldest first)
    orders: Vec<L3Order>,
    /// Cached total quantity (sum of all order quantities)
    total_qty: Decimal,
}

impl L3PriceLevel {
    /// Create a new empty price level
    pub fn new(price: Decimal) -> Self {
        Self {
            price,
            orders: Vec::new(),
            total_qty: Decimal::ZERO,
        }
    }

    /// Add an order to this level (appended at the end - newest)
    pub fn add_order(&mut self, order: L3Order) {
        self.total_qty += order.qty;
        self.orders.push(order);
    }

    /// Remove an order by ID
    ///
    /// Returns the removed order if found
    pub fn remove_order(&mut self, order_id: &str) -> Option<L3Order> {
        if let Some(idx) = self.orders.iter().position(|o| o.order_id == order_id) {
            let order = self.orders.remove(idx);
            self.total_qty -= order.qty;
            Some(order)
        } else {
            None
        }
    }

    /// Modify an order's quantity
    ///
    /// Returns true if the order was found and modified
    pub fn modify_order(&mut self, order_id: &str, new_qty: Decimal) -> bool {
        if let Some(order) = self.orders.iter_mut().find(|o| o.order_id == order_id) {
            self.total_qty = self.total_qty - order.qty + new_qty;
            order.qty = new_qty;
            true
        } else {
            false
        }
    }

    /// Get an order by ID
    pub fn get_order(&self, order_id: &str) -> Option<&L3Order> {
        self.orders.iter().find(|o| o.order_id == order_id)
    }

    /// Get queue position for an order
    ///
    /// Returns the position in the queue (0-indexed) and the total quantity
    /// ahead of this order. Returns None if order not found.
    pub fn queue_position(&self, order_id: &str) -> Option<QueuePosition> {
        let mut qty_ahead = Decimal::ZERO;

        for (idx, order) in self.orders.iter().enumerate() {
            if order.order_id == order_id {
                return Some(QueuePosition {
                    position: idx,
                    orders_ahead: idx,
                    qty_ahead,
                    total_orders: self.orders.len(),
                    total_qty: self.total_qty,
                });
            }
            qty_ahead += order.qty;
        }
        None
    }

    /// Get total quantity at this price level
    pub fn total_qty(&self) -> Decimal {
        self.total_qty
    }

    /// Get number of orders at this level
    pub fn order_count(&self) -> usize {
        self.orders.len()
    }

    /// Check if level is empty
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    /// Iterate over orders (oldest first)
    pub fn orders(&self) -> impl Iterator<Item = &L3Order> {
        self.orders.iter()
    }

    /// Get oldest order (front of queue)
    pub fn oldest(&self) -> Option<&L3Order> {
        self.orders.first()
    }

    /// Get newest order (back of queue)
    pub fn newest(&self) -> Option<&L3Order> {
        self.orders.last()
    }

    /// Get the average order size at this level
    pub fn avg_order_size(&self) -> Option<Decimal> {
        if self.orders.is_empty() {
            None
        } else {
            Some(self.total_qty / Decimal::from(self.orders.len()))
        }
    }

    /// Recalculate the total quantity (for validation)
    pub fn recalculate_total(&mut self) {
        self.total_qty = self.orders.iter().map(|o| o.qty).sum();
    }
}

/// Queue position information for an order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuePosition {
    /// 0-indexed position in the queue
    pub position: usize,
    /// Number of orders ahead (same as position)
    pub orders_ahead: usize,
    /// Total quantity ahead of this order
    pub qty_ahead: Decimal,
    /// Total number of orders at this price level
    pub total_orders: usize,
    /// Total quantity at this price level
    pub total_qty: Decimal,
}

impl QueuePosition {
    /// Calculate fill probability (rough estimate)
    ///
    /// Returns a value between 0.0 and 1.0 representing the
    /// approximate probability of being filled before other orders.
    pub fn fill_probability(&self) -> f64 {
        if self.total_orders == 0 {
            return 0.0;
        }
        1.0 - (self.position as f64 / self.total_orders as f64)
    }

    /// Check if this order is at the front of the queue
    pub fn is_first(&self) -> bool {
        self.position == 0
    }

    /// Check if this order is at the back of the queue
    pub fn is_last(&self) -> bool {
        self.position == self.total_orders.saturating_sub(1)
    }
}

/// Location of an order in the book (for index lookup)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderLocation {
    /// Price level
    pub price: Decimal,
    /// Side (bid or ask)
    pub side: L3Side,
}

/// Side of the orderbook
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum L3Side {
    Bid,
    Ask,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_l3_order_creation() {
        let order = L3Order::new("order1", dec!(100.50), dec!(1.5));
        assert_eq!(order.order_id, "order1");
        assert_eq!(order.price, dec!(100.50));
        assert_eq!(order.qty, dec!(1.5));
    }

    #[test]
    fn test_price_level_add_order() {
        let mut level = L3PriceLevel::new(dec!(100));

        level.add_order(L3Order::new("o1", dec!(100), dec!(1)));
        level.add_order(L3Order::new("o2", dec!(100), dec!(2)));

        assert_eq!(level.order_count(), 2);
        assert_eq!(level.total_qty(), dec!(3));
    }

    #[test]
    fn test_price_level_remove_order() {
        let mut level = L3PriceLevel::new(dec!(100));

        level.add_order(L3Order::new("o1", dec!(100), dec!(1)));
        level.add_order(L3Order::new("o2", dec!(100), dec!(2)));

        let removed = level.remove_order("o1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().order_id, "o1");
        assert_eq!(level.order_count(), 1);
        assert_eq!(level.total_qty(), dec!(2));
    }

    #[test]
    fn test_price_level_modify_order() {
        let mut level = L3PriceLevel::new(dec!(100));
        level.add_order(L3Order::new("o1", dec!(100), dec!(5)));

        assert!(level.modify_order("o1", dec!(3)));
        assert_eq!(level.total_qty(), dec!(3));
        assert_eq!(level.get_order("o1").unwrap().qty, dec!(3));
    }

    #[test]
    fn test_queue_position() {
        let mut level = L3PriceLevel::new(dec!(100));

        level.add_order(L3Order::new("o1", dec!(100), dec!(1)));
        level.add_order(L3Order::new("o2", dec!(100), dec!(2)));
        level.add_order(L3Order::new("o3", dec!(100), dec!(3)));

        // First order
        let pos1 = level.queue_position("o1").unwrap();
        assert_eq!(pos1.position, 0);
        assert_eq!(pos1.qty_ahead, dec!(0));
        assert!(pos1.is_first());

        // Second order
        let pos2 = level.queue_position("o2").unwrap();
        assert_eq!(pos2.position, 1);
        assert_eq!(pos2.qty_ahead, dec!(1));

        // Third order
        let pos3 = level.queue_position("o3").unwrap();
        assert_eq!(pos3.position, 2);
        assert_eq!(pos3.qty_ahead, dec!(3));
        assert!(pos3.is_last());
    }

    #[test]
    fn test_fifo_order() {
        let mut level = L3PriceLevel::new(dec!(100));

        level.add_order(L3Order::new("first", dec!(100), dec!(1)));
        level.add_order(L3Order::new("second", dec!(100), dec!(1)));
        level.add_order(L3Order::new("third", dec!(100), dec!(1)));

        // Oldest should be first
        assert_eq!(level.oldest().unwrap().order_id, "first");
        // Newest should be last
        assert_eq!(level.newest().unwrap().order_id, "third");
    }

    #[test]
    fn test_queue_position_fill_probability() {
        let mut level = L3PriceLevel::new(dec!(100));

        level.add_order(L3Order::new("o1", dec!(100), dec!(1)));
        level.add_order(L3Order::new("o2", dec!(100), dec!(1)));
        level.add_order(L3Order::new("o3", dec!(100), dec!(1)));
        level.add_order(L3Order::new("o4", dec!(100), dec!(1)));

        let pos1 = level.queue_position("o1").unwrap();
        assert_eq!(pos1.fill_probability(), 1.0);

        let pos4 = level.queue_position("o4").unwrap();
        assert_eq!(pos4.fill_probability(), 0.25);
    }
}
