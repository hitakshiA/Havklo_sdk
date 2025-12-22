//! Prometheus metrics for Kraken SDK
//!
//! This module provides optional Prometheus metrics for monitoring
//! the SDK's performance and health in production environments.
//!
//! # Enabling Metrics
//!
//! Add the `metrics` feature to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! kraken-sdk = { version = "0.1", features = ["metrics"] }
//! ```
//!
//! # Available Metrics
//!
//! ## Counters
//! - `kraken_messages_received_total` - Total messages received by channel
//! - `kraken_messages_sent_total` - Total messages sent by type
//! - `kraken_errors_total` - Total errors by type
//! - `kraken_reconnections_total` - Total reconnection attempts
//! - `kraken_checksum_failures_total` - Checksum validation failures
//!
//! ## Gauges
//! - `kraken_connection_status` - Connection status (0=disconnected, 1=connected)
//! - `kraken_subscriptions_active` - Number of active subscriptions
//!
//! ## Histograms
//! - `kraken_message_processing_seconds` - Message processing latency

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Gauge, HistogramVec,
};

lazy_static! {
    // Counters
    pub static ref MESSAGES_RECEIVED: CounterVec = register_counter_vec!(
        "kraken_messages_received_total",
        "Total messages received from Kraken",
        &["channel", "symbol"]
    ).unwrap();

    pub static ref MESSAGES_SENT: CounterVec = register_counter_vec!(
        "kraken_messages_sent_total",
        "Total messages sent to Kraken",
        &["type"]
    ).unwrap();

    pub static ref ERRORS: CounterVec = register_counter_vec!(
        "kraken_errors_total",
        "Total errors encountered",
        &["type"]
    ).unwrap();

    pub static ref RECONNECTIONS: CounterVec = register_counter_vec!(
        "kraken_reconnections_total",
        "Total reconnection attempts",
        &["reason"]
    ).unwrap();

    pub static ref CHECKSUM_FAILURES: CounterVec = register_counter_vec!(
        "kraken_checksum_failures_total",
        "Total checksum validation failures",
        &["symbol"]
    ).unwrap();

    // Gauges
    pub static ref CONNECTION_STATUS: Gauge = register_gauge!(
        "kraken_connection_status",
        "Connection status (0=disconnected, 1=connected)"
    ).unwrap();

    pub static ref SUBSCRIPTIONS_ACTIVE: Gauge = register_gauge!(
        "kraken_subscriptions_active",
        "Number of active subscriptions"
    ).unwrap();

    // Histograms
    pub static ref MESSAGE_PROCESSING_DURATION: HistogramVec = register_histogram_vec!(
        "kraken_message_processing_seconds",
        "Message processing duration in seconds",
        &["channel"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    ).unwrap();
}

/// Record a received message
pub fn record_message_received(channel: &str, symbol: &str) {
    MESSAGES_RECEIVED.with_label_values(&[channel, symbol]).inc();
}

/// Record a sent message
pub fn record_message_sent(msg_type: &str) {
    MESSAGES_SENT.with_label_values(&[msg_type]).inc();
}

/// Record an error
pub fn record_error(error_type: &str) {
    ERRORS.with_label_values(&[error_type]).inc();
}

/// Record a reconnection attempt
pub fn record_reconnection(reason: &str) {
    RECONNECTIONS.with_label_values(&[reason]).inc();
}

/// Record a checksum failure
pub fn record_checksum_failure(symbol: &str) {
    CHECKSUM_FAILURES.with_label_values(&[symbol]).inc();
}

/// Set connection status (0=disconnected, 1=connected)
pub fn set_connection_status(connected: bool) {
    CONNECTION_STATUS.set(if connected { 1.0 } else { 0.0 });
}

/// Set the number of active subscriptions
pub fn set_subscriptions_active(count: usize) {
    SUBSCRIPTIONS_ACTIVE.set(count as f64);
}

/// Record message processing duration
pub fn record_processing_duration(channel: &str, duration_secs: f64) {
    MESSAGE_PROCESSING_DURATION
        .with_label_values(&[channel])
        .observe(duration_secs);
}

/// Get the default Prometheus registry for exposing metrics
pub fn registry() -> &'static prometheus::Registry {
    prometheus::default_registry()
}

/// Encode all metrics as text for Prometheus scraping
pub fn encode_metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
