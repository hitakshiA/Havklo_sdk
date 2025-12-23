//! Prometheus metrics for Kraken SDK
//!
//! This module provides comprehensive Prometheus metrics for monitoring
//! the SDK's performance, health, and trading activity in production.
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
//! - `kraken_rate_limit_rejections_total` - Rate limit rejections by category
//! - `kraken_rest_requests_total` - REST API requests by endpoint
//! - `kraken_orders_total` - Orders by type and status
//! - `kraken_token_refreshes_total` - Token refresh operations
//!
//! ## Gauges
//! - `kraken_connection_status` - Connection status (0=disconnected, 1=connected)
//! - `kraken_subscriptions_active` - Number of active subscriptions
//! - `kraken_rate_limit_tokens` - Available rate limit tokens by category
//! - `kraken_orderbook_depth` - Orderbook depth by symbol and side
//! - `kraken_orderbook_spread` - Current spread by symbol
//! - `kraken_orderbook_imbalance` - Bid/ask imbalance ratio by symbol
//! - `kraken_position_pnl` - Unrealized P&L by product (futures)
//! - `kraken_funding_rate` - Current funding rate by product (futures)
//!
//! ## Histograms
//! - `kraken_message_processing_seconds` - Message processing latency
//! - `kraken_rest_request_seconds` - REST API request latency
//! - `kraken_network_latency_seconds` - WebSocket ping/pong latency
//! - `kraken_orderbook_update_seconds` - Orderbook update processing time

use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge, register_gauge_vec, register_histogram_vec,
    CounterVec, Gauge, GaugeVec, HistogramVec,
};

lazy_static! {
    // ========================================================================
    // Counters - Monotonically increasing values
    // ========================================================================

    /// Total messages received from Kraken
    pub static ref MESSAGES_RECEIVED: CounterVec = register_counter_vec!(
        "kraken_messages_received_total",
        "Total messages received from Kraken",
        &["channel", "symbol"]
    ).unwrap();

    /// Total messages sent to Kraken
    pub static ref MESSAGES_SENT: CounterVec = register_counter_vec!(
        "kraken_messages_sent_total",
        "Total messages sent to Kraken",
        &["type"]
    ).unwrap();

    /// Total errors encountered
    pub static ref ERRORS: CounterVec = register_counter_vec!(
        "kraken_errors_total",
        "Total errors encountered",
        &["type", "category"]
    ).unwrap();

    /// Total reconnection attempts
    pub static ref RECONNECTIONS: CounterVec = register_counter_vec!(
        "kraken_reconnections_total",
        "Total reconnection attempts",
        &["reason", "success"]
    ).unwrap();

    /// Checksum validation failures
    pub static ref CHECKSUM_FAILURES: CounterVec = register_counter_vec!(
        "kraken_checksum_failures_total",
        "Total checksum validation failures",
        &["symbol"]
    ).unwrap();

    /// Rate limit rejections by category
    pub static ref RATE_LIMIT_REJECTIONS: CounterVec = register_counter_vec!(
        "kraken_rate_limit_rejections_total",
        "Total rate limit rejections",
        &["category"]
    ).unwrap();

    /// REST API requests by endpoint
    pub static ref REST_REQUESTS: CounterVec = register_counter_vec!(
        "kraken_rest_requests_total",
        "Total REST API requests",
        &["endpoint", "method", "status"]
    ).unwrap();

    /// Orders by type and status
    pub static ref ORDERS: CounterVec = register_counter_vec!(
        "kraken_orders_total",
        "Total orders submitted",
        &["type", "side", "status"]
    ).unwrap();

    /// Token refresh operations
    pub static ref TOKEN_REFRESHES: CounterVec = register_counter_vec!(
        "kraken_token_refreshes_total",
        "Total token refresh operations",
        &["success"]
    ).unwrap();

    /// L3 order events
    pub static ref L3_ORDER_EVENTS: CounterVec = register_counter_vec!(
        "kraken_l3_order_events_total",
        "Total L3 order events",
        &["symbol", "event_type"]
    ).unwrap();

    // ========================================================================
    // Gauges - Point-in-time values
    // ========================================================================

    /// Connection status (0=disconnected, 1=connected)
    pub static ref CONNECTION_STATUS: Gauge = register_gauge!(
        "kraken_connection_status",
        "Connection status (0=disconnected, 1=connected)"
    ).unwrap();

    /// Number of active subscriptions
    pub static ref SUBSCRIPTIONS_ACTIVE: Gauge = register_gauge!(
        "kraken_subscriptions_active",
        "Number of active subscriptions"
    ).unwrap();

    /// Available rate limit tokens by category
    pub static ref RATE_LIMIT_TOKENS: GaugeVec = register_gauge_vec!(
        "kraken_rate_limit_tokens",
        "Available rate limit tokens",
        &["category"]
    ).unwrap();

    /// Rate limit utilization (0.0-1.0)
    pub static ref RATE_LIMIT_UTILIZATION: GaugeVec = register_gauge_vec!(
        "kraken_rate_limit_utilization",
        "Rate limit utilization (0.0-1.0)",
        &["category"]
    ).unwrap();

    /// Orderbook depth by symbol and side
    pub static ref ORDERBOOK_DEPTH: GaugeVec = register_gauge_vec!(
        "kraken_orderbook_depth",
        "Orderbook depth (number of levels)",
        &["symbol", "side"]
    ).unwrap();

    /// Current spread by symbol (in quote currency)
    pub static ref ORDERBOOK_SPREAD: GaugeVec = register_gauge_vec!(
        "kraken_orderbook_spread",
        "Current orderbook spread",
        &["symbol"]
    ).unwrap();

    /// Best bid price by symbol
    pub static ref ORDERBOOK_BEST_BID: GaugeVec = register_gauge_vec!(
        "kraken_orderbook_best_bid",
        "Best bid price",
        &["symbol"]
    ).unwrap();

    /// Best ask price by symbol
    pub static ref ORDERBOOK_BEST_ASK: GaugeVec = register_gauge_vec!(
        "kraken_orderbook_best_ask",
        "Best ask price",
        &["symbol"]
    ).unwrap();

    /// Mid price by symbol
    pub static ref ORDERBOOK_MID_PRICE: GaugeVec = register_gauge_vec!(
        "kraken_orderbook_mid_price",
        "Mid price",
        &["symbol"]
    ).unwrap();

    /// Bid/ask imbalance ratio by symbol (-1.0 to 1.0)
    pub static ref ORDERBOOK_IMBALANCE: GaugeVec = register_gauge_vec!(
        "kraken_orderbook_imbalance",
        "Bid/ask imbalance ratio (-1.0 to 1.0)",
        &["symbol"]
    ).unwrap();

    /// Total quantity on each side
    pub static ref ORDERBOOK_TOTAL_QTY: GaugeVec = register_gauge_vec!(
        "kraken_orderbook_total_qty",
        "Total quantity on side",
        &["symbol", "side"]
    ).unwrap();

    /// Order count in L3 orderbook
    pub static ref L3_ORDER_COUNT: GaugeVec = register_gauge_vec!(
        "kraken_l3_order_count",
        "Number of orders in L3 book",
        &["symbol"]
    ).unwrap();

    /// Unrealized P&L by product (futures)
    pub static ref POSITION_PNL: GaugeVec = register_gauge_vec!(
        "kraken_position_pnl",
        "Unrealized P&L",
        &["product"]
    ).unwrap();

    /// Position size by product (futures)
    pub static ref POSITION_SIZE: GaugeVec = register_gauge_vec!(
        "kraken_position_size",
        "Position size",
        &["product", "side"]
    ).unwrap();

    /// Current funding rate by product (futures)
    pub static ref FUNDING_RATE: GaugeVec = register_gauge_vec!(
        "kraken_funding_rate",
        "Current funding rate (8h)",
        &["product"]
    ).unwrap();

    /// Mark price by product (futures)
    pub static ref MARK_PRICE: GaugeVec = register_gauge_vec!(
        "kraken_mark_price",
        "Mark price",
        &["product"]
    ).unwrap();

    /// Index price by product (futures)
    pub static ref INDEX_PRICE: GaugeVec = register_gauge_vec!(
        "kraken_index_price",
        "Index/spot price",
        &["product"]
    ).unwrap();

    /// Premium/discount to spot (futures)
    pub static ref PREMIUM_DISCOUNT: GaugeVec = register_gauge_vec!(
        "kraken_premium_discount",
        "Premium/discount to spot (%)",
        &["product"]
    ).unwrap();

    /// Token expiry timestamp
    pub static ref TOKEN_EXPIRY: Gauge = register_gauge!(
        "kraken_token_expiry_timestamp",
        "Token expiry Unix timestamp"
    ).unwrap();

    /// Seconds until token expiry
    pub static ref TOKEN_TTL: Gauge = register_gauge!(
        "kraken_token_ttl_seconds",
        "Seconds until token expiry"
    ).unwrap();

    // ========================================================================
    // Histograms - Distribution of values
    // ========================================================================

    /// Message processing duration
    pub static ref MESSAGE_PROCESSING_DURATION: HistogramVec = register_histogram_vec!(
        "kraken_message_processing_seconds",
        "Message processing duration in seconds",
        &["channel"],
        vec![0.00001, 0.00005, 0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    ).unwrap();

    /// REST API request latency
    pub static ref REST_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "kraken_rest_request_seconds",
        "REST API request duration in seconds",
        &["endpoint", "method"],
        vec![0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).unwrap();

    /// WebSocket ping/pong latency
    pub static ref NETWORK_LATENCY: HistogramVec = register_histogram_vec!(
        "kraken_network_latency_seconds",
        "WebSocket network latency in seconds",
        &["endpoint"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    ).unwrap();

    /// Orderbook update processing time
    pub static ref ORDERBOOK_UPDATE_DURATION: HistogramVec = register_histogram_vec!(
        "kraken_orderbook_update_seconds",
        "Orderbook update processing duration",
        &["symbol", "type"],
        vec![0.00001, 0.00005, 0.0001, 0.0005, 0.001, 0.005, 0.01]
    ).unwrap();

    /// Order roundtrip time (submission to acknowledgement)
    pub static ref ORDER_ROUNDTRIP: HistogramVec = register_histogram_vec!(
        "kraken_order_roundtrip_seconds",
        "Order roundtrip time",
        &["type"],
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    ).unwrap();

    /// Rate limit wait time
    pub static ref RATE_LIMIT_WAIT: HistogramVec = register_histogram_vec!(
        "kraken_rate_limit_wait_seconds",
        "Time spent waiting for rate limit",
        &["category"],
        vec![0.0, 0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0]
    ).unwrap();
}

// ============================================================================
// Helper Functions - Basic Recording
// ============================================================================

/// Record a received message
pub fn record_message_received(channel: &str, symbol: &str) {
    MESSAGES_RECEIVED.with_label_values(&[channel, symbol]).inc();
}

/// Record a sent message
pub fn record_message_sent(msg_type: &str) {
    MESSAGES_SENT.with_label_values(&[msg_type]).inc();
}

/// Record an error with category
pub fn record_error(error_type: &str, category: &str) {
    ERRORS.with_label_values(&[error_type, category]).inc();
}

/// Record a reconnection attempt
pub fn record_reconnection(reason: &str, success: bool) {
    RECONNECTIONS
        .with_label_values(&[reason, if success { "true" } else { "false" }])
        .inc();
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

// ============================================================================
// Helper Functions - Rate Limiting
// ============================================================================

/// Record a rate limit rejection
pub fn record_rate_limit_rejection(category: &str) {
    RATE_LIMIT_REJECTIONS.with_label_values(&[category]).inc();
}

/// Update rate limit token count
pub fn set_rate_limit_tokens(category: &str, tokens: f64) {
    RATE_LIMIT_TOKENS.with_label_values(&[category]).set(tokens);
}

/// Update rate limit utilization (0.0-1.0)
pub fn set_rate_limit_utilization(category: &str, utilization: f64) {
    RATE_LIMIT_UTILIZATION
        .with_label_values(&[category])
        .set(utilization);
}

/// Record time spent waiting for rate limit
pub fn record_rate_limit_wait(category: &str, duration_secs: f64) {
    RATE_LIMIT_WAIT
        .with_label_values(&[category])
        .observe(duration_secs);
}

// ============================================================================
// Helper Functions - Orderbook
// ============================================================================

/// Update orderbook depth metric
pub fn set_orderbook_depth(symbol: &str, side: &str, depth: usize) {
    ORDERBOOK_DEPTH
        .with_label_values(&[symbol, side])
        .set(depth as f64);
}

/// Update orderbook spread metric
pub fn set_orderbook_spread(symbol: &str, spread: f64) {
    ORDERBOOK_SPREAD.with_label_values(&[symbol]).set(spread);
}

/// Update best bid price
pub fn set_orderbook_best_bid(symbol: &str, price: f64) {
    ORDERBOOK_BEST_BID.with_label_values(&[symbol]).set(price);
}

/// Update best ask price
pub fn set_orderbook_best_ask(symbol: &str, price: f64) {
    ORDERBOOK_BEST_ASK.with_label_values(&[symbol]).set(price);
}

/// Update mid price
pub fn set_orderbook_mid_price(symbol: &str, price: f64) {
    ORDERBOOK_MID_PRICE.with_label_values(&[symbol]).set(price);
}

/// Update orderbook imbalance metric
pub fn set_orderbook_imbalance(symbol: &str, imbalance: f64) {
    ORDERBOOK_IMBALANCE.with_label_values(&[symbol]).set(imbalance);
}

/// Update total quantity on a side
pub fn set_orderbook_total_qty(symbol: &str, side: &str, qty: f64) {
    ORDERBOOK_TOTAL_QTY
        .with_label_values(&[symbol, side])
        .set(qty);
}

/// Record orderbook update duration
pub fn record_orderbook_update(symbol: &str, update_type: &str, duration_secs: f64) {
    ORDERBOOK_UPDATE_DURATION
        .with_label_values(&[symbol, update_type])
        .observe(duration_secs);
}

/// Update all orderbook metrics at once
pub fn update_orderbook_metrics(
    symbol: &str,
    bid_depth: usize,
    ask_depth: usize,
    best_bid: f64,
    best_ask: f64,
    spread: f64,
    mid_price: f64,
    imbalance: f64,
    total_bid_qty: f64,
    total_ask_qty: f64,
) {
    set_orderbook_depth(symbol, "bid", bid_depth);
    set_orderbook_depth(symbol, "ask", ask_depth);
    set_orderbook_best_bid(symbol, best_bid);
    set_orderbook_best_ask(symbol, best_ask);
    set_orderbook_spread(symbol, spread);
    set_orderbook_mid_price(symbol, mid_price);
    set_orderbook_imbalance(symbol, imbalance);
    set_orderbook_total_qty(symbol, "bid", total_bid_qty);
    set_orderbook_total_qty(symbol, "ask", total_ask_qty);
}

// ============================================================================
// Helper Functions - L3 Orderbook
// ============================================================================

/// Record L3 order event
pub fn record_l3_order_event(symbol: &str, event_type: &str) {
    L3_ORDER_EVENTS
        .with_label_values(&[symbol, event_type])
        .inc();
}

/// Update L3 order count
pub fn set_l3_order_count(symbol: &str, count: usize) {
    L3_ORDER_COUNT.with_label_values(&[symbol]).set(count as f64);
}

// ============================================================================
// Helper Functions - REST API
// ============================================================================

/// Record REST API request
pub fn record_rest_request(endpoint: &str, method: &str, status: &str) {
    REST_REQUESTS
        .with_label_values(&[endpoint, method, status])
        .inc();
}

/// Record REST API request duration
pub fn record_rest_duration(endpoint: &str, method: &str, duration_secs: f64) {
    REST_REQUEST_DURATION
        .with_label_values(&[endpoint, method])
        .observe(duration_secs);
}

// ============================================================================
// Helper Functions - Orders
// ============================================================================

/// Record an order submission
pub fn record_order(order_type: &str, side: &str, status: &str) {
    ORDERS.with_label_values(&[order_type, side, status]).inc();
}

/// Record order roundtrip time
pub fn record_order_roundtrip(order_type: &str, duration_secs: f64) {
    ORDER_ROUNDTRIP
        .with_label_values(&[order_type])
        .observe(duration_secs);
}

// ============================================================================
// Helper Functions - Token Management
// ============================================================================

/// Record token refresh operation
pub fn record_token_refresh(success: bool) {
    TOKEN_REFRESHES
        .with_label_values(&[if success { "true" } else { "false" }])
        .inc();
}

/// Set token expiry timestamp
pub fn set_token_expiry(timestamp: f64) {
    TOKEN_EXPIRY.set(timestamp);
}

/// Set token TTL in seconds
pub fn set_token_ttl(seconds: f64) {
    TOKEN_TTL.set(seconds);
}

// ============================================================================
// Helper Functions - Futures
// ============================================================================

/// Update position P&L
pub fn set_position_pnl(product: &str, pnl: f64) {
    POSITION_PNL.with_label_values(&[product]).set(pnl);
}

/// Update position size
pub fn set_position_size(product: &str, side: &str, size: f64) {
    POSITION_SIZE.with_label_values(&[product, side]).set(size);
}

/// Update funding rate
pub fn set_funding_rate(product: &str, rate: f64) {
    FUNDING_RATE.with_label_values(&[product]).set(rate);
}

/// Update mark price
pub fn set_mark_price(product: &str, price: f64) {
    MARK_PRICE.with_label_values(&[product]).set(price);
}

/// Update index price
pub fn set_index_price(product: &str, price: f64) {
    INDEX_PRICE.with_label_values(&[product]).set(price);
}

/// Update premium/discount percentage
pub fn set_premium_discount(product: &str, premium_pct: f64) {
    PREMIUM_DISCOUNT.with_label_values(&[product]).set(premium_pct);
}

/// Update all futures position metrics
pub fn update_position_metrics(
    product: &str,
    side: &str,
    size: f64,
    pnl: f64,
    mark_price: f64,
    index_price: f64,
    funding_rate: f64,
) {
    set_position_size(product, side, size);
    set_position_pnl(product, pnl);
    set_mark_price(product, mark_price);
    set_index_price(product, index_price);
    set_funding_rate(product, funding_rate);

    // Calculate premium/discount
    if index_price > 0.0 {
        let premium = (mark_price - index_price) / index_price * 100.0;
        set_premium_discount(product, premium);
    }
}

// ============================================================================
// Helper Functions - Network
// ============================================================================

/// Record network latency (ping/pong)
pub fn record_network_latency(endpoint: &str, duration_secs: f64) {
    NETWORK_LATENCY
        .with_label_values(&[endpoint])
        .observe(duration_secs);
}

// ============================================================================
// Registry and Export
// ============================================================================

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

/// Reset all metrics (useful for testing)
#[cfg(test)]
pub fn reset_all() {
    // Note: Prometheus metrics cannot be truly reset, but this
    // is useful for test isolation by setting gauges to 0
    CONNECTION_STATUS.set(0.0);
    SUBSCRIPTIONS_ACTIVE.set(0.0);
    TOKEN_EXPIRY.set(0.0);
    TOKEN_TTL.set(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registration() {
        // Verify all metrics can be accessed without panic
        let _ = &*MESSAGES_RECEIVED;
        let _ = &*ERRORS;
        let _ = &*CONNECTION_STATUS;
        let _ = &*ORDERBOOK_SPREAD;
        let _ = &*MESSAGE_PROCESSING_DURATION;
    }

    #[test]
    fn test_encode_metrics() {
        // Record some test data
        record_message_received("book", "BTC/USD");
        set_connection_status(true);

        // Encode and verify output
        let output = encode_metrics();
        assert!(output.contains("kraken_messages_received_total"));
        assert!(output.contains("kraken_connection_status"));
    }

    #[test]
    fn test_orderbook_metrics() {
        update_orderbook_metrics(
            "BTC/USD",
            10,
            10,
            50000.0,
            50001.0,
            1.0,
            50000.5,
            0.1,
            100.0,
            90.0,
        );

        let output = encode_metrics();
        assert!(output.contains("kraken_orderbook_spread"));
        assert!(output.contains("kraken_orderbook_imbalance"));
    }
}
