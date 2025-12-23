//! Error types for REST API operations

use kraken_types::error_codes::{KrakenApiError, RecoveryStrategy};

/// Errors that can occur during REST API operations
#[derive(Debug, thiserror::Error)]
pub enum RestError {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid API credentials
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    /// Missing API credentials for private endpoint
    #[error("Authentication required for this endpoint")]
    AuthRequired,

    /// API returned an error
    #[error("API error: {message}")]
    Api {
        /// Parsed error
        error: KrakenApiError,
        /// Original error message from API
        message: String,
    },

    /// Failed to parse response
    #[error("Parse error: {0}")]
    Parse(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded, retry after {retry_after_ms}ms")]
    RateLimited {
        /// Milliseconds to wait before retrying
        retry_after_ms: u64,
    },

    /// Request timed out
    #[error("Request timed out")]
    Timeout,

    /// Invalid request parameters
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Environment variable not set
    #[error("Environment variable not set: {0}")]
    EnvVarNotSet(String),
}

impl RestError {
    /// Create an API error from error strings returned by Kraken
    pub fn from_api_errors(errors: Vec<String>) -> Self {
        let message = errors.join(", ");
        let error = if !errors.is_empty() {
            KrakenApiError::parse(&errors[0])
        } else {
            KrakenApiError::parse("Unknown error")
        };

        Self::Api { error, message }
    }

    /// Get the recovery strategy for this error
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            Self::Api { error, .. } => error.recovery_strategy(),
            Self::RateLimited { retry_after_ms } => RecoveryStrategy::Backoff {
                initial_ms: *retry_after_ms,
                max_ms: *retry_after_ms * 2,
                multiplier: 1,
            },
            Self::Timeout => RecoveryStrategy::Retry {
                max_attempts: 3,
                delay_ms: 1000,
            },
            Self::Http(_) => RecoveryStrategy::Retry {
                max_attempts: 3,
                delay_ms: 1000,
            },
            Self::InvalidCredentials(_) | Self::AuthRequired => RecoveryStrategy::Fatal,
            Self::Parse(_) | Self::InvalidParameter(_) | Self::EnvVarNotSet(_) => {
                RecoveryStrategy::Fatal
            }
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        self.recovery_strategy().allows_retry()
    }

    /// Check if this error indicates rate limiting
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::RateLimited { .. })
            || matches!(
                self,
                Self::Api { error, .. } if error.is_rate_limit()
            )
    }
}

/// Result type for REST operations
pub type RestResult<T> = Result<T, RestError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_api_error() {
        let err = RestError::from_api_errors(vec!["EAPI:Rate limit exceeded".to_string()]);
        assert!(err.is_rate_limited());
        assert!(err.is_retryable());
    }

    #[test]
    fn test_recovery_strategies() {
        let rate_limited = RestError::RateLimited { retry_after_ms: 1000 };
        assert!(rate_limited.is_retryable());

        let auth_error = RestError::AuthRequired;
        assert!(!auth_error.is_retryable());
    }
}
