//! Observability hooks for connection lifecycle monitoring
//!
//! This module provides a flexible hooks API for observing connection events
//! without consuming from the main event stream. Useful for logging, metrics,
//! and custom monitoring integrations.
//!
//! # Example
//!
//! ```
//! use kraken_ws::hooks::Hooks;
//!
//! let hooks = Hooks::new()
//!     .on_connect(|info| {
//!         println!("Connected: {:?}", info);
//!     })
//!     .on_disconnect(|reason| {
//!         eprintln!("Disconnected: {:?}", reason);
//!     })
//!     .on_reconnect_attempt(|attempt, delay| {
//!         println!("Reconnecting (attempt {}), waiting {:?}", attempt, delay);
//!     });
//! ```

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// Information about a successful connection
#[derive(Debug, Clone)]
pub struct ConnectInfo {
    /// API version reported by server
    pub api_version: String,
    /// Connection ID assigned by server
    pub connection_id: u64,
    /// Whether this is a reconnection
    pub is_reconnection: bool,
}

/// Reason for disconnection
#[derive(Debug, Clone)]
pub enum DisconnectInfo {
    /// Server closed the connection
    ServerClosed,
    /// Network error occurred
    NetworkError(String),
    /// Connection timed out
    Timeout,
    /// Client requested shutdown
    Shutdown,
    /// Authentication failed
    AuthFailed,
    /// No heartbeat received within timeout
    HeartbeatTimeout,
}

/// Subscription status change information
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    /// Channel name
    pub channel: String,
    /// Symbols involved
    pub symbols: Vec<String>,
    /// Whether subscription was accepted
    pub accepted: bool,
    /// Rejection reason (if any)
    pub reason: Option<String>,
}

/// Orderbook checksum mismatch information
#[derive(Debug, Clone)]
pub struct ChecksumInfo {
    /// Symbol that failed checksum
    pub symbol: String,
    /// Expected checksum from Kraken
    pub expected: u32,
    /// Computed checksum locally
    pub computed: u32,
}

/// Type alias for hook callbacks
pub type ConnectHook = Arc<dyn Fn(&ConnectInfo) + Send + Sync>;
pub type DisconnectHook = Arc<dyn Fn(&DisconnectInfo) + Send + Sync>;
pub type ReconnectAttemptHook = Arc<dyn Fn(u32, Duration) + Send + Sync>;
pub type SubscriptionHook = Arc<dyn Fn(&SubscriptionInfo) + Send + Sync>;
pub type ChecksumHook = Arc<dyn Fn(&ChecksumInfo) + Send + Sync>;
pub type MessageHook = Arc<dyn Fn(usize) + Send + Sync>;
pub type ErrorHook = Arc<dyn Fn(&str) + Send + Sync>;

/// Observability hooks container
///
/// Allows registering callbacks for various connection lifecycle events.
/// All hooks are optional and executed synchronously. Keep hook callbacks
/// fast to avoid blocking the connection loop.
pub struct Hooks {
    /// Called when connection is established
    pub(crate) on_connect: Option<ConnectHook>,
    /// Called when connection is lost
    pub(crate) on_disconnect: Option<DisconnectHook>,
    /// Called before each reconnection attempt
    pub(crate) on_reconnect_attempt: Option<ReconnectAttemptHook>,
    /// Called when subscription status changes
    pub(crate) on_subscription: Option<SubscriptionHook>,
    /// Called when checksum mismatch is detected
    pub(crate) on_checksum_mismatch: Option<ChecksumHook>,
    /// Called on each message received (with byte count)
    pub(crate) on_message: Option<MessageHook>,
    /// Called on errors (with error message)
    pub(crate) on_error: Option<ErrorHook>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Hooks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hooks")
            .field("on_connect", &self.on_connect.as_ref().map(|_| "..."))
            .field("on_disconnect", &self.on_disconnect.as_ref().map(|_| "..."))
            .field("on_reconnect_attempt", &self.on_reconnect_attempt.as_ref().map(|_| "..."))
            .field("on_subscription", &self.on_subscription.as_ref().map(|_| "..."))
            .field("on_checksum_mismatch", &self.on_checksum_mismatch.as_ref().map(|_| "..."))
            .field("on_message", &self.on_message.as_ref().map(|_| "..."))
            .field("on_error", &self.on_error.as_ref().map(|_| "..."))
            .finish()
    }
}

impl Clone for Hooks {
    fn clone(&self) -> Self {
        Self {
            on_connect: self.on_connect.clone(),
            on_disconnect: self.on_disconnect.clone(),
            on_reconnect_attempt: self.on_reconnect_attempt.clone(),
            on_subscription: self.on_subscription.clone(),
            on_checksum_mismatch: self.on_checksum_mismatch.clone(),
            on_message: self.on_message.clone(),
            on_error: self.on_error.clone(),
        }
    }
}

impl Hooks {
    /// Create a new empty hooks container
    pub fn new() -> Self {
        Self {
            on_connect: None,
            on_disconnect: None,
            on_reconnect_attempt: None,
            on_subscription: None,
            on_checksum_mismatch: None,
            on_message: None,
            on_error: None,
        }
    }

    /// Register a callback for successful connections
    ///
    /// Called each time a connection is established (including reconnections).
    pub fn on_connect<F>(mut self, f: F) -> Self
    where
        F: Fn(&ConnectInfo) + Send + Sync + 'static,
    {
        self.on_connect = Some(Arc::new(f));
        self
    }

    /// Register a callback for disconnections
    ///
    /// Called when the connection is lost (before reconnection attempts begin).
    pub fn on_disconnect<F>(mut self, f: F) -> Self
    where
        F: Fn(&DisconnectInfo) + Send + Sync + 'static,
    {
        self.on_disconnect = Some(Arc::new(f));
        self
    }

    /// Register a callback for reconnection attempts
    ///
    /// Called before each reconnection attempt with the attempt number (1-indexed)
    /// and the delay before this attempt.
    pub fn on_reconnect_attempt<F>(mut self, f: F) -> Self
    where
        F: Fn(u32, Duration) + Send + Sync + 'static,
    {
        self.on_reconnect_attempt = Some(Arc::new(f));
        self
    }

    /// Register a callback for subscription status changes
    ///
    /// Called when a subscription is confirmed or rejected by the server.
    pub fn on_subscription<F>(mut self, f: F) -> Self
    where
        F: Fn(&SubscriptionInfo) + Send + Sync + 'static,
    {
        self.on_subscription = Some(Arc::new(f));
        self
    }

    /// Register a callback for checksum mismatches
    ///
    /// Called when an orderbook checksum validation fails.
    /// The SDK will automatically request a new snapshot.
    pub fn on_checksum_mismatch<F>(mut self, f: F) -> Self
    where
        F: Fn(&ChecksumInfo) + Send + Sync + 'static,
    {
        self.on_checksum_mismatch = Some(Arc::new(f));
        self
    }

    /// Register a callback for received messages
    ///
    /// Called on each WebSocket message with the message size in bytes.
    /// Useful for bandwidth monitoring.
    pub fn on_message<F>(mut self, f: F) -> Self
    where
        F: Fn(usize) + Send + Sync + 'static,
    {
        self.on_message = Some(Arc::new(f));
        self
    }

    /// Register a callback for errors
    ///
    /// Called when an error occurs with the error message.
    pub fn on_error<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_error = Some(Arc::new(f));
        self
    }

    // Internal helper methods for invoking hooks
    // These are provided for integration with the connection module.
    // Allow dead_code since they may not be used in all configurations.

    #[allow(dead_code)]
    pub(crate) fn invoke_connect(&self, info: &ConnectInfo) {
        if let Some(ref hook) = self.on_connect {
            hook(info);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invoke_disconnect(&self, info: &DisconnectInfo) {
        if let Some(ref hook) = self.on_disconnect {
            hook(info);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invoke_reconnect_attempt(&self, attempt: u32, delay: Duration) {
        if let Some(ref hook) = self.on_reconnect_attempt {
            hook(attempt, delay);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invoke_subscription(&self, info: &SubscriptionInfo) {
        if let Some(ref hook) = self.on_subscription {
            hook(info);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invoke_checksum_mismatch(&self, info: &ChecksumInfo) {
        if let Some(ref hook) = self.on_checksum_mismatch {
            hook(info);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invoke_message(&self, size: usize) {
        if let Some(ref hook) = self.on_message {
            hook(size);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invoke_error(&self, msg: &str) {
        if let Some(ref hook) = self.on_error {
            hook(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_hooks_builder() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let hooks = Hooks::new()
            .on_connect(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });

        let info = ConnectInfo {
            api_version: "v2".to_string(),
            connection_id: 123,
            is_reconnection: false,
        };

        hooks.invoke_connect(&info);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_hooks_clone() {
        let hooks = Hooks::new()
            .on_connect(|_| {})
            .on_disconnect(|_| {});

        let cloned = hooks.clone();
        assert!(cloned.on_connect.is_some());
        assert!(cloned.on_disconnect.is_some());
    }

    #[test]
    fn test_hooks_default() {
        let hooks = Hooks::default();
        // Should not panic when invoking empty hooks
        hooks.invoke_connect(&ConnectInfo {
            api_version: "v2".to_string(),
            connection_id: 0,
            is_reconnection: false,
        });
    }
}
