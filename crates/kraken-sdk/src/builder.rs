//! Client builder pattern with typed-builder

use kraken_types::Depth;
use kraken_ws::{ConnectionConfig, Endpoint, ReconnectConfig};
use std::time::Duration;

/// Builder for configuring a Kraken client
#[derive(Debug, Clone)]
pub struct KrakenClientBuilder {
    /// Symbols to subscribe to
    pub symbols: Vec<String>,

    /// Orderbook depth
    pub depth: Depth,

    /// WebSocket endpoint
    pub endpoint: Endpoint,

    /// Enable automatic reconnection
    pub reconnect: bool,

    /// Reconnection configuration
    pub reconnect_config: ReconnectConfig,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Subscribe to orderbook channel
    pub subscribe_book: bool,

    /// Subscribe to ticker channel
    pub subscribe_ticker: bool,

    /// Subscribe to trade channel
    pub subscribe_trade: bool,
}

impl KrakenClientBuilder {
    /// Create a new builder with the specified symbols
    pub fn new(symbols: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            symbols: symbols.into_iter().map(Into::into).collect(),
            depth: Depth::D10,
            endpoint: Endpoint::Public,
            reconnect: true,
            reconnect_config: ReconnectConfig::default(),
            connect_timeout: Duration::from_secs(10),
            subscribe_book: true,
            subscribe_ticker: false,
            subscribe_trade: false,
        }
    }

    /// Set the orderbook depth
    pub fn with_depth(mut self, depth: Depth) -> Self {
        self.depth = depth;
        self
    }

    /// Set the WebSocket endpoint
    pub fn with_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.endpoint = endpoint;
        self
    }

    /// Enable or disable automatic reconnection
    pub fn with_reconnect(mut self, enabled: bool) -> Self {
        self.reconnect = enabled;
        self
    }

    /// Set the reconnection configuration
    pub fn with_reconnect_config(mut self, config: ReconnectConfig) -> Self {
        self.reconnect_config = config;
        self
    }

    /// Set the connection timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Subscribe to the book channel
    pub fn with_book(mut self, enabled: bool) -> Self {
        self.subscribe_book = enabled;
        self
    }

    /// Subscribe to the ticker channel
    pub fn with_ticker(mut self, enabled: bool) -> Self {
        self.subscribe_ticker = enabled;
        self
    }

    /// Subscribe to the trade channel
    pub fn with_trade(mut self, enabled: bool) -> Self {
        self.subscribe_trade = enabled;
        self
    }

    /// Convert to connection config
    pub fn to_connection_config(&self) -> ConnectionConfig {
        let mut config = ConnectionConfig::new()
            .with_endpoint(self.endpoint)
            .with_depth(self.depth)
            .with_timeout(self.connect_timeout);

        if self.reconnect {
            config = config.with_reconnect(self.reconnect_config.clone());
        } else {
            config = config.without_reconnect();
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let builder = KrakenClientBuilder::new(["BTC/USD", "ETH/USD"])
            .with_depth(Depth::D25)
            .with_endpoint(Endpoint::PublicBeta)
            .with_reconnect(true);

        assert_eq!(builder.symbols.len(), 2);
        assert_eq!(builder.depth, Depth::D25);
        assert_eq!(builder.endpoint, Endpoint::PublicBeta);
        assert!(builder.reconnect);
    }
}
