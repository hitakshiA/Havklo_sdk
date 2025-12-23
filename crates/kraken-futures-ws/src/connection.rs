//! WebSocket connection management for Kraken Futures

use crate::auth::{AuthState, FuturesCredentials};
use crate::channels::{BookChannel, PositionChannel, SubscriptionRequest, TickerChannel, TradeChannel};
use crate::error::{FuturesError, FuturesResult};
use crate::types::FuturesEvent;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// Futures WebSocket endpoints
pub mod endpoints {
    /// Production endpoint
    pub const PRODUCTION: &str = "wss://futures.kraken.com/ws/v1";
    /// Demo/sandbox endpoint
    pub const DEMO: &str = "wss://demo-futures.kraken.com/ws/v1";
}

/// Connection configuration
#[derive(Debug, Clone)]
pub struct FuturesConfig {
    /// WebSocket endpoint URL
    pub endpoint: String,
    /// Credentials (optional for public channels)
    pub credentials: Option<FuturesCredentials>,
    /// Products to subscribe to
    pub products: Vec<String>,
    /// Orderbook depth
    pub book_depth: usize,
    /// Enable reconnection
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay
    pub reconnect_delay: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
}

impl Default for FuturesConfig {
    fn default() -> Self {
        Self {
            endpoint: endpoints::PRODUCTION.to_string(),
            credentials: None,
            products: vec!["PI_XBTUSD".to_string()],
            book_depth: 25,
            auto_reconnect: true,
            max_reconnect_attempts: 10,
            reconnect_delay: Duration::from_secs(1),
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

impl FuturesConfig {
    /// Create a new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the endpoint
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Use demo/sandbox endpoint
    pub fn demo(mut self) -> Self {
        self.endpoint = endpoints::DEMO.to_string();
        self
    }

    /// Set credentials
    pub fn with_credentials(mut self, credentials: FuturesCredentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Set products to subscribe to
    pub fn with_products(mut self, products: Vec<String>) -> Self {
        self.products = products;
        self
    }

    /// Add a product
    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.products.push(symbol.into());
        self
    }

    /// Set book depth
    pub fn with_book_depth(mut self, depth: usize) -> Self {
        self.book_depth = depth;
        self
    }

    /// Disable auto reconnection
    pub fn without_reconnect(mut self) -> Self {
        self.auto_reconnect = false;
        self
    }
}

/// Connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and ready
    Connected,
    /// Authenticating
    Authenticating,
    /// Authenticated and ready for private channels
    Authenticated,
    /// Reconnecting
    Reconnecting { attempt: u32 },
    /// Connection failed
    Failed(String),
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// WebSocket connection for Kraken Futures
#[allow(dead_code)]
pub struct FuturesConnection {
    config: FuturesConfig,
    state: Arc<RwLock<ConnectionState>>,
    auth_state: Arc<RwLock<AuthState>>,
    event_tx: mpsc::Sender<FuturesEvent>,
    event_rx: Option<mpsc::Receiver<FuturesEvent>>,
    // Channel handlers
    book_channel: Arc<RwLock<BookChannel>>,
    ticker_channel: Arc<RwLock<TickerChannel>>,
    trade_channel: Arc<TradeChannel>,
    position_channel: Arc<RwLock<PositionChannel>>,
}

impl FuturesConnection {
    /// Create a new connection
    pub fn new(config: FuturesConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1000);

        Self {
            book_channel: Arc::new(RwLock::new(BookChannel::new(config.book_depth))),
            ticker_channel: Arc::new(RwLock::new(TickerChannel::new())),
            trade_channel: Arc::new(TradeChannel::new()),
            position_channel: Arc::new(RwLock::new(PositionChannel::new())),
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            auth_state: Arc::new(RwLock::new(AuthState::Unauthenticated)),
            event_tx,
            event_rx: Some(event_rx),
        }
    }

    /// Take the event receiver
    pub fn take_event_receiver(&mut self) -> Option<mpsc::Receiver<FuturesEvent>> {
        self.event_rx.take()
    }

    /// Get current connection state
    pub async fn state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }

    /// Get current auth state
    pub async fn auth_state(&self) -> AuthState {
        self.auth_state.read().await.clone()
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        matches!(
            *self.state.read().await,
            ConnectionState::Connected | ConnectionState::Authenticated
        )
    }

    /// Connect and run the event loop
    pub async fn connect_and_run(&self) -> FuturesResult<()> {
        let mut reconnect_attempts = 0;

        loop {
            // Update state
            *self.state.write().await = ConnectionState::Connecting;

            match self.run_connection().await {
                Ok(()) => {
                    info!("Connection closed gracefully");
                    break;
                }
                Err(e) => {
                    error!("Connection error: {}", e);

                    if !self.config.auto_reconnect {
                        *self.state.write().await = ConnectionState::Failed(e.to_string());
                        return Err(e);
                    }

                    reconnect_attempts += 1;
                    if reconnect_attempts >= self.config.max_reconnect_attempts {
                        error!("Max reconnection attempts reached");
                        *self.state.write().await =
                            ConnectionState::Failed("Max reconnection attempts".to_string());
                        return Err(FuturesError::ConnectionClosed(
                            "Max reconnection attempts".to_string(),
                        ));
                    }

                    *self.state.write().await =
                        ConnectionState::Reconnecting { attempt: reconnect_attempts };

                    let delay = self.config.reconnect_delay * reconnect_attempts;
                    warn!(
                        "Reconnecting in {:?} (attempt {}/{})",
                        delay, reconnect_attempts, self.config.max_reconnect_attempts
                    );

                    // Send reconnecting event
                    let _ = self.event_tx.send(FuturesEvent::Disconnected {
                        reason: format!("Reconnecting attempt {}", reconnect_attempts),
                    }).await;

                    tokio::time::sleep(delay).await;
                }
            }
        }

        Ok(())
    }

    /// Run a single connection
    async fn run_connection(&self) -> FuturesResult<()> {
        info!("Connecting to {}", self.config.endpoint);

        let (ws_stream, _) = connect_async(&self.config.endpoint).await?;
        let (mut write, mut read) = ws_stream.split();

        *self.state.write().await = ConnectionState::Connected;

        // Send connected event
        let _ = self.event_tx.send(FuturesEvent::Connected {
            server_time: chrono::Utc::now().to_rfc3339(),
        }).await;

        info!("Connected to Kraken Futures");

        // Subscribe to channels
        self.subscribe_all(&mut write).await?;

        // Event loop
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    self.handle_message(&text).await?;
                }
                Ok(Message::Close(_)) => {
                    info!("Server closed connection");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    let _ = write.send(Message::Pong(data)).await;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    return Err(e.into());
                }
            }
        }

        *self.state.write().await = ConnectionState::Disconnected;
        let _ = self.event_tx.send(FuturesEvent::Disconnected {
            reason: "Connection closed".to_string(),
        }).await;

        Ok(())
    }

    /// Subscribe to all configured channels
    async fn subscribe_all<S>(&self, write: &mut S) -> FuturesResult<()>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::error::Error + Send + Sync + 'static,
    {
        let products = self.config.products.clone();

        if products.is_empty() {
            return Ok(());
        }

        // Subscribe to ticker
        let ticker_sub = SubscriptionRequest::new("ticker", products.clone());
        self.send_subscription(write, ticker_sub).await?;

        // Subscribe to book
        let book_sub = SubscriptionRequest::new("book", products.clone());
        self.send_subscription(write, book_sub).await?;

        // Subscribe to trades
        let trade_sub = SubscriptionRequest::new("trade", products.clone());
        self.send_subscription(write, trade_sub).await?;

        // Subscribe to heartbeat
        let heartbeat_sub = SubscriptionRequest::new("heartbeat", vec![]);
        self.send_subscription(write, heartbeat_sub).await?;

        Ok(())
    }

    /// Send a subscription request
    async fn send_subscription<S>(
        &self,
        write: &mut S,
        sub: SubscriptionRequest,
    ) -> FuturesResult<()>
    where
        S: SinkExt<Message> + Unpin,
        S::Error: std::error::Error + Send + Sync + 'static,
    {
        let msg = sub.to_json().to_string();
        debug!("Sending subscription: {}", msg);

        write
            .send(Message::Text(msg))
            .await
            .map_err(|e| FuturesError::ConnectionClosed(e.to_string()))?;

        Ok(())
    }

    /// Handle an incoming message
    async fn handle_message(&self, text: &str) -> FuturesResult<()> {
        let value: serde_json::Value = serde_json::from_str(text)?;

        // Check for feed type
        if let Some(feed) = value.get("feed").and_then(|v| v.as_str()) {
            match feed {
                "ticker" => {
                    if let Ok(ticker) = serde_json::from_value(value["data"].clone()) {
                        let event = self.ticker_channel.write().await.process_ticker(ticker);
                        let _ = self.event_tx.send(event).await;
                    }
                }
                "book_snapshot" => {
                    if let Ok(snapshot) = serde_json::from_value(value.clone()) {
                        let event = self.book_channel.write().await.process_snapshot(snapshot);
                        let _ = self.event_tx.send(event).await;
                    }
                }
                "book" => {
                    if let Ok(update) = serde_json::from_value(value.clone()) {
                        if let Some(event) = self.book_channel.write().await.process_update(update) {
                            let _ = self.event_tx.send(event).await;
                        }
                    }
                }
                "trade" => {
                    if let Ok(trade) = serde_json::from_value(value.clone()) {
                        let event = self.trade_channel.process_trade(trade);
                        let _ = self.event_tx.send(event).await;
                    }
                }
                "heartbeat" => {
                    let _ = self.event_tx.send(FuturesEvent::Heartbeat).await;
                }
                _ => {
                    debug!("Unhandled feed: {}", feed);
                }
            }
        }

        // Check for event type
        if let Some(event) = value.get("event").and_then(|v| v.as_str()) {
            match event {
                "subscribed" => {
                    let feed = value.get("feed").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let product_ids: Vec<String> = value
                        .get("product_ids")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();

                    info!("Subscribed to {} for {:?}", feed, product_ids);
                    let _ = self.event_tx.send(FuturesEvent::Subscribed { feed, product_ids }).await;
                }
                "error" => {
                    let message = value.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
                    error!("Server error: {}", message);
                }
                "info" => {
                    let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
                    info!("Server info: version {}", version);
                }
                _ => {
                    debug!("Unhandled event: {}", event);
                }
            }
        }

        Ok(())
    }

    // Public API methods

    /// Subscribe to orderbook
    pub async fn subscribe_book(&self) -> FuturesResult<()> {
        // Will be handled by subscribe_all for now
        Ok(())
    }

    /// Get best bid for a product
    pub async fn best_bid(&self, product_id: &str) -> Option<(rust_decimal::Decimal, rust_decimal::Decimal)> {
        self.book_channel.read().await.best_bid(product_id)
    }

    /// Get best ask for a product
    pub async fn best_ask(&self, product_id: &str) -> Option<(rust_decimal::Decimal, rust_decimal::Decimal)> {
        self.book_channel.read().await.best_ask(product_id)
    }

    /// Get spread for a product
    pub async fn spread(&self, product_id: &str) -> Option<rust_decimal::Decimal> {
        self.book_channel.read().await.spread(product_id)
    }

    /// Get ticker for a product
    pub async fn ticker(&self, product_id: &str) -> Option<crate::types::FuturesTicker> {
        self.ticker_channel.read().await.ticker(product_id).cloned()
    }

    /// Get last trade price for a product
    pub async fn last_price(&self, product_id: &str) -> Option<rust_decimal::Decimal> {
        self.trade_channel.last_price(product_id)
    }

    /// Get total trade count
    pub fn trade_count(&self) -> u64 {
        self.trade_channel.trade_count()
    }
}

// Add chrono for timestamp generation
fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

// Re-use chrono if available, otherwise use simple timestamp
mod chrono {
    pub struct Utc;
    impl Utc {
        pub fn now() -> Self { Utc }
        pub fn to_rfc3339(&self) -> String {
            super::chrono_now()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = FuturesConfig::new()
            .with_symbol("PI_XBTUSD")
            .with_symbol("PI_ETHUSD")
            .with_book_depth(50)
            .demo();

        assert_eq!(config.endpoint, endpoints::DEMO);
        assert_eq!(config.products.len(), 3); // Default + 2 added
        assert_eq!(config.book_depth, 50);
    }

    #[test]
    fn test_connection_state() {
        assert_eq!(ConnectionState::default(), ConnectionState::Disconnected);
    }
}
