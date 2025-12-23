//! Main REST client implementation

use crate::auth::Credentials;
use crate::endpoints::{AccountEndpoints, EarnEndpoints, FundingEndpoints, MarketEndpoints, TradingEndpoints};
use crate::error::{RestError, RestResult};
use crate::types::{
    BalanceInfo, CancelOrderResult, EditOrderResult, OrderRequest, OrderResponse, OrderbookData,
    TickerInfo,
};
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;

/// Default request timeout
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Kraken REST API client
///
/// Provides access to both public and private endpoints.
///
/// # Example
///
/// ```no_run
/// use kraken_rest::{KrakenRestClient, Credentials};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Public endpoints only
///     let client = KrakenRestClient::new();
///     let ticker = client.get_ticker("XBTUSD").await?;
///
///     // With authentication for private endpoints
///     let creds = Credentials::from_env()?;
///     let auth_client = KrakenRestClient::with_credentials(creds);
///     let balance = auth_client.get_balance().await?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct KrakenRestClient {
    http_client: Client,
    credentials: Option<Credentials>,
}

impl KrakenRestClient {
    /// Create a new client without authentication
    ///
    /// Only public endpoints will be available.
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new client with credentials
    ///
    /// All endpoints (public and private) will be available.
    pub fn with_credentials(credentials: Credentials) -> Self {
        let mut config = ClientConfig::default();
        config.credentials = Some(credentials);
        Self::with_config(config)
    }

    /// Create a new client with custom configuration
    pub fn with_config(config: ClientConfig) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent(config.user_agent.as_deref().unwrap_or("kraken-rest/0.1.0"))
            .build()
            .expect("Failed to create HTTP client");

        info!("Created Kraken REST client");

        Self {
            http_client,
            credentials: config.credentials,
        }
    }

    /// Check if the client has credentials for private endpoints
    pub fn has_credentials(&self) -> bool {
        self.credentials.is_some()
    }

    // ========================================================================
    // Public Market Endpoints
    // ========================================================================

    /// Get market endpoints
    pub fn market(&self) -> MarketEndpoints<'_> {
        MarketEndpoints::new(&self.http_client)
    }

    /// Get ticker information for a trading pair
    ///
    /// # Arguments
    /// * `pair` - Trading pair (e.g., "XBTUSD", "ETHUSD")
    pub async fn get_ticker(&self, pair: &str) -> RestResult<HashMap<String, TickerInfo>> {
        self.market().get_ticker(pair).await
    }

    /// Get ticker information for multiple trading pairs
    pub async fn get_tickers(&self, pairs: &[&str]) -> RestResult<HashMap<String, TickerInfo>> {
        self.market().get_tickers(pairs).await
    }

    /// Get orderbook depth for a trading pair
    ///
    /// # Arguments
    /// * `pair` - Trading pair
    /// * `count` - Number of price levels (1-500)
    pub async fn get_orderbook(
        &self,
        pair: &str,
        count: Option<u16>,
    ) -> RestResult<HashMap<String, OrderbookData>> {
        self.market().get_orderbook(pair, count).await
    }

    // ========================================================================
    // Private Account Endpoints
    // ========================================================================

    /// Get account endpoints (requires credentials)
    pub fn account(&self) -> RestResult<AccountEndpoints<'_>> {
        let creds = self.credentials.as_ref().ok_or(RestError::AuthRequired)?;
        Ok(AccountEndpoints::new(&self.http_client, creds))
    }

    /// Get account balance
    pub async fn get_balance(&self) -> RestResult<BalanceInfo> {
        self.account()?.get_balance().await
    }

    /// Get open orders
    pub async fn get_open_orders(&self) -> RestResult<crate::endpoints::account::OpenOrdersResult> {
        self.account()?.get_open_orders(None, None).await
    }

    // ========================================================================
    // Private Trading Endpoints
    // ========================================================================

    /// Get trading endpoints (requires credentials)
    pub fn trading(&self) -> RestResult<TradingEndpoints<'_>> {
        let creds = self.credentials.as_ref().ok_or(RestError::AuthRequired)?;
        Ok(TradingEndpoints::new(&self.http_client, creds))
    }

    /// Place a new order
    pub async fn add_order(&self, order: &OrderRequest) -> RestResult<OrderResponse> {
        self.trading()?.add_order(order).await
    }

    /// Cancel an order by transaction ID
    pub async fn cancel_order(&self, txid: &str) -> RestResult<CancelOrderResult> {
        self.trading()?.cancel_order(txid).await
    }

    /// Cancel all open orders
    pub async fn cancel_all_orders(&self) -> RestResult<CancelOrderResult> {
        self.trading()?.cancel_all_orders().await
    }

    /// Edit an existing order
    pub async fn edit_order(
        &self,
        txid: &str,
        pair: &str,
        volume: Option<&str>,
        price: Option<&str>,
    ) -> RestResult<EditOrderResult> {
        self.trading()?
            .edit_order(txid, pair, volume, price, None, None, false)
            .await
    }

    // ========================================================================
    // Private Funding Endpoints
    // ========================================================================

    /// Get funding endpoints (requires credentials)
    pub fn funding(&self) -> RestResult<FundingEndpoints<'_>> {
        let creds = self.credentials.as_ref().ok_or(RestError::AuthRequired)?;
        Ok(FundingEndpoints::new(&self.http_client, creds))
    }

    /// Get deposit methods for an asset
    pub async fn get_deposit_methods(
        &self,
        asset: &str,
    ) -> RestResult<Vec<crate::types::DepositMethod>> {
        self.funding()?.get_deposit_methods(asset).await
    }

    // ========================================================================
    // Private Earn Endpoints (Staking)
    // ========================================================================

    /// Get earn endpoints (requires credentials)
    pub fn earn(&self) -> RestResult<EarnEndpoints<'_>> {
        let creds = self.credentials.as_ref().ok_or(RestError::AuthRequired)?;
        Ok(EarnEndpoints::new(&self.http_client, creds))
    }

    /// List available staking strategies
    pub async fn list_earn_strategies(
        &self,
        asset: Option<&str>,
    ) -> RestResult<crate::endpoints::earn::StrategiesResponse> {
        self.earn()?.list_strategies(asset, None).await
    }

    /// List current allocations
    pub async fn list_earn_allocations(
        &self,
    ) -> RestResult<crate::endpoints::earn::AllocationsResponse> {
        self.earn()?.list_allocations(None, None, None).await
    }

    /// Allocate funds to a staking strategy
    pub async fn allocate_earn(
        &self,
        strategy_id: &str,
        amount: rust_decimal::Decimal,
    ) -> RestResult<crate::endpoints::earn::AllocationResult> {
        self.earn()?.allocate(strategy_id, amount).await
    }

    /// Deallocate funds from a staking strategy
    pub async fn deallocate_earn(
        &self,
        strategy_id: &str,
        amount: rust_decimal::Decimal,
    ) -> RestResult<crate::endpoints::earn::DeallocationResult> {
        self.earn()?.deallocate(strategy_id, amount).await
    }
}

impl Default for KrakenRestClient {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for KrakenRestClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KrakenRestClient")
            .field("has_credentials", &self.has_credentials())
            .finish()
    }
}

/// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// API credentials (optional)
    pub credentials: Option<Credentials>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Custom user agent
    pub user_agent: Option<String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            credentials: None,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            user_agent: None,
        }
    }
}

impl ClientConfig {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set credentials
    pub fn with_credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_without_credentials() {
        let client = KrakenRestClient::new();
        assert!(!client.has_credentials());
    }

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfig::new()
            .with_timeout(60)
            .with_user_agent("test-agent");

        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.user_agent, Some("test-agent".to_string()));
    }

    #[test]
    fn test_auth_required_error() {
        let client = KrakenRestClient::new();
        let result = client.account();
        assert!(matches!(result, Err(RestError::AuthRequired)));
    }
}
