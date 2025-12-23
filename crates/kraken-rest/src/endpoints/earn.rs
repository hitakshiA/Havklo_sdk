//! Earn endpoints for staking and yield generation
//!
//! These endpoints require authentication and allow users to:
//! - View available staking strategies
//! - Allocate/deallocate funds to earn yield
//! - Track pending allocations and deallocations

use crate::auth::{Credentials, RequestSigner};
use crate::error::{RestError, RestResult};
use crate::types::ApiResponse;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

const BASE_URL: &str = "https://api.kraken.com";

/// Earn endpoints for staking operations
pub struct EarnEndpoints<'a> {
    client: &'a Client,
    credentials: &'a Credentials,
}

impl<'a> EarnEndpoints<'a> {
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

    /// List available earn strategies
    ///
    /// Returns all available staking/earn strategies with their parameters.
    ///
    /// # Arguments
    /// * `asset` - Filter by asset (optional, e.g., "ETH", "DOT")
    /// * `lock_type` - Filter by lock type (optional): "flex", "bonded", "timed", "instant"
    #[instrument(skip(self))]
    pub async fn list_strategies(
        &self,
        asset: Option<&str>,
        lock_type: Option<LockType>,
    ) -> RestResult<StrategiesResponse> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(asset) = asset {
            params.push(("asset", asset.to_string()));
        }
        if let Some(lock_type) = lock_type {
            params.push(("lock_type", lock_type.as_str().to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!("Listing earn strategies");
        self.post("/0/private/Earn/Strategies", &params_ref).await
    }

    /// List current allocations
    ///
    /// Returns all current staking allocations for the account.
    ///
    /// # Arguments
    /// * `ascending` - Sort order (optional, default false)
    /// * `converted_asset` - Convert amounts to this asset (optional)
    /// * `hide_zero_allocations` - Hide strategies with zero allocation (optional)
    #[instrument(skip(self))]
    pub async fn list_allocations(
        &self,
        ascending: Option<bool>,
        converted_asset: Option<&str>,
        hide_zero_allocations: Option<bool>,
    ) -> RestResult<AllocationsResponse> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(ascending) = ascending {
            params.push(("ascending", ascending.to_string()));
        }
        if let Some(asset) = converted_asset {
            params.push(("converted_asset", asset.to_string()));
        }
        if let Some(hide_zero) = hide_zero_allocations {
            params.push(("hide_zero_allocations", hide_zero.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!("Listing allocations");
        self.post("/0/private/Earn/Allocations", &params_ref).await
    }

    /// Allocate funds to an earn strategy
    ///
    /// Stakes funds in a specific earning strategy.
    ///
    /// # Arguments
    /// * `strategy_id` - The strategy ID to allocate to
    /// * `amount` - Amount to allocate
    #[instrument(skip(self))]
    pub async fn allocate(
        &self,
        strategy_id: &str,
        amount: Decimal,
    ) -> RestResult<AllocationResult> {
        let amount_str = amount.to_string();
        let params = [
            ("strategy_id", strategy_id),
            ("amount", &amount_str),
        ];

        debug!("Allocating {} to strategy {}", amount, strategy_id);
        self.post("/0/private/Earn/Allocate", &params).await
    }

    /// Deallocate funds from an earn strategy
    ///
    /// Unstakes funds from a specific earning strategy.
    ///
    /// # Arguments
    /// * `strategy_id` - The strategy ID to deallocate from
    /// * `amount` - Amount to deallocate
    #[instrument(skip(self))]
    pub async fn deallocate(
        &self,
        strategy_id: &str,
        amount: Decimal,
    ) -> RestResult<DeallocationResult> {
        let amount_str = amount.to_string();
        let params = [
            ("strategy_id", strategy_id),
            ("amount", &amount_str),
        ];

        debug!("Deallocating {} from strategy {}", amount, strategy_id);
        self.post("/0/private/Earn/Deallocate", &params).await
    }

    /// Get allocation status
    ///
    /// Check the status of pending allocations.
    ///
    /// # Arguments
    /// * `strategy_id` - Filter by strategy ID (optional)
    #[instrument(skip(self))]
    pub async fn get_allocate_status(
        &self,
        strategy_id: Option<&str>,
    ) -> RestResult<AllocationStatusResponse> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(strategy_id) = strategy_id {
            params.push(("strategy_id", strategy_id.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!("Getting allocation status");
        self.post("/0/private/Earn/AllocateStatus", &params_ref).await
    }

    /// Get deallocation status
    ///
    /// Check the status of pending deallocations.
    ///
    /// # Arguments
    /// * `strategy_id` - Filter by strategy ID (optional)
    #[instrument(skip(self))]
    pub async fn get_deallocate_status(
        &self,
        strategy_id: Option<&str>,
    ) -> RestResult<DeallocationStatusResponse> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(strategy_id) = strategy_id {
            params.push(("strategy_id", strategy_id.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        debug!("Getting deallocation status");
        self.post("/0/private/Earn/DeallocateStatus", &params_ref).await
    }
}

// Response types for Earn endpoints

/// Lock type for staking strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LockType {
    /// Flexible staking - can unstake anytime
    Flex,
    /// Bonded staking - locked for a period
    Bonded,
    /// Timed staking - fixed duration
    Timed,
    /// Instant unstaking available
    Instant,
}

impl LockType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LockType::Flex => "flex",
            LockType::Bonded => "bonded",
            LockType::Timed => "timed",
            LockType::Instant => "instant",
        }
    }
}

/// Auto-compound setting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutoCompound {
    /// Enabled
    Enabled,
    /// Disabled
    Disabled,
    /// Optional (user can choose)
    Optional,
}

/// APR estimate type
#[derive(Debug, Clone, Deserialize)]
pub struct AprEstimate {
    /// Low estimate (percentage)
    pub low: String,
    /// High estimate (percentage)
    pub high: String,
}

/// Yield source
#[derive(Debug, Clone, Deserialize)]
pub struct YieldSource {
    /// Source type (e.g., "staking_rewards", "opt_in_rewards")
    #[serde(rename = "type")]
    pub source_type: String,
    /// Description
    pub description: Option<String>,
}

/// Earn strategy
#[derive(Debug, Clone, Deserialize)]
pub struct EarnStrategy {
    /// Unique strategy ID
    pub id: String,
    /// Strategy allocation fee (percentage)
    pub allocation_fee: String,
    /// Allocation restriction info
    pub allocation_restriction_info: Vec<String>,
    /// APR estimate
    pub apr_estimate: Option<AprEstimate>,
    /// Asset to stake
    pub asset: String,
    /// Auto-compound setting
    pub auto_compound: AutoCompound,
    /// Whether new allocations can be made
    pub can_allocate: bool,
    /// Whether deallocations are allowed
    pub can_deallocate: bool,
    /// Deallocation fee (percentage)
    pub deallocation_fee: String,
    /// Lock type
    pub lock_type: LockType,
    /// User limit for this strategy
    pub user_cap: Option<String>,
    /// Minimum amount
    pub user_min_allocation: Option<String>,
    /// Yield sources
    pub yield_source: Option<YieldSource>,
}

/// Strategies response
#[derive(Debug, Clone, Deserialize)]
pub struct StrategiesResponse {
    /// List of available strategies
    pub items: Vec<EarnStrategy>,
    /// Next page cursor
    pub next_cursor: Option<String>,
}

/// Allocation entry
#[derive(Debug, Clone, Deserialize)]
pub struct Allocation {
    /// Strategy ID
    pub strategy_id: String,
    /// Native asset
    pub native_asset: String,
    /// Allocated amount (native)
    pub amount_allocated: AmountInfo,
    /// Total rewarded
    pub total_rewarded: AmountInfo,
    /// Payout info
    pub payout: Option<PayoutInfo>,
}

/// Amount information
#[derive(Debug, Clone, Deserialize)]
pub struct AmountInfo {
    /// Bonding allocations
    #[serde(default)]
    pub bonding: String,
    /// Exit queue allocations
    #[serde(default)]
    pub exit_queue: String,
    /// Pending allocation
    #[serde(default)]
    pub pending: String,
    /// Total amount
    #[serde(default)]
    pub total: String,
    /// Unbonding amount
    #[serde(default)]
    pub unbonding: String,
}

/// Payout information
#[derive(Debug, Clone, Deserialize)]
pub struct PayoutInfo {
    /// Accumulated rewards
    pub accumulated_reward: Option<AccumulatedReward>,
    /// Estimated reward
    pub estimated_reward: Option<EstimatedReward>,
    /// Period end
    pub period_end: Option<String>,
    /// Period start
    pub period_start: Option<String>,
}

/// Accumulated reward
#[derive(Debug, Clone, Deserialize)]
pub struct AccumulatedReward {
    /// Allocated amount at start of period
    pub allocated_at_start: String,
    /// Reward amount
    pub reward: String,
}

/// Estimated reward
#[derive(Debug, Clone, Deserialize)]
pub struct EstimatedReward {
    /// Estimated amount
    pub amount: String,
    /// APR used for estimate
    pub apr: String,
}

/// Allocations response
#[derive(Debug, Clone, Deserialize)]
pub struct AllocationsResponse {
    /// Converted asset (if requested)
    pub converted_asset: Option<String>,
    /// List of allocations
    pub items: Vec<Allocation>,
    /// Total allocated (in converted asset)
    pub total_allocated: Option<String>,
    /// Total rewarded (in converted asset)
    pub total_rewarded: Option<String>,
}

/// Allocation result
#[derive(Debug, Clone, Deserialize)]
pub struct AllocationResult {
    /// Whether allocation was successful
    pub pending: bool,
}

/// Deallocation result
#[derive(Debug, Clone, Deserialize)]
pub struct DeallocationResult {
    /// Whether deallocation was successful
    pub pending: bool,
}

/// Pending allocation status
#[derive(Debug, Clone, Deserialize)]
pub struct PendingAllocation {
    /// Strategy ID
    pub strategy_id: String,
    /// Amount
    pub amount: String,
    /// Created timestamp
    pub created_at: String,
    /// Status
    pub status: String,
}

/// Allocation status response
#[derive(Debug, Clone, Deserialize)]
pub struct AllocationStatusResponse {
    /// Pending allocations
    pub pending: bool,
    /// Pending allocations details
    #[serde(default)]
    pub items: Vec<PendingAllocation>,
}

/// Pending deallocation status
#[derive(Debug, Clone, Deserialize)]
pub struct PendingDeallocation {
    /// Strategy ID
    pub strategy_id: String,
    /// Amount
    pub amount: String,
    /// Created timestamp
    pub created_at: String,
    /// Status
    pub status: String,
}

/// Deallocation status response
#[derive(Debug, Clone, Deserialize)]
pub struct DeallocationStatusResponse {
    /// Pending deallocations
    pub pending: bool,
    /// Pending deallocations details
    #[serde(default)]
    pub items: Vec<PendingDeallocation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_type_as_str() {
        assert_eq!(LockType::Flex.as_str(), "flex");
        assert_eq!(LockType::Bonded.as_str(), "bonded");
        assert_eq!(LockType::Timed.as_str(), "timed");
        assert_eq!(LockType::Instant.as_str(), "instant");
    }

    #[test]
    fn test_deserialize_strategy() {
        let json = r#"{
            "id": "SETH2-EARN",
            "allocation_fee": "0",
            "allocation_restriction_info": [],
            "apr_estimate": {"low": "3.0", "high": "5.0"},
            "asset": "ETH",
            "auto_compound": "enabled",
            "can_allocate": true,
            "can_deallocate": true,
            "deallocation_fee": "0",
            "lock_type": "bonded",
            "user_cap": null,
            "user_min_allocation": "0.0001"
        }"#;

        let strategy: EarnStrategy = serde_json::from_str(json).unwrap();
        assert_eq!(strategy.id, "SETH2-EARN");
        assert_eq!(strategy.asset, "ETH");
        assert!(strategy.can_allocate);
        assert_eq!(strategy.lock_type, LockType::Bonded);
    }

    #[test]
    fn test_deserialize_allocation() {
        let json = r#"{
            "strategy_id": "SETH2-EARN",
            "native_asset": "ETH",
            "amount_allocated": {
                "bonding": "0",
                "exit_queue": "0",
                "pending": "0",
                "total": "1.5",
                "unbonding": "0"
            },
            "total_rewarded": {
                "bonding": "0",
                "exit_queue": "0",
                "pending": "0",
                "total": "0.05",
                "unbonding": "0"
            }
        }"#;

        let allocation: Allocation = serde_json::from_str(json).unwrap();
        assert_eq!(allocation.strategy_id, "SETH2-EARN");
        assert_eq!(allocation.amount_allocated.total, "1.5");
    }
}
