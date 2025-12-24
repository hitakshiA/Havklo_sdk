//! WebSocket connection management

use crate::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::endpoint::Endpoint;
use crate::events::{ConnectionEvent, DisconnectReason, Event, L3Event, MarketEvent, SubscriptionEvent};
use crate::reconnect::ReconnectConfig;
use crate::subscription::{Subscription, SubscriptionManager};

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use kraken_book::Orderbook;
use kraken_types::{Channel, Depth, KrakenError, MethodResponse, WsMessage};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, instrument, warn};

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

/// Backpressure policy when event channel is full
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackpressurePolicy {
    /// Drop newest messages when channel is full (default)
    #[default]
    DropNewest,
    /// Block until space is available (may cause connection issues)
    Block,
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
    /// Heartbeat timeout - disconnect if no heartbeat received within this duration
    /// Kraken sends heartbeats every ~5 seconds; default timeout is 30 seconds
    pub heartbeat_timeout: Option<Duration>,
    /// Event channel capacity (None = unbounded)
    pub channel_capacity: Option<usize>,
    /// Backpressure policy when channel is full
    pub backpressure_policy: BackpressurePolicy,
    /// Circuit breaker configuration (None = disabled)
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            endpoint: Endpoint::Public,
            reconnect: ReconnectConfig::default(),
            connect_timeout: Duration::from_secs(10),
            depth: Depth::D10,
            heartbeat_timeout: Some(Duration::from_secs(30)),
            channel_capacity: None, // Unbounded by default for backwards compatibility
            backpressure_policy: BackpressurePolicy::default(),
            circuit_breaker: Some(CircuitBreakerConfig::default()), // Enabled by default
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

    /// Set heartbeat timeout
    ///
    /// If no message is received within this duration, the connection is
    /// considered dead and will be reconnected.
    pub fn with_heartbeat_timeout(mut self, timeout: Duration) -> Self {
        self.heartbeat_timeout = Some(timeout);
        self
    }

    /// Disable heartbeat timeout monitoring
    pub fn without_heartbeat_timeout(mut self) -> Self {
        self.heartbeat_timeout = None;
        self
    }

    /// Set bounded channel capacity for backpressure handling
    ///
    /// When the channel is full and a new event arrives:
    /// - `DropNewest`: The new event is dropped (default)
    /// - `Block`: The sender blocks until space is available (may cause connection issues)
    ///
    /// Recommended capacity: 1000-10000 depending on message rate
    pub fn with_channel_capacity(mut self, capacity: usize, policy: BackpressurePolicy) -> Self {
        self.channel_capacity = Some(capacity);
        self.backpressure_policy = policy;
        self
    }

    /// Use unbounded channel (no backpressure, unlimited memory growth)
    pub fn with_unbounded_channel(mut self) -> Self {
        self.channel_capacity = None;
        self
    }

    /// Enable circuit breaker with custom configuration
    ///
    /// The circuit breaker prevents repeated connection attempts when the
    /// service appears unhealthy, giving it time to recover.
    pub fn with_circuit_breaker(mut self, config: CircuitBreakerConfig) -> Self {
        self.circuit_breaker = Some(config);
        self
    }

    /// Disable circuit breaker
    pub fn without_circuit_breaker(mut self) -> Self {
        self.circuit_breaker = None;
        self
    }
}

/// Event sender that handles both bounded and unbounded channels
enum EventSender {
    Unbounded(mpsc::UnboundedSender<Event>),
    Bounded {
        sender: mpsc::Sender<Event>,
        policy: BackpressurePolicy,
        dropped_count: std::sync::atomic::AtomicU64,
    },
}

impl EventSender {
    fn send(&self, event: Event) {
        match self {
            EventSender::Unbounded(tx) => {
                let _ = tx.send(event);
            }
            EventSender::Bounded { sender, policy, dropped_count } => {
                match policy {
                    BackpressurePolicy::DropNewest => {
                        if sender.try_send(event).is_err() {
                            dropped_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    BackpressurePolicy::Block => {
                        // Use blocking send - this may cause issues if channel is full
                        let _ = sender.blocking_send(event);
                    }
                }
            }
        }
    }

    fn dropped_count(&self) -> u64 {
        match self {
            EventSender::Unbounded(_) => 0,
            EventSender::Bounded { dropped_count, .. } => dropped_count.load(Ordering::Relaxed),
        }
    }
}

/// Event receiver wrapper
pub enum EventReceiver {
    /// Unbounded receiver
    Unbounded(mpsc::UnboundedReceiver<Event>),
    /// Bounded receiver
    Bounded(mpsc::Receiver<Event>),
}

impl EventReceiver {
    /// Receive the next event
    #[instrument(skip(self), level = "trace")]
    pub async fn recv(&mut self) -> Option<Event> {
        match self {
            EventReceiver::Unbounded(rx) => rx.recv().await,
            EventReceiver::Bounded(rx) => rx.recv().await,
        }
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
    event_tx: EventSender,
    /// Event receiver (for public consumption)
    event_rx: Arc<RwLock<Option<EventReceiver>>>,
    /// Last message timestamp for heartbeat monitoring
    last_message_time: Arc<RwLock<std::time::Instant>>,
    /// Circuit breaker for connection reliability
    circuit_breaker: Option<CircuitBreaker>,
}

impl KrakenConnection {
    /// Create a new connection with the given configuration
    pub fn new(config: ConnectionConfig) -> Self {
        let (event_tx, event_rx) = match config.channel_capacity {
            Some(capacity) => {
                let (tx, rx) = mpsc::channel(capacity);
                (
                    EventSender::Bounded {
                        sender: tx,
                        policy: config.backpressure_policy,
                        dropped_count: std::sync::atomic::AtomicU64::new(0),
                    },
                    EventReceiver::Bounded(rx),
                )
            }
            None => {
                let (tx, rx) = mpsc::unbounded_channel();
                (EventSender::Unbounded(tx), EventReceiver::Unbounded(rx))
            }
        };

        let circuit_breaker = config.circuit_breaker.clone().map(CircuitBreaker::new);

        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            orderbooks: Arc::new(DashMap::new()),
            subscriptions: Arc::new(RwLock::new(SubscriptionManager::new())),
            reconnect_attempt: AtomicU32::new(0),
            shutdown: AtomicBool::new(false),
            event_tx,
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
            last_message_time: Arc::new(RwLock::new(std::time::Instant::now())),
            circuit_breaker,
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
    pub fn take_event_receiver(&self) -> Option<EventReceiver> {
        self.event_rx.write().take()
    }

    /// Get the number of dropped events due to backpressure
    ///
    /// Only meaningful when using a bounded channel with DropNewest policy.
    pub fn dropped_event_count(&self) -> u64 {
        self.event_tx.dropped_count()
    }

    /// Get an orderbook by symbol
    pub fn orderbook(&self, symbol: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Orderbook>>
    {
        self.orderbooks.get(symbol)
    }

    /// Subscribe to orderbook updates for symbols
    #[instrument(skip(self), fields(symbols = ?symbols))]
    pub fn subscribe_orderbook(&self, symbols: Vec<String>) -> u64 {
        let sub = Subscription::orderbook(symbols, self.config.depth);
        self.subscriptions.write().add(sub)
    }

    /// Subscribe to ticker updates
    #[instrument(skip(self), fields(symbols = ?symbols))]
    pub fn subscribe_ticker(&self, symbols: Vec<String>) -> u64 {
        let sub = Subscription::ticker(symbols);
        self.subscriptions.write().add(sub)
    }

    /// Subscribe to trade updates
    #[instrument(skip(self), fields(symbols = ?symbols))]
    pub fn subscribe_trade(&self, symbols: Vec<String>) -> u64 {
        let sub = Subscription::trade(symbols);
        self.subscriptions.write().add(sub)
    }

    /// Subscribe to L3 (Level 3) orderbook updates
    ///
    /// Note: L3 requires connection to the Level3 endpoint and special access.
    /// Create a connection with `Endpoint::Level3` to use this subscription.
    #[instrument(skip(self), fields(symbols = ?symbols))]
    pub fn subscribe_l3(&self, symbols: Vec<String>) -> u64 {
        let sub = Subscription::level3(symbols);
        self.subscriptions.write().add(sub)
    }

    /// Connect and run the connection loop
    #[instrument(skip(self), name = "kraken_connection")]
    pub async fn connect_and_run(&self) -> Result<(), KrakenError> {
        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Check circuit breaker before attempting connection
            if let Some(ref breaker) = self.circuit_breaker {
                if !breaker.allow_request() {
                    let stats = breaker.stats();
                    warn!(
                        "Circuit breaker is open (tripped {} times), waiting for recovery",
                        stats.trips
                    );
                    self.emit(ConnectionEvent::CircuitBreakerOpen {
                        trips: stats.trips,
                    });
                    // Wait for the circuit breaker timeout before retrying
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
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
                    // Normal shutdown - record success
                    if let Some(ref breaker) = self.circuit_breaker {
                        breaker.record_success();
                    }
                    break;
                }
                Err(e) => {
                    // Record failure with circuit breaker
                    if let Some(ref breaker) = self.circuit_breaker {
                        breaker.record_failure();
                    }

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
                    reason: e.to_string(),
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
                    if let Ok(WsMessage::Status(status_msg)) = WsMessage::parse(&text) {
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

        // Subscribe to instrument channel first to get precision info
        // This is needed for correct checksum calculation
        let requests = self.subscriptions.write().restoration_requests();

        // Collect symbols from pending book subscriptions
        let book_symbols: Vec<String> = requests
            .iter()
            .filter_map(|(_, req)| {
                if req.params.channel == Channel::Book {
                    Some(req.params.symbol.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        // Subscribe to instrument channel if we have symbols
        if !book_symbols.is_empty() {
            let instrument_request = serde_json::json!({
                "method": "subscribe",
                "params": {
                    "channel": "instrument",
                    "snapshot": true
                }
            });
            let json = instrument_request.to_string();
            debug!("Sending instrument subscription: {}", json);
            write
                .send(Message::Text(json))
                .await
                .map_err(|e| KrakenError::WebSocket(e.to_string()))?;

            // Wait briefly for instrument data to arrive
            // This ensures we have precision info before processing book data
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // Send subscription requests
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

        // Reset heartbeat timer
        *self.last_message_time.write() = std::time::Instant::now();

        // Main message loop with heartbeat timeout
        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                info!("Shutdown requested, closing connection");
                let _ = write.send(Message::Close(None)).await;
                break;
            }

            // Use heartbeat timeout or a default long timeout
            let heartbeat_timeout = self.config.heartbeat_timeout.unwrap_or(Duration::from_secs(3600));

            let msg_result = tokio::select! {
                msg = read.next() => msg,
                _ = tokio::time::sleep(heartbeat_timeout) => {
                    // Check if we've actually timed out
                    let elapsed = self.last_message_time.read().elapsed();
                    if elapsed >= heartbeat_timeout {
                        warn!("Heartbeat timeout: no message received for {:?}", elapsed);
                        self.emit(ConnectionEvent::Disconnected {
                            reason: DisconnectReason::HeartbeatTimeout,
                        });
                        return Err(KrakenError::WebSocket("Heartbeat timeout".into()));
                    }
                    continue;
                }
            };

            match msg_result {
                Some(Ok(Message::Text(text))) => {
                    *self.last_message_time.write() = std::time::Instant::now();
                    self.handle_message(&text);
                }
                Some(Ok(Message::Ping(data))) => {
                    *self.last_message_time.write() = std::time::Instant::now();
                    let _ = write.send(Message::Pong(data)).await;
                }
                Some(Ok(Message::Pong(_))) => {
                    *self.last_message_time.write() = std::time::Instant::now();
                }
                Some(Ok(Message::Close(_))) => {
                    info!("Server closed connection");
                    self.emit(ConnectionEvent::Disconnected {
                        reason: DisconnectReason::ServerClosed,
                    });
                    return Err(KrakenError::WebSocket("Server closed connection".into()));
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    self.emit(ConnectionEvent::Disconnected {
                        reason: DisconnectReason::NetworkError(e.to_string()),
                    });
                    return Err(KrakenError::WebSocket(e.to_string()));
                }
                Some(Ok(_)) => {}
                None => {
                    info!("WebSocket stream ended");
                    break;
                }
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
                WsMessage::Ticker(_ticker_msg) => {
                    // Ticker channel - emit via MarketEvent in future version
                    debug!("Ticker update received");
                }
                WsMessage::Trade(_trade_msg) => {
                    // Trade channel - emit via MarketEvent in future version
                    debug!("Trade update received");
                }
                WsMessage::Ohlc(_ohlc_msg) => {
                    // OHLC channel - emit via MarketEvent in future version
                    debug!("OHLC update received");
                }
                WsMessage::Instrument(instrument_msg) => {
                    // Update precision for each trading pair from instrument data
                    for pair in &instrument_msg.data.pairs {
                        let symbol = &pair.symbol;

                        // Get or create orderbook and update its precision
                        let mut orderbook =
                            self.orderbooks.entry(symbol.clone()).or_insert_with(|| {
                                Orderbook::with_depth(symbol, self.config.depth as u32)
                            });

                        orderbook.set_precision(pair.price_precision, pair.qty_precision);

                        debug!(
                            "Updated precision for {}: price={}, qty={}",
                            symbol, pair.price_precision, pair.qty_precision
                        );
                    }
                }
                WsMessage::Executions(_executions_msg) => {
                    // Private channel: order executions - requires auth feature
                    debug!("Executions update received");
                }
                WsMessage::Balances(_balances_msg) => {
                    // Private channel: account balances - requires auth feature
                    debug!("Balances update received");
                }
                WsMessage::Level3(l3_msg) => {
                    // L3 orderbook data
                    if let Some(data) = l3_msg.data.first() {
                        let is_snapshot = l3_msg.msg_type == "snapshot";
                        let event = L3Event::from_data(data, is_snapshot);
                        debug!(
                            "L3 {} received for {} ({} bids, {} asks)",
                            if is_snapshot { "snapshot" } else { "update" },
                            data.symbol,
                            data.bids.len(),
                            data.asks.len()
                        );
                        self.emit(event);
                    }
                }
                WsMessage::Heartbeat => {
                    self.emit(MarketEvent::Heartbeat);
                }
                WsMessage::Unknown(_) => {
                    debug!("Unknown message: {}", text);
                }
                // Required for #[non_exhaustive] - handle future variants
                _ => {
                    debug!("Unhandled message variant");
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
        self.event_tx.send(event.into());
    }

    /// Request shutdown
    #[instrument(skip(self))]
    pub fn shutdown(&self) {
        info!("Shutdown requested");
        self.shutdown.store(true, Ordering::Relaxed);
        *self.state.write() = ConnectionState::ShuttingDown;
    }

    /// Request shutdown and wait for disconnection
    ///
    /// This is a graceful shutdown that waits until the connection
    /// has fully closed before returning.
    #[instrument(skip(self))]
    pub async fn shutdown_gracefully(&self, timeout: Duration) -> bool {
        info!("Graceful shutdown requested with timeout {:?}", timeout);
        self.shutdown.store(true, Ordering::Relaxed);
        *self.state.write() = ConnectionState::ShuttingDown;

        // Wait for disconnected state or timeout
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if self.state() == ConnectionState::Disconnected {
                info!("Graceful shutdown complete");
                return true;
            }
            if std::time::Instant::now() >= deadline {
                warn!("Shutdown timed out after {:?}", timeout);
                return false;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Check if shutdown has been requested
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    /// Get the time since last message was received
    pub fn time_since_last_message(&self) -> Duration {
        self.last_message_time.read().elapsed()
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
