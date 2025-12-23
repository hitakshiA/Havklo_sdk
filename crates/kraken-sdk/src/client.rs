//! High-level Kraken client

use crate::builder::KrakenClientBuilder;
use kraken_book::Orderbook;
use kraken_types::KrakenError;
use kraken_ws::{ConnectionState, EventReceiver, KrakenConnection};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{info, instrument};

/// High-level client for Kraken WebSocket API
///
/// Provides a simple interface for connecting to Kraken and accessing
/// orderbook data with automatic reconnection handling.
///
/// # Example
///
/// ```no_run
/// use kraken_sdk::KrakenClient;
/// use kraken_types::Depth;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut client = KrakenClient::builder(["BTC/USD", "ETH/USD"])
///         .with_depth(Depth::D10)
///         .connect()
///         .await?;
///
///     // Get orderbook data
///     if let Some(spread) = client.spread("BTC/USD") {
///         println!("BTC/USD spread: {}", spread);
///     }
///
///     // Process events
///     let mut events = client.events().unwrap();
///     while let Some(event) = events.recv().await {
///         println!("{:?}", event);
///     }
///
///     Ok(())
/// }
/// ```
pub struct KrakenClient {
    /// Underlying connection
    connection: Arc<KrakenConnection>,
    /// Event receiver
    event_rx: Option<EventReceiver>,
    /// Configured symbols
    symbols: Vec<String>,
}

impl KrakenClient {
    /// Create a new client builder
    pub fn builder(
        symbols: impl IntoIterator<Item = impl Into<String>>,
    ) -> KrakenClientBuilder {
        KrakenClientBuilder::new(symbols)
    }

    /// Get the connection state
    pub fn state(&self) -> ConnectionState {
        self.connection.state()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection.is_connected()
    }

    /// Get the subscribed symbols
    pub fn symbols(&self) -> &[String] {
        &self.symbols
    }

    /// Get an orderbook by symbol
    pub fn orderbook(
        &self,
        symbol: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, String, Orderbook>> {
        self.connection.orderbook(symbol)
    }

    /// Get the best bid for a symbol
    pub fn best_bid(&self, symbol: &str) -> Option<Decimal> {
        self.orderbook(symbol)
            .and_then(|book| book.best_bid().map(|l| l.price))
    }

    /// Get the best ask for a symbol
    pub fn best_ask(&self, symbol: &str) -> Option<Decimal> {
        self.orderbook(symbol)
            .and_then(|book| book.best_ask().map(|l| l.price))
    }

    /// Get the spread for a symbol
    pub fn spread(&self, symbol: &str) -> Option<Decimal> {
        self.orderbook(symbol).and_then(|book| book.spread())
    }

    /// Get the mid price for a symbol
    pub fn mid_price(&self, symbol: &str) -> Option<Decimal> {
        self.orderbook(symbol).and_then(|book| book.mid_price())
    }

    /// Get the last checksum for a symbol
    pub fn checksum(&self, symbol: &str) -> Option<u32> {
        self.orderbook(symbol).map(|book| book.last_checksum())
    }

    /// Check if orderbook is synced for a symbol
    pub fn is_synced(&self, symbol: &str) -> bool {
        self.orderbook(symbol)
            .map(|book| book.is_synced())
            .unwrap_or(false)
    }

    /// Take the event receiver (can only be called once)
    ///
    /// Returns the event stream for processing market data and connection events.
    /// Returns `None` if `events()` has already been called.
    pub fn events(&mut self) -> Option<EventReceiver> {
        self.event_rx.take()
    }

    /// Get the number of events dropped due to backpressure
    ///
    /// Only meaningful when using bounded channels with DropNewest policy.
    pub fn dropped_event_count(&self) -> u64 {
        self.connection.dropped_event_count()
    }

    /// Request graceful shutdown
    #[instrument(skip(self))]
    pub fn shutdown(&self) {
        self.connection.shutdown();
    }
}

impl KrakenClientBuilder {
    /// Connect to Kraken and return a client
    #[instrument(skip(self), fields(symbols = ?self.symbols))]
    pub async fn connect(self) -> Result<KrakenClient, KrakenError> {
        // Validate configuration
        if self.symbols.is_empty() {
            return Err(KrakenError::InvalidState {
                expected: "at least one symbol".to_string(),
                actual: "no symbols provided".to_string(),
            });
        }

        // Create connection
        let config = self.to_connection_config();
        let connection = KrakenConnection::new(config);

        // Set up subscriptions
        if self.subscribe_book {
            connection.subscribe_orderbook(self.symbols.clone());
        }
        if self.subscribe_ticker {
            connection.subscribe_ticker(self.symbols.clone());
        }
        if self.subscribe_trade {
            connection.subscribe_trade(self.symbols.clone());
        }

        // Take the event receiver before spawning
        let event_rx = connection.take_event_receiver();

        // Wrap in Arc for shared ownership
        let connection = Arc::new(connection);
        let conn_clone = Arc::clone(&connection);

        // Spawn connection task
        tokio::spawn(async move {
            if let Err(e) = conn_clone.connect_and_run().await {
                tracing::error!("Connection error: {}", e);
            }
        });

        info!(
            "Kraken client created for symbols: {:?}",
            self.symbols
        );

        Ok(KrakenClient {
            connection,
            event_rx,
            symbols: self.symbols,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_types::Depth;

    #[test]
    fn test_builder_creation() {
        let builder = KrakenClient::builder(["BTC/USD"])
            .with_depth(Depth::D10)
            .with_ticker(true);

        assert_eq!(builder.symbols, vec!["BTC/USD"]);
        assert!(builder.subscribe_book);
        assert!(builder.subscribe_ticker);
    }
}
