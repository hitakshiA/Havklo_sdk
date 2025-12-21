//! Reconnection configuration with exponential backoff

use std::time::Duration;

/// Configuration for automatic reconnection with exponential backoff
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial delay before first reconnection attempt
    pub initial_delay: Duration,
    /// Maximum delay between reconnection attempts
    pub max_delay: Duration,
    /// Multiplier for exponential backoff (e.g., 2.0 doubles delay each attempt)
    pub multiplier: f64,
    /// Random jitter factor (0.0 to 1.0) to prevent thundering herd
    pub jitter: f64,
    /// Maximum number of reconnection attempts (None = unlimited)
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            jitter: 0.2,
            max_attempts: None, // Retry forever
        }
    }
}

impl ReconnectConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set maximum delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set backoff multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Set jitter factor
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter = jitter.clamp(0.0, 1.0);
        self
    }

    /// Set maximum attempts
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = Some(max);
        self
    }

    /// Disable reconnection
    pub fn disabled() -> Self {
        Self {
            max_attempts: Some(0),
            ..Default::default()
        }
    }

    /// Calculate delay for a given attempt number (1-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let exponent = attempt.saturating_sub(1) as i32;
        let delay_ms = self.initial_delay.as_millis() as f64 * self.multiplier.powi(exponent);
        let delay = Duration::from_millis(delay_ms as u64);

        std::cmp::min(delay, self.max_delay)
    }

    /// Apply jitter to a base delay
    pub fn apply_jitter(&self, base: Duration) -> Duration {
        if self.jitter == 0.0 {
            return base;
        }

        let jitter_range = base.as_millis() as f64 * self.jitter;
        let jitter = rand::random::<f64>() * 2.0 * jitter_range - jitter_range;
        let adjusted_ms = (base.as_millis() as f64 + jitter).max(0.0) as u64;

        Duration::from_millis(adjusted_ms)
    }

    /// Get delay with jitter applied for a given attempt
    pub fn delay_with_jitter(&self, attempt: u32) -> Duration {
        let base = self.delay_for_attempt(attempt);
        self.apply_jitter(base)
    }

    /// Check if should attempt reconnection
    pub fn should_reconnect(&self, attempt: u32) -> bool {
        match self.max_attempts {
            Some(max) => attempt < max,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ReconnectConfig::default();
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.multiplier, 2.0);
        assert!(config.max_attempts.is_none());
    }

    #[test]
    fn test_delay_calculation() {
        let config = ReconnectConfig::new()
            .with_initial_delay(Duration::from_millis(100))
            .with_multiplier(2.0)
            .with_max_delay(Duration::from_secs(10))
            .with_jitter(0.0); // Disable jitter for predictable tests

        assert_eq!(config.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(config.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(config.delay_for_attempt(3), Duration::from_millis(400));
        assert_eq!(config.delay_for_attempt(4), Duration::from_millis(800));

        // Should cap at max_delay
        assert_eq!(config.delay_for_attempt(10), Duration::from_secs(10));
    }

    #[test]
    fn test_should_reconnect() {
        let unlimited = ReconnectConfig::default();
        assert!(unlimited.should_reconnect(0));
        assert!(unlimited.should_reconnect(100));

        let limited = ReconnectConfig::default().with_max_attempts(3);
        assert!(limited.should_reconnect(0));
        assert!(limited.should_reconnect(2));
        assert!(!limited.should_reconnect(3));
        assert!(!limited.should_reconnect(10));

        let disabled = ReconnectConfig::disabled();
        assert!(!disabled.should_reconnect(0));
    }
}
