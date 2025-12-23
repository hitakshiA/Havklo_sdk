//! Circuit Breaker Pattern
//!
//! Prevents a system from repeatedly trying to execute an operation that's
//! likely to fail, giving the external service time to recover.
//!
//! # States
//!
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Circuit is tripped after failures, requests fail immediately
//! - **HalfOpen**: After timeout, one request is allowed to test recovery
//!
//! # Example
//!
//! ```
//! use kraken_ws::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
//! use std::time::Duration;
//!
//! let config = CircuitBreakerConfig {
//!     failure_threshold: 5,
//!     success_threshold: 2,
//!     timeout: Duration::from_secs(30),
//! };
//!
//! let breaker = CircuitBreaker::new(config);
//!
//! // Record failures
//! for _ in 0..5 {
//!     breaker.record_failure();
//! }
//!
//! // Circuit is now open
//! assert!(!breaker.allow_request());
//! ```

use parking_lot::RwLock;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Circuit tripped - requests fail immediately
    Open,
    /// Testing if service recovered - limited requests allowed
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

/// Configuration for the circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures to trip the circuit
    pub failure_threshold: u32,
    /// Number of consecutive successes to close the circuit (in half-open state)
    pub success_threshold: u32,
    /// Time to wait before transitioning from Open to HalfOpen
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new config with custom thresholds
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
        }
    }

    /// Create a sensitive config (trips quickly, recovers slowly)
    pub fn sensitive() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
        }
    }

    /// Create a resilient config (tolerates more failures)
    pub fn resilient() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 1,
            timeout: Duration::from_secs(15),
        }
    }
}

/// Internal state for the circuit breaker
struct InnerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    total_failures: u64,
    total_successes: u64,
    trips: u64,
}

impl Default for InnerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            total_failures: 0,
            total_successes: 0,
            trips: 0,
        }
    }
}

/// Circuit breaker for connection reliability
///
/// Tracks failures and successes to automatically trip when a service
/// appears unhealthy, preventing cascade failures.
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: RwLock<InnerState>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(InnerState::default()),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Get the current circuit state
    pub fn state(&self) -> CircuitState {
        let mut state = self.state.write();
        self.maybe_transition_to_half_open(&mut state);
        state.state
    }

    /// Check if a request should be allowed
    ///
    /// Returns `true` if the request can proceed, `false` if it should be rejected.
    pub fn allow_request(&self) -> bool {
        let mut state = self.state.write();
        self.maybe_transition_to_half_open(&mut state);

        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true, // Allow test request
        }
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        let mut state = self.state.write();
        state.total_successes += 1;

        match state.state {
            CircuitState::Closed => {
                // Reset failure count on success
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                debug!(
                    "Circuit breaker: success in half-open state ({}/{})",
                    state.success_count, self.config.success_threshold
                );

                if state.success_count >= self.config.success_threshold {
                    info!("Circuit breaker: closing circuit after successful recovery");
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                state.failure_count = 0;
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&self) {
        let mut state = self.state.write();
        state.total_failures += 1;
        state.failure_count += 1;
        state.last_failure_time = Some(Instant::now());

        match state.state {
            CircuitState::Closed => {
                debug!(
                    "Circuit breaker: failure {}/{}",
                    state.failure_count, self.config.failure_threshold
                );

                if state.failure_count >= self.config.failure_threshold {
                    warn!(
                        "Circuit breaker: opening circuit after {} consecutive failures",
                        state.failure_count
                    );
                    state.state = CircuitState::Open;
                    state.trips += 1;
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker: failure in half-open state, reopening circuit");
                state.state = CircuitState::Open;
                state.success_count = 0;
            }
            CircuitState::Open => {
                // Already open, just update failure time
            }
        }
    }

    /// Check if we should transition from Open to HalfOpen
    fn maybe_transition_to_half_open(&self, state: &mut InnerState) {
        if state.state == CircuitState::Open {
            if let Some(last_failure) = state.last_failure_time {
                if last_failure.elapsed() >= self.config.timeout {
                    info!(
                        "Circuit breaker: timeout elapsed, transitioning to half-open state"
                    );
                    state.state = CircuitState::HalfOpen;
                    state.success_count = 0;
                    state.failure_count = 0;
                }
            }
        }
    }

    /// Force the circuit to open (manual trip)
    pub fn trip(&self) {
        let mut state = self.state.write();
        if state.state != CircuitState::Open {
            warn!("Circuit breaker: manually tripped");
            state.state = CircuitState::Open;
            state.last_failure_time = Some(Instant::now());
            state.trips += 1;
        }
    }

    /// Force the circuit to close (manual reset)
    pub fn reset(&self) {
        let mut state = self.state.write();
        info!("Circuit breaker: manually reset");
        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
    }

    /// Get statistics about the circuit breaker
    pub fn stats(&self) -> CircuitBreakerStats {
        let state = self.state.read();
        CircuitBreakerStats {
            state: state.state,
            total_failures: state.total_failures,
            total_successes: state.total_successes,
            consecutive_failures: state.failure_count,
            trips: state.trips,
            last_failure: state.last_failure_time,
        }
    }

    /// Check if the circuit is currently open (blocking requests)
    pub fn is_open(&self) -> bool {
        self.state() == CircuitState::Open
    }

    /// Check if the circuit is closed (normal operation)
    pub fn is_closed(&self) -> bool {
        self.state() == CircuitState::Closed
    }
}

/// Statistics about circuit breaker operation
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Current state
    pub state: CircuitState,
    /// Total failures recorded
    pub total_failures: u64,
    /// Total successes recorded
    pub total_successes: u64,
    /// Current consecutive failure count
    pub consecutive_failures: u32,
    /// Number of times the circuit has been tripped
    pub trips: u64,
    /// Time of last failure
    pub last_failure: Option<Instant>,
}

impl CircuitBreakerStats {
    /// Calculate failure rate (failures / total)
    pub fn failure_rate(&self) -> f64 {
        let total = self.total_failures + self.total_successes;
        if total == 0 {
            0.0
        } else {
            self.total_failures as f64 / total as f64
        }
    }

    /// Time since last failure (if any)
    pub fn time_since_last_failure(&self) -> Option<Duration> {
        self.last_failure.map(|t| t.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_trips_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 1,
            timeout: Duration::from_secs(1),
        };
        let breaker = CircuitBreaker::new(config);

        assert!(breaker.is_closed());
        assert!(breaker.allow_request());

        // Record failures up to threshold
        breaker.record_failure();
        breaker.record_failure();
        assert!(breaker.is_closed());

        breaker.record_failure();
        assert!(breaker.is_open());
        assert!(!breaker.allow_request());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 1,
            timeout: Duration::from_secs(1),
        };
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure();
        breaker.record_failure();
        breaker.record_success(); // Should reset count

        assert!(breaker.is_closed());

        breaker.record_failure();
        breaker.record_failure();
        assert!(breaker.is_closed()); // Still closed, count was reset
    }

    #[test]
    fn test_circuit_breaker_stats() {
        let breaker = CircuitBreaker::with_defaults();

        breaker.record_success();
        breaker.record_success();
        breaker.record_failure();

        let stats = breaker.stats();
        assert_eq!(stats.total_successes, 2);
        assert_eq!(stats.total_failures, 1);
        assert!((stats.failure_rate() - 0.333).abs() < 0.01);
    }
}
