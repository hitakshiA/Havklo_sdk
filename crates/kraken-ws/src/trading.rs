//! WebSocket trading functionality
//!
//! Provides trading operations over WebSocket connection for low-latency order management.
//! The `TradingClient` generates JSON-serializable request objects that can be sent over
//! a WebSocket connection.
//!
//! # Example
//!
//! ```
//! use kraken_ws::trading::TradingClient;
//! use kraken_types::{Side, Decimal};
//!
//! let client = TradingClient::new("your_ws_token".to_string());
//!
//! // Create a limit order request
//! let order_request = client.limit_order(
//!     "BTC/USD",
//!     Side::Buy,
//!     Decimal::new(1, 3),  // 0.001 BTC
//!     Decimal::new(50000, 0),  // $50,000
//! );
//!
//! // Serialize to JSON for sending over WebSocket
//! let json = serde_json::to_string(&order_request).unwrap();
//! assert!(json.contains("add_order"));
//!
//! // Create a cancel order request
//! let cancel_request = client.cancel_order("ORDER123");
//! ```

use kraken_types::{
    AddOrderParams, AddOrderRequest, AmendOrderParams, AmendOrderRequest,
    BatchAddParams, BatchAddRequest, BatchCancelParams, BatchCancelRequest,
    BatchOrder, CancelAllRequest, CancelOnDisconnectRequest, CancelOrderParams,
    CancelOrderRequest, Decimal, Side, TimeInForce,
};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

/// Trading client for WebSocket order management
///
/// This client generates properly formatted trading requests that can be
/// sent over a WebSocket connection.
#[derive(Debug)]
pub struct TradingClient {
    /// WebSocket authentication token
    token: String,
    /// Request ID counter
    req_id_counter: AtomicU64,
}

impl TradingClient {
    /// Create a new trading client with the given authentication token
    pub fn new(token: String) -> Self {
        Self {
            token,
            req_id_counter: AtomicU64::new(1),
        }
    }

    /// Get the next request ID
    fn next_req_id(&self) -> u64 {
        self.req_id_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Update the authentication token
    pub fn set_token(&mut self, token: String) {
        self.token = token;
    }

    /// Get the current token
    pub fn token(&self) -> &str {
        &self.token
    }

    // ========================================================================
    // Order Creation
    // ========================================================================

    /// Create a market order request
    pub fn market_order(&self, symbol: &str, side: Side, qty: Decimal) -> AddOrderRequest {
        let params = AddOrderParams {
            order_type: "market".to_string(),
            side,
            symbol: symbol.to_string(),
            order_qty: qty,
            limit_price: None,
            time_in_force: None,
            trigger_price: None,
            cl_ord_id: None,
            post_only: None,
            reduce_only: None,
            token: self.token.clone(),
        };
        AddOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a limit order request
    pub fn limit_order(
        &self,
        symbol: &str,
        side: Side,
        qty: Decimal,
        price: Decimal,
    ) -> AddOrderRequest {
        let params = AddOrderParams {
            order_type: "limit".to_string(),
            side,
            symbol: symbol.to_string(),
            order_qty: qty,
            limit_price: Some(price),
            time_in_force: Some(TimeInForce::GTC),
            trigger_price: None,
            cl_ord_id: None,
            post_only: None,
            reduce_only: None,
            token: self.token.clone(),
        };
        AddOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a post-only limit order request
    pub fn post_only_order(
        &self,
        symbol: &str,
        side: Side,
        qty: Decimal,
        price: Decimal,
    ) -> AddOrderRequest {
        let params = AddOrderParams {
            order_type: "limit".to_string(),
            side,
            symbol: symbol.to_string(),
            order_qty: qty,
            limit_price: Some(price),
            time_in_force: Some(TimeInForce::GTC),
            trigger_price: None,
            cl_ord_id: None,
            post_only: Some(true),
            reduce_only: None,
            token: self.token.clone(),
        };
        AddOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a stop-loss order request
    pub fn stop_loss_order(
        &self,
        symbol: &str,
        side: Side,
        qty: Decimal,
        trigger_price: Decimal,
    ) -> AddOrderRequest {
        let params = AddOrderParams {
            order_type: "stop-loss".to_string(),
            side,
            symbol: symbol.to_string(),
            order_qty: qty,
            limit_price: None,
            time_in_force: None,
            trigger_price: Some(trigger_price),
            cl_ord_id: None,
            post_only: None,
            reduce_only: None,
            token: self.token.clone(),
        };
        AddOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a stop-loss limit order request
    pub fn stop_loss_limit_order(
        &self,
        symbol: &str,
        side: Side,
        qty: Decimal,
        trigger_price: Decimal,
        limit_price: Decimal,
    ) -> AddOrderRequest {
        let params = AddOrderParams {
            order_type: "stop-loss-limit".to_string(),
            side,
            symbol: symbol.to_string(),
            order_qty: qty,
            limit_price: Some(limit_price),
            time_in_force: Some(TimeInForce::GTC),
            trigger_price: Some(trigger_price),
            cl_ord_id: None,
            post_only: None,
            reduce_only: None,
            token: self.token.clone(),
        };
        AddOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a take-profit order request
    pub fn take_profit_order(
        &self,
        symbol: &str,
        side: Side,
        qty: Decimal,
        trigger_price: Decimal,
    ) -> AddOrderRequest {
        let params = AddOrderParams {
            order_type: "take-profit".to_string(),
            side,
            symbol: symbol.to_string(),
            order_qty: qty,
            limit_price: None,
            time_in_force: None,
            trigger_price: Some(trigger_price),
            cl_ord_id: None,
            post_only: None,
            reduce_only: None,
            token: self.token.clone(),
        };
        AddOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a custom add order request with all parameters
    pub fn custom_order(&self, params: AddOrderParams) -> AddOrderRequest {
        AddOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    // ========================================================================
    // Order Amendment
    // ========================================================================

    /// Create an amend order request to change price
    pub fn amend_price(&self, order_id: &str, new_price: Decimal) -> AmendOrderRequest {
        let params = AmendOrderParams {
            order_id: order_id.to_string(),
            limit_price: Some(new_price),
            trigger_price: None,
            order_qty: None,
            post_only: None,
            token: self.token.clone(),
        };
        AmendOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create an amend order request to change quantity
    pub fn amend_qty(&self, order_id: &str, new_qty: Decimal) -> AmendOrderRequest {
        let params = AmendOrderParams {
            order_id: order_id.to_string(),
            limit_price: None,
            trigger_price: None,
            order_qty: Some(new_qty),
            post_only: None,
            token: self.token.clone(),
        };
        AmendOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create an amend order request with custom parameters
    pub fn amend_order(&self, params: AmendOrderParams) -> AmendOrderRequest {
        AmendOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    // ========================================================================
    // Order Cancellation
    // ========================================================================

    /// Create a cancel order request for a single order
    pub fn cancel_order(&self, order_id: &str) -> CancelOrderRequest {
        let params = CancelOrderParams {
            order_id: vec![order_id.to_string()],
            cl_ord_id: None,
            token: self.token.clone(),
        };
        CancelOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a cancel order request for multiple orders
    pub fn cancel_orders(&self, order_ids: Vec<String>) -> CancelOrderRequest {
        let params = CancelOrderParams {
            order_id: order_ids,
            cl_ord_id: None,
            token: self.token.clone(),
        };
        CancelOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a cancel order request by client order ID
    pub fn cancel_by_client_id(&self, cl_ord_id: &str) -> CancelOrderRequest {
        let params = CancelOrderParams {
            order_id: vec![],
            cl_ord_id: Some(vec![cl_ord_id.to_string()]),
            token: self.token.clone(),
        };
        CancelOrderRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a cancel all orders request
    pub fn cancel_all(&self) -> CancelAllRequest {
        CancelAllRequest::new(self.token.clone()).with_req_id(self.next_req_id())
    }

    /// Create a cancel on disconnect request
    ///
    /// After timeout seconds of disconnect, all orders will be canceled.
    /// Set timeout to 0 to disable.
    pub fn cancel_on_disconnect(&self, timeout_seconds: u32) -> CancelOnDisconnectRequest {
        CancelOnDisconnectRequest::new(timeout_seconds, self.token.clone())
            .with_req_id(self.next_req_id())
    }

    // ========================================================================
    // Batch Operations
    // ========================================================================

    /// Create a batch add orders request
    pub fn batch_add(&self, orders: Vec<BatchOrder>) -> BatchAddRequest {
        let params = BatchAddParams {
            orders,
            token: self.token.clone(),
            deadline: None,
            validate: None,
        };
        BatchAddRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a batch add orders request with validation only
    pub fn batch_add_validate(&self, orders: Vec<BatchOrder>) -> BatchAddRequest {
        let params = BatchAddParams {
            orders,
            token: self.token.clone(),
            deadline: None,
            validate: Some(true),
        };
        BatchAddRequest::new(params).with_req_id(self.next_req_id())
    }

    /// Create a batch cancel orders request
    pub fn batch_cancel(&self, order_ids: Vec<String>) -> BatchCancelRequest {
        let params = BatchCancelParams {
            orders: order_ids,
            cl_ord_id: None,
            token: self.token.clone(),
        };
        BatchCancelRequest::new(params).with_req_id(self.next_req_id())
    }
}

/// Trait for types that can be serialized to JSON for WebSocket sending
pub trait ToWsJson: Serialize {
    /// Serialize to JSON string
    fn to_ws_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl ToWsJson for AddOrderRequest {}
impl ToWsJson for AmendOrderRequest {}
impl ToWsJson for CancelOrderRequest {}
impl ToWsJson for CancelAllRequest {}
impl ToWsJson for CancelOnDisconnectRequest {}
impl ToWsJson for BatchAddRequest {}
impl ToWsJson for BatchCancelRequest {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_order() {
        let client = TradingClient::new("test_token".to_string());
        let order = client.market_order("BTC/USD", Side::Buy, Decimal::new(1, 3));

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("\"method\":\"add_order\""));
        assert!(json.contains("\"order_type\":\"market\""));
        assert!(json.contains("\"side\":\"buy\""));
        assert!(json.contains("\"symbol\":\"BTC/USD\""));
    }

    #[test]
    fn test_limit_order() {
        let client = TradingClient::new("test_token".to_string());
        let order = client.limit_order(
            "ETH/USD",
            Side::Sell,
            Decimal::new(5, 1),
            Decimal::new(3000, 0),
        );

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("\"order_type\":\"limit\""));
        assert!(json.contains("\"side\":\"sell\""));
        assert!(json.contains("\"limit_price\":\"3000\""));
    }

    #[test]
    fn test_cancel_order() {
        let client = TradingClient::new("test_token".to_string());
        let cancel = client.cancel_order("ORDER123");

        let json = serde_json::to_string(&cancel).unwrap();
        assert!(json.contains("\"method\":\"cancel_order\""));
        assert!(json.contains("ORDER123"));
    }

    #[test]
    fn test_request_id_increment() {
        let client = TradingClient::new("test_token".to_string());

        let order1 = client.market_order("BTC/USD", Side::Buy, Decimal::ONE);
        let order2 = client.market_order("BTC/USD", Side::Buy, Decimal::ONE);

        assert!(order1.req_id.unwrap() < order2.req_id.unwrap());
    }
}
