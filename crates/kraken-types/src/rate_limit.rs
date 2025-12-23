//! Client-side rate limiting for Kraken API
//!
//! This module provides token bucket-based rate limiting to prevent hitting
//! Kraken's API rate limits. It supports different rate limits for different
//! endpoint types (public REST, private REST, WebSocket orders, etc.)

use std::time::{Duration, Instant};

/// Token bucket rate limiter
///
/// Implements the token bucket algorithm for rate limiting.
/// Tokens are consumed when making requests and refill at a constant rate.
#[derive(Debug)]
pub struct TokenBucket {
    /// Maximum number of tokens (bucket capacity)
    capacity: u32,
    /// Current number of available tokens
    tokens: f64,
    /// Tokens added per second (refill rate)
    refill_rate: f64,
    /// Last time tokens were refilled
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a new token bucket
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of tokens the bucket can hold
    /// * `refill_rate` - Number of tokens added per second
    pub fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to acquire tokens from the bucket
    ///
    /// Returns `Ok(())` if tokens were acquired, or `Err(Duration)` with the
    /// time to wait before enough tokens will be available.
    pub fn try_acquire(&mut self, tokens: u32) -> Result<(), Duration> {
        self.refill();

        let tokens_f64 = tokens as f64;
        if self.tokens >= tokens_f64 {
            self.tokens -= tokens_f64;
            Ok(())
        } else {
            // Calculate wait time for enough tokens
            let needed = tokens_f64 - self.tokens;
            let wait_secs = needed / self.refill_rate;
            Err(Duration::from_secs_f64(wait_secs))
        }
    }

    /// Acquire tokens, blocking if necessary (for async contexts)
    ///
    /// Returns the duration waited, if any.
    pub fn acquire_blocking(&mut self, tokens: u32) -> Duration {
        match self.try_acquire(tokens) {
            Ok(()) => Duration::ZERO,
            Err(wait) => {
                std::thread::sleep(wait);
                self.refill();
                self.tokens -= tokens as f64;
                wait
            }
        }
    }

    /// Check if tokens are available without consuming them
    pub fn check_available(&mut self, tokens: u32) -> bool {
        self.refill();
        self.tokens >= tokens as f64
    }

    /// Get current available tokens
    pub fn available(&mut self) -> u32 {
        self.refill();
        self.tokens.floor() as u32
    }

    /// Get the capacity of this bucket
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Get the refill rate (tokens per second)
    pub fn refill_rate(&self) -> f64 {
        self.refill_rate
    }

    /// Reset the bucket to full capacity
    pub fn reset(&mut self) {
        self.tokens = self.capacity as f64;
        self.last_refill = Instant::now();
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let added = elapsed.as_secs_f64() * self.refill_rate;
        self.tokens = (self.tokens + added).min(self.capacity as f64);
        self.last_refill = now;
    }
}

/// Rate limit configuration for different Kraken API endpoints
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Cloudflare connection limit: 150 per 10 minutes
    pub connection_limit: TokenBucketConfig,
    /// REST public endpoints
    pub rest_public: TokenBucketConfig,
    /// REST private endpoints
    pub rest_private: TokenBucketConfig,
    /// WebSocket order rate: 15 per second
    pub ws_orders: TokenBucketConfig,
    /// L3 subscription rate counter for depth=10
    pub l3_depth_10: TokenBucketConfig,
    /// L3 subscription rate counter for depth=100
    pub l3_depth_100: TokenBucketConfig,
    /// L3 subscription rate counter for depth=1000
    pub l3_depth_1000: TokenBucketConfig,
}

/// Configuration for a single token bucket
#[derive(Debug, Clone, Copy)]
pub struct TokenBucketConfig {
    /// Maximum tokens
    pub capacity: u32,
    /// Tokens per second refill rate
    pub refill_rate: f64,
}

impl TokenBucketConfig {
    /// Create a new token bucket configuration
    pub const fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            capacity,
            refill_rate,
        }
    }

    /// Create a token bucket from this configuration
    pub fn create_bucket(&self) -> TokenBucket {
        TokenBucket::new(self.capacity, self.refill_rate)
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::kraken_defaults()
    }
}

impl RateLimitConfig {
    /// Create rate limit configuration with Kraken's documented limits
    pub fn kraken_defaults() -> Self {
        Self {
            // Cloudflare: ~150 connections per 10 minutes = 0.25/sec
            connection_limit: TokenBucketConfig::new(150, 0.25),

            // REST public: Higher capacity, more lenient
            // Kraken starter tier: 15 calls, 0.5/sec decay
            rest_public: TokenBucketConfig::new(15, 0.5),

            // REST private: More restricted
            // Kraken starter tier: 20 calls, ~0.33/sec decay
            rest_private: TokenBucketConfig::new(20, 0.33),

            // WebSocket orders: 15 per second per connection
            ws_orders: TokenBucketConfig::new(15, 15.0),

            // L3 depth rate counters (from Kraken docs)
            // These are rate counters per subscription
            l3_depth_10: TokenBucketConfig::new(5, 1.0),
            l3_depth_100: TokenBucketConfig::new(25, 5.0),
            l3_depth_1000: TokenBucketConfig::new(100, 20.0),
        }
    }

    /// Create configuration for high-tier API access
    pub fn high_tier() -> Self {
        Self {
            connection_limit: TokenBucketConfig::new(150, 0.25),
            // Pro tier limits are higher
            rest_public: TokenBucketConfig::new(45, 1.0),
            rest_private: TokenBucketConfig::new(60, 1.0),
            ws_orders: TokenBucketConfig::new(60, 60.0),
            l3_depth_10: TokenBucketConfig::new(5, 1.0),
            l3_depth_100: TokenBucketConfig::new(25, 5.0),
            l3_depth_1000: TokenBucketConfig::new(100, 20.0),
        }
    }

    /// Create a very permissive configuration (for testing)
    pub fn permissive() -> Self {
        Self {
            connection_limit: TokenBucketConfig::new(1000, 100.0),
            rest_public: TokenBucketConfig::new(1000, 100.0),
            rest_private: TokenBucketConfig::new(1000, 100.0),
            ws_orders: TokenBucketConfig::new(1000, 100.0),
            l3_depth_10: TokenBucketConfig::new(1000, 100.0),
            l3_depth_100: TokenBucketConfig::new(1000, 100.0),
            l3_depth_1000: TokenBucketConfig::new(1000, 100.0),
        }
    }
}

/// Rate limiter category for different API endpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitCategory {
    /// Cloudflare connection rate limit
    Connection,
    /// Public REST API endpoints
    RestPublic,
    /// Private REST API endpoints (authenticated)
    RestPrivate,
    /// WebSocket order submissions
    WsOrders,
    /// L3 orderbook subscription (depth 10)
    L3Depth10,
    /// L3 orderbook subscription (depth 100)
    L3Depth100,
    /// L3 orderbook subscription (depth 1000)
    L3Depth1000,
}

impl RateLimitCategory {
    /// Get the configuration for this category from the rate limit config
    pub fn get_config(self, config: &RateLimitConfig) -> TokenBucketConfig {
        match self {
            Self::Connection => config.connection_limit,
            Self::RestPublic => config.rest_public,
            Self::RestPrivate => config.rest_private,
            Self::WsOrders => config.ws_orders,
            Self::L3Depth10 => config.l3_depth_10,
            Self::L3Depth100 => config.l3_depth_100,
            Self::L3Depth1000 => config.l3_depth_1000,
        }
    }

    /// Get L3 depth category from depth value
    pub fn from_l3_depth(depth: u32) -> Self {
        match depth {
            0..=10 => Self::L3Depth10,
            11..=100 => Self::L3Depth100,
            _ => Self::L3Depth1000,
        }
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed,
    /// Request is rate limited, wait the specified duration
    Limited { wait: Duration, category: RateLimitCategory },
}

impl RateLimitResult {
    /// Check if the request is allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// Get the wait duration if rate limited
    pub fn wait_duration(&self) -> Option<Duration> {
        match self {
            Self::Allowed => None,
            Self::Limited { wait, .. } => Some(*wait),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(10, 1.0);
        assert_eq!(bucket.capacity(), 10);
        assert_eq!(bucket.refill_rate(), 1.0);
    }

    #[test]
    fn test_token_bucket_acquire() {
        let mut bucket = TokenBucket::new(10, 1.0);

        // Should succeed - we have 10 tokens
        assert!(bucket.try_acquire(5).is_ok());
        assert_eq!(bucket.available(), 5);

        // Should succeed - we have 5 tokens left
        assert!(bucket.try_acquire(5).is_ok());
        assert_eq!(bucket.available(), 0);

        // Should fail - no tokens left
        let result = bucket.try_acquire(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(10, 100.0); // 100 tokens per second

        // Consume all tokens
        assert!(bucket.try_acquire(10).is_ok());
        assert_eq!(bucket.available(), 0);

        // Wait a bit for refill (10ms = 1 token at 100/sec)
        std::thread::sleep(Duration::from_millis(15));

        // Should have some tokens back
        assert!(bucket.available() >= 1);
    }

    #[test]
    fn test_token_bucket_check_available() {
        let mut bucket = TokenBucket::new(10, 1.0);

        assert!(bucket.check_available(5));
        assert!(bucket.check_available(10));
        assert!(!bucket.check_available(11));
    }

    #[test]
    fn test_token_bucket_reset() {
        let mut bucket = TokenBucket::new(10, 1.0);

        bucket.try_acquire(10).unwrap();
        assert_eq!(bucket.available(), 0);

        bucket.reset();
        assert_eq!(bucket.available(), 10);
    }

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = RateLimitConfig::kraken_defaults();

        assert_eq!(config.connection_limit.capacity, 150);
        assert_eq!(config.ws_orders.capacity, 15);
        assert_eq!(config.rest_public.capacity, 15);
    }

    #[test]
    fn test_rate_limit_category() {
        let config = RateLimitConfig::kraken_defaults();

        let connection = RateLimitCategory::Connection.get_config(&config);
        assert_eq!(connection.capacity, 150);

        let ws_orders = RateLimitCategory::WsOrders.get_config(&config);
        assert_eq!(ws_orders.capacity, 15);
    }

    #[test]
    fn test_l3_depth_category() {
        assert_eq!(RateLimitCategory::from_l3_depth(10), RateLimitCategory::L3Depth10);
        assert_eq!(RateLimitCategory::from_l3_depth(25), RateLimitCategory::L3Depth100);
        assert_eq!(RateLimitCategory::from_l3_depth(500), RateLimitCategory::L3Depth1000);
    }

    #[test]
    fn test_rate_limit_result() {
        let allowed = RateLimitResult::Allowed;
        assert!(allowed.is_allowed());
        assert!(allowed.wait_duration().is_none());

        let limited = RateLimitResult::Limited {
            wait: Duration::from_secs(5),
            category: RateLimitCategory::RestPublic,
        };
        assert!(!limited.is_allowed());
        assert_eq!(limited.wait_duration(), Some(Duration::from_secs(5)));
    }
}
