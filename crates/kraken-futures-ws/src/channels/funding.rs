//! Funding rate channel handler
//!
//! This module provides dedicated handling for funding rate data,
//! useful for monitoring perpetual swap funding costs.

use crate::types::{FundingRate, FuturesEvent};
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::debug;

/// Maximum number of historical funding rates to keep per product
const MAX_HISTORY_SIZE: usize = 100;

/// Funding rate entry with timestamp
#[derive(Debug, Clone)]
pub struct FundingRateEntry {
    /// The funding rate
    pub rate: FundingRate,
    /// When this rate was received
    pub received_at: u64,
}

/// Funding rate statistics
#[derive(Debug, Clone, Default)]
pub struct FundingStats {
    /// Number of funding rate updates received
    pub update_count: u64,
    /// Sum of all funding rates (for average calculation)
    pub rate_sum: Decimal,
    /// Highest funding rate seen
    pub max_rate: Option<Decimal>,
    /// Lowest funding rate seen
    pub min_rate: Option<Decimal>,
}

impl FundingStats {
    /// Get average funding rate
    pub fn average_rate(&self) -> Option<Decimal> {
        if self.update_count == 0 {
            return None;
        }
        Some(self.rate_sum / Decimal::from(self.update_count))
    }

    /// Get average annualized rate
    pub fn average_annualized(&self) -> Option<Decimal> {
        self.average_rate().map(|r| r * Decimal::from(3 * 365))
    }
}

/// Funding channel handler for tracking funding rates
pub struct FundingChannel {
    /// Current funding rates by product ID
    current_rates: HashMap<String, FundingRate>,
    /// Historical funding rates by product ID
    history: HashMap<String, VecDeque<FundingRateEntry>>,
    /// Statistics by product ID
    stats: HashMap<String, FundingStats>,
    /// Products with positive funding (longs pay shorts)
    positive_funding_products: Vec<String>,
    /// Products with negative funding (shorts pay longs)
    negative_funding_products: Vec<String>,
}

impl FundingChannel {
    /// Create a new funding channel handler
    pub fn new() -> Self {
        Self {
            current_rates: HashMap::new(),
            history: HashMap::new(),
            stats: HashMap::new(),
            positive_funding_products: Vec::new(),
            negative_funding_products: Vec::new(),
        }
    }

    /// Process a funding rate update
    pub fn process_funding(&mut self, rate: FundingRate) -> FuturesEvent {
        let product_id = rate.product_id.clone();
        debug!("Funding rate update for {}: {}", product_id, rate.funding_rate);

        // Update current rate
        self.current_rates.insert(product_id.clone(), rate.clone());

        // Add to history
        let entry = FundingRateEntry {
            rate: rate.clone(),
            received_at: current_timestamp(),
        };

        let history = self.history.entry(product_id.clone()).or_default();
        history.push_back(entry);
        if history.len() > MAX_HISTORY_SIZE {
            history.pop_front();
        }

        // Update statistics
        let stats = self.stats.entry(product_id.clone()).or_default();
        stats.update_count += 1;
        stats.rate_sum += rate.funding_rate;
        stats.max_rate = Some(
            stats.max_rate.map_or(rate.funding_rate, |m| m.max(rate.funding_rate))
        );
        stats.min_rate = Some(
            stats.min_rate.map_or(rate.funding_rate, |m| m.min(rate.funding_rate))
        );

        // Update positive/negative lists
        self.update_funding_lists(&product_id, rate.funding_rate);

        FuturesEvent::Funding(rate)
    }

    /// Update the positive/negative funding lists
    fn update_funding_lists(&mut self, product_id: &str, rate: Decimal) {
        // Remove from both lists first
        self.positive_funding_products.retain(|p| p != product_id);
        self.negative_funding_products.retain(|p| p != product_id);

        // Add to appropriate list
        if rate > Decimal::ZERO {
            self.positive_funding_products.push(product_id.to_string());
        } else if rate < Decimal::ZERO {
            self.negative_funding_products.push(product_id.to_string());
        }
    }

    /// Get current funding rate for a product
    pub fn current_rate(&self, product_id: &str) -> Option<&FundingRate> {
        self.current_rates.get(product_id)
    }

    /// Get current funding rate value
    pub fn funding_rate(&self, product_id: &str) -> Option<Decimal> {
        self.current_rates.get(product_id).map(|r| r.funding_rate)
    }

    /// Get annualized funding rate
    pub fn annualized_rate(&self, product_id: &str) -> Option<Decimal> {
        self.current_rates.get(product_id).map(|r| r.annualized())
    }

    /// Get next funding time
    pub fn next_funding_time(&self, product_id: &str) -> Option<&str> {
        self.current_rates.get(product_id).map(|r| r.next_funding_rate_time.as_str())
    }

    /// Get funding rate history for a product
    pub fn history(&self, product_id: &str) -> Option<&VecDeque<FundingRateEntry>> {
        self.history.get(product_id)
    }

    /// Get statistics for a product
    pub fn stats(&self, product_id: &str) -> Option<&FundingStats> {
        self.stats.get(product_id)
    }

    /// Get all tracked product IDs
    pub fn product_ids(&self) -> Vec<&str> {
        self.current_rates.keys().map(|s| s.as_str()).collect()
    }

    /// Get products with positive funding (longs pay)
    pub fn positive_funding_products(&self) -> &[String] {
        &self.positive_funding_products
    }

    /// Get products with negative funding (shorts pay)
    pub fn negative_funding_products(&self) -> &[String] {
        &self.negative_funding_products
    }

    /// Calculate estimated funding payment for a position
    ///
    /// # Arguments
    /// * `product_id` - The product
    /// * `position_value` - Position value in quote currency
    /// * `is_long` - True if long position
    ///
    /// # Returns
    /// Estimated payment (positive = you pay, negative = you receive)
    pub fn estimate_funding_payment(
        &self,
        product_id: &str,
        position_value: Decimal,
        is_long: bool,
    ) -> Option<Decimal> {
        let rate = self.current_rates.get(product_id)?;

        // Funding payment = Position Value × Funding Rate
        // If long and rate > 0: you pay
        // If short and rate < 0: you pay
        let payment = position_value * rate.funding_rate;

        if is_long {
            Some(payment) // Positive rate = longs pay
        } else {
            Some(-payment) // Negative = shorts pay
        }
    }

    /// Get products sorted by funding rate (highest first)
    pub fn products_by_funding_rate(&self) -> Vec<(&str, Decimal)> {
        let mut products: Vec<_> = self.current_rates
            .iter()
            .map(|(id, rate)| (id.as_str(), rate.funding_rate))
            .collect();

        products.sort_by(|a, b| b.1.cmp(&a.1));
        products
    }

    /// Get products with funding rate above threshold
    pub fn high_funding_products(&self, threshold: Decimal) -> Vec<&str> {
        self.current_rates
            .iter()
            .filter(|(_, rate)| rate.funding_rate.abs() > threshold)
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Get summary of all funding rates
    pub fn summary(&self) -> FundingSummary {
        let mut total_positive = Decimal::ZERO;
        let mut total_negative = Decimal::ZERO;
        let mut count_positive = 0;
        let mut count_negative = 0;

        for rate in self.current_rates.values() {
            if rate.funding_rate > Decimal::ZERO {
                total_positive += rate.funding_rate;
                count_positive += 1;
            } else if rate.funding_rate < Decimal::ZERO {
                total_negative += rate.funding_rate;
                count_negative += 1;
            }
        }

        FundingSummary {
            total_products: self.current_rates.len(),
            positive_funding_count: count_positive,
            negative_funding_count: count_negative,
            average_positive_rate: if count_positive > 0 {
                Some(total_positive / Decimal::from(count_positive))
            } else {
                None
            },
            average_negative_rate: if count_negative > 0 {
                Some(total_negative / Decimal::from(count_negative))
            } else {
                None
            },
        }
    }
}

impl Default for FundingChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of funding rates across all products
#[derive(Debug, Clone)]
pub struct FundingSummary {
    /// Total number of tracked products
    pub total_products: usize,
    /// Number of products with positive funding
    pub positive_funding_count: usize,
    /// Number of products with negative funding
    pub negative_funding_count: usize,
    /// Average positive funding rate
    pub average_positive_rate: Option<Decimal>,
    /// Average negative funding rate
    pub average_negative_rate: Option<Decimal>,
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_funding_rate(product_id: &str, rate: Decimal) -> FundingRate {
        FundingRate {
            product_id: product_id.to_string(),
            funding_rate: rate,
            relative_funding_rate: None,
            next_funding_rate_time: "2024-01-01T08:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_funding_channel_new() {
        let channel = FundingChannel::new();
        assert!(channel.product_ids().is_empty());
    }

    #[test]
    fn test_process_funding() {
        let mut channel = FundingChannel::new();
        let rate = create_test_funding_rate("PI_XBTUSD", Decimal::new(1, 4)); // 0.0001

        channel.process_funding(rate);

        assert!(channel.current_rate("PI_XBTUSD").is_some());
        assert_eq!(
            channel.funding_rate("PI_XBTUSD"),
            Some(Decimal::new(1, 4))
        );
    }

    #[test]
    fn test_annualized_rate() {
        let mut channel = FundingChannel::new();
        let rate = create_test_funding_rate("PI_XBTUSD", Decimal::new(1, 4)); // 0.0001

        channel.process_funding(rate);

        // 0.0001 * 3 * 365 = 0.1095 (10.95% APR)
        let annualized = channel.annualized_rate("PI_XBTUSD").unwrap();
        assert!(annualized > Decimal::new(10, 2));
    }

    #[test]
    fn test_funding_lists() {
        let mut channel = FundingChannel::new();

        // Positive funding
        channel.process_funding(create_test_funding_rate("PI_XBTUSD", Decimal::new(1, 4)));
        // Negative funding
        channel.process_funding(create_test_funding_rate("PI_ETHUSD", Decimal::new(-1, 4)));

        assert!(channel.positive_funding_products().contains(&"PI_XBTUSD".to_string()));
        assert!(channel.negative_funding_products().contains(&"PI_ETHUSD".to_string()));
    }

    #[test]
    fn test_estimate_funding_payment() {
        let mut channel = FundingChannel::new();
        channel.process_funding(create_test_funding_rate("PI_XBTUSD", Decimal::new(1, 4)));

        // Position value: $10000, Long, Rate: 0.0001
        // Payment = 10000 * 0.0001 = $1 (you pay)
        let payment = channel.estimate_funding_payment(
            "PI_XBTUSD",
            Decimal::from(10000),
            true // long
        ).unwrap();

        assert_eq!(payment, Decimal::ONE);
    }

    #[test]
    fn test_history() {
        let mut channel = FundingChannel::new();

        for i in 0..5 {
            let rate = create_test_funding_rate(
                "PI_XBTUSD",
                Decimal::new(i + 1, 4)
            );
            channel.process_funding(rate);
        }

        let history = channel.history("PI_XBTUSD").unwrap();
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_stats() {
        let mut channel = FundingChannel::new();

        channel.process_funding(create_test_funding_rate("PI_XBTUSD", Decimal::new(1, 4)));
        channel.process_funding(create_test_funding_rate("PI_XBTUSD", Decimal::new(3, 4)));

        let stats = channel.stats("PI_XBTUSD").unwrap();
        assert_eq!(stats.update_count, 2);
        assert_eq!(stats.average_rate(), Some(Decimal::new(2, 4))); // (0.0001 + 0.0003) / 2
    }
}
