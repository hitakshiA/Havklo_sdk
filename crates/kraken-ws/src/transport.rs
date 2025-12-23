//! WebSocket transport abstraction
//!
//! This module provides a trait-based abstraction over WebSocket connections,
//! enabling unit testing of connection logic without real network calls.
//!
//! # Example
//!
//! ```no_run
//! use kraken_ws::transport::{Transport, WsTransport, TransportError};
//!
//! async fn example() -> Result<(), TransportError> {
//!     let mut transport = WsTransport::new("wss://ws.kraken.com/v2");
//!     transport.connect().await?;
//!     transport.send(r#"{"method":"ping"}"#).await?;
//!     if let Some(response) = transport.recv().await? {
//!         println!("Received: {}", response);
//!     }
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};
use tracing::{debug, instrument};

/// Transport layer errors
#[derive(Error, Debug)]
pub enum TransportError {
    /// Connection failed
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// Connection closed
    #[error("connection closed")]
    ConnectionClosed,

    /// Send failed
    #[error("send failed: {0}")]
    SendFailed(String),

    /// Receive failed
    #[error("receive failed: {0}")]
    ReceiveFailed(String),

    /// Connection timeout
    #[error("connection timeout after {0:?}")]
    Timeout(Duration),

    /// Not connected
    #[error("not connected")]
    NotConnected,

    /// Protocol error
    #[error("protocol error: {0}")]
    Protocol(String),
}

/// Trait for WebSocket transport abstraction
///
/// This trait enables unit testing of connection logic by allowing
/// mock implementations to be injected instead of real WebSocket connections.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to the WebSocket endpoint
    async fn connect(&mut self) -> Result<(), TransportError>;

    /// Send a text message
    async fn send(&mut self, message: &str) -> Result<(), TransportError>;

    /// Receive a text message
    ///
    /// Returns `None` if the connection was closed gracefully.
    async fn recv(&mut self) -> Result<Option<String>, TransportError>;

    /// Close the connection gracefully
    async fn close(&mut self) -> Result<(), TransportError>;

    /// Check if currently connected
    fn is_connected(&self) -> bool;

    /// Get the endpoint URL
    fn endpoint(&self) -> &str;
}

/// Real WebSocket transport using tokio-tungstenite
pub struct WsTransport {
    url: String,
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    connect_timeout: Duration,
}

impl WsTransport {
    /// Create a new WebSocket transport
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            stream: None,
            connect_timeout: Duration::from_secs(10),
        }
    }

    /// Set connection timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
}

#[async_trait]
impl Transport for WsTransport {
    #[instrument(skip(self), fields(url = %self.url))]
    async fn connect(&mut self) -> Result<(), TransportError> {
        debug!("Connecting to WebSocket");

        let connect_future = connect_async(&self.url);

        let (ws_stream, _response) = timeout(self.connect_timeout, connect_future)
            .await
            .map_err(|_| TransportError::Timeout(self.connect_timeout))?
            .map_err(|e| TransportError::ConnectionFailed(e.to_string()))?;

        self.stream = Some(ws_stream);
        debug!("WebSocket connected");
        Ok(())
    }

    #[instrument(skip(self, message), fields(len = message.len()))]
    async fn send(&mut self, message: &str) -> Result<(), TransportError> {
        let stream = self.stream.as_mut().ok_or(TransportError::NotConnected)?;

        stream
            .send(Message::Text(message.to_string()))
            .await
            .map_err(|e| TransportError::SendFailed(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn recv(&mut self) -> Result<Option<String>, TransportError> {
        let stream = self.stream.as_mut().ok_or(TransportError::NotConnected)?;

        match stream.next().await {
            Some(Ok(Message::Text(text))) => Ok(Some(text)),
            Some(Ok(Message::Binary(data))) => {
                // Try to convert binary to string
                String::from_utf8(data)
                    .map(Some)
                    .map_err(|e| TransportError::Protocol(e.to_string()))
            }
            Some(Ok(Message::Close(_))) => {
                self.stream = None;
                Ok(None)
            }
            Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                // Skip ping/pong, recurse to get actual message
                Box::pin(self.recv()).await
            }
            Some(Ok(Message::Frame(_))) => {
                // Raw frame, skip
                Box::pin(self.recv()).await
            }
            Some(Err(e)) => Err(TransportError::ReceiveFailed(e.to_string())),
            None => {
                self.stream = None;
                Err(TransportError::ConnectionClosed)
            }
        }
    }

    #[instrument(skip(self))]
    async fn close(&mut self) -> Result<(), TransportError> {
        if let Some(mut stream) = self.stream.take() {
            stream
                .close(None)
                .await
                .map_err(|e| TransportError::SendFailed(e.to_string()))?;
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    fn endpoint(&self) -> &str {
        &self.url
    }
}

/// Mock transport for testing
///
/// Allows injecting predefined responses and capturing sent messages.
#[cfg(any(test, feature = "test-utils"))]
pub struct MockTransport {
    url: String,
    connected: bool,
    /// Messages to return on recv()
    pub responses: std::collections::VecDeque<Result<Option<String>, TransportError>>,
    /// Messages captured from send()
    pub sent_messages: Vec<String>,
    /// Simulate connection failure
    pub fail_connect: bool,
    /// Simulate send failure
    pub fail_send: bool,
}

#[cfg(any(test, feature = "test-utils"))]
impl MockTransport {
    /// Create a new mock transport
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connected: false,
            responses: std::collections::VecDeque::new(),
            sent_messages: Vec::new(),
            fail_connect: false,
            fail_send: false,
        }
    }

    /// Add a response to be returned on recv()
    pub fn push_response(&mut self, msg: impl Into<String>) {
        self.responses.push_back(Ok(Some(msg.into())));
    }

    /// Add multiple responses
    pub fn push_responses(&mut self, msgs: impl IntoIterator<Item = impl Into<String>>) {
        for msg in msgs {
            self.push_response(msg);
        }
    }

    /// Simulate a close
    pub fn push_close(&mut self) {
        self.responses.push_back(Ok(None));
    }

    /// Simulate a receive error
    pub fn push_error(&mut self, error: TransportError) {
        self.responses.push_back(Err(error));
    }

    /// Get sent messages
    pub fn take_sent(&mut self) -> Vec<String> {
        std::mem::take(&mut self.sent_messages)
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl Transport for MockTransport {
    async fn connect(&mut self) -> Result<(), TransportError> {
        if self.fail_connect {
            return Err(TransportError::ConnectionFailed("mock connection failure".into()));
        }
        self.connected = true;
        Ok(())
    }

    async fn send(&mut self, message: &str) -> Result<(), TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        if self.fail_send {
            return Err(TransportError::SendFailed("mock send failure".into()));
        }
        self.sent_messages.push(message.to_string());
        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<String>, TransportError> {
        if !self.connected {
            return Err(TransportError::NotConnected);
        }
        self.responses
            .pop_front()
            .unwrap_or(Err(TransportError::ConnectionClosed))
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn endpoint(&self) -> &str {
        &self.url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_transport_send_recv() {
        let mut transport = MockTransport::new("wss://mock.test");
        transport.push_response(r#"{"type":"pong"}"#);

        transport.connect().await.unwrap();
        assert!(transport.is_connected());

        transport.send(r#"{"type":"ping"}"#).await.unwrap();
        assert_eq!(transport.sent_messages.len(), 1);
        assert!(transport.sent_messages[0].contains("ping"));

        let response = transport.recv().await.unwrap();
        assert!(response.unwrap().contains("pong"));
    }

    #[tokio::test]
    async fn test_mock_transport_connection_failure() {
        let mut transport = MockTransport::new("wss://mock.test");
        transport.fail_connect = true;

        let result = transport.connect().await;
        assert!(result.is_err());
        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_mock_transport_close() {
        let mut transport = MockTransport::new("wss://mock.test");
        transport.push_close();

        transport.connect().await.unwrap();
        let response = transport.recv().await.unwrap();
        assert!(response.is_none()); // Closed
    }
}
