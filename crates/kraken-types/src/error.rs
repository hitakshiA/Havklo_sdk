//! Error types for Kraken SDK

use std::time::Duration;
use thiserror::Error;

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
        matches!(
            self,
            Self::ConnectionFailed { .. }
                | Self::ConnectionTimeout { .. }
                | Self::RateLimited { .. }
                | Self::WebSocket(_)
                | Self::ChecksumMismatch { .. }
        )
    }

    /// Returns suggested retry delay, if applicable
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after } => Some(*retry_after),
            Self::CloudflareLimit => Some(Duration::from_secs(600)),
            Self::ConnectionFailed { .. } => Some(Duration::from_millis(100)),
            Self::ConnectionTimeout { .. } => Some(Duration::from_millis(500)),
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
}
