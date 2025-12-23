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

use kraken_book::{HistoryBuffer, Orderbook, OrderbookState};
use kraken_types::WsMessage;
use rust_decimal::prelude::ToPrimitive;
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
