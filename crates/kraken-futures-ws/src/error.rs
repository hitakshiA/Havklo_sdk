//! Error types for Futures WebSocket operations

use kraken_types::error_codes::{KrakenApiError, RecoveryStrategy};

/// Errors that can occur during Futures WebSocket operations
#[derive(Debug, thiserror::Error)]
pub enum FuturesError {
    /// WebSocket connection failed
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// JSON parsing error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// Invalid API credentials
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    /// Subscription failed
    #[error("Subscription failed: {channel} - {reason}")]
    SubscriptionFailed {
        /// Channel that failed
        channel: String,
        /// Reason for failure
        reason: String,
    },

    /// API returned an error
    #[error("API error: {message}")]
    Api {
        /// Parsed error
        error: KrakenApiError,
        /// Original error message
        message: String,
    },

    /// Connection closed
    #[error("Connection closed: {0}")]
    ConnectionClosed(String),

    /// Timeout waiting for response
    #[error("Timeout waiting for {0}")]
    Timeout(String),

    /// Invalid message format
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// Environment variable not set
    #[error("Environment variable not set: {0}")]
    EnvVarNotSet(String),

    /// Channel send error
    #[error("Channel closed")]
    ChannelClosed,
}

impl FuturesError {
    /// Create an API error from error string
    pub fn from_api_error(error: &str) -> Self {
        let parsed = KrakenApiError::parse(error);
        Self::Api {
            message: error.to_string(),
            error: parsed,
        }
    }

    /// Get the recovery strategy for this error
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            Self::Api { error, .. } => error.recovery_strategy(),
            Self::WebSocket(_) | Self::ConnectionClosed(_) => RecoveryStrategy::Retry {
                max_attempts: 5,
                delay_ms: 1000,
            },
            Self::Timeout(_) => RecoveryStrategy::Retry {
                max_attempts: 3,
                delay_ms: 2000,
            },
            Self::AuthFailed(_) | Self::InvalidCredentials(_) => RecoveryStrategy::Fatal,
            Self::SubscriptionFailed { .. } => RecoveryStrategy::Retry {
                max_attempts: 3,
                delay_ms: 1000,
            },
            Self::Json(_) | Self::InvalidMessage(_) => RecoveryStrategy::Skip,
            Self::EnvVarNotSet(_) | Self::ChannelClosed => RecoveryStrategy::Fatal,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        self.recovery_strategy().allows_retry()
    }
}

/// Result type for Futures operations
pub type FuturesResult<T> = Result<T, FuturesError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_not_retryable() {
        let err = FuturesError::AuthFailed("invalid signature".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_connection_error_retryable() {
        let err = FuturesError::ConnectionClosed("server closed".to_string());
        assert!(err.is_retryable());
    }
}
