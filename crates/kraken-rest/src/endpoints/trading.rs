//! Trading endpoints for order management
//!
//! These endpoints require authentication.

use crate::auth::{Credentials, RequestSigner};
use crate::error::{RestError, RestResult};
use crate::types::{
    ApiResponse, CancelOrderResult, EditOrderResult, OrderRequest, OrderResponse,
};
use reqwest::Client;
use tracing::{debug, instrument, warn};

const BASE_URL: &str = "https://api.kraken.com";

/// Trading endpoints for order management
pub struct TradingEndpoints<'a> {
    client: &'a Client,
    credentials: &'a Credentials,
}

impl<'a> TradingEndpoints<'a> {
    pub fn new(client: &'a Client, credentials: &'a Credentials) -> Self {
        Self { client, credentials }
    }

    /// Make an authenticated POST request
    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> RestResult<T> {
        let signer = RequestSigner::new(self.credentials, path);
        let nonce = signer.nonce();

        // Build POST data with nonce
        let mut post_params: Vec<(&str, &str)> = vec![("nonce", nonce)];
        post_params.extend_from_slice(params);

        let post_data = serde_urlencoded::to_string(&post_params)
            .map_err(|e| RestError::InvalidParameter(e.to_string()))?;

        let signature = signer.sign(&post_data);
        let url = format!("{}{}", BASE_URL, path);

        debug!("Making authenticated request to {}", path);

        let response: ApiResponse<T> = self
            .client
            .post(&url)
            .header("API-Key", signer.api_key())
            .header("API-Sign", signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(post_data)
            .send()
            .await?
            .json()
            .await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Add a new order
    ///
    /// # Arguments
    /// * `order` - Order request with all parameters
    ///
    /// # Returns
    /// Order response with transaction ID(s)
    #[instrument(skip(self, order), fields(pair = %order.pair, side = ?order.side, order_type = ?order.order_type))]
    pub async fn add_order(&self, order: &OrderRequest) -> RestResult<OrderResponse> {
        let mut params: Vec<(&str, String)> = vec![
            ("pair", order.pair.clone()),
            ("type", order.side.to_string()),
            ("ordertype", order.order_type.to_string()),
            ("volume", order.volume.to_string()),
        ];

        if let Some(price) = &order.price {
            params.push(("price", price.to_string()));
        }
        if let Some(price2) = &order.price2 {
            params.push(("price2", price2.to_string()));
        }
        if let Some(tif) = &order.time_in_force {
            params.push(("timeinforce", tif.to_string()));
        }
        if let Some(leverage) = &order.leverage {
            params.push(("leverage", leverage.clone()));
        }
        if !order.flags.is_empty() {
            let flags: Vec<&str> = order.flags.iter().map(|f| f.as_str()).collect();
            params.push(("oflags", flags.join(",")));
        }
        if let Some(starttm) = &order.starttm {
            params.push(("starttm", starttm.clone()));
        }
        if let Some(expiretm) = &order.expiretm {
            params.push(("expiretm", expiretm.clone()));
        }
        if let Some(userref) = order.userref {
            params.push(("userref", userref.to_string()));
        }
        if order.validate {
            params.push(("validate", "true".to_string()));
        }
        if let Some(close_type) = &order.close_order_type {
            params.push(("close[ordertype]", close_type.to_string()));
        }
        if let Some(close_price) = &order.close_price {
            params.push(("close[price]", close_price.to_string()));
        }
        if let Some(close_price2) = &order.close_price2 {
            params.push(("close[price2]", close_price2.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!(
            "Placing {} {} order for {} {}",
            order.side, order.order_type, order.volume, order.pair
        );

        self.post("/0/private/AddOrder", &params_ref).await
    }

    /// Add multiple orders in a batch
    ///
    /// # Arguments
    /// * `pair` - Trading pair (all orders must be for same pair)
    /// * `orders` - List of order requests
    /// * `validate` - Validate only, don't submit
    ///
    /// # Returns
    /// Results for each order
    #[instrument(skip(self, orders), fields(pair = %pair, count = orders.len()))]
    pub async fn add_order_batch(
        &self,
        pair: &str,
        orders: &[OrderRequest],
        validate: bool,
    ) -> RestResult<BatchOrderResult> {
        if orders.is_empty() {
            return Err(RestError::InvalidParameter("Empty order list".to_string()));
        }

        if orders.len() > 15 {
            warn!("Batch order limit is 15, submitting first 15 orders only");
        }

        // Build orders JSON
        let orders_json: Vec<serde_json::Value> = orders
            .iter()
            .take(15)
            .map(|o| {
                let mut order = serde_json::json!({
                    "type": o.side.to_string(),
                    "ordertype": o.order_type.to_string(),
                    "volume": o.volume.to_string(),
                });

                if let Some(price) = &o.price {
                    order["price"] = serde_json::json!(price.to_string());
                }
                if let Some(price2) = &o.price2 {
                    order["price2"] = serde_json::json!(price2.to_string());
                }
                if let Some(tif) = &o.time_in_force {
                    order["timeinforce"] = serde_json::json!(tif.to_string());
                }
                if !o.flags.is_empty() {
                    let flags: Vec<&str> = o.flags.iter().map(|f| f.as_str()).collect();
                    order["oflags"] = serde_json::json!(flags.join(","));
                }
                if let Some(userref) = o.userref {
                    order["userref"] = serde_json::json!(userref);
                }

                order
            })
            .collect();

        let orders_str = serde_json::to_string(&orders_json)
            .map_err(|e| RestError::InvalidParameter(e.to_string()))?;

        let mut params: Vec<(&str, &str)> = vec![("pair", pair), ("orders", &orders_str)];

        if validate {
            params.push(("validate", "true"));
        }

        debug!("Placing batch of {} orders for {}", orders.len().min(15), pair);

        self.post("/0/private/AddOrderBatch", &params).await
    }

    /// Cancel an order
    ///
    /// # Arguments
    /// * `txid` - Transaction ID of order to cancel
    #[instrument(skip(self))]
    pub async fn cancel_order(&self, txid: &str) -> RestResult<CancelOrderResult> {
        let params = [("txid", txid)];
        debug!("Cancelling order {}", txid);
        self.post("/0/private/CancelOrder", &params).await
    }

    /// Cancel all open orders
    #[instrument(skip(self))]
    pub async fn cancel_all_orders(&self) -> RestResult<CancelOrderResult> {
        debug!("Cancelling all open orders");
        self.post("/0/private/CancelAll", &[]).await
    }

    /// Cancel all orders after timeout (dead man's switch)
    ///
    /// # Arguments
    /// * `timeout` - Timeout in seconds (0 to disable)
    #[instrument(skip(self))]
    pub async fn cancel_all_orders_after(&self, timeout: u32) -> RestResult<CancelAllAfterResult> {
        let timeout_str = timeout.to_string();
        let params = [("timeout", timeout_str.as_str())];
        debug!("Setting cancel-all-after to {} seconds", timeout);
        self.post("/0/private/CancelAllOrdersAfter", &params).await
    }

    /// Edit an existing order
    ///
    /// # Arguments
    /// * `txid` - Transaction ID of order to edit
    /// * `pair` - Trading pair
    /// * `volume` - New volume (optional)
    /// * `price` - New price (optional)
    /// * `price2` - New secondary price (optional)
    /// * `userref` - New user reference (optional)
    /// * `validate` - Validate only, don't submit
    #[instrument(skip(self))]
    pub async fn edit_order(
        &self,
        txid: &str,
        pair: &str,
        volume: Option<&str>,
        price: Option<&str>,
        price2: Option<&str>,
        userref: Option<i32>,
        validate: bool,
    ) -> RestResult<EditOrderResult> {
        let mut params: Vec<(&str, String)> = vec![
            ("txid", txid.to_string()),
            ("pair", pair.to_string()),
        ];

        if let Some(volume) = volume {
            params.push(("volume", volume.to_string()));
        }
        if let Some(price) = price {
            params.push(("price", price.to_string()));
        }
        if let Some(price2) = price2 {
            params.push(("price2", price2.to_string()));
        }
        if let Some(userref) = userref {
            params.push(("userref", userref.to_string()));
        }
        if validate {
            params.push(("validate", "true".to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!("Editing order {}", txid);
        self.post("/0/private/EditOrder", &params_ref).await
    }

    /// Cancel multiple orders by transaction ID
    ///
    /// # Arguments
    /// * `txids` - List of transaction IDs to cancel
    #[instrument(skip(self), fields(count = txids.len()))]
    pub async fn cancel_order_batch(&self, txids: &[&str]) -> RestResult<BatchCancelResult> {
        if txids.is_empty() {
            return Err(RestError::InvalidParameter("Empty txid list".to_string()));
        }

        let orders_json = serde_json::to_string(&txids)
            .map_err(|e| RestError::InvalidParameter(e.to_string()))?;

        let params = [("orders", orders_json.as_str())];

        debug!("Cancelling {} orders", txids.len());
        self.post("/0/private/CancelOrderBatch", &params).await
    }
}

// Response types specific to trading endpoints

use serde::Deserialize;

/// Batch order result
#[derive(Debug, Clone, Deserialize)]
pub struct BatchOrderResult {
    /// Individual order results
    pub orders: Vec<BatchOrderEntry>,
}

/// Individual order in batch result
#[derive(Debug, Clone, Deserialize)]
pub struct BatchOrderEntry {
    /// Transaction ID (if successful)
    pub txid: Option<String>,
    /// Order description
    pub descr: Option<OrderResponseDescr>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Order description in response
#[derive(Debug, Clone, Deserialize)]
pub struct OrderResponseDescr {
    /// Order description
    pub order: String,
}

/// Cancel all after result
#[derive(Debug, Clone, Deserialize)]
pub struct CancelAllAfterResult {
    /// Current time
    #[serde(rename = "currentTime")]
    pub current_time: String,
    /// Trigger time
    #[serde(rename = "triggerTime")]
    pub trigger_time: String,
}

/// Batch cancel result
#[derive(Debug, Clone, Deserialize)]
pub struct BatchCancelResult {
    /// Number cancelled
    pub count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderFlag, OrderSide, OrderType};
    use rust_decimal::Decimal;

    #[test]
    fn test_order_request_serialization() {
        let order = OrderRequest::limit("XBTUSD", OrderSide::Buy, Decimal::new(1, 3), Decimal::from(50000))
            .post_only()
            .with_userref(12345);

        assert_eq!(order.pair, "XBTUSD");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.order_type, OrderType::Limit);
        assert!(order.flags.contains(&OrderFlag::PostOnly));
        assert_eq!(order.userref, Some(12345));
    }
}
