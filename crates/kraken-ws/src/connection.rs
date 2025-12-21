//! WebSocket connection management

use crate::endpoint::Endpoint;
use crate::events::{ConnectionEvent, DisconnectReason, Event, MarketEvent, SubscriptionEvent};
use crate::reconnect::ReconnectConfig;
use crate::subscription::{Subscription, SubscriptionManager};

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use kraken_book::Orderbook;
use kraken_types::{Depth, KrakenError, MethodResponse, WsMessage};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connection in progress
    Connecting,
    /// Connected and ready
    Connected,
    /// Reconnecting after disconnect
    Reconnecting,
    /// Shutting down
    ShuttingDown,
}

/// Configuration for the WebSocket connection
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// WebSocket endpoint
    pub endpoint: Endpoint,
    /// Reconnection settings
    pub reconnect: ReconnectConfig,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Orderbook depth to subscribe with
    pub depth: Depth,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            endpoint: Endpoint::Public,
            reconnect: ReconnectConfig::default(),
            connect_timeout: Duration::from_secs(10),
            depth: Depth::D10,
        }
    }
}

impl ConnectionConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the endpoint
    pub fn with_endpoint(mut self, endpoint: Endpoint) -> Self {
        self.endpoint = endpoint;
        self
    }

    /// Set reconnection config
    pub fn with_reconnect(mut self, config: ReconnectConfig) -> Self {
        self.reconnect = config;
        self
    }

    /// Disable automatic reconnection
    pub fn without_reconnect(mut self) -> Self {
        self.reconnect = ReconnectConfig::disabled();
        self
    }

    /// Set connection timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set orderbook depth
    pub fn with_depth(mut self, depth: Depth) -> Self {
        self.depth = depth;
        self
    }
}

/// WebSocket connection to Kraken
pub struct KrakenConnection {
    /// Configuration
    config: ConnectionConfig,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Orderbooks by symbol
    orderbooks: Arc<DashMap<String, Orderbook>>,
    /// Subscription manager
    subscriptions: Arc<RwLock<SubscriptionManager>>,
    /// Reconnection attempt counter
    reconnect_attempt: AtomicU32,
    /// Shutdown flag
    shutdown: AtomicBool,
    /// Event sender
    event_tx: mpsc::UnboundedSender<Event>,
    /// Event receiver (for public consumption)
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<Event>>>>,
}

impl KrakenConnection {
    /// Create a new connection with the given configuration
    pub fn new(config: ConnectionConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            orderbooks: Arc::new(DashMap::new()),
            subscriptions: Arc::new(RwLock::new(SubscriptionManager::new())),
            reconnect_attempt: AtomicU32::new(0),
            shutdown: AtomicBool::new(false),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
        }
    }

    /// Create a connection with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ConnectionConfig::default())
    }

    /// Get the current connection state
    pub fn state(&self) -> ConnectionState {
        *self.state.read()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.state() == ConnectionState::Connected
    }

    /// Take the event receiver (can only be called once)
    pub fn take_event_receiver(&self) -> Option<mpsc::UnboundedReceiver<Event>> {
        self.event_rx.write().take()
    }

    /// Get an orderbook by symbol
    pub fn orderbook(&self, symbol: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Orderbook>>
    {
        self.orderbooks.get(symbol)
    }

    /// Subscribe to orderbook updates for symbols
    pub fn subscribe_orderbook(&self, symbols: Vec<String>) -> u64 {
        let sub = Subscription::orderbook(symbols, self.config.depth);
        self.subscriptions.write().add(sub)
    }

    /// Subscribe to ticker updates
    pub fn subscribe_ticker(&self, symbols: Vec<String>) -> u64 {
        let sub = Subscription::ticker(symbols);
        self.subscriptions.write().add(sub)
    }

    /// Subscribe to trade updates
    pub fn subscribe_trade(&self, symbols: Vec<String>) -> u64 {
        let sub = Subscription::trade(symbols);
        self.subscriptions.write().add(sub)
    }

    /// Connect and run the connection loop
    pub async fn connect_and_run(&self) -> Result<(), KrakenError> {
        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Update state
            {
                let mut state = self.state.write();
                if *state == ConnectionState::Reconnecting {
                    // Already reconnecting
                } else {
                    *state = ConnectionState::Connecting;
                }
            }

            match self.connect_internal().await {
                Ok(()) => {
                    // Normal shutdown
                    break;
                }
                Err(e) => {
                    let attempt = self.reconnect_attempt.fetch_add(1, Ordering::Relaxed) + 1;

                    if !self.config.reconnect.should_reconnect(attempt) {
                        error!("Reconnection attempts exhausted after {} tries", attempt);
                        self.emit(ConnectionEvent::ReconnectFailed {
                            error: e.to_string(),
                        });
                        return Err(e);
                    }

                    let delay = self.config.reconnect.delay_with_jitter(attempt);
                    warn!(
                        "Connection failed, reconnecting in {:?} (attempt {}): {}",
                        delay, attempt, e
                    );

                    self.emit(ConnectionEvent::Reconnecting { attempt, delay });
                    *self.state.write() = ConnectionState::Reconnecting;

                    tokio::time::sleep(delay).await;
                }
            }
        }

        *self.state.write() = ConnectionState::Disconnected;
        Ok(())
    }

    /// Internal connection logic
    async fn connect_internal(&self) -> Result<(), KrakenError> {
        let url = self.config.endpoint.url();
        info!("Connecting to {}", url);

        // Connect with timeout
        let connect_result = timeout(self.config.connect_timeout, connect_async(url)).await;

        let (ws_stream, _response) = match connect_result {
            Ok(Ok((stream, response))) => (stream, response),
            Ok(Err(e)) => {
                return Err(KrakenError::ConnectionFailed {
                    url: url.to_string(),
                    source: std::io::Error::other(e.to_string()),
                });
            }
            Err(_) => {
                return Err(KrakenError::ConnectionTimeout {
                    url: url.to_string(),
                    timeout: self.config.connect_timeout,
                });
            }
        };

        let (mut write, mut read) = ws_stream.split();

        // Wait for status message
        let mut connected = false;
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_msg) = WsMessage::parse(&text) {
                        if let WsMessage::Status(status_msg) = ws_msg {
                            if let Some(data) = status_msg.data.first() {
                                info!(
                                    "Connected to Kraken API {} (connection_id: {})",
                                    data.api_version, data.connection_id
                                );

                                self.emit(ConnectionEvent::Connected {
                                    api_version: data.api_version.clone(),
                                    connection_id: data.connection_id,
                                });

                                connected = true;
                                break;
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    return Err(KrakenError::WebSocket("Connection closed before ready".into()));
                }
                Err(e) => {
                    return Err(KrakenError::WebSocket(e.to_string()));
                }
                _ => {}
            }
        }

        if !connected {
            return Err(KrakenError::WebSocket(
                "No status message received".into(),
            ));
        }

        // Update state and reset reconnect counter
        *self.state.write() = ConnectionState::Connected;
        self.reconnect_attempt.store(0, Ordering::Relaxed);

        // Send subscription requests
        let requests = self.subscriptions.write().restoration_requests();
        for (_req_id, request) in &requests {
            let json = serde_json::to_string(request).map_err(|e| {
                KrakenError::InvalidJson {
                    message: e.to_string(),
                    raw: None,
                }
            })?;
            debug!("Sending subscription: {}", json);
            write
                .send(Message::Text(json))
                .await
                .map_err(|e| KrakenError::WebSocket(e.to_string()))?;
        }

        if !requests.is_empty() {
            self.emit(ConnectionEvent::SubscriptionsRestored {
                count: requests.len(),
            });
        }

        // Main message loop
        while let Some(msg_result) = read.next().await {
            if self.shutdown.load(Ordering::Relaxed) {
                info!("Shutdown requested, closing connection");
                let _ = write.send(Message::Close(None)).await;
                break;
            }

            match msg_result {
                Ok(Message::Text(text)) => {
                    self.handle_message(&text);
                }
                Ok(Message::Ping(data)) => {
                    let _ = write.send(Message::Pong(data)).await;
                }
                Ok(Message::Close(_)) => {
                    info!("Server closed connection");
                    self.emit(ConnectionEvent::Disconnected {
                        reason: DisconnectReason::ServerClosed,
                    });
                    return Err(KrakenError::WebSocket("Server closed connection".into()));
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    self.emit(ConnectionEvent::Disconnected {
                        reason: DisconnectReason::NetworkError(e.to_string()),
                    });
                    return Err(KrakenError::WebSocket(e.to_string()));
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle an incoming message
    fn handle_message(&self, text: &str) {
        match WsMessage::parse(text) {
            Ok(msg) => match msg {
                WsMessage::Status(status_msg) => {
                    if let Some(data) = status_msg.data.first() {
                        self.emit(MarketEvent::Status {
                            system: data.system.to_string(),
                            version: data.api_version.clone(),
                        });
                    }
                }
                WsMessage::Method(resp) => {
                    self.handle_subscribe_response(&resp);
                }
                WsMessage::Book(book_msg) => {
                    if let Some(data) = book_msg.data.first() {
                        let symbol = &data.symbol;
                        let is_snapshot = book_msg.msg_type == "snapshot";

                        // Get or create orderbook
                        let mut orderbook =
                            self.orderbooks.entry(symbol.clone()).or_insert_with(|| {
                                Orderbook::with_depth(symbol, self.config.depth as u32)
                            });

                        // Apply the update
                        match orderbook.apply_book_data(data, is_snapshot) {
                            Ok(_result) => {
                                let snapshot = orderbook.snapshot();
                                let event = if is_snapshot {
                                    MarketEvent::OrderbookSnapshot {
                                        symbol: symbol.clone(),
                                        snapshot,
                                    }
                                } else {
                                    MarketEvent::OrderbookUpdate {
                                        symbol: symbol.clone(),
                                        snapshot,
                                    }
                                };
                                self.emit(event);
                            }
                            Err(mismatch) => {
                                warn!(
                                    "Checksum mismatch for {}: expected {}, computed {}",
                                    mismatch.symbol, mismatch.expected, mismatch.computed
                                );
                                self.emit(MarketEvent::ChecksumMismatch {
                                    symbol: symbol.clone(),
                                    expected: mismatch.expected,
                                    computed: mismatch.computed,
                                });
                            }
                        }
                    }
                }
                WsMessage::Ticker(_) => {
                    // TODO: Handle ticker updates
                    debug!("Ticker update received");
                }
                WsMessage::Trade(_) => {
                    // TODO: Handle trade updates
                    debug!("Trade update received");
                }
                WsMessage::Ohlc(_) => {
                    // TODO: Handle OHLC updates
                    debug!("OHLC update received");
                }
                WsMessage::Heartbeat => {
                    self.emit(MarketEvent::Heartbeat);
                }
                WsMessage::Unknown(_) => {
                    debug!("Unknown message: {}", text);
                }
            },
            Err(e) => {
                warn!("Failed to parse message: {} - {}", e, text);
            }
        }
    }

    /// Handle subscription response
    fn handle_subscribe_response(&self, resp: &MethodResponse) {
        if let Some(req_id) = resp.req_id {
            if resp.success {
                self.subscriptions.write().confirm(req_id);

                if let Some(result) = &resp.result {
                    self.emit(SubscriptionEvent::Subscribed {
                        channel: result.channel.clone(),
                        symbols: result.symbol.clone().into_iter().collect(),
                    });
                }
            } else {
                self.subscriptions.write().reject(req_id);

                self.emit(SubscriptionEvent::Rejected {
                    channel: "unknown".to_string(),
                    reason: resp.error.clone().unwrap_or_default(),
                });
            }
        }
    }

    /// Emit an event
    fn emit(&self, event: impl Into<Event>) {
        let _ = self.event_tx.send(event.into());
    }

    /// Request shutdown
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        *self.state.write() = ConnectionState::ShuttingDown;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_config() {
        let config = ConnectionConfig::new()
            .with_endpoint(Endpoint::PublicBeta)
            .with_depth(Depth::D25)
            .with_timeout(Duration::from_secs(5));

        assert_eq!(config.endpoint, Endpoint::PublicBeta);
        assert_eq!(config.depth, Depth::D25);
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_connection_state() {
        let conn = KrakenConnection::with_defaults();
        assert_eq!(conn.state(), ConnectionState::Disconnected);
        assert!(!conn.is_connected());
    }
}
