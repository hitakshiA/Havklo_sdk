//! Error types for Kraken SDK

use std::time::Duration;
use thiserror::Error;

use crate::error_codes::{KrakenApiError as ParsedApiError, KrakenErrorCode, RecoveryStrategy};

/// Main error type for Kraken SDK operations
#[derive(Error, Debug)]
pub enum KrakenError {
    // === Connection Errors ===
    /// Failed to establish WebSocket connection
    #[error("Failed to connect to {url}: {source}")]
    ConnectionFailed {
        url: String,
        #[source]
        source: std::io::Error,
    },

    /// Connection attempt timed out
    #[error("Connection timeout after {timeout:?} to {url}")]
    ConnectionTimeout { url: String, timeout: Duration },

    /// WebSocket protocol error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    // === Protocol Errors ===
    /// Failed to parse JSON message
    #[error("Invalid JSON: {message}")]
    InvalidJson { message: String, raw: Option<String> },

    /// Orderbook checksum mismatch
    #[error("Checksum mismatch for {symbol}: expected {expected}, computed {computed}")]
    ChecksumMismatch {
        symbol: String,
        expected: u32,
        computed: u32,
    },

    /// Unexpected message format
    #[error("Unexpected message format: {0}")]
    UnexpectedMessage(String),

    // === Subscription Errors ===
    /// Subscription was rejected by server
    #[error("Subscription rejected for {channel}: {reason}")]
    SubscriptionRejected { channel: String, reason: String },

    /// Symbol not found or not supported
    #[error("Symbol not found: {symbol}")]
    SymbolNotFound { symbol: String },

    /// Subscription request timed out
    #[error("Subscription timeout: no response within {timeout:?}")]
    SubscriptionTimeout { timeout: Duration },

    // === Authentication Errors ===
    /// Authentication failed
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    /// Token has expired
    #[error("Token expired, please re-authenticate")]
    TokenExpired,

    // === Rate Limit Errors ===
    /// Rate limited by server
    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Duration },

    /// Cloudflare connection limit exceeded
    #[error("Cloudflare connection limit exceeded, wait 10 minutes")]
    CloudflareLimit,

    // === API Errors (from Kraken responses) ===
    /// Parsed Kraken API error with structured recovery
    #[error("Kraken API error: {message}")]
    ApiError {
        /// The parsed error code (if recognized)
        code: Option<KrakenErrorCode>,
        /// Human-readable message
        message: String,
        /// Raw error string from Kraken
        raw: String,
        /// Recovery strategy for this error
        recovery: RecoveryStrategy,
    },

    // === Internal Errors ===
    /// Internal channel was closed unexpectedly
    #[error("Internal channel closed unexpectedly")]
    ChannelClosed,

    /// SDK is shutting down
    #[error("Shutdown in progress")]
    ShuttingDown,

    /// Invalid state transition
    #[error("Invalid state: expected {expected}, got {actual}")]
    InvalidState { expected: String, actual: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl KrakenError {
    /// Returns true if this error is potentially recoverable via retry
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ConnectionFailed { .. }
            | Self::ConnectionTimeout { .. }
            | Self::RateLimited { .. }
            | Self::WebSocket(_)
            | Self::ChecksumMismatch { .. } => true,
            Self::ApiError { recovery, .. } => recovery.allows_retry(),
            _ => false,
        }
    }

    /// Returns suggested retry delay, if applicable
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after } => Some(*retry_after),
            Self::CloudflareLimit => Some(Duration::from_secs(600)),
            Self::ConnectionFailed { .. } => Some(Duration::from_millis(100)),
            Self::ConnectionTimeout { .. } => Some(Duration::from_millis(500)),
            Self::ApiError { recovery, .. } => recovery.initial_delay(),
            _ => None,
        }
    }

    /// Returns true if this error requires reconnection
    pub fn requires_reconnect(&self) -> bool {
        matches!(
            self,
            Self::WebSocket(_) | Self::ConnectionFailed { .. } | Self::ChannelClosed
        )
    }

    /// Returns true if this error requires re-authentication
    pub fn requires_reauth(&self) -> bool {
        match self {
            Self::TokenExpired | Self::AuthenticationFailed { .. } => true,
            Self::ApiError { recovery, .. } => {
                matches!(recovery, RecoveryStrategy::Reauthenticate)
            }
            _ => false,
        }
    }

    /// Returns true if this is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        match self {
            Self::RateLimited { .. } | Self::CloudflareLimit => true,
            Self::ApiError { code, .. } => {
                code.map(|c| c.is_rate_limit()).unwrap_or(false)
            }
            _ => false,
        }
    }

    /// Get the recovery strategy for this error
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            Self::ApiError { recovery, .. } => recovery.clone(),
            Self::RateLimited { .. } | Self::CloudflareLimit => {
                RecoveryStrategy::rate_limit_backoff()
            }
            Self::ConnectionFailed { .. } | Self::ConnectionTimeout { .. } | Self::WebSocket(_) => {
                RecoveryStrategy::Backoff {
                    initial_ms: 100,
                    max_ms: 30000,
                    multiplier: 2,
                }
            }
            Self::ChecksumMismatch { .. } => RecoveryStrategy::RequestSnapshot,
            Self::TokenExpired | Self::AuthenticationFailed { .. } => {
                RecoveryStrategy::Reauthenticate
            }
            Self::ChannelClosed | Self::ShuttingDown => RecoveryStrategy::Fatal,
            Self::InvalidState { .. } | Self::Configuration(_) => RecoveryStrategy::Fatal,
            Self::InvalidJson { .. } | Self::UnexpectedMessage(_) => RecoveryStrategy::Skip,
            Self::SubscriptionRejected { .. }
            | Self::SymbolNotFound { .. }
            | Self::SubscriptionTimeout { .. } => RecoveryStrategy::Skip,
        }
    }

    /// Create a checksum mismatch error
    pub fn checksum_mismatch(symbol: impl Into<String>, expected: u32, computed: u32) -> Self {
        Self::ChecksumMismatch {
            symbol: symbol.into(),
            expected,
            computed,
        }
    }

    /// Create a subscription rejected error
    pub fn subscription_rejected(channel: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::SubscriptionRejected {
            channel: channel.into(),
            reason: reason.into(),
        }
    }

    /// Create an API error from a Kraken error string
    ///
    /// This parses the error string and determines the appropriate recovery strategy.
    pub fn from_api_error(error: impl AsRef<str>) -> Self {
        let parsed = ParsedApiError::parse(error.as_ref());
        Self::ApiError {
            code: parsed.code,
            message: parsed.message.clone(),
            raw: parsed.raw.clone(),
            recovery: parsed.recovery_strategy(),
        }
    }

    /// Create an API error from multiple Kraken error strings
    ///
    /// Returns the first error if multiple are present (Kraken returns arrays).
    pub fn from_api_errors(errors: &[String]) -> Self {
        if errors.is_empty() {
            return Self::ApiError {
                code: None,
                message: "Unknown error".to_string(),
                raw: String::new(),
                recovery: RecoveryStrategy::Manual,
            };
        }
        Self::from_api_error(&errors[0])
    }

    /// Get the error code if this is an API error
    pub fn error_code(&self) -> Option<KrakenErrorCode> {
        match self {
            Self::ApiError { code, .. } => *code,
            _ => None,
        }
    }
}

/// Result type alias for Kraken operations
pub type KrakenResult<T> = Result<T, KrakenError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let err = KrakenError::RateLimited {
            retry_after: Duration::from_secs(5),
        };
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(Duration::from_secs(5)));

        let err = KrakenError::AuthenticationFailed {
            reason: "bad token".into(),
        };
        assert!(!err.is_retryable());
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn test_error_requires_reconnect() {
        let err = KrakenError::WebSocket("connection reset".into());
        assert!(err.requires_reconnect());

        let err = KrakenError::ChecksumMismatch {
            symbol: "BTC/USD".into(),
            expected: 123,
            computed: 456,
        };
        assert!(!err.requires_reconnect());
    }

    #[test]
    fn test_from_api_error() {
        let err = KrakenError::from_api_error("EAPI:Rate limit exceeded");
        assert!(err.is_rate_limit());
        assert!(err.is_retryable());
        assert_eq!(err.error_code(), Some(KrakenErrorCode::RateLimitExceeded));
    }

    #[test]
    fn test_from_api_error_auth() {
        let err = KrakenError::from_api_error("EAPI:Invalid key");
        assert!(err.requires_reauth());
        assert_eq!(err.error_code(), Some(KrakenErrorCode::InvalidKey));
    }

    #[test]
    fn test_from_api_error_insufficient_funds() {
        let err = KrakenError::from_api_error("EOrder:Insufficient funds");
        assert!(!err.is_retryable());
        assert!(matches!(
            err.recovery_strategy(),
            RecoveryStrategy::UserAction { .. }
        ));
    }

    #[test]
    fn test_from_api_errors() {
        let errors = vec![
            "EOrder:Insufficient funds".to_string(),
            "EOrder:Order minimum not met".to_string(),
        ];
        let err = KrakenError::from_api_errors(&errors);
        assert_eq!(err.error_code(), Some(KrakenErrorCode::InsufficientFunds));
    }

    #[test]
    fn test_recovery_strategy() {
        let err = KrakenError::ChecksumMismatch {
            symbol: "BTC/USD".into(),
            expected: 123,
            computed: 456,
        };
        assert!(matches!(
            err.recovery_strategy(),
            RecoveryStrategy::RequestSnapshot
        ));

        let err = KrakenError::TokenExpired;
        assert!(matches!(
            err.recovery_strategy(),
            RecoveryStrategy::Reauthenticate
        ));
    }
}
