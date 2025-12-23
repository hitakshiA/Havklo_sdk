//! Thread-safe rate limiter for Kraken WebSocket connections
//!
//! Provides a multi-category rate limiter that can be shared across async tasks
//! to prevent hitting Kraken's API rate limits.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tracing::instrument;
use kraken_types::{
    RateLimitCategory, RateLimitConfig, RateLimitResult, TokenBucket, TokenBucketConfig,
};

/// Thread-safe rate limiter for managing API rate limits
///
/// This rate limiter maintains separate token buckets for different categories
/// of API operations (connections, REST public/private, WebSocket orders, etc.)
#[derive(Debug)]
pub struct KrakenRateLimiter {
    /// Rate limit configuration
    config: RateLimitConfig,
    /// Token buckets by category
    buckets: HashMap<RateLimitCategory, Mutex<TokenBucket>>,
    /// Custom per-symbol buckets (for L3 subscriptions)
    symbol_buckets: Mutex<HashMap<String, TokenBucket>>,
}

impl Default for KrakenRateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::kraken_defaults())
    }
}

impl KrakenRateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        let mut buckets = HashMap::new();

        // Initialize buckets for each category
        buckets.insert(
            RateLimitCategory::Connection,
            Mutex::new(config.connection_limit.create_bucket()),
        );
        buckets.insert(
            RateLimitCategory::RestPublic,
            Mutex::new(config.rest_public.create_bucket()),
        );
        buckets.insert(
            RateLimitCategory::RestPrivate,
            Mutex::new(config.rest_private.create_bucket()),
        );
        buckets.insert(
            RateLimitCategory::WsOrders,
            Mutex::new(config.ws_orders.create_bucket()),
        );
        buckets.insert(
            RateLimitCategory::L3Depth10,
            Mutex::new(config.l3_depth_10.create_bucket()),
        );
        buckets.insert(
            RateLimitCategory::L3Depth100,
            Mutex::new(config.l3_depth_100.create_bucket()),
        );
        buckets.insert(
            RateLimitCategory::L3Depth1000,
            Mutex::new(config.l3_depth_1000.create_bucket()),
        );

        Self {
            config,
            buckets,
            symbol_buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Create a rate limiter with Kraken's default limits
    pub fn kraken_defaults() -> Self {
        Self::new(RateLimitConfig::kraken_defaults())
    }

    /// Create a rate limiter with high-tier limits
    pub fn high_tier() -> Self {
        Self::new(RateLimitConfig::high_tier())
    }

    /// Create a permissive rate limiter (for testing)
    pub fn permissive() -> Self {
        Self::new(RateLimitConfig::permissive())
    }

    /// Try to acquire capacity for the given category
    ///
    /// Returns `RateLimitResult::Allowed` if the request can proceed,
    /// or `RateLimitResult::Limited` with the wait duration if rate limited.
    pub fn try_acquire(&self, category: RateLimitCategory) -> RateLimitResult {
        self.try_acquire_n(category, 1)
    }

    /// Try to acquire multiple tokens for the given category
    pub fn try_acquire_n(&self, category: RateLimitCategory, tokens: u32) -> RateLimitResult {
        if let Some(bucket) = self.buckets.get(&category) {
            let mut bucket = bucket.lock();
            match bucket.try_acquire(tokens) {
                Ok(()) => RateLimitResult::Allowed,
                Err(wait) => RateLimitResult::Limited { wait, category },
            }
        } else {
            // Unknown category - allow by default
            RateLimitResult::Allowed
        }
    }

    /// Check if the request would be allowed without consuming tokens
    pub fn check(&self, category: RateLimitCategory) -> bool {
        self.check_n(category, 1)
    }

    /// Check if N tokens would be available
    pub fn check_n(&self, category: RateLimitCategory, tokens: u32) -> bool {
        if let Some(bucket) = self.buckets.get(&category) {
            let mut bucket = bucket.lock();
            bucket.check_available(tokens)
        } else {
            true
        }
    }

    /// Get available tokens for a category
    pub fn available(&self, category: RateLimitCategory) -> u32 {
        if let Some(bucket) = self.buckets.get(&category) {
            let mut bucket = bucket.lock();
            bucket.available()
        } else {
            u32::MAX
        }
    }

    /// Wait until the request can proceed, then acquire the token
    ///
    /// This is an async method that will sleep if rate limited.
    #[instrument(skip(self), level = "debug")]
    pub async fn acquire(&self, category: RateLimitCategory) {
        self.acquire_n(category, 1).await
    }

    /// Wait and acquire multiple tokens
    #[instrument(skip(self), level = "debug")]
    pub async fn acquire_n(&self, category: RateLimitCategory, tokens: u32) {
        loop {
            match self.try_acquire_n(category, tokens) {
                RateLimitResult::Allowed => return,
                RateLimitResult::Limited { wait, .. } => {
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }

    /// Try to acquire for L3 subscription with specific depth
    pub fn try_acquire_l3(&self, depth: u32) -> RateLimitResult {
        let category = RateLimitCategory::from_l3_depth(depth);
        self.try_acquire(category)
    }

    /// Try to acquire for a WebSocket order submission
    pub fn try_acquire_ws_order(&self) -> RateLimitResult {
        self.try_acquire(RateLimitCategory::WsOrders)
    }

    /// Try to acquire for a connection attempt
    pub fn try_acquire_connection(&self) -> RateLimitResult {
        self.try_acquire(RateLimitCategory::Connection)
    }

    /// Try to acquire a per-symbol rate limit (for custom limits)
    pub fn try_acquire_symbol(&self, symbol: &str, config: TokenBucketConfig) -> RateLimitResult {
        let mut symbol_buckets = self.symbol_buckets.lock();

        let bucket = symbol_buckets
            .entry(symbol.to_string())
            .or_insert_with(|| config.create_bucket());

        match bucket.try_acquire(1) {
            Ok(()) => RateLimitResult::Allowed,
            Err(wait) => RateLimitResult::Limited {
                wait,
                category: RateLimitCategory::RestPublic, // Default category for reporting
            },
        }
    }

    /// Reset all rate limiters
    pub fn reset_all(&self) {
        for bucket in self.buckets.values() {
            bucket.lock().reset();
        }
        self.symbol_buckets.lock().clear();
    }

    /// Reset a specific category
    pub fn reset(&self, category: RateLimitCategory) {
        if let Some(bucket) = self.buckets.get(&category) {
            bucket.lock().reset();
        }
    }

    /// Get the configuration for a category
    pub fn get_config(&self, category: RateLimitCategory) -> TokenBucketConfig {
        category.get_config(&self.config)
    }

    /// Get rate limit utilization as a percentage (0.0 to 1.0)
    pub fn utilization(&self, category: RateLimitCategory) -> f64 {
        if let Some(bucket) = self.buckets.get(&category) {
            let mut bucket = bucket.lock();
            1.0 - (bucket.available() as f64 / bucket.capacity() as f64)
        } else {
            0.0
        }
    }
}

/// Shared rate limiter that can be cloned and used across tasks
pub type SharedRateLimiter = Arc<KrakenRateLimiter>;

/// Create a shared rate limiter with default Kraken limits
pub fn shared_rate_limiter() -> SharedRateLimiter {
    Arc::new(KrakenRateLimiter::kraken_defaults())
}

/// Create a shared rate limiter with custom configuration
pub fn shared_rate_limiter_with_config(config: RateLimitConfig) -> SharedRateLimiter {
    Arc::new(KrakenRateLimiter::new(config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = KrakenRateLimiter::kraken_defaults();
        assert!(limiter.check(RateLimitCategory::Connection));
    }

    #[test]
    fn test_rate_limiter_acquire() {
        let limiter = KrakenRateLimiter::kraken_defaults();

        // Should succeed initially
        let result = limiter.try_acquire(RateLimitCategory::WsOrders);
        assert!(result.is_allowed());

        // Connection bucket has 150 capacity
        for _ in 0..150 {
            let result = limiter.try_acquire(RateLimitCategory::Connection);
            assert!(result.is_allowed());
        }

        // Should be rate limited now
        let result = limiter.try_acquire(RateLimitCategory::Connection);
        assert!(!result.is_allowed());
        assert!(result.wait_duration().is_some());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = KrakenRateLimiter::kraken_defaults();

        // Exhaust the WsOrders bucket (15 tokens)
        for _ in 0..15 {
            limiter.try_acquire(RateLimitCategory::WsOrders);
        }

        // Should be limited
        assert!(!limiter.check(RateLimitCategory::WsOrders));

        // Reset
        limiter.reset(RateLimitCategory::WsOrders);

        // Should be available again
        assert!(limiter.check(RateLimitCategory::WsOrders));
    }

    #[test]
    fn test_rate_limiter_utilization() {
        let limiter = KrakenRateLimiter::kraken_defaults();

        // Initially no utilization
        let util = limiter.utilization(RateLimitCategory::WsOrders);
        assert!(util < 0.01);

        // Use half the tokens (WsOrders has 15)
        for _ in 0..7 {
            limiter.try_acquire(RateLimitCategory::WsOrders);
        }

        // Should be ~50% utilized
        let util = limiter.utilization(RateLimitCategory::WsOrders);
        assert!(util > 0.4 && util < 0.6);
    }

    #[test]
    fn test_l3_depth_acquire() {
        let limiter = KrakenRateLimiter::kraken_defaults();

        // L3 depth 10 has capacity 5
        for _ in 0..5 {
            let result = limiter.try_acquire_l3(10);
            assert!(result.is_allowed());
        }

        // Should be limited
        let result = limiter.try_acquire_l3(10);
        assert!(!result.is_allowed());
    }

    #[test]
    fn test_shared_rate_limiter() {
        let limiter = shared_rate_limiter();
        let limiter2 = Arc::clone(&limiter);

        // Use tokens from one reference
        limiter.try_acquire(RateLimitCategory::WsOrders);

        // Should affect the other reference
        assert_eq!(limiter.available(RateLimitCategory::WsOrders), limiter2.available(RateLimitCategory::WsOrders));
    }

    #[tokio::test]
    async fn test_async_acquire() {
        let limiter = KrakenRateLimiter::permissive();

        // Should complete immediately
        limiter.acquire(RateLimitCategory::Connection).await;
    }
}
