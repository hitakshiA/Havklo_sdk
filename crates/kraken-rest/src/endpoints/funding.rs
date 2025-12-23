//! Funding endpoints for deposits and withdrawals
//!
//! These endpoints require authentication.

use crate::auth::{Credentials, RequestSigner};
use crate::error::{RestError, RestResult};
use crate::types::{ApiResponse, DepositAddress, DepositMethod, WithdrawInfo};
use reqwest::Client;
use tracing::{debug, instrument};

const BASE_URL: &str = "https://api.kraken.com";

/// Funding endpoints for deposits and withdrawals
pub struct FundingEndpoints<'a> {
    client: &'a Client,
    credentials: &'a Credentials,
}

impl<'a> FundingEndpoints<'a> {
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

    /// Get deposit methods for an asset
    ///
    /// # Arguments
    /// * `asset` - Asset to get deposit methods for (e.g., "XBT", "ETH")
    #[instrument(skip(self))]
    pub async fn get_deposit_methods(&self, asset: &str) -> RestResult<Vec<DepositMethod>> {
        let params = [("asset", asset)];
        debug!("Getting deposit methods for {}", asset);
        self.post("/0/private/DepositMethods", &params).await
    }

    /// Get deposit addresses
    ///
    /// # Arguments
    /// * `asset` - Asset to get addresses for
    /// * `method` - Deposit method name
    /// * `new` - Generate new address
    #[instrument(skip(self))]
    pub async fn get_deposit_addresses(
        &self,
        asset: &str,
        method: &str,
        new: Option<bool>,
    ) -> RestResult<Vec<DepositAddress>> {
        let mut params: Vec<(&str, String)> = vec![
            ("asset", asset.to_string()),
            ("method", method.to_string()),
        ];

        if let Some(new) = new {
            params.push(("new", new.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!("Getting deposit addresses for {} via {}", asset, method);
        self.post("/0/private/DepositAddresses", &params_ref).await
    }

    /// Get status of recent deposits
    ///
    /// # Arguments
    /// * `asset` - Filter by asset (optional)
    /// * `method` - Filter by method (optional)
    #[instrument(skip(self))]
    pub async fn get_deposit_status(
        &self,
        asset: Option<&str>,
        method: Option<&str>,
    ) -> RestResult<Vec<DepositStatus>> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(asset) = asset {
            params.push(("asset", asset.to_string()));
        }
        if let Some(method) = method {
            params.push(("method", method.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!("Getting deposit status");
        self.post("/0/private/DepositStatus", &params_ref).await
    }

    /// Get withdrawal info
    ///
    /// # Arguments
    /// * `asset` - Asset to withdraw
    /// * `key` - Withdrawal key name (from account settings)
    /// * `amount` - Amount to withdraw
    #[instrument(skip(self))]
    pub async fn get_withdraw_info(
        &self,
        asset: &str,
        key: &str,
        amount: &str,
    ) -> RestResult<WithdrawInfo> {
        let params = [("asset", asset), ("key", key), ("amount", amount)];
        debug!("Getting withdrawal info for {} {}", amount, asset);
        self.post("/0/private/WithdrawInfo", &params).await
    }

    /// Withdraw funds
    ///
    /// # Arguments
    /// * `asset` - Asset to withdraw
    /// * `key` - Withdrawal key name
    /// * `amount` - Amount to withdraw
    #[instrument(skip(self))]
    pub async fn withdraw(
        &self,
        asset: &str,
        key: &str,
        amount: &str,
    ) -> RestResult<WithdrawResult> {
        let params = [("asset", asset), ("key", key), ("amount", amount)];
        debug!("Withdrawing {} {}", amount, asset);
        self.post("/0/private/Withdraw", &params).await
    }

    /// Get status of recent withdrawals
    ///
    /// # Arguments
    /// * `asset` - Filter by asset (optional)
    /// * `method` - Filter by method (optional)
    #[instrument(skip(self))]
    pub async fn get_withdraw_status(
        &self,
        asset: Option<&str>,
        method: Option<&str>,
    ) -> RestResult<Vec<WithdrawStatus>> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(asset) = asset {
            params.push(("asset", asset.to_string()));
        }
        if let Some(method) = method {
            params.push(("method", method.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!("Getting withdrawal status");
        self.post("/0/private/WithdrawStatus", &params_ref).await
    }

    /// Cancel a pending withdrawal
    ///
    /// # Arguments
    /// * `asset` - Asset of the withdrawal
    /// * `refid` - Withdrawal reference ID
    #[instrument(skip(self))]
    pub async fn cancel_withdraw(&self, asset: &str, refid: &str) -> RestResult<bool> {
        let params = [("asset", asset), ("refid", refid)];
        debug!("Cancelling withdrawal {}", refid);
        self.post("/0/private/WithdrawCancel", &params).await
    }

    /// Request wallet transfer
    ///
    /// # Arguments
    /// * `asset` - Asset to transfer
    /// * `from` - Source wallet
    /// * `to` - Destination wallet
    /// * `amount` - Amount to transfer
    #[instrument(skip(self))]
    pub async fn wallet_transfer(
        &self,
        asset: &str,
        from: &str,
        to: &str,
        amount: &str,
    ) -> RestResult<WalletTransferResult> {
        let params = [
            ("asset", asset),
            ("from", from),
            ("to", to),
            ("amount", amount),
        ];
        debug!("Transferring {} {} from {} to {}", amount, asset, from, to);
        self.post("/0/private/WalletTransfer", &params).await
    }
}

// Response types specific to funding endpoints

use serde::Deserialize;

/// Deposit status
#[derive(Debug, Clone, Deserialize)]
pub struct DepositStatus {
    /// Deposit method
    pub method: String,
    /// Asset class
    pub aclass: String,
    /// Asset
    pub asset: String,
    /// Reference ID
    pub refid: String,
    /// Transaction ID
    pub txid: String,
    /// Info
    pub info: String,
    /// Amount
    pub amount: String,
    /// Fee
    pub fee: Option<String>,
    /// Time
    pub time: f64,
    /// Status
    pub status: String,
    /// Status property
    pub status_prop: Option<String>,
}

/// Withdraw result
#[derive(Debug, Clone, Deserialize)]
pub struct WithdrawResult {
    /// Reference ID
    pub refid: String,
}

/// Withdrawal status
#[derive(Debug, Clone, Deserialize)]
pub struct WithdrawStatus {
    /// Withdrawal method
    pub method: String,
    /// Asset class
    pub aclass: String,
    /// Asset
    pub asset: String,
    /// Reference ID
    pub refid: String,
    /// Transaction ID
    pub txid: Option<String>,
    /// Info
    pub info: String,
    /// Amount
    pub amount: String,
    /// Fee
    pub fee: String,
    /// Time
    pub time: f64,
    /// Status
    pub status: String,
    /// Status property
    pub status_prop: Option<String>,
}

/// Wallet transfer result
#[derive(Debug, Clone, Deserialize)]
pub struct WalletTransferResult {
    /// Reference ID
    pub refid: String,
}
