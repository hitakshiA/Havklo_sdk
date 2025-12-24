//! Client Builder Pattern
//!
//! Provides a fluent builder API for configuring the Havklo SDK client
//! with sensible defaults and validation.
//!
//! # Example
//!
//! ```
//! use kraken_sdk::builder::KrakenClientBuilder;
//! use kraken_types::Depth;
//!
//! let builder = KrakenClientBuilder::new(["BTC/USD", "ETH/USD"])
//!     .with_depth(Depth::D25)
//!     .with_ticker(true)
//!     .with_trade(true);
//! ```

use crate::filter::EventFilter;
use kraken_types::{Channel, Depth};
use kraken_ws::{ConnectionConfig, Endpoint, ReconnectConfig};
use std::collections::HashSet;
use std::time::Duration;

/// Configuration validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    /// No symbols specified
    #[error("at least one symbol must be specified")]
    NoSymbols,

    /// Invalid symbol format
    #[error("invalid symbol format: {symbol} (expected format: BASE/QUOTE, e.g., BTC/USD)")]
    InvalidSymbol { symbol: String },

    /// L3 requires special endpoint
    #[error("L3 subscription requires Level3 endpoint")]
    L3RequiresLevel3Endpoint,

    /// OHLC interval not supported
    #[error("invalid OHLC interval: {interval} (supported: 1, 5, 15, 30, 60, 240, 1440, 10080, 21600)")]
    InvalidOhlcInterval { interval: u32 },

    /// Depth not valid
    #[error("invalid depth: {depth} (supported: 10, 25, 100, 500, 1000)")]
    InvalidDepth { depth: u32 },

    /// Timeout too short
    #[error("connection timeout must be at least 1 second")]
    TimeoutTooShort,
}

/// OHLC (candlestick) interval in minutes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OhlcInterval {
    /// 1 minute
    M1 = 1,
    /// 5 minutes
    M5 = 5,
    /// 15 minutes
    M15 = 15,
    /// 30 minutes
    M30 = 30,
    /// 1 hour
    H1 = 60,
    /// 4 hours
    H4 = 240,
    /// 1 day
    D1 = 1440,
    /// 1 week
    W1 = 10080,
    /// 15 days
    D15 = 21600,
}

impl OhlcInterval {
    /// Get the interval in minutes
    pub fn as_minutes(&self) -> u32 {
        *self as u32
    }
}

/// Builder for configuring a Kraken client
///
/// Provides a fluent API for setting up the client with various options:
/// - Symbol subscriptions
/// - Channel subscriptions (orderbook, ticker, trade, OHLC, L3)
/// - Connection settings (timeout, reconnection)
/// - Event filtering
#[derive(Debug, Clone)]
pub struct KrakenClientBuilder {
    /// Symbols to subscribe to
    pub symbols: Vec<String>,

    /// L3 symbols (separate from regular book symbols)
    pub l3_symbols: Vec<String>,

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

    /// Subscribe to L3 channel
    pub subscribe_l3: bool,

    /// OHLC intervals to subscribe to
    pub ohlc_intervals: HashSet<OhlcInterval>,

    /// Event filter (optional)
    pub event_filter: Option<EventFilter>,

    /// Additional channels to subscribe to
    pub additional_channels: Vec<Channel>,

    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for KrakenClientBuilder {
    fn default() -> Self {
        Self {
            symbols: Vec::new(),
            l3_symbols: Vec::new(),
            depth: Depth::D10,
            endpoint: Endpoint::Public,
            reconnect: true,
            reconnect_config: ReconnectConfig::default(),
            connect_timeout: Duration::from_secs(10),
            subscribe_book: true,
            subscribe_ticker: false,
            subscribe_trade: false,
            subscribe_l3: false,
            ohlc_intervals: HashSet::new(),
            event_filter: None,
            additional_channels: Vec::new(),
            verbose: false,
        }
    }
}

impl KrakenClientBuilder {
    /// Create a new builder with the specified symbols
    ///
    /// # Example
    ///
    /// ```
    /// use kraken_sdk::builder::KrakenClientBuilder;
    ///
    /// let builder = KrakenClientBuilder::new(["BTC/USD", "ETH/USD"]);
    /// assert_eq!(builder.symbols.len(), 2);
    /// ```
    pub fn new(symbols: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            symbols: symbols.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }

    /// Add a single symbol
    ///
    /// # Example
    ///
    /// ```
    /// use kraken_sdk::builder::KrakenClientBuilder;
    ///
    /// let builder = KrakenClientBuilder::new(["BTC/USD"])
    ///     .with_symbol("ETH/USD");
    /// assert_eq!(builder.symbols.len(), 2);
    /// ```
    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbols.push(symbol.into());
        self
    }

    /// Add multiple symbols
    pub fn with_symbols(mut self, symbols: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.symbols.extend(symbols.into_iter().map(Into::into));
        self
    }

    /// Set the orderbook depth
    ///
    /// Available depths: D10, D25, D100, D500, D1000
    pub fn with_depth(mut self, depth: Depth) -> Self {
        self.depth = depth;
        self
    }

    /// Set the WebSocket endpoint
    ///
    /// Use `Endpoint::Level3` for L3 orderbook data.
    pub fn with_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.endpoint = endpoint;
        self
    }

    /// Enable or disable automatic reconnection
    pub fn with_reconnect(mut self, enabled: bool) -> Self {
        self.reconnect = enabled;
        self
    }

    /// Disable automatic reconnection
    pub fn without_reconnect(mut self) -> Self {
        self.reconnect = false;
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

    /// Subscribe to the orderbook channel
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

    /// Subscribe to L3 orderbook updates for specific symbols
    ///
    /// Note: L3 data requires connection to the Level3 endpoint.
    /// Call `.with_endpoint(Endpoint::Level3)` to enable.
    pub fn with_l3(mut self, symbols: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.l3_symbols = symbols.into_iter().map(Into::into).collect();
        self.subscribe_l3 = true;
        self
    }

    /// Enable L3 orderbook for all configured symbols
    pub fn with_l3_enabled(mut self) -> Self {
        self.subscribe_l3 = true;
        self
    }

    /// Subscribe to OHLC (candlestick) data
    pub fn with_ohlc(mut self, interval: OhlcInterval) -> Self {
        self.ohlc_intervals.insert(interval);
        self
    }

    /// Subscribe to multiple OHLC intervals
    pub fn with_ohlc_intervals(
        mut self,
        intervals: impl IntoIterator<Item = OhlcInterval>,
    ) -> Self {
        self.ohlc_intervals.extend(intervals);
        self
    }

    /// Set an event filter
    ///
    /// Events that don't match the filter will be dropped.
    pub fn with_filter(mut self, filter: EventFilter) -> Self {
        self.event_filter = Some(filter);
        self
    }

    /// Subscribe to additional channels
    pub fn with_channels(mut self, channels: impl IntoIterator<Item = Channel>) -> Self {
        self.additional_channels.extend(channels);
        self
    }

    /// Enable verbose logging
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Subscribe to all market data channels (book, ticker, trade)
    pub fn all_market_data(mut self) -> Self {
        self.subscribe_book = true;
        self.subscribe_ticker = true;
        self.subscribe_trade = true;
        self
    }

    /// Validate the configuration
    ///
    /// Returns `Ok(())` if the configuration is valid, otherwise returns
    /// a `ConfigError` describing the problem.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check for at least one symbol
        if self.symbols.is_empty() && self.l3_symbols.is_empty() {
            return Err(ConfigError::NoSymbols);
        }

        // Validate symbol format
        for symbol in &self.symbols {
            if !Self::is_valid_symbol(symbol) {
                return Err(ConfigError::InvalidSymbol {
                    symbol: symbol.clone(),
                });
            }
        }

        for symbol in &self.l3_symbols {
            if !Self::is_valid_symbol(symbol) {
                return Err(ConfigError::InvalidSymbol {
                    symbol: symbol.clone(),
                });
            }
        }

        // Check L3 requires Level3 endpoint
        if self.subscribe_l3 && !matches!(self.endpoint, Endpoint::Level3) {
            return Err(ConfigError::L3RequiresLevel3Endpoint);
        }

        // Check timeout
        if self.connect_timeout < Duration::from_secs(1) {
            return Err(ConfigError::TimeoutTooShort);
        }

        Ok(())
    }

    /// Check if a symbol has valid format (BASE/QUOTE)
    fn is_valid_symbol(symbol: &str) -> bool {
        let parts: Vec<&str> = symbol.split('/').collect();
        if parts.len() != 2 {
            return false;
        }
        // Both parts should be non-empty and alphabetic (with possible numbers like BTC2)
        parts[0].len() >= 2 && parts[1].len() >= 2
    }

    /// Build and validate the configuration
    ///
    /// Returns the validated builder if successful, otherwise returns a `ConfigError`.
    pub fn build(self) -> Result<Self, ConfigError> {
        self.validate()?;
        Ok(self)
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

    /// Get all symbols including L3 symbols
    pub fn all_symbols(&self) -> Vec<&str> {
        self.symbols
            .iter()
            .chain(self.l3_symbols.iter())
            .map(String::as_str)
            .collect()
    }

    /// Check if any subscriptions are configured
    pub fn has_subscriptions(&self) -> bool {
        self.subscribe_book
            || self.subscribe_ticker
            || self.subscribe_trade
            || self.subscribe_l3
            || !self.ohlc_intervals.is_empty()
            || !self.additional_channels.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_fluent_api() {
        let builder = KrakenClientBuilder::new(["BTC/USD"])
            .with_symbol("ETH/USD")
            .with_depth(Depth::D25)
            .with_endpoint(Endpoint::PublicBeta)
            .all_market_data()
            .with_ohlc(OhlcInterval::M1);

        assert_eq!(builder.symbols.len(), 2);
        assert_eq!(builder.depth, Depth::D25);
        assert!(builder.subscribe_book);
        assert!(builder.subscribe_ticker);
        assert!(builder.subscribe_trade);
        assert!(builder.ohlc_intervals.contains(&OhlcInterval::M1));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_builder_validation() {
        // No symbols
        assert!(matches!(
            KrakenClientBuilder::default().validate(),
            Err(ConfigError::NoSymbols)
        ));

        // Invalid symbol format
        assert!(matches!(
            KrakenClientBuilder::new(["INVALID"]).validate(),
            Err(ConfigError::InvalidSymbol { .. })
        ));

        // L3 requires Level3 endpoint
        assert!(matches!(
            KrakenClientBuilder::new(["BTC/USD"]).with_l3(["BTC/USD"]).validate(),
            Err(ConfigError::L3RequiresLevel3Endpoint)
        ));

        // L3 with correct endpoint works
        assert!(KrakenClientBuilder::new(["BTC/USD"])
            .with_endpoint(Endpoint::Level3)
            .with_l3(["BTC/USD"])
            .validate()
            .is_ok());
    }

    #[test]
    fn test_ohlc_intervals() {
        assert_eq!(OhlcInterval::M1.as_minutes(), 1);
        assert_eq!(OhlcInterval::M5.as_minutes(), 5);
        assert_eq!(OhlcInterval::M15.as_minutes(), 15);
        assert_eq!(OhlcInterval::M30.as_minutes(), 30);
        assert_eq!(OhlcInterval::H1.as_minutes(), 60);
        assert_eq!(OhlcInterval::H4.as_minutes(), 240);
        assert_eq!(OhlcInterval::D1.as_minutes(), 1440);
        assert_eq!(OhlcInterval::W1.as_minutes(), 10080);
        assert_eq!(OhlcInterval::D15.as_minutes(), 21600);
    }

    #[test]
    fn test_builder_default_values() {
        let builder = KrakenClientBuilder::new(["BTC/USD"]);
        assert_eq!(builder.depth, Depth::D10);
        assert_eq!(builder.endpoint, Endpoint::Public);
        assert!(builder.subscribe_book); // Book is enabled by default
        assert!(!builder.subscribe_ticker);
        assert!(!builder.subscribe_trade);
        assert!(builder.l3_symbols.is_empty());
        assert!(builder.ohlc_intervals.is_empty());
    }

    #[test]
    fn test_builder_multiple_symbols() {
        let builder = KrakenClientBuilder::new(["BTC/USD", "ETH/USD", "SOL/USD"])
            .with_symbol("XRP/USD")
            .with_symbols(["DOT/USD", "LINK/USD"]);

        assert_eq!(builder.symbols.len(), 6);
        assert!(builder.symbols.contains(&"BTC/USD".to_string()));
        assert!(builder.symbols.contains(&"LINK/USD".to_string()));
    }

    #[test]
    fn test_builder_depth_levels() {
        let builder = KrakenClientBuilder::new(["BTC/USD"]).with_depth(Depth::D100);
        assert_eq!(builder.depth, Depth::D100);

        let builder = KrakenClientBuilder::new(["BTC/USD"]).with_depth(Depth::D500);
        assert_eq!(builder.depth, Depth::D500);

        let builder = KrakenClientBuilder::new(["BTC/USD"]).with_depth(Depth::D1000);
        assert_eq!(builder.depth, Depth::D1000);
    }

    #[test]
    fn test_builder_channel_subscriptions() {
        let builder = KrakenClientBuilder::new(["BTC/USD"])
            .with_book(true)
            .with_ticker(true)
            .with_trade(true);

        assert!(builder.subscribe_book);
        assert!(builder.subscribe_ticker);
        assert!(builder.subscribe_trade);

        // Test toggling off
        let builder = builder.with_book(false);
        assert!(!builder.subscribe_book);
    }

    #[test]
    fn test_builder_multiple_ohlc_intervals() {
        let builder = KrakenClientBuilder::new(["BTC/USD"])
            .with_ohlc(OhlcInterval::M1)
            .with_ohlc(OhlcInterval::M5)
            .with_ohlc(OhlcInterval::H1);

        assert_eq!(builder.ohlc_intervals.len(), 3);
        assert!(builder.ohlc_intervals.contains(&OhlcInterval::M1));
        assert!(builder.ohlc_intervals.contains(&OhlcInterval::M5));
        assert!(builder.ohlc_intervals.contains(&OhlcInterval::H1));
    }

    #[test]
    fn test_builder_has_subscriptions() {
        // Default has book enabled
        let builder = KrakenClientBuilder::new(["BTC/USD"]);
        assert!(builder.has_subscriptions());

        // Disabling book means no subscriptions
        let builder = builder.with_book(false);
        assert!(!builder.has_subscriptions());

        // Adding ticker enables subscriptions
        let builder = builder.with_ticker(true);
        assert!(builder.has_subscriptions());

        // OHLC also counts as subscription
        let builder = KrakenClientBuilder::new(["BTC/USD"])
            .with_book(false)
            .with_ohlc(OhlcInterval::M1);
        assert!(builder.has_subscriptions());
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::NoSymbols;
        assert_eq!(err.to_string(), "at least one symbol must be specified");

        let err = ConfigError::InvalidSymbol { symbol: "BAD".into() };
        assert!(err.to_string().contains("BAD"));

        let err = ConfigError::L3RequiresLevel3Endpoint;
        assert!(err.to_string().contains("Level3"));
    }

    #[test]
    fn test_symbol_validation_formats() {
        // Valid formats
        assert!(KrakenClientBuilder::new(["BTC/USD"]).validate().is_ok());
        assert!(KrakenClientBuilder::new(["ETH/EUR"]).validate().is_ok());
        assert!(KrakenClientBuilder::new(["SOL/USDT"]).validate().is_ok());

        // Invalid formats
        assert!(KrakenClientBuilder::new(["BTCUSD"]).validate().is_err());
        assert!(KrakenClientBuilder::new(["BTC"]).validate().is_err());
        assert!(KrakenClientBuilder::new([""]).validate().is_err());
    }
}
