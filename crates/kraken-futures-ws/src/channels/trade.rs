//! Trade channel handler

use crate::types::{FuturesEvent, FuturesTrade, TradeSide};
#[cfg(test)]
use crate::types::TradeType;
use rust_decimal::Decimal;
use std::collections::VecDeque;
use tracing::debug;

/// Maximum number of trades to keep per product
const MAX_TRADES_PER_PRODUCT: usize = 1000;

/// Trade channel handler
pub struct TradeChannel {
    /// Recent trades by product ID
    trades: dashmap::DashMap<String, VecDeque<FuturesTrade>>,
    /// Trade count since start
    trade_count: std::sync::atomic::AtomicU64,
}

impl TradeChannel {
    /// Create a new trade channel handler
    pub fn new() -> Self {
        Self {
            trades: dashmap::DashMap::new(),
            trade_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Process a trade
    pub fn process_trade(&self, trade: FuturesTrade) -> FuturesEvent {
        debug!(
            "Trade for {}: {} {:?} @ {}",
            trade.product_id, trade.qty, trade.side, trade.price
        );

        // Get or create trade queue
        let mut trades = self
            .trades
            .entry(trade.product_id.clone())
            .or_insert_with(VecDeque::new);

        // Add trade to front (most recent first)
        trades.push_front(trade.clone());

        // Trim if needed
        while trades.len() > MAX_TRADES_PER_PRODUCT {
            trades.pop_back();
        }

        // Increment counter
        self.trade_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        FuturesEvent::Trade(trade)
    }

    /// Get recent trades for a product
    pub fn recent_trades(&self, product_id: &str, limit: usize) -> Vec<FuturesTrade> {
        self.trades
            .get(product_id)
            .map(|trades| trades.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }

    /// Get last trade for a product
    pub fn last_trade(&self, product_id: &str) -> Option<FuturesTrade> {
        self.trades.get(product_id)?.front().cloned()
    }

    /// Get last trade price for a product
    pub fn last_price(&self, product_id: &str) -> Option<Decimal> {
        Some(self.last_trade(product_id)?.price)
    }

    /// Get total trade count since start
    pub fn trade_count(&self) -> u64 {
        self.trade_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Calculate VWAP for recent trades
    pub fn vwap(&self, product_id: &str, limit: usize) -> Option<Decimal> {
        let trades = self.trades.get(product_id)?;
        let trades: Vec<_> = trades.iter().take(limit).collect();

        if trades.is_empty() {
            return None;
        }

        let total_value: Decimal = trades.iter().map(|t| t.price * t.qty).sum();
        let total_qty: Decimal = trades.iter().map(|t| t.qty).sum();

        if total_qty.is_zero() {
            None
        } else {
            Some(total_value / total_qty)
        }
    }

    /// Get buy/sell ratio for recent trades
    pub fn buy_sell_ratio(&self, product_id: &str, limit: usize) -> Option<Decimal> {
        let trades = self.trades.get(product_id)?;
        let trades: Vec<_> = trades.iter().take(limit).collect();

        if trades.is_empty() {
            return None;
        }

        let buy_volume: Decimal = trades
            .iter()
            .filter(|t| t.side == TradeSide::Buy)
            .map(|t| t.qty)
            .sum();

        let sell_volume: Decimal = trades
            .iter()
            .filter(|t| t.side == TradeSide::Sell)
            .map(|t| t.qty)
            .sum();

        let total = buy_volume + sell_volume;
        if total.is_zero() {
            None
        } else {
            Some(buy_volume / total)
        }
    }
}

impl Default for TradeChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_trade(side: TradeSide, price: i64, qty: i64) -> FuturesTrade {
        FuturesTrade {
            product_id: "PI_XBTUSD".to_string(),
            uid: "test123".to_string(),
            side,
            trade_type: TradeType::Fill,
            price: Decimal::from(price),
            qty: Decimal::from(qty),
            time: "2024-01-01T00:00:00Z".to_string(),
            seq: Some(1),
        }
    }

    #[test]
    fn test_trade_channel() {
        let channel = TradeChannel::new();

        channel.process_trade(create_test_trade(TradeSide::Buy, 50000, 1));
        channel.process_trade(create_test_trade(TradeSide::Sell, 50001, 2));

        assert_eq!(channel.trade_count(), 2);
        assert_eq!(channel.last_price("PI_XBTUSD"), Some(Decimal::from(50001)));
    }

    #[test]
    fn test_vwap() {
        let channel = TradeChannel::new();

        // 1 @ 50000, 2 @ 50002
        // VWAP = (50000 + 100004) / 3 = 50001.33...
        channel.process_trade(create_test_trade(TradeSide::Buy, 50000, 1));
        channel.process_trade(create_test_trade(TradeSide::Buy, 50002, 2));

        let vwap = channel.vwap("PI_XBTUSD", 10).unwrap();
        // (50000*1 + 50002*2) / 3 = 150004/3 = 50001.333...
        assert!(vwap > Decimal::from(50001));
        assert!(vwap < Decimal::from(50002));
    }
}
