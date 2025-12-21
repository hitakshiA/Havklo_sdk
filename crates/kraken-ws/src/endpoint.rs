//! WebSocket endpoint definitions

use std::fmt;

/// Kraken WebSocket API v2 endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Endpoint {
    /// Public market data (default)
    #[default]
    Public,
    /// Authenticated private data
    Private,
    /// Public beta/testing
    PublicBeta,
    /// Authenticated beta/testing
    PrivateBeta,
    /// Level 3 full orderbook (requires special access)
    Level3,
}

impl Endpoint {
    /// Get the WebSocket URL for this endpoint
    pub fn url(&self) -> &'static str {
        match self {
            Self::Public => "wss://ws.kraken.com/v2",
            Self::Private => "wss://ws-auth.kraken.com/v2",
            Self::PublicBeta => "wss://beta-ws.kraken.com/v2",
            Self::PrivateBeta => "wss://beta-ws-auth.kraken.com/v2",
            Self::Level3 => "wss://ws-l3.kraken.com/v2",
        }
    }

    /// Check if this endpoint requires authentication
    pub fn requires_auth(&self) -> bool {
        matches!(self, Self::Private | Self::PrivateBeta)
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.url())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_urls() {
        assert_eq!(Endpoint::Public.url(), "wss://ws.kraken.com/v2");
        assert_eq!(Endpoint::Private.url(), "wss://ws-auth.kraken.com/v2");
    }

    #[test]
    fn test_requires_auth() {
        assert!(!Endpoint::Public.requires_auth());
        assert!(Endpoint::Private.requires_auth());
        assert!(!Endpoint::PublicBeta.requires_auth());
        assert!(Endpoint::PrivateBeta.requires_auth());
    }
}
