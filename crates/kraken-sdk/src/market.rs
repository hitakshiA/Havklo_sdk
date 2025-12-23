//! Unified market state manager
//!
//! Provides a single source of truth for all market data across subscribed symbols.
//! This module enables building 10+ different demo applications with clean abstractions.
//!
//! # Features
//!
//! - **Orderbook State**: Best bid/ask, spread, mid price, depth snapshots
//! - **VWAP Calculation**: Volume-weighted average price for order sizing
//! - **Trade History**: Recent trades with configurable buffer size
//! - **Volatility Estimation**: Realized volatility from trade data
//! - **Event Filtering**: Subscribe to specific symbols/channels with filters
//!
//! # Example
//!
//! ```no_run
//! use kraken_sdk::market::{MarketState, Spread};
//! use kraken_types::Decimal;
//!
//! let mut state = MarketState::new();
//!
//! // Query market data
//! if let Some(spread) = state.spread("BTC/USD") {
//!     println!("BTC/USD spread: {} ({}bp)", spread.absolute, spread.basis_points);
//! }
//!
//! // Calculate VWAP for order sizing
//! if let Some(vwap) = state.vwap_buy("BTC/USD", Decimal::from(10)) {
//!     println!("VWAP to buy 10 BTC: {}", vwap);
//! }
//! ```
//!
//! # Demo Applications Enabled
//!
//! | Demo | Primary Methods |
//! |------|----------------|
//! | Simple Ticker | `spread()`, `mid_price()` |
//! | Orderbook Depth | `book_snapshot()`, `bbo()` |
//! | Spread Monitor | `spread()` with history |
//! | Arbitrage Detector | `mid_price()` cross-symbol |
//! | VWAP Calculator | `vwap_buy()`, `vwap_sell()` |
//! | Market Maker | `bbo()`, `book_snapshot()`, `imbalance()` |
//! | Trade Analytics | `recent_trades()`, `trade_volume()` |
//! | Volatility Tracker | `volatility()` |

use kraken_book::{Orderbook, OrderbookSnapshot, OrderbookState};
use kraken_types::{BookData, Decimal, Level, Side};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tracing::instrument;

/// Maximum number of trades to keep in history per symbol
const DEFAULT_TRADE_HISTORY_SIZE: usize = 1000;

/// Spread information with multiple representations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Spread {
    /// Absolute spread (ask - bid)
    pub absolute: Decimal,
    /// Spread as basis points relative to mid price
    pub basis_points: Decimal,
    /// Spread as percentage of mid price
    pub percentage: Decimal,
    /// Best bid price
    pub bid: Decimal,
    /// Best ask price
    pub ask: Decimal,
    /// Mid price
    pub mid: Decimal,
}

impl Spread {
    /// Create a new spread from bid and ask prices
    pub fn new(bid: Decimal, ask: Decimal) -> Self {
        let absolute = ask - bid;
        let mid = (bid + ask) / dec!(2);

        // Avoid division by zero
        let (basis_points, percentage) = if mid.is_zero() {
            (Decimal::ZERO, Decimal::ZERO)
        } else {
            let pct = absolute / mid;
            (pct * dec!(10000), pct * dec!(100))
        };

        Self {
            absolute,
            basis_points,
            percentage,
            bid,
            ask,
            mid,
        }
    }

    /// Check if spread is within acceptable threshold (in basis points)
    pub fn is_tight(&self, max_bps: Decimal) -> bool {
        self.basis_points <= max_bps
    }
}

impl std::fmt::Display for Spread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({:.1}bp) [{} / {}]",
            self.absolute,
            self.basis_points,
            self.bid,
            self.ask
        )
    }
}

/// Best bid and offer (BBO)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub struct BBO {
    /// Best bid level
    pub bid: Level,
    /// Best ask level
    pub ask: Level,
    /// Spread information
    pub spread: Spread,
    /// Book imbalance at top of book (-1 to +1)
    /// Positive = more bids (buy pressure), Negative = more asks (sell pressure)
    pub imbalance: Decimal,
}

impl BBO {
    /// Create from bid and ask levels
    pub fn new(bid: Level, ask: Level) -> Self {
        let spread = Spread::new(bid.price, ask.price);
        let total_qty = bid.qty + ask.qty;
        let imbalance = if total_qty.is_zero() {
            Decimal::ZERO
        } else {
            (bid.qty - ask.qty) / total_qty
        };

        Self {
            bid,
            ask,
            spread,
            imbalance,
        }
    }
}

/// Trade record for history tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    /// Trading pair symbol
    pub symbol: String,
    /// Trade price
    pub price: Decimal,
    /// Trade quantity
    pub qty: Decimal,
    /// Trade side (buy/sell)
    pub side: Side,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Trade ID (if available)
    pub trade_id: Option<String>,
}

impl TradeRecord {
    /// Create a new trade record
    pub fn new(symbol: String, price: Decimal, qty: Decimal, side: Side, timestamp: String) -> Self {
        Self {
            symbol,
            price,
            qty,
            side,
            timestamp,
            trade_id: None,
        }
    }

    /// Trade value (price * qty)
    pub fn value(&self) -> Decimal {
        self.price * self.qty
    }
}

/// Orderbook imbalance across multiple levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookImbalance {
    /// Imbalance ratio (-1 to +1)
    /// Positive = more bids (buy pressure), Negative = more asks (sell pressure)
    pub ratio: Decimal,
    /// Total bid quantity in calculation
    pub bid_qty: Decimal,
    /// Total ask quantity in calculation
    pub ask_qty: Decimal,
    /// Number of levels included
    pub levels: usize,
    /// Interpretation
    pub signal: ImbalanceSignal,
}

/// Interpretation of book imbalance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImbalanceSignal {
    /// Strong buy pressure (imbalance > 0.3)
    StrongBuy,
    /// Moderate buy pressure (0.1 < imbalance <= 0.3)
    ModerateBuy,
    /// Neutral (-0.1 <= imbalance <= 0.1)
    Neutral,
    /// Moderate sell pressure (-0.3 <= imbalance < -0.1)
    ModerateSell,
    /// Strong sell pressure (imbalance < -0.3)
    StrongSell,
}

impl ImbalanceSignal {
    /// Create from imbalance ratio
    pub fn from_ratio(ratio: Decimal) -> Self {
        if ratio > dec!(0.3) {
            Self::StrongBuy
        } else if ratio > dec!(0.1) {
            Self::ModerateBuy
        } else if ratio >= dec!(-0.1) {
            Self::Neutral
        } else if ratio >= dec!(-0.3) {
            Self::ModerateSell
        } else {
            Self::StrongSell
        }
    }
}

/// Per-symbol market state
struct SymbolState {
    /// L2 orderbook
    orderbook: Orderbook,
    /// Recent trades
    trades: VecDeque<TradeRecord>,
    /// Maximum trade history size
    max_trades: usize,
}

impl std::fmt::Debug for SymbolState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymbolState")
            .field("orderbook_symbol", &self.orderbook.symbol())
            .field("orderbook_state", &self.orderbook.state())
            .field("trades_count", &self.trades.len())
            .field("max_trades", &self.max_trades)
            .finish()
    }
}

impl SymbolState {
    fn new(symbol: &str, max_trades: usize) -> Self {
        Self {
            orderbook: Orderbook::new(symbol),
            trades: VecDeque::with_capacity(max_trades),
            max_trades,
        }
    }

    fn add_trade(&mut self, trade: TradeRecord) {
        if self.trades.len() >= self.max_trades {
            self.trades.pop_front();
        }
        self.trades.push_back(trade);
    }
}

/// Unified market state manager
///
/// Provides a single abstraction for accessing all market data across symbols.
/// This is the foundation for building various trading applications.
#[derive(Debug)]
pub struct MarketState {
    /// Per-symbol state
    symbols: HashMap<String, SymbolState>,
    /// Trade history size per symbol
    trade_history_size: usize,
}

impl Default for MarketState {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketState {
    /// Create a new market state manager
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            trade_history_size: DEFAULT_TRADE_HISTORY_SIZE,
        }
    }

    /// Create with custom trade history size
    pub fn with_trade_history_size(mut self, size: usize) -> Self {
        self.trade_history_size = size;
        self
    }

    /// Get or create symbol state
    fn get_or_create_symbol(&mut self, symbol: &str) -> &mut SymbolState {
        self.symbols.entry(symbol.to_string()).or_insert_with(|| {
            SymbolState::new(symbol, self.trade_history_size)
        })
    }

    /// Get symbol state (immutable)
    fn get_symbol(&self, symbol: &str) -> Option<&SymbolState> {
        self.symbols.get(symbol)
    }

    // =========================================================================
    // Orderbook Operations
    // =========================================================================

    /// Apply orderbook data from WebSocket
    #[instrument(skip(self, data), fields(symbol = %data.symbol, is_snapshot))]
    pub fn apply_book_data(
        &mut self,
        data: &BookData,
        is_snapshot: bool,
    ) -> Result<(), kraken_book::ChecksumMismatch> {
        let state = self.get_or_create_symbol(&data.symbol);
        state.orderbook.apply_book_data(data, is_snapshot)?;
        Ok(())
    }

    /// Get orderbook snapshot for a symbol
    pub fn book_snapshot(&self, symbol: &str) -> Option<OrderbookSnapshot> {
        self.get_symbol(symbol).map(|s| s.orderbook.snapshot())
    }

    /// Get orderbook state for a symbol
    pub fn orderbook_state(&self, symbol: &str) -> Option<OrderbookState> {
        self.get_symbol(symbol).map(|s| s.orderbook.state())
    }

    /// Check if orderbook is synced
    pub fn is_synced(&self, symbol: &str) -> bool {
        self.get_symbol(symbol)
            .map(|s| s.orderbook.is_synced())
            .unwrap_or(false)
    }

    /// Get the spread for a symbol
    pub fn spread(&self, symbol: &str) -> Option<Spread> {
        let state = self.get_symbol(symbol)?;
        let bid = state.orderbook.best_bid()?;
        let ask = state.orderbook.best_ask()?;
        Some(Spread::new(bid.price, ask.price))
    }

    /// Get the mid price for a symbol
    pub fn mid_price(&self, symbol: &str) -> Option<Decimal> {
        self.spread(symbol).map(|s| s.mid)
    }

    /// Get best bid and offer (BBO)
    pub fn bbo(&self, symbol: &str) -> Option<BBO> {
        let state = self.get_symbol(symbol)?;
        let bid = state.orderbook.best_bid()?.clone();
        let ask = state.orderbook.best_ask()?.clone();
        Some(BBO::new(bid, ask))
    }

    /// Get top N levels from the orderbook
    pub fn top_levels(&self, symbol: &str, n: usize) -> Option<(Vec<Level>, Vec<Level>)> {
        let state = self.get_symbol(symbol)?;
        let bids = state.orderbook.top_bids(n);
        let asks = state.orderbook.top_asks(n);
        Some((bids, asks))
    }

    /// Calculate orderbook imbalance across top N levels
    pub fn imbalance(&self, symbol: &str, levels: usize) -> Option<BookImbalance> {
        let (bids, asks) = self.top_levels(symbol, levels)?;

        let bid_qty: Decimal = bids.iter().map(|l| l.qty).sum();
        let ask_qty: Decimal = asks.iter().map(|l| l.qty).sum();

        let total = bid_qty + ask_qty;
        let ratio = if total.is_zero() {
            Decimal::ZERO
        } else {
            (bid_qty - ask_qty) / total
        };

        Some(BookImbalance {
            ratio,
            bid_qty,
            ask_qty,
            levels,
            signal: ImbalanceSignal::from_ratio(ratio),
        })
    }

    // =========================================================================
    // VWAP Calculations
    // =========================================================================

    /// Calculate VWAP for buying a given quantity
    ///
    /// Walks through the ask side to determine average price to buy the specified quantity.
    #[instrument(skip(self))]
    pub fn vwap_buy(&self, symbol: &str, qty: Decimal) -> Option<Decimal> {
        let state = self.get_symbol(symbol)?;
        Self::calculate_vwap(state.orderbook.asks_vec().into_iter(), qty)
    }

    /// Calculate VWAP for selling a given quantity
    ///
    /// Walks through the bid side to determine average price to sell the specified quantity.
    #[instrument(skip(self))]
    pub fn vwap_sell(&self, symbol: &str, qty: Decimal) -> Option<Decimal> {
        let state = self.get_symbol(symbol)?;
        Self::calculate_vwap(state.orderbook.bids_vec().into_iter(), qty)
    }

    /// Generic VWAP calculation across price levels
    fn calculate_vwap(levels: impl Iterator<Item = Level>, target_qty: Decimal) -> Option<Decimal> {
        let mut remaining = target_qty;
        let mut total_value = Decimal::ZERO;
        let mut total_qty = Decimal::ZERO;

        for level in levels {
            if remaining.is_zero() {
                break;
            }

            let fill_qty = remaining.min(level.qty);
            total_value += fill_qty * level.price;
            total_qty += fill_qty;
            remaining -= fill_qty;
        }

        // Return None if we couldn't fill the entire quantity
        if remaining > Decimal::ZERO {
            return None;
        }

        if total_qty.is_zero() {
            return None;
        }

        Some(total_value / total_qty)
    }

    /// Calculate slippage for a market order
    ///
    /// Returns (vwap, slippage_bps) where slippage is the difference from mid price.
    pub fn market_order_slippage(
        &self,
        symbol: &str,
        side: Side,
        qty: Decimal,
    ) -> Option<(Decimal, Decimal)> {
        let mid = self.mid_price(symbol)?;
        let vwap = match side {
            Side::Buy => self.vwap_buy(symbol, qty)?,
            Side::Sell => self.vwap_sell(symbol, qty)?,
        };

        let slippage = match side {
            Side::Buy => (vwap - mid) / mid * dec!(10000),
            Side::Sell => (mid - vwap) / mid * dec!(10000),
        };

        Some((vwap, slippage))
    }

    // =========================================================================
    // Trade History Operations
    // =========================================================================

    /// Record a trade
    pub fn record_trade(&mut self, trade: TradeRecord) {
        let symbol = trade.symbol.clone();
        let state = self.get_or_create_symbol(&symbol);
        state.add_trade(trade);
    }

    /// Get recent trades for a symbol
    pub fn recent_trades(&self, symbol: &str, count: usize) -> Vec<&TradeRecord> {
        self.get_symbol(symbol)
            .map(|s| {
                s.trades
                    .iter()
                    .rev()
                    .take(count)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all trades in history for a symbol
    pub fn all_trades(&self, symbol: &str) -> Vec<&TradeRecord> {
        self.get_symbol(symbol)
            .map(|s| s.trades.iter().collect())
            .unwrap_or_default()
    }

    /// Calculate total volume traded in recent history
    pub fn trade_volume(&self, symbol: &str) -> Decimal {
        self.get_symbol(symbol)
            .map(|s| s.trades.iter().map(|t| t.qty).sum())
            .unwrap_or(Decimal::ZERO)
    }

    /// Calculate VWAP from recent trades
    pub fn trade_vwap(&self, symbol: &str) -> Option<Decimal> {
        let state = self.get_symbol(symbol)?;
        if state.trades.is_empty() {
            return None;
        }

        let total_value: Decimal = state.trades.iter().map(|t| t.value()).sum();
        let total_qty: Decimal = state.trades.iter().map(|t| t.qty).sum();

        if total_qty.is_zero() {
            return None;
        }

        Some(total_value / total_qty)
    }

    /// Calculate realized volatility from trade data
    ///
    /// Uses log returns of trade prices to estimate volatility.
    /// Returns annualized volatility as a decimal (e.g., 0.5 = 50%).
    pub fn volatility(&self, symbol: &str) -> Option<Decimal> {
        let state = self.get_symbol(symbol)?;
        if state.trades.len() < 2 {
            return None;
        }

        // Calculate log returns
        let prices: Vec<f64> = state
            .trades
            .iter()
            .map(|t| t.price.to_string().parse::<f64>().unwrap_or(0.0))
            .collect();

        let mut returns = Vec::with_capacity(prices.len() - 1);
        for i in 1..prices.len() {
            if prices[i - 1] > 0.0 && prices[i] > 0.0 {
                returns.push((prices[i] / prices[i - 1]).ln());
            }
        }

        if returns.is_empty() {
            return None;
        }

        // Calculate standard deviation
        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance: f64 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
            / returns.len() as f64;
        let std_dev = variance.sqrt();

        // Annualize (assume ~31,536,000 seconds per year, trades are ~1/minute avg)
        // This is a rough approximation - real implementation would use actual timestamps
        let annualized = std_dev * (525600.0_f64).sqrt(); // sqrt(minutes per year)

        // Convert back to Decimal
        parse_decimal(&format!("{:.6}", annualized))
    }

    // =========================================================================
    // Multi-Symbol Operations
    // =========================================================================

    /// Get all tracked symbols
    pub fn symbols(&self) -> Vec<&str> {
        self.symbols.keys().map(|s| s.as_str()).collect()
    }

    /// Get spreads for all symbols
    pub fn all_spreads(&self) -> HashMap<&str, Spread> {
        self.symbols
            .keys()
            .filter_map(|s| self.spread(s).map(|sp| (s.as_str(), sp)))
            .collect()
    }

    /// Find symbols with spread below threshold (in basis points)
    pub fn tight_spreads(&self, max_bps: Decimal) -> Vec<(&str, Spread)> {
        self.symbols
            .keys()
            .filter_map(|s| {
                self.spread(s)
                    .filter(|sp| sp.is_tight(max_bps))
                    .map(|sp| (s.as_str(), sp))
            })
            .collect()
    }

    /// Compare mid prices across symbols (for arbitrage detection)
    pub fn compare_prices<'a>(&self, symbols: &[&'a str]) -> HashMap<&'a str, Decimal> {
        symbols
            .iter()
            .filter_map(|s| self.mid_price(s).map(|p| (*s, p)))
            .collect()
    }

    /// Reset state for a symbol
    pub fn reset_symbol(&mut self, symbol: &str) {
        if let Some(state) = self.symbols.get_mut(symbol) {
            state.orderbook.reset();
            state.trades.clear();
        }
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.symbols.clear();
    }
}

/// Helper function for Decimal parsing
fn parse_decimal(s: &str) -> Option<Decimal> {
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_spread_and_bbo() {
        // Spread calculation
        let spread = Spread::new(dec!(100), dec!(101));
        assert_eq!(spread.absolute, dec!(1));
        assert_eq!(spread.mid, dec!(100.5));
        assert!(spread.basis_points > dec!(99) && spread.basis_points < dec!(100));

        // BBO imbalance
        let bbo = BBO::new(
            Level::new(dec!(100), dec!(10)),
            Level::new(dec!(101), dec!(5)),
        );
        assert!(bbo.imbalance > Decimal::ZERO); // More bids = positive

        // Imbalance signals
        assert_eq!(ImbalanceSignal::from_ratio(dec!(0.5)), ImbalanceSignal::StrongBuy);
        assert_eq!(ImbalanceSignal::from_ratio(dec!(-0.5)), ImbalanceSignal::StrongSell);
    }

    #[test]
    fn test_market_state_trades() {
        let mut state = MarketState::new().with_trade_history_size(5);

        // Record trades
        for i in 0..10 {
            state.record_trade(TradeRecord::new(
                "BTC/USD".to_string(),
                Decimal::from(100 + i),
                Decimal::ONE,
                Side::Buy,
                format!("2024-01-01T00:0{}:00Z", i),
            ));
        }

        // History limited to 5
        let trades = state.recent_trades("BTC/USD", 10);
        assert_eq!(trades.len(), 5);

        // Volume and VWAP
        assert_eq!(state.trade_volume("BTC/USD"), dec!(5));
        assert!(state.trade_vwap("BTC/USD").is_some());
    }
}
