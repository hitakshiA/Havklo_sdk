//! WASM bindings for Kraken orderbook engine
//!
//! This crate provides JavaScript-friendly bindings for the orderbook engine,
//! enabling browser-based orderbook visualization and time-travel features.
//!
//! # Usage (JavaScript)
//!
//! ```javascript
//! import init, { WasmOrderbook } from 'kraken-wasm';
//!
//! await init();
//!
//! const book = new WasmOrderbook('BTC/USD');
//! book.enable_history(100);
//!
//! ws.onmessage = (event) => {
//!     try {
//!         book.apply_message(event.data);
//!         console.log('Spread:', book.get_spread());
//!         console.log('Bids:', book.get_bids());
//!     } catch (e) {
//!         console.error('Orderbook error:', e);
//!     }
//! };
//! ```

use kraken_book::{HistoryBuffer, Orderbook, OrderbookState, L3Book, L3Order, L3Side};
use kraken_types::WsMessage;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use wasm_bindgen::prelude::*;

/// Initialize panic hook for better error messages in browser console
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// WASM-compatible orderbook wrapper
///
/// Provides a JavaScript-friendly API for managing orderbook state.
#[wasm_bindgen]
pub struct WasmOrderbook {
    inner: Orderbook,
    history: Option<HistoryBuffer>,
}

#[wasm_bindgen]
impl WasmOrderbook {
    /// Create a new orderbook for a trading pair
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USD")
    #[wasm_bindgen(constructor)]
    pub fn new(symbol: &str) -> WasmOrderbook {
        WasmOrderbook {
            inner: Orderbook::new(symbol),
            history: None,
        }
    }

    /// Create a new orderbook with specific depth
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol
    /// * `depth` - Orderbook depth (10, 25, 100, 500, or 1000)
    #[wasm_bindgen]
    pub fn with_depth(symbol: &str, depth: u32) -> WasmOrderbook {
        WasmOrderbook {
            inner: Orderbook::with_depth(symbol, depth),
            history: None,
        }
    }

    /// Apply a raw JSON message from the WebSocket
    ///
    /// Browser calls this with `event.data` from `ws.onmessage`.
    /// Returns the message type: "snapshot", "update", "ignored", or throws on error.
    #[wasm_bindgen]
    pub fn apply_message(&mut self, json: &str) -> Result<String, JsValue> {
        let msg = WsMessage::parse(json).map_err(|e| JsValue::from_str(&e.to_string()))?;

        match msg {
            WsMessage::Book(book_msg) => {
                if let Some(data) = book_msg.data.first() {
                    let is_snapshot = book_msg.msg_type == "snapshot";
                    let result = self
                        .inner
                        .apply_book_data(data, is_snapshot)
                        .map_err(|e| JsValue::from_str(&e.to_string()))?;

                    // Save to history if enabled
                    if let Some(history) = &mut self.history {
                        history.push(self.inner.snapshot());
                    }

                    match result {
                        kraken_book::ApplyResult::Snapshot => Ok("snapshot".to_string()),
                        kraken_book::ApplyResult::Update => Ok("update".to_string()),
                        kraken_book::ApplyResult::Ignored => Ok("ignored".to_string()),
                    }
                } else {
                    Ok("ignored".to_string())
                }
            }
            _ => Ok("ignored".to_string()),
        }
    }

    /// Get the trading pair symbol
    #[wasm_bindgen]
    pub fn get_symbol(&self) -> String {
        self.inner.symbol().to_string()
    }

    /// Check if the orderbook is synchronized
    #[wasm_bindgen]
    pub fn is_synced(&self) -> bool {
        self.inner.is_synced()
    }

    /// Get the current state as a string
    #[wasm_bindgen]
    pub fn get_state(&self) -> String {
        match self.inner.state() {
            OrderbookState::Uninitialized => "uninitialized".to_string(),
            OrderbookState::AwaitingSnapshot => "awaiting_snapshot".to_string(),
            OrderbookState::Synced => "synced".to_string(),
            OrderbookState::Desynchronized => "desynchronized".to_string(),
        }
    }

    /// Get all bids as a JavaScript array
    ///
    /// Returns array of objects: `[{price: number, qty: number}, ...]`
    #[wasm_bindgen]
    pub fn get_bids(&self) -> JsValue {
        let bids: Vec<JsLevel> = self
            .inner
            .bids_vec()
            .iter()
            .map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&bids).unwrap_or(JsValue::NULL)
    }

    /// Get all asks as a JavaScript array
    ///
    /// Returns array of objects: `[{price: number, qty: number}, ...]`
    #[wasm_bindgen]
    pub fn get_asks(&self) -> JsValue {
        let asks: Vec<JsLevel> = self
            .inner
            .asks_vec()
            .iter()
            .map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&asks).unwrap_or(JsValue::NULL)
    }

    /// Get top N bids
    #[wasm_bindgen]
    pub fn get_top_bids(&self, n: u32) -> JsValue {
        let bids: Vec<JsLevel> = self
            .inner
            .top_bids(n as usize)
            .iter()
            .map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&bids).unwrap_or(JsValue::NULL)
    }

    /// Get top N asks
    #[wasm_bindgen]
    pub fn get_top_asks(&self, n: u32) -> JsValue {
        let asks: Vec<JsLevel> = self
            .inner
            .top_asks(n as usize)
            .iter()
            .map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&asks).unwrap_or(JsValue::NULL)
    }

    /// Get the spread (ask - bid) as a number
    ///
    /// Returns 0 if either side is empty.
    #[wasm_bindgen]
    pub fn get_spread(&self) -> f64 {
        self.inner
            .spread()
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0)
    }

    /// Get the mid price ((ask + bid) / 2)
    ///
    /// Returns 0 if either side is empty.
    #[wasm_bindgen]
    pub fn get_mid_price(&self) -> f64 {
        self.inner
            .mid_price()
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0)
    }

    /// Get the best bid price
    #[wasm_bindgen]
    pub fn get_best_bid(&self) -> f64 {
        self.inner
            .best_bid()
            .map(|l| l.price_f64())
            .unwrap_or(0.0)
    }

    /// Get the best ask price
    #[wasm_bindgen]
    pub fn get_best_ask(&self) -> f64 {
        self.inner
            .best_ask()
            .map(|l| l.price_f64())
            .unwrap_or(0.0)
    }

    /// Get the last validated checksum
    #[wasm_bindgen]
    pub fn get_checksum(&self) -> u32 {
        self.inner.last_checksum()
    }

    /// Get the number of bid levels
    #[wasm_bindgen]
    pub fn get_bid_count(&self) -> u32 {
        self.inner.bid_count() as u32
    }

    /// Get the number of ask levels
    #[wasm_bindgen]
    pub fn get_ask_count(&self) -> u32 {
        self.inner.ask_count() as u32
    }

    /// Reset the orderbook to uninitialized state
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Set precision for checksum calculation
    ///
    /// Each trading pair has specific precision values for price and quantity.
    /// This must be set correctly for checksum validation to work.
    ///
    /// # Arguments
    /// * `price_precision` - Decimal places for prices (e.g., 1 for BTC/USD, 2 for ETH/USD)
    /// * `qty_precision` - Decimal places for quantities (usually 8)
    #[wasm_bindgen]
    pub fn set_precision(&mut self, price_precision: u8, qty_precision: u8) {
        self.inner.set_precision(price_precision, qty_precision);
    }

    // ========== History/Time-Travel Features ==========

    /// Enable history tracking for time-travel feature
    ///
    /// # Arguments
    /// * `max_snapshots` - Maximum number of snapshots to retain
    #[wasm_bindgen]
    pub fn enable_history(&mut self, max_snapshots: u32) {
        self.history = Some(HistoryBuffer::new(max_snapshots as usize));
    }

    /// Disable history tracking
    #[wasm_bindgen]
    pub fn disable_history(&mut self) {
        self.history = None;
    }

    /// Check if history is enabled
    #[wasm_bindgen]
    pub fn is_history_enabled(&self) -> bool {
        self.history.is_some()
    }

    /// Get the number of stored history snapshots
    #[wasm_bindgen]
    pub fn get_history_length(&self) -> u32 {
        self.history.as_ref().map(|h| h.len() as u32).unwrap_or(0)
    }

    /// Get a historical snapshot by index (0 = oldest)
    ///
    /// Returns an object with bids, asks, and checksum, or null if not found.
    #[wasm_bindgen]
    pub fn get_snapshot_at(&self, index: u32) -> JsValue {
        self.history
            .as_ref()
            .and_then(|h| h.get(index as usize))
            .map(|entry| {
                let snapshot = JsSnapshot {
                    sequence: entry.sequence,
                    bids: entry
                        .snapshot
                        .bids
                        .iter()
                        .map(|l| JsLevel {
                            price: l.price_f64(),
                            qty: l.qty_f64(),
                        })
                        .collect(),
                    asks: entry
                        .snapshot
                        .asks
                        .iter()
                        .map(|l| JsLevel {
                            price: l.price_f64(),
                            qty: l.qty_f64(),
                        })
                        .collect(),
                    checksum: entry.snapshot.checksum,
                };
                serde_wasm_bindgen::to_value(&snapshot).unwrap_or(JsValue::NULL)
            })
            .unwrap_or(JsValue::NULL)
    }

    /// Get the latest history sequence number
    #[wasm_bindgen]
    pub fn get_latest_sequence(&self) -> u64 {
        self.history
            .as_ref()
            .and_then(|h| h.last_sequence())
            .unwrap_or(0)
    }

    /// Clear history buffer
    #[wasm_bindgen]
    pub fn clear_history(&mut self) {
        if let Some(history) = &mut self.history {
            history.clear();
        }
    }
}

/// JavaScript-friendly price level
#[derive(serde::Serialize)]
struct JsLevel {
    price: f64,
    qty: f64,
}

/// JavaScript-friendly snapshot
#[derive(serde::Serialize)]
struct JsSnapshot {
    sequence: u64,
    bids: Vec<JsLevel>,
    asks: Vec<JsLevel>,
    checksum: u32,
}

// ============================================================================
// L3 Orderbook WASM Bindings
// ============================================================================

/// WASM-compatible L3 orderbook wrapper
///
/// Provides Level 3 (order-level) orderbook functionality for JavaScript,
/// enabling individual order tracking, queue position calculation, and
/// advanced market making features.
///
/// # Usage (JavaScript)
///
/// ```javascript
/// import init, { WasmL3Book } from 'kraken-wasm';
///
/// await init();
///
/// const book = new WasmL3Book('BTC/USD', 100);
///
/// // Add orders
/// book.add_order('order1', 50000.0, 1.5, 'bid');
/// book.add_order('order2', 50001.0, 2.0, 'ask');
///
/// // Check queue position
/// const pos = book.get_queue_position('order1');
/// console.log('Position:', pos.position, 'Qty ahead:', pos.qty_ahead);
///
/// // Get aggregated view (L2 format)
/// console.log('Top bids:', book.get_aggregated_bids(10));
/// ```
#[wasm_bindgen]
pub struct WasmL3Book {
    inner: L3Book,
}

#[wasm_bindgen]
impl WasmL3Book {
    /// Create a new L3 orderbook
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTC/USD")
    /// * `depth` - Maximum depth (10, 100, or 1000)
    #[wasm_bindgen(constructor)]
    pub fn new(symbol: &str, depth: u32) -> WasmL3Book {
        WasmL3Book {
            inner: L3Book::new(symbol, depth),
        }
    }

    /// Get the trading pair symbol
    #[wasm_bindgen]
    pub fn get_symbol(&self) -> String {
        self.inner.symbol().to_string()
    }

    /// Get the maximum depth
    #[wasm_bindgen]
    pub fn get_depth(&self) -> u32 {
        self.inner.depth()
    }

    /// Set precision for checksum calculation
    #[wasm_bindgen]
    pub fn set_precision(&mut self, price_precision: u8, qty_precision: u8) {
        self.inner.set_precision(price_precision, qty_precision);
    }

    // ========== Order Operations ==========

    /// Add a new order to the book
    ///
    /// # Arguments
    /// * `order_id` - Unique order identifier
    /// * `price` - Order price
    /// * `qty` - Order quantity
    /// * `side` - "bid" or "ask"
    ///
    /// Returns true if added, false if order already exists
    #[wasm_bindgen]
    pub fn add_order(&mut self, order_id: &str, price: f64, qty: f64, side: &str) -> bool {
        let side = match side.to_lowercase().as_str() {
            "bid" | "buy" => L3Side::Bid,
            "ask" | "sell" => L3Side::Ask,
            _ => return false,
        };

        let order = L3Order::new(
            order_id,
            Decimal::try_from(price).unwrap_or(Decimal::ZERO),
            Decimal::try_from(qty).unwrap_or(Decimal::ZERO),
        );

        self.inner.add_order(order, side)
    }

    /// Add a new order with full metadata
    ///
    /// # Arguments
    /// * `order_id` - Unique order identifier
    /// * `price` - Order price
    /// * `qty` - Order quantity
    /// * `side` - "bid" or "ask"
    /// * `timestamp` - Microseconds since epoch
    /// * `sequence` - Sequence number for ordering
    #[wasm_bindgen]
    pub fn add_order_with_metadata(
        &mut self,
        order_id: &str,
        price: f64,
        qty: f64,
        side: &str,
        timestamp: u64,
        sequence: u64,
    ) -> bool {
        let side = match side.to_lowercase().as_str() {
            "bid" | "buy" => L3Side::Bid,
            "ask" | "sell" => L3Side::Ask,
            _ => return false,
        };

        let order = L3Order::with_metadata(
            order_id,
            Decimal::try_from(price).unwrap_or(Decimal::ZERO),
            Decimal::try_from(qty).unwrap_or(Decimal::ZERO),
            timestamp,
            sequence,
        );

        self.inner.add_order(order, side)
    }

    /// Remove an order from the book
    ///
    /// Returns the removed order as a JS object, or null if not found
    #[wasm_bindgen]
    pub fn remove_order(&mut self, order_id: &str) -> JsValue {
        self.inner
            .remove_order(order_id)
            .map(|order| {
                let js_order = JsL3Order::from_order(&order, self.inner.order_side(order_id));
                serde_wasm_bindgen::to_value(&js_order).unwrap_or(JsValue::NULL)
            })
            .unwrap_or(JsValue::NULL)
    }

    /// Modify an order's quantity
    ///
    /// Returns true if modified, false if order not found
    #[wasm_bindgen]
    pub fn modify_order(&mut self, order_id: &str, new_qty: f64) -> bool {
        self.inner.modify_order(
            order_id,
            Decimal::try_from(new_qty).unwrap_or(Decimal::ZERO),
        )
    }

    /// Check if an order exists
    #[wasm_bindgen]
    pub fn has_order(&self, order_id: &str) -> bool {
        self.inner.has_order(order_id)
    }

    /// Get an order by ID
    ///
    /// Returns the order as a JS object, or null if not found
    #[wasm_bindgen]
    pub fn get_order(&self, order_id: &str) -> JsValue {
        self.inner
            .get_order(order_id)
            .map(|order| {
                let side = self.inner.order_side(order_id);
                let js_order = JsL3Order::from_order(order, side);
                serde_wasm_bindgen::to_value(&js_order).unwrap_or(JsValue::NULL)
            })
            .unwrap_or(JsValue::NULL)
    }

    /// Get the side of an order
    ///
    /// Returns "bid", "ask", or null if not found
    #[wasm_bindgen]
    pub fn get_order_side(&self, order_id: &str) -> JsValue {
        self.inner
            .order_side(order_id)
            .map(|side| JsValue::from_str(match side {
                L3Side::Bid => "bid",
                L3Side::Ask => "ask",
            }))
            .unwrap_or(JsValue::NULL)
    }

    // ========== Queue Position ==========

    /// Get the queue position for an order
    ///
    /// Returns an object with position info, or null if order not found:
    /// ```javascript
    /// {
    ///   position: number,      // 0-indexed position in queue
    ///   orders_ahead: number,  // Same as position
    ///   qty_ahead: number,     // Total quantity ahead
    ///   total_orders: number,  // Total orders at this level
    ///   total_qty: number,     // Total quantity at this level
    ///   fill_probability: number // 0.0 to 1.0
    /// }
    /// ```
    #[wasm_bindgen]
    pub fn get_queue_position(&self, order_id: &str) -> JsValue {
        self.inner
            .queue_position(order_id)
            .map(|pos| {
                let js_pos = JsQueuePosition {
                    position: pos.position as u32,
                    orders_ahead: pos.orders_ahead as u32,
                    qty_ahead: pos.qty_ahead.to_f64().unwrap_or(0.0),
                    total_orders: pos.total_orders as u32,
                    total_qty: pos.total_qty.to_f64().unwrap_or(0.0),
                    fill_probability: pos.fill_probability(),
                };
                serde_wasm_bindgen::to_value(&js_pos).unwrap_or(JsValue::NULL)
            })
            .unwrap_or(JsValue::NULL)
    }

    // ========== Book Queries ==========

    /// Get the best bid price
    #[wasm_bindgen]
    pub fn get_best_bid(&self) -> f64 {
        self.inner
            .best_bid_price()
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0)
    }

    /// Get the best ask price
    #[wasm_bindgen]
    pub fn get_best_ask(&self) -> f64 {
        self.inner
            .best_ask_price()
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0)
    }

    /// Get the spread
    #[wasm_bindgen]
    pub fn get_spread(&self) -> f64 {
        self.inner
            .spread()
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0)
    }

    /// Get the mid price
    #[wasm_bindgen]
    pub fn get_mid_price(&self) -> f64 {
        self.inner
            .mid_price()
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0)
    }

    /// Get the number of bid levels
    #[wasm_bindgen]
    pub fn get_bid_level_count(&self) -> u32 {
        self.inner.bid_level_count() as u32
    }

    /// Get the number of ask levels
    #[wasm_bindgen]
    pub fn get_ask_level_count(&self) -> u32 {
        self.inner.ask_level_count() as u32
    }

    /// Get the total number of orders in the book
    #[wasm_bindgen]
    pub fn get_order_count(&self) -> u32 {
        self.inner.order_count() as u32
    }

    /// Check if the book is empty
    #[wasm_bindgen]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    // ========== Aggregated Views (L2 Format) ==========

    /// Get all bid levels aggregated (L2 format)
    ///
    /// Returns array of `[{price: number, qty: number}, ...]`
    #[wasm_bindgen]
    pub fn get_aggregated_bids(&self) -> JsValue {
        let levels: Vec<JsLevel> = self
            .inner
            .aggregated_bids()
            .iter()
            .map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&levels).unwrap_or(JsValue::NULL)
    }

    /// Get all ask levels aggregated (L2 format)
    #[wasm_bindgen]
    pub fn get_aggregated_asks(&self) -> JsValue {
        let levels: Vec<JsLevel> = self
            .inner
            .aggregated_asks()
            .iter()
            .map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&levels).unwrap_or(JsValue::NULL)
    }

    /// Get top N aggregated bid levels
    #[wasm_bindgen]
    pub fn get_top_aggregated_bids(&self, n: u32) -> JsValue {
        let levels: Vec<JsLevel> = self
            .inner
            .top_aggregated_bids(n as usize)
            .iter()
            .map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&levels).unwrap_or(JsValue::NULL)
    }

    /// Get top N aggregated ask levels
    #[wasm_bindgen]
    pub fn get_top_aggregated_asks(&self, n: u32) -> JsValue {
        let levels: Vec<JsLevel> = self
            .inner
            .top_aggregated_asks(n as usize)
            .iter()
            .map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            })
            .collect();
        serde_wasm_bindgen::to_value(&levels).unwrap_or(JsValue::NULL)
    }

    // ========== L3 Level Details ==========

    /// Get all orders at the best bid level
    #[wasm_bindgen]
    pub fn get_best_bid_orders(&self) -> JsValue {
        self.inner
            .best_bid()
            .map(|level| {
                let orders: Vec<JsL3Order> = level
                    .orders()
                    .map(|o| JsL3Order::from_order(o, Some(L3Side::Bid)))
                    .collect();
                serde_wasm_bindgen::to_value(&orders).unwrap_or(JsValue::NULL)
            })
            .unwrap_or(JsValue::NULL)
    }

    /// Get all orders at the best ask level
    #[wasm_bindgen]
    pub fn get_best_ask_orders(&self) -> JsValue {
        self.inner
            .best_ask()
            .map(|level| {
                let orders: Vec<JsL3Order> = level
                    .orders()
                    .map(|o| JsL3Order::from_order(o, Some(L3Side::Ask)))
                    .collect();
                serde_wasm_bindgen::to_value(&orders).unwrap_or(JsValue::NULL)
            })
            .unwrap_or(JsValue::NULL)
    }

    // ========== Analytics ==========

    /// Get total bid quantity
    #[wasm_bindgen]
    pub fn get_total_bid_qty(&self) -> f64 {
        self.inner.total_bid_qty().to_f64().unwrap_or(0.0)
    }

    /// Get total ask quantity
    #[wasm_bindgen]
    pub fn get_total_ask_qty(&self) -> f64 {
        self.inner.total_ask_qty().to_f64().unwrap_or(0.0)
    }

    /// Get the bid/ask imbalance ratio
    ///
    /// Returns a value between -1.0 (all asks) and 1.0 (all bids)
    #[wasm_bindgen]
    pub fn get_imbalance(&self) -> f64 {
        self.inner.imbalance().unwrap_or(0.0)
    }

    /// Get VWAP for buying a quantity
    ///
    /// Returns the volume-weighted average price to buy the given quantity
    #[wasm_bindgen]
    pub fn get_vwap_ask(&self, qty: f64) -> f64 {
        self.inner
            .vwap_ask(Decimal::try_from(qty).unwrap_or(Decimal::ZERO))
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0)
    }

    /// Get VWAP for selling a quantity
    ///
    /// Returns the volume-weighted average price to sell the given quantity
    #[wasm_bindgen]
    pub fn get_vwap_bid(&self, qty: f64) -> f64 {
        self.inner
            .vwap_bid(Decimal::try_from(qty).unwrap_or(Decimal::ZERO))
            .and_then(|d| d.to_f64())
            .unwrap_or(0.0)
    }

    // ========== Checksum ==========

    /// Compute the checksum for the current book state
    #[wasm_bindgen]
    pub fn compute_checksum(&self) -> u32 {
        self.inner.compute_checksum()
    }

    /// Validate the book against an expected checksum
    ///
    /// Returns true if checksum matches, false otherwise
    #[wasm_bindgen]
    pub fn validate_checksum(&self, expected: u32) -> bool {
        self.inner.validate_checksum(expected).is_ok()
    }

    // ========== Book Management ==========

    /// Clear all orders and levels
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Truncate book to maximum depth
    ///
    /// Removes levels beyond the configured depth limit
    #[wasm_bindgen]
    pub fn truncate(&mut self) {
        self.inner.truncate();
    }

    /// Get the last processed sequence number
    #[wasm_bindgen]
    pub fn get_last_sequence(&self) -> u64 {
        self.inner.last_sequence()
    }

    /// Update the last sequence number
    #[wasm_bindgen]
    pub fn set_last_sequence(&mut self, seq: u64) {
        self.inner.set_last_sequence(seq);
    }

    /// Take a snapshot of the current book state
    ///
    /// Returns an object with aggregated levels and all orders
    #[wasm_bindgen]
    pub fn snapshot(&self) -> JsValue {
        let snap = self.inner.snapshot();
        let js_snap = JsL3Snapshot {
            symbol: snap.symbol,
            bids: snap.bids.iter().map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            }).collect(),
            asks: snap.asks.iter().map(|l| JsLevel {
                price: l.price_f64(),
                qty: l.qty_f64(),
            }).collect(),
            bid_orders: snap.bid_orders.iter().map(|o| JsL3Order::from_order(o, Some(L3Side::Bid))).collect(),
            ask_orders: snap.ask_orders.iter().map(|o| JsL3Order::from_order(o, Some(L3Side::Ask))).collect(),
            checksum: snap.checksum,
            sequence: snap.sequence,
        };
        serde_wasm_bindgen::to_value(&js_snap).unwrap_or(JsValue::NULL)
    }
}

// ============================================================================
// L3 JavaScript Types
// ============================================================================

/// JavaScript-friendly L3 order
#[derive(serde::Serialize)]
struct JsL3Order {
    order_id: String,
    price: f64,
    qty: f64,
    side: String,
    timestamp: u64,
    sequence: u64,
}

impl JsL3Order {
    fn from_order(order: &L3Order, side: Option<L3Side>) -> Self {
        Self {
            order_id: order.order_id.clone(),
            price: order.price.to_f64().unwrap_or(0.0),
            qty: order.qty.to_f64().unwrap_or(0.0),
            side: match side {
                Some(L3Side::Bid) => "bid".to_string(),
                Some(L3Side::Ask) => "ask".to_string(),
                None => "unknown".to_string(),
            },
            timestamp: order.timestamp,
            sequence: order.sequence,
        }
    }
}

/// JavaScript-friendly queue position
#[derive(serde::Serialize)]
struct JsQueuePosition {
    position: u32,
    orders_ahead: u32,
    qty_ahead: f64,
    total_orders: u32,
    total_qty: f64,
    fill_probability: f64,
}

/// JavaScript-friendly L3 snapshot
#[derive(serde::Serialize)]
struct JsL3Snapshot {
    symbol: String,
    bids: Vec<JsLevel>,
    asks: Vec<JsLevel>,
    bid_orders: Vec<JsL3Order>,
    ask_orders: Vec<JsL3Order>,
    checksum: u32,
    sequence: u64,
}

// ============================================================================
// REST Client WASM Bindings
// ============================================================================

use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// WASM-compatible REST client for Kraken public endpoints
///
/// Uses the browser's fetch API to make HTTP requests to Kraken's REST API.
/// Only public endpoints are supported (no authentication required).
///
/// # Usage (JavaScript)
///
/// ```javascript
/// import init, { WasmRestClient } from 'kraken-wasm';
///
/// await init();
///
/// const client = new WasmRestClient();
///
/// // Get ticker data
/// const ticker = await client.get_ticker('XBTUSD');
/// console.log('BTC price:', ticker.XXBTZUSD.c[0]);
///
/// // Get orderbook
/// const book = await client.get_orderbook('ETHUSD', 10);
/// console.log('ETH bids:', book.XETHZUSD.bids);
/// ```
#[wasm_bindgen]
pub struct WasmRestClient {
    base_url: String,
}

#[wasm_bindgen]
impl WasmRestClient {
    /// Create a new REST client
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmRestClient {
        WasmRestClient {
            base_url: "https://api.kraken.com".to_string(),
        }
    }

    /// Create a REST client with custom base URL (for testing)
    #[wasm_bindgen]
    pub fn with_base_url(base_url: &str) -> WasmRestClient {
        WasmRestClient {
            base_url: base_url.to_string(),
        }
    }

    /// Get the base URL
    #[wasm_bindgen]
    pub fn get_base_url(&self) -> String {
        self.base_url.clone()
    }

    // ========== Public Market Data Endpoints ==========

    /// Get server time
    ///
    /// Returns the server's current time
    #[wasm_bindgen]
    pub async fn get_server_time(&self) -> Result<JsValue, JsValue> {
        self.fetch_public("/0/public/Time").await
    }

    /// Get system status
    ///
    /// Returns the current system status (online, maintenance, etc.)
    #[wasm_bindgen]
    pub async fn get_system_status(&self) -> Result<JsValue, JsValue> {
        self.fetch_public("/0/public/SystemStatus").await
    }

    /// Get asset info
    ///
    /// Returns information about all available assets
    #[wasm_bindgen]
    pub async fn get_assets(&self) -> Result<JsValue, JsValue> {
        self.fetch_public("/0/public/Assets").await
    }

    /// Get specific asset info
    ///
    /// # Arguments
    /// * `asset` - Asset name (e.g., "XBT", "ETH")
    #[wasm_bindgen]
    pub async fn get_asset(&self, asset: &str) -> Result<JsValue, JsValue> {
        let url = format!("/0/public/Assets?asset={}", asset);
        self.fetch_public(&url).await
    }

    /// Get tradeable asset pairs
    ///
    /// Returns information about all tradeable pairs
    #[wasm_bindgen]
    pub async fn get_asset_pairs(&self) -> Result<JsValue, JsValue> {
        self.fetch_public("/0/public/AssetPairs").await
    }

    /// Get specific asset pair info
    ///
    /// # Arguments
    /// * `pair` - Trading pair (e.g., "XBTUSD", "ETHUSD")
    #[wasm_bindgen]
    pub async fn get_asset_pair(&self, pair: &str) -> Result<JsValue, JsValue> {
        let url = format!("/0/public/AssetPairs?pair={}", pair);
        self.fetch_public(&url).await
    }

    /// Get ticker information
    ///
    /// # Arguments
    /// * `pair` - Trading pair(s), comma-separated (e.g., "XBTUSD" or "XBTUSD,ETHUSD")
    #[wasm_bindgen]
    pub async fn get_ticker(&self, pair: &str) -> Result<JsValue, JsValue> {
        let url = format!("/0/public/Ticker?pair={}", pair);
        self.fetch_public(&url).await
    }

    /// Get OHLC data
    ///
    /// # Arguments
    /// * `pair` - Trading pair
    /// * `interval` - Time interval in minutes (1, 5, 15, 30, 60, 240, 1440, 10080, 21600)
    /// * `since` - Optional Unix timestamp to get data since
    #[wasm_bindgen]
    pub async fn get_ohlc(&self, pair: &str, interval: u32, since: Option<u64>) -> Result<JsValue, JsValue> {
        let mut url = format!("/0/public/OHLC?pair={}&interval={}", pair, interval);
        if let Some(s) = since {
            url.push_str(&format!("&since={}", s));
        }
        self.fetch_public(&url).await
    }

    /// Get orderbook
    ///
    /// # Arguments
    /// * `pair` - Trading pair
    /// * `count` - Maximum number of bids/asks (1-500)
    #[wasm_bindgen]
    pub async fn get_orderbook(&self, pair: &str, count: Option<u16>) -> Result<JsValue, JsValue> {
        let mut url = format!("/0/public/Depth?pair={}", pair);
        if let Some(c) = count {
            url.push_str(&format!("&count={}", c));
        }
        self.fetch_public(&url).await
    }

    /// Get recent trades
    ///
    /// # Arguments
    /// * `pair` - Trading pair
    /// * `since` - Optional trade ID to get trades since
    /// * `count` - Optional max number of trades
    #[wasm_bindgen]
    pub async fn get_recent_trades(&self, pair: &str, since: Option<String>, count: Option<u32>) -> Result<JsValue, JsValue> {
        let mut url = format!("/0/public/Trades?pair={}", pair);
        if let Some(s) = since {
            url.push_str(&format!("&since={}", s));
        }
        if let Some(c) = count {
            url.push_str(&format!("&count={}", c));
        }
        self.fetch_public(&url).await
    }

    /// Get recent spread data
    ///
    /// # Arguments
    /// * `pair` - Trading pair
    /// * `since` - Optional timestamp to get spreads since
    #[wasm_bindgen]
    pub async fn get_spread(&self, pair: &str, since: Option<u64>) -> Result<JsValue, JsValue> {
        let mut url = format!("/0/public/Spread?pair={}", pair);
        if let Some(s) = since {
            url.push_str(&format!("&since={}", s));
        }
        self.fetch_public(&url).await
    }

    // ========== Internal Helpers ==========

    /// Fetch from a public endpoint
    async fn fetch_public(&self, path: &str) -> Result<JsValue, JsValue> {
        let url = format!("{}{}", self.base_url, path);

        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| JsValue::from_str(&format!("Failed to create request: {:?}", e)))?;

        request.headers()
            .set("Accept", "application/json")
            .map_err(|e| JsValue::from_str(&format!("Failed to set header: {:?}", e)))?;

        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("No window object available"))?;

        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| JsValue::from_str(&format!("Fetch failed: {:?}", e)))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| JsValue::from_str("Response is not a Response object"))?;

        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "HTTP error: {} {}",
                resp.status(),
                resp.status_text()
            )));
        }

        let json = JsFuture::from(
            resp.json()
                .map_err(|e| JsValue::from_str(&format!("Failed to get JSON: {:?}", e)))?,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to parse JSON: {:?}", e)))?;

        // Check for Kraken API errors in response
        if let Ok(error_array) = js_sys::Reflect::get(&json, &JsValue::from_str("error")) {
            if let Some(arr) = error_array.dyn_ref::<js_sys::Array>() {
                if arr.length() > 0 {
                    let errors: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_string())
                        .collect();
                    if !errors.is_empty() {
                        return Err(JsValue::from_str(&format!("Kraken API error: {}", errors.join(", "))));
                    }
                }
            }
        }

        // Return the result field
        js_sys::Reflect::get(&json, &JsValue::from_str("result"))
            .map_err(|_| JsValue::from_str("Response missing 'result' field"))
    }
}

impl Default for WasmRestClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Rate Limiter WASM Bindings
// ============================================================================

use std::cell::RefCell;
use std::rc::Rc;

/// WASM-compatible rate limiter for client-side request throttling
///
/// Uses a token bucket algorithm to rate limit requests. This helps prevent
/// hitting Kraken's API rate limits when making requests from the browser.
///
/// # Usage (JavaScript)
///
/// ```javascript
/// import init, { WasmRateLimiter } from 'kraken-wasm';
///
/// await init();
///
/// // Create a limiter for public endpoints (15 req/min, refill 0.25/sec)
/// const limiter = new WasmRateLimiter(15, 0.25);
///
/// // Before making a request
/// if (limiter.try_acquire()) {
///     await client.get_ticker('XBTUSD');
/// } else {
///     const waitTime = limiter.time_until_available();
///     console.log(`Rate limited, wait ${waitTime}ms`);
/// }
///
/// // Or wait for availability
/// await limiter.wait_for_token();
/// await client.get_ticker('ETHUSD');
/// ```
#[wasm_bindgen]
pub struct WasmRateLimiter {
    capacity: f64,
    tokens: Rc<RefCell<f64>>,
    refill_rate: f64,  // tokens per second
    last_refill: Rc<RefCell<f64>>,  // timestamp in ms
}

#[wasm_bindgen]
impl WasmRateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of tokens (requests)
    /// * `refill_rate` - Tokens added per second
    #[wasm_bindgen(constructor)]
    pub fn new(capacity: f64, refill_rate: f64) -> WasmRateLimiter {
        let now = js_sys::Date::now();
        WasmRateLimiter {
            capacity,
            tokens: Rc::new(RefCell::new(capacity)),
            refill_rate,
            last_refill: Rc::new(RefCell::new(now)),
        }
    }

    /// Create a rate limiter with Kraken's default public endpoint limits
    ///
    /// 15 requests, refilling at 0.5 per second (30 per minute)
    #[wasm_bindgen]
    pub fn kraken_public() -> WasmRateLimiter {
        WasmRateLimiter::new(15.0, 0.5)
    }

    /// Create a rate limiter with Kraken's default private endpoint limits
    ///
    /// 20 requests, refilling at 0.33 per second (20 per minute)
    #[wasm_bindgen]
    pub fn kraken_private() -> WasmRateLimiter {
        WasmRateLimiter::new(20.0, 0.33)
    }

    /// Refill tokens based on time elapsed
    fn refill(&self) {
        let now = js_sys::Date::now();
        let mut last = self.last_refill.borrow_mut();
        let elapsed_secs = (now - *last) / 1000.0;

        if elapsed_secs > 0.0 {
            let mut tokens = self.tokens.borrow_mut();
            let new_tokens = *tokens + (elapsed_secs * self.refill_rate);
            *tokens = new_tokens.min(self.capacity);
            *last = now;
        }
    }

    /// Try to acquire a token for making a request
    ///
    /// Returns true if a token was acquired, false if rate limited
    #[wasm_bindgen]
    pub fn try_acquire(&self) -> bool {
        self.refill();

        let mut tokens = self.tokens.borrow_mut();
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get the number of available tokens
    #[wasm_bindgen]
    pub fn available(&self) -> f64 {
        self.refill();
        *self.tokens.borrow()
    }

    /// Get the maximum capacity
    #[wasm_bindgen]
    pub fn capacity(&self) -> f64 {
        self.capacity
    }

    /// Get time until a token is available (in milliseconds)
    ///
    /// Returns 0 if a token is immediately available
    #[wasm_bindgen]
    pub fn time_until_available(&self) -> f64 {
        self.refill();

        let tokens = *self.tokens.borrow();
        if tokens >= 1.0 {
            0.0
        } else {
            let needed = 1.0 - tokens;
            (needed / self.refill_rate) * 1000.0
        }
    }

    /// Get utilization percentage (0.0 to 1.0)
    ///
    /// 0.0 = no tokens used, 1.0 = all tokens used
    #[wasm_bindgen]
    pub fn utilization(&self) -> f64 {
        self.refill();
        let tokens = *self.tokens.borrow();
        1.0 - (tokens / self.capacity)
    }

    /// Reset the limiter to full capacity
    #[wasm_bindgen]
    pub fn reset(&self) {
        *self.tokens.borrow_mut() = self.capacity;
        *self.last_refill.borrow_mut() = js_sys::Date::now();
    }

    /// Wait for a token to become available (returns a Promise)
    ///
    /// This is useful for async/await patterns in JavaScript
    #[wasm_bindgen]
    pub fn wait_for_token(&self) -> js_sys::Promise {
        let tokens = self.tokens.clone();
        let last_refill = self.last_refill.clone();
        let capacity = self.capacity;
        let refill_rate = self.refill_rate;

        js_sys::Promise::new(&mut |resolve, _reject| {
            // Check current availability
            let now = js_sys::Date::now();
            let elapsed_secs = (now - *last_refill.borrow()) / 1000.0;
            let current_tokens = (*tokens.borrow() + elapsed_secs * refill_rate).min(capacity);

            if current_tokens >= 1.0 {
                // Token available, resolve immediately
                *tokens.borrow_mut() = current_tokens - 1.0;
                *last_refill.borrow_mut() = now;
                resolve.call0(&JsValue::UNDEFINED).ok();
            } else {
                // Need to wait
                let needed = 1.0 - current_tokens;
                let wait_ms = (needed / refill_rate) * 1000.0;

                let tokens_clone = tokens.clone();
                let last_refill_clone = last_refill.clone();

                let closure = wasm_bindgen::closure::Closure::once(Box::new(move || {
                    let now = js_sys::Date::now();
                    let elapsed = (now - *last_refill_clone.borrow()) / 1000.0;
                    let new_tokens = (*tokens_clone.borrow() + elapsed * refill_rate).min(capacity);
                    *tokens_clone.borrow_mut() = (new_tokens - 1.0).max(0.0);
                    *last_refill_clone.borrow_mut() = now;
                    resolve.call0(&JsValue::UNDEFINED).ok();
                }) as Box<dyn FnOnce()>);

                let window = web_sys::window().expect("window");
                window
                    .set_timeout_with_callback_and_timeout_and_arguments_0(
                        closure.as_ref().unchecked_ref(),
                        wait_ms as i32,
                    )
                    .ok();
                closure.forget();
            }
        })
    }

    /// Check if making a request would exceed the rate limit
    ///
    /// Returns true if the rate limit would be exceeded
    #[wasm_bindgen]
    pub fn is_limited(&self) -> bool {
        self.refill();
        *self.tokens.borrow() < 1.0
    }
}
