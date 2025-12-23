//! Ticker channel handler

use crate::types::{FuturesEvent, FuturesTicker, FundingRate, IndexPrice, MarkPrice};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::debug;

/// Ticker channel handler
pub struct TickerChannel {
    /// Latest tickers by product ID
    tickers: HashMap<String, FuturesTicker>,
    /// Latest funding rates by product ID
    funding_rates: HashMap<String, FundingRate>,
}

impl TickerChannel {
    /// Create a new ticker channel handler
    pub fn new() -> Self {
        Self {
            tickers: HashMap::new(),
            funding_rates: HashMap::new(),
        }
    }

    /// Process a ticker update
    pub fn process_ticker(&mut self, ticker: FuturesTicker) -> FuturesEvent {
        debug!("Ticker update for {}", ticker.product_id);
        self.tickers.insert(ticker.product_id.clone(), ticker.clone());
        FuturesEvent::Ticker(ticker)
    }

    /// Process a funding rate update
    pub fn process_funding(&mut self, rate: FundingRate) -> FuturesEvent {
        debug!("Funding rate update for {}: {}", rate.product_id, rate.funding_rate);
        self.funding_rates.insert(rate.product_id.clone(), rate.clone());
        FuturesEvent::Funding(rate)
    }

    /// Process a mark price update
    pub fn process_mark_price(&mut self, mark: MarkPrice) -> FuturesEvent {
        // Update ticker if we have one
        if let Some(ticker) = self.tickers.get_mut(&mark.product_id) {
            ticker.mark_price = Some(mark.mark_price);
        }
        FuturesEvent::MarkPrice(mark)
    }

    /// Process an index price update
    pub fn process_index_price(&mut self, index: IndexPrice) -> FuturesEvent {
        // Update ticker if we have one
        if let Some(ticker) = self.tickers.get_mut(&index.product_id) {
            ticker.index_price = Some(index.index_price);
        }
        FuturesEvent::IndexPrice(index)
    }

    /// Get latest ticker for a product
    pub fn ticker(&self, product_id: &str) -> Option<&FuturesTicker> {
        self.tickers.get(product_id)
    }

    /// Get latest funding rate for a product
    pub fn funding_rate(&self, product_id: &str) -> Option<&FundingRate> {
        self.funding_rates.get(product_id)
    }

    /// Get mark price for a product
    pub fn mark_price(&self, product_id: &str) -> Option<Decimal> {
        self.tickers.get(product_id)?.mark_price
    }

    /// Get index price for a product
    pub fn index_price(&self, product_id: &str) -> Option<Decimal> {
        self.tickers.get(product_id)?.index_price
    }

    /// Get all tracked product IDs
    pub fn product_ids(&self) -> Vec<&str> {
        self.tickers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for TickerChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_ticker() -> FuturesTicker {
        FuturesTicker {
            product_id: "PI_XBTUSD".to_string(),
            last: Some(Decimal::from(50000)),
            last_qty: Some(Decimal::ONE),
            last_time: Some("2024-01-01T00:00:00Z".to_string()),
            bid: Some(Decimal::from(49999)),
            bid_size: Some(Decimal::from(10)),
            ask: Some(Decimal::from(50001)),
            ask_size: Some(Decimal::from(10)),
            vol24h: Some(Decimal::from(1000)),
            volume_quote: None,
            open_interest: Some(Decimal::from(50000)),
            mark_price: Some(Decimal::from(50000)),
            index_price: Some(Decimal::from(49995)),
            funding_rate: Some(Decimal::new(1, 4)),
            next_funding_rate_time: Some("2024-01-01T08:00:00Z".to_string()),
            open24h: Some(Decimal::from(49000)),
            high24h: Some(Decimal::from(51000)),
            low24h: Some(Decimal::from(48500)),
            change24h: Some(Decimal::from(1000)),
            change_pct24h: Some(Decimal::from(2)),
            premium: Some(Decimal::new(1, 2)),
            suspended: Some(false),
            post_only: Some(false),
        }
    }

    #[test]
    fn test_ticker_channel() {
        let mut channel = TickerChannel::new();
        let ticker = create_test_ticker();

        channel.process_ticker(ticker);

        assert!(channel.ticker("PI_XBTUSD").is_some());
        assert_eq!(channel.mark_price("PI_XBTUSD"), Some(Decimal::from(50000)));
    }
}
