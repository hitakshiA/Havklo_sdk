//! Order Lifecycle Tracker
//!
//! Provides complete order lifecycle tracking with state management, fill aggregation,
//! slippage calculation, and timing metrics. This is a key innovation feature that
//! no other Havklo SDK provides.
//!
//! # Features
//!
//! - **Request-Order Correlation**: Track orders from submission to completion
//! - **State Machine**: Pending → New → PartialFill → Filled/Canceled
//! - **Fill Aggregation**: Calculate average fill price across partial fills
//! - **Slippage Tracking**: Compare expected vs actual execution price
//! - **Timing Metrics**: Time to first fill, time to complete
//! - **Query API**: Filter orders by status, symbol, or custom criteria
//!
//! # Example
//!
//! ```no_run
//! use kraken_ws::order_tracker::{OrderTracker, TrackerConfig};
//! use kraken_types::{AddOrderRequest, AddOrderParams};
//!
//! let mut tracker = OrderTracker::new();
//!
//! // Track a new order submission
//! tracker.track_submission("req_123", "BTC/USD", kraken_types::Side::Buy,
//!     rust_decimal_macros::dec!(100), Some(rust_decimal_macros::dec!(50000)));
//!
//! // Later, when execution event arrives
//! // tracker.handle_execution(&execution_data);
//!
//! // Query order state
//! if let Some(order) = tracker.get_by_request_id("req_123") {
//!     println!("Order: {:?}, Status: {:?}", order.order_id, order.lifecycle_state);
//!     if let Some(avg) = order.avg_fill_price() {
//!         println!("Average fill price: {}", avg);
//!     }
//! }
//! ```
//!
//! # Lifecycle States
//!
//! ```text
//! ┌─────────────┐
//! │   Pending   │──────────────────────────────────────┐
//! └──────┬──────┘                                      │
//!        │ Order acknowledged                          │
//!        ▼                                             │
//! ┌─────────────┐                                      │
//! │     New     │──────────────────────┐               │
//! └──────┬──────┘                      │               │
//!        │ Partial fill                │               │
//!        ▼                             │               │
//! ┌─────────────┐                      │               │
//! │PartialFill  │◄─────────────────────┤               │
//! └──────┬──────┘                      │               │
//!        │ Complete fill               │ Cancel        │ Reject
//!        ▼                             ▼               ▼
//! ┌─────────────┐               ┌─────────────┐ ┌─────────────┐
//! │   Filled    │               │  Canceled   │ │  Rejected   │
//! └─────────────┘               └─────────────┘ └─────────────┘
//! ```

use kraken_types::{Decimal, ExecutionData, Side};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, instrument, warn};

/// Order lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifecycleState {
    /// Order submitted, awaiting acknowledgment
    Pending,
    /// Order acknowledged, resting in book
    New,
    /// Order has been partially filled
    PartiallyFilled,
    /// Order completely filled
    Filled,
    /// Order was canceled (by user or system)
    Canceled,
    /// Order expired (time-in-force)
    Expired,
    /// Order was rejected
    Rejected,
}

impl LifecycleState {
    /// Parse from Kraken status string
    pub fn from_kraken_status(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "pending" | "pending-new" => Self::Pending,
            "new" | "open" => Self::New,
            "partially_filled" | "partiallyfilled" | "partial" => Self::PartiallyFilled,
            "filled" | "closed" => Self::Filled,
            "canceled" | "cancelled" => Self::Canceled,
            "expired" => Self::Expired,
            _ => Self::Rejected,
        }
    }

    /// Check if order is still active (can receive fills)
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::New | Self::PartiallyFilled)
    }

    /// Check if order is terminal (no more state changes)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Filled | Self::Canceled | Self::Expired | Self::Rejected)
    }

    /// Check if order was successfully completed
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Filled)
    }
}

impl std::fmt::Display for LifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::New => write!(f, "New"),
            Self::PartiallyFilled => write!(f, "PartiallyFilled"),
            Self::Filled => write!(f, "Filled"),
            Self::Canceled => write!(f, "Canceled"),
            Self::Expired => write!(f, "Expired"),
            Self::Rejected => write!(f, "Rejected"),
        }
    }
}

/// Individual fill record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    /// Execution/trade ID
    pub exec_id: Option<String>,
    /// Fill price
    pub price: Decimal,
    /// Fill quantity
    pub qty: Decimal,
    /// Fee amount
    pub fee: Decimal,
    /// Fee currency
    pub fee_currency: Option<String>,
    /// Fill timestamp (ISO 8601)
    pub timestamp: String,
    /// Time since order submission (if tracking enabled)
    #[serde(skip)]
    pub latency: Option<Duration>,
}

impl Fill {
    /// Fill value (price * qty)
    pub fn value(&self) -> Decimal {
        self.price * self.qty
    }
}

/// Tracked order with complete lifecycle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleOrder {
    /// Original request ID (for correlation)
    pub request_id: Option<String>,
    /// Kraken order ID (assigned after acknowledgment)
    pub order_id: Option<String>,
    /// User reference (cl_ord_id)
    pub user_ref: Option<String>,
    /// Trading symbol
    pub symbol: String,
    /// Order side
    pub side: Side,
    /// Order type (limit, market, etc.)
    pub order_type: String,
    /// Original quantity
    pub original_qty: Decimal,
    /// Limit price (if limit order)
    pub limit_price: Option<Decimal>,
    /// Current lifecycle state
    pub lifecycle_state: LifecycleState,
    /// Cumulative filled quantity
    pub filled_qty: Decimal,
    /// All fills for this order
    pub fills: Vec<Fill>,
    /// Total fees paid
    pub total_fees: Decimal,
    /// Fee currency
    pub fee_currency: Option<String>,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last update timestamp (ISO 8601)
    pub updated_at: String,
    /// Cancel reason (if canceled)
    pub cancel_reason: Option<String>,
    /// Reject reason (if rejected)
    pub reject_reason: Option<String>,
    /// Internal tracking: submission time
    #[serde(skip)]
    submission_time: Option<Instant>,
    /// Internal tracking: first fill time
    #[serde(skip)]
    first_fill_time: Option<Instant>,
    /// Internal tracking: completion time
    #[serde(skip)]
    completion_time: Option<Instant>,
}

impl LifecycleOrder {
    /// Create a new pending order
    pub fn new_pending(
        request_id: Option<String>,
        symbol: String,
        side: Side,
        qty: Decimal,
        limit_price: Option<Decimal>,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            request_id,
            order_id: None,
            user_ref: None,
            symbol,
            side,
            order_type: if limit_price.is_some() { "limit" } else { "market" }.to_string(),
            original_qty: qty,
            limit_price,
            lifecycle_state: LifecycleState::Pending,
            filled_qty: Decimal::ZERO,
            fills: Vec::new(),
            total_fees: Decimal::ZERO,
            fee_currency: None,
            created_at: now.clone(),
            updated_at: now,
            cancel_reason: None,
            reject_reason: None,
            submission_time: Some(Instant::now()),
            first_fill_time: None,
            completion_time: None,
        }
    }

    /// Remaining quantity to be filled
    pub fn remaining_qty(&self) -> Decimal {
        self.original_qty - self.filled_qty
    }

    /// Fill percentage (0.0 to 100.0)
    pub fn fill_percentage(&self) -> Decimal {
        if self.original_qty.is_zero() {
            return Decimal::ZERO;
        }
        (self.filled_qty / self.original_qty) * dec!(100)
    }

    /// Average fill price (weighted by quantity)
    pub fn avg_fill_price(&self) -> Option<Decimal> {
        if self.fills.is_empty() || self.filled_qty.is_zero() {
            return None;
        }

        let total_value: Decimal = self.fills.iter().map(|f| f.value()).sum();
        Some(total_value / self.filled_qty)
    }

    /// Calculate slippage vs limit price (in basis points)
    ///
    /// Positive = worse than expected (paid more for buy, received less for sell)
    /// Negative = better than expected
    pub fn slippage_bps(&self) -> Option<Decimal> {
        let limit_price = self.limit_price?;
        let avg_price = self.avg_fill_price()?;

        if limit_price.is_zero() {
            return None;
        }

        let slippage = match self.side {
            Side::Buy => (avg_price - limit_price) / limit_price,
            Side::Sell => (limit_price - avg_price) / limit_price,
        };

        Some(slippage * dec!(10000))
    }

    /// Calculate slippage vs a reference price (for market orders)
    pub fn slippage_vs_reference(&self, reference_price: Decimal) -> Option<Decimal> {
        let avg_price = self.avg_fill_price()?;

        if reference_price.is_zero() {
            return None;
        }

        let slippage = match self.side {
            Side::Buy => (avg_price - reference_price) / reference_price,
            Side::Sell => (reference_price - avg_price) / reference_price,
        };

        Some(slippage * dec!(10000))
    }

    /// Time from submission to first fill
    pub fn time_to_first_fill(&self) -> Option<Duration> {
        match (self.submission_time, self.first_fill_time) {
            (Some(sub), Some(first)) => Some(first.duration_since(sub)),
            _ => None,
        }
    }

    /// Time from submission to completion
    pub fn time_to_complete(&self) -> Option<Duration> {
        match (self.submission_time, self.completion_time) {
            (Some(sub), Some(comp)) => Some(comp.duration_since(sub)),
            _ => None,
        }
    }

    /// Time order was active in the market
    pub fn active_duration(&self) -> Option<Duration> {
        let end = self.completion_time.or(Some(Instant::now()))?;
        self.submission_time.map(|start| end.duration_since(start))
    }

    /// Number of fills
    pub fn fill_count(&self) -> usize {
        self.fills.len()
    }

    /// Check if order has any fills
    pub fn has_fills(&self) -> bool {
        !self.fills.is_empty()
    }

    /// Update order state from execution data
    #[instrument(skip(self, exec))]
    pub fn apply_execution(&mut self, exec: &ExecutionData) {
        let now = chrono::Utc::now().to_rfc3339();
        self.updated_at = now.clone();

        // Update order ID if we get it
        if self.order_id.is_none() && !exec.order_id.is_empty() {
            self.order_id = Some(exec.order_id.clone());
            debug!(order_id = %exec.order_id, "Order ID assigned");
        }

        // Update cumulative filled quantity
        if let Some(cum_qty) = exec.cum_qty {
            self.filled_qty = cum_qty;
        }

        // Track fees
        if let Some(fee) = exec.fee_paid {
            self.total_fees = fee;
        }
        if exec.fee_currency.is_some() {
            self.fee_currency = exec.fee_currency.clone();
        }

        // Add fill if this is a trade execution
        if let (Some(last_price), Some(last_qty)) = (exec.last_price, exec.last_qty) {
            let is_first_fill = self.fills.is_empty();

            let fill = Fill {
                exec_id: exec.exec_id.clone(),
                price: last_price,
                qty: last_qty,
                fee: exec.fee_paid.unwrap_or(Decimal::ZERO),
                fee_currency: exec.fee_currency.clone(),
                timestamp: exec.timestamp.clone(),
                latency: self.submission_time.map(|t| t.elapsed()),
            };
            self.fills.push(fill);

            // Track first fill time
            if is_first_fill {
                self.first_fill_time = Some(Instant::now());
                debug!("First fill recorded");
            }
        }

        // Update lifecycle state
        if let Some(ref status) = exec.order_status {
            let new_state = LifecycleState::from_kraken_status(status);

            // Track state transition
            if self.lifecycle_state != new_state {
                debug!(
                    old_state = %self.lifecycle_state,
                    new_state = %new_state,
                    "Lifecycle state transition"
                );
                self.lifecycle_state = new_state;

                // Track completion time
                if new_state.is_terminal() {
                    self.completion_time = Some(Instant::now());
                }
            }
        }

        // Set cancel/reject reason based on exec_type if terminal
        if self.lifecycle_state == LifecycleState::Canceled && self.cancel_reason.is_none() {
            self.cancel_reason = Some(exec.exec_type.clone());
        } else if self.lifecycle_state == LifecycleState::Rejected && self.reject_reason.is_none() {
            self.reject_reason = Some(exec.exec_type.clone());
        }
    }
}

/// Configuration for the order tracker
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Maximum number of completed orders to keep in history
    pub max_history: usize,
    /// Whether to track timing metrics
    pub track_timing: bool,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            max_history: 1000,
            track_timing: true,
        }
    }
}

/// Order lifecycle tracker
///
/// Provides complete order lifecycle management including correlation,
/// state tracking, fill aggregation, and timing metrics.
#[derive(Debug)]
pub struct OrderTracker {
    /// Orders by Kraken order ID
    orders_by_id: HashMap<String, LifecycleOrder>,
    /// Orders by request ID (for correlation before order_id is assigned)
    orders_by_request_id: HashMap<String, String>, // request_id -> order_id
    /// Pending orders (no order_id yet)
    pending_orders: HashMap<String, LifecycleOrder>, // request_id -> order
    /// Configuration
    #[allow(dead_code)]
    config: TrackerConfig,
    /// Order count for statistics
    stats: TrackerStats,
}

/// Tracker statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TrackerStats {
    /// Total orders tracked
    pub total_tracked: u64,
    /// Currently active orders
    pub active_orders: u64,
    /// Filled orders
    pub filled_count: u64,
    /// Canceled orders
    pub canceled_count: u64,
    /// Rejected orders
    pub rejected_count: u64,
    /// Total fills processed
    pub total_fills: u64,
}

impl Default for OrderTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderTracker {
    /// Create a new order tracker with default config
    pub fn new() -> Self {
        Self::with_config(TrackerConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: TrackerConfig) -> Self {
        Self {
            orders_by_id: HashMap::new(),
            orders_by_request_id: HashMap::new(),
            pending_orders: HashMap::new(),
            config,
            stats: TrackerStats::default(),
        }
    }

    /// Track a new order submission
    #[instrument(skip(self))]
    pub fn track_submission(
        &mut self,
        request_id: &str,
        symbol: &str,
        side: Side,
        qty: Decimal,
        limit_price: Option<Decimal>,
    ) -> &LifecycleOrder {
        let order = LifecycleOrder::new_pending(
            Some(request_id.to_string()),
            symbol.to_string(),
            side,
            qty,
            limit_price,
        );

        self.stats.total_tracked += 1;
        self.stats.active_orders += 1;

        self.pending_orders.insert(request_id.to_string(), order);
        self.pending_orders.get(request_id).unwrap()
    }

    /// Handle execution event from WebSocket
    #[instrument(skip(self, exec))]
    pub fn handle_execution(&mut self, exec: &ExecutionData) -> Option<&LifecycleOrder> {
        let order_id = &exec.order_id;

        // Try to find existing order
        if self.orders_by_id.contains_key(order_id) {
            let order = self.orders_by_id.get_mut(order_id).unwrap();
            let was_active = order.lifecycle_state.is_active();
            order.apply_execution(exec);

            // Update statistics
            if was_active && !order.lifecycle_state.is_active() {
                self.stats.active_orders = self.stats.active_orders.saturating_sub(1);
                match order.lifecycle_state {
                    LifecycleState::Filled => self.stats.filled_count += 1,
                    LifecycleState::Canceled => self.stats.canceled_count += 1,
                    LifecycleState::Rejected => self.stats.rejected_count += 1,
                    _ => {}
                }
            }
            if exec.last_qty.is_some() {
                self.stats.total_fills += 1;
            }

            return self.orders_by_id.get(order_id);
        }

        // Try to correlate with pending order by symbol/side
        // This is a simplified correlation - in practice, use user_ref (cl_ord_id)
        // Find matching pending order by symbol and side
        let matching_key = self
            .pending_orders
            .iter()
            .find(|(_, o)| o.symbol == exec.symbol && o.side == exec.side)
            .map(|(k, _)| k.clone());

        if let Some(req_id) = matching_key {
            if let Some(mut pending) = self.pending_orders.remove(&req_id) {
                pending.order_id = Some(order_id.clone());
                pending.apply_execution(exec);

                self.orders_by_request_id.insert(req_id, order_id.clone());

                if exec.last_qty.is_some() {
                    self.stats.total_fills += 1;
                }
                if pending.lifecycle_state.is_terminal() {
                    self.stats.active_orders = self.stats.active_orders.saturating_sub(1);
                    match pending.lifecycle_state {
                        LifecycleState::Filled => self.stats.filled_count += 1,
                        LifecycleState::Canceled => self.stats.canceled_count += 1,
                        LifecycleState::Rejected => self.stats.rejected_count += 1,
                        _ => {}
                    }
                }

                self.orders_by_id.insert(order_id.clone(), pending);
                return self.orders_by_id.get(order_id);
            }
        }

        // New order we haven't seen before (e.g., from another session)
        let mut order = LifecycleOrder::new_pending(
            None,  // No request ID from execution data
            exec.symbol.clone(),
            exec.side,
            exec.order_qty.unwrap_or(Decimal::ZERO),
            exec.limit_price,
        );
        order.order_id = Some(order_id.clone());
        order.apply_execution(exec);

        self.stats.total_tracked += 1;
        if order.lifecycle_state.is_active() {
            self.stats.active_orders += 1;
        } else {
            match order.lifecycle_state {
                LifecycleState::Filled => self.stats.filled_count += 1,
                LifecycleState::Canceled => self.stats.canceled_count += 1,
                LifecycleState::Rejected => self.stats.rejected_count += 1,
                _ => {}
            }
        }
        if exec.last_qty.is_some() {
            self.stats.total_fills += 1;
        }

        self.orders_by_id.insert(order_id.clone(), order);
        self.orders_by_id.get(order_id)
    }

    // =========================================================================
    // Query API
    // =========================================================================

    /// Get order by Kraken order ID
    pub fn get(&self, order_id: &str) -> Option<&LifecycleOrder> {
        self.orders_by_id.get(order_id)
    }

    /// Get order by request ID
    pub fn get_by_request_id(&self, request_id: &str) -> Option<&LifecycleOrder> {
        // Check pending orders first
        if let Some(order) = self.pending_orders.get(request_id) {
            return Some(order);
        }

        // Check correlated orders
        self.orders_by_request_id
            .get(request_id)
            .and_then(|id| self.orders_by_id.get(id))
    }

    /// Get all orders by lifecycle state
    pub fn by_state(&self, state: LifecycleState) -> Vec<&LifecycleOrder> {
        self.orders_by_id
            .values()
            .filter(|o| o.lifecycle_state == state)
            .collect()
    }

    /// Get all active orders
    pub fn active_orders(&self) -> Vec<&LifecycleOrder> {
        let mut active: Vec<_> = self
            .orders_by_id
            .values()
            .filter(|o| o.lifecycle_state.is_active())
            .collect();

        // Include pending orders
        active.extend(self.pending_orders.values());
        active
    }

    /// Get all orders for a symbol
    pub fn by_symbol(&self, symbol: &str) -> Vec<&LifecycleOrder> {
        self.orders_by_id
            .values()
            .filter(|o| o.symbol == symbol)
            .collect()
    }

    /// Get all orders for a side
    pub fn by_side(&self, side: Side) -> Vec<&LifecycleOrder> {
        self.orders_by_id
            .values()
            .filter(|o| o.side == side)
            .collect()
    }

    /// Get orders matching custom predicate
    pub fn filter<F>(&self, predicate: F) -> Vec<&LifecycleOrder>
    where
        F: Fn(&LifecycleOrder) -> bool,
    {
        self.orders_by_id.values().filter(|o| predicate(o)).collect()
    }

    /// Get tracker statistics
    pub fn stats(&self) -> &TrackerStats {
        &self.stats
    }

    /// Get count of orders by state
    pub fn count_by_state(&self) -> HashMap<LifecycleState, usize> {
        let mut counts = HashMap::new();
        for order in self.orders_by_id.values() {
            *counts.entry(order.lifecycle_state).or_insert(0) += 1;
        }
        // Include pending orders
        *counts.entry(LifecycleState::Pending).or_insert(0) += self.pending_orders.len();
        counts
    }

    /// Calculate aggregate fill statistics for completed orders
    pub fn fill_stats(&self) -> FillStatistics {
        let completed: Vec<_> = self
            .orders_by_id
            .values()
            .filter(|o| o.lifecycle_state == LifecycleState::Filled)
            .collect();

        if completed.is_empty() {
            return FillStatistics::default();
        }

        let times_to_first: Vec<Duration> = completed
            .iter()
            .filter_map(|o| o.time_to_first_fill())
            .collect();

        let times_to_complete: Vec<Duration> = completed
            .iter()
            .filter_map(|o| o.time_to_complete())
            .collect();

        let slippages: Vec<Decimal> = completed
            .iter()
            .filter_map(|o| o.slippage_bps())
            .collect();

        let fill_counts: Vec<usize> = completed.iter().map(|o| o.fill_count()).collect();

        FillStatistics {
            order_count: completed.len(),
            avg_time_to_first_fill: average_duration(&times_to_first),
            avg_time_to_complete: average_duration(&times_to_complete),
            avg_slippage_bps: average_decimal(&slippages),
            avg_fills_per_order: fill_counts.iter().sum::<usize>() as f64 / completed.len() as f64,
            total_fills: fill_counts.iter().sum(),
        }
    }

    /// Clear completed orders (keep only active)
    pub fn clear_completed(&mut self) {
        self.orders_by_id.retain(|_, o| o.lifecycle_state.is_active());
        // Also clean up correlation map
        self.orders_by_request_id
            .retain(|_, id| self.orders_by_id.contains_key(id));
    }

    /// Clear all tracked orders
    pub fn clear(&mut self) {
        self.orders_by_id.clear();
        self.orders_by_request_id.clear();
        self.pending_orders.clear();
        self.stats = TrackerStats::default();
    }
}

/// Aggregate fill statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FillStatistics {
    /// Number of completed orders analyzed
    pub order_count: usize,
    /// Average time to first fill
    #[serde(skip)]
    pub avg_time_to_first_fill: Option<Duration>,
    /// Average time to complete fill
    #[serde(skip)]
    pub avg_time_to_complete: Option<Duration>,
    /// Average slippage in basis points
    pub avg_slippage_bps: Option<Decimal>,
    /// Average number of fills per order
    pub avg_fills_per_order: f64,
    /// Total fills across all orders
    pub total_fills: usize,
}

// Helper functions
fn average_duration(durations: &[Duration]) -> Option<Duration> {
    if durations.is_empty() {
        return None;
    }
    let total: Duration = durations.iter().sum();
    Some(total / durations.len() as u32)
}

fn average_decimal(values: &[Decimal]) -> Option<Decimal> {
    if values.is_empty() {
        return None;
    }
    let total: Decimal = values.iter().sum();
    Some(total / Decimal::from(values.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_lifecycle_states() {
        // State parsing
        assert_eq!(LifecycleState::from_kraken_status("new"), LifecycleState::New);
        assert_eq!(LifecycleState::from_kraken_status("filled"), LifecycleState::Filled);
        assert_eq!(LifecycleState::from_kraken_status("canceled"), LifecycleState::Canceled);

        // Active vs terminal
        assert!(LifecycleState::New.is_active());
        assert!(LifecycleState::Filled.is_terminal());
        assert!(!LifecycleState::New.is_terminal());
    }

    #[test]
    fn test_order_tracking() {
        let mut tracker = OrderTracker::new();
        tracker.track_submission("req1", "BTC/USD", Side::Buy, dec!(10), Some(dec!(100)));

        assert_eq!(tracker.stats().total_tracked, 1);
        assert_eq!(tracker.stats().active_orders, 1);

        let order = tracker.get_by_request_id("req1").unwrap();
        assert_eq!(order.symbol, "BTC/USD");
        assert_eq!(order.lifecycle_state, LifecycleState::Pending);
    }

    #[test]
    fn test_fill_calculations() {
        let mut order = LifecycleOrder::new_pending(
            Some("req1".to_string()),
            "BTC/USD".to_string(),
            Side::Buy,
            dec!(10),
            Some(dec!(100)),
        );

        // Add fills: 5 @ 100, 5 @ 102
        order.fills.push(Fill {
            exec_id: Some("e1".to_string()),
            price: dec!(100),
            qty: dec!(5),
            fee: Decimal::ZERO,
            fee_currency: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            latency: None,
        });
        order.fills.push(Fill {
            exec_id: Some("e2".to_string()),
            price: dec!(102),
            qty: dec!(5),
            fee: Decimal::ZERO,
            fee_currency: None,
            timestamp: "2024-01-01T00:00:01Z".to_string(),
            latency: None,
        });
        order.filled_qty = dec!(10);

        // Average: (500 + 510) / 10 = 101
        assert_eq!(order.avg_fill_price().unwrap(), dec!(101));
        // Slippage: (101 - 100) / 100 * 10000 = 100bp
        assert_eq!(order.slippage_bps().unwrap(), dec!(100));
    }
}
