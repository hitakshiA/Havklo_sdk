//! Comprehensive Kraken API error code mapping with recovery strategies
//!
//! This module provides structured error handling for all known Kraken API
//! error codes across REST, WebSocket, and trading operations.

use std::time::Duration;

/// Recovery strategy for handling API errors
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    /// Exponential backoff before retry
    Backoff {
        initial_ms: u64,
        max_ms: u64,
        multiplier: u32,  // Multiplier as integer (e.g., 2 = 2x)
    },
    /// Fixed delay retry
    Retry { delay_ms: u64, max_attempts: u32 },
    /// Request a new snapshot (for orderbook desync)
    RequestSnapshot,
    /// Re-authenticate (token expired or invalid)
    Reauthenticate,
    /// Cannot recover programmatically - fatal error
    Fatal,
    /// Requires user intervention (e.g., add funds)
    UserAction { message: &'static str },
    /// Skip this message and continue
    Skip,
    /// Manual investigation needed
    Manual,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        Self::Manual
    }
}

impl RecoveryStrategy {
    /// Default exponential backoff for rate limits
    pub fn rate_limit_backoff() -> Self {
        Self::Backoff {
            initial_ms: 1000,
            max_ms: 60000,
            multiplier: 2,
        }
    }

    /// Default retry for transient service errors
    pub fn service_retry() -> Self {
        Self::Retry {
            delay_ms: 5000,
            max_attempts: 3,
        }
    }

    /// Get the initial delay duration
    pub fn initial_delay(&self) -> Option<Duration> {
        match self {
            Self::Backoff { initial_ms, .. } => Some(Duration::from_millis(*initial_ms)),
            Self::Retry { delay_ms, .. } => Some(Duration::from_millis(*delay_ms)),
            _ => None,
        }
    }

    /// Check if this strategy allows retry
    pub fn allows_retry(&self) -> bool {
        matches!(
            self,
            Self::Backoff { .. }
                | Self::Retry { .. }
                | Self::RequestSnapshot
                | Self::Reauthenticate
        )
    }
}

/// Kraken API error categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// EAPI:* errors - API-level issues
    Api,
    /// EGeneral:* errors - General errors
    General,
    /// EService:* errors - Service availability
    Service,
    /// EOrder:* errors - Trading/order errors
    Order,
    /// EFunding:* errors - Deposit/withdrawal errors
    Funding,
    /// EQuery:* errors - Query/search errors
    Query,
    /// ETrade:* errors - Trade execution errors
    Trade,
    /// Unknown error category
    Unknown,
}

/// Parsed Kraken API error with metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrakenApiError {
    /// The original error string from Kraken
    pub raw: String,
    /// Parsed error code (if recognized)
    pub code: Option<KrakenErrorCode>,
    /// Error category
    pub category: ErrorCategory,
    /// Human-readable message
    pub message: String,
}

impl KrakenApiError {
    /// Parse a Kraken error string into a structured error
    pub fn parse(error: &str) -> Self {
        let (category, code, message) = Self::parse_error_string(error);

        Self {
            raw: error.to_string(),
            code,
            category,
            message,
        }
    }

    /// Parse multiple Kraken errors (API returns array of errors)
    pub fn parse_many(errors: &[String]) -> Vec<Self> {
        errors.iter().map(|e| Self::parse(e)).collect()
    }

    fn parse_error_string(error: &str) -> (ErrorCategory, Option<KrakenErrorCode>, String) {
        // Kraken errors are formatted as "ECATEGORY:Message"
        if let Some(colon_pos) = error.find(':') {
            let prefix = &error[..colon_pos];
            let message = error[colon_pos + 1..].trim().to_string();

            let category = match prefix {
                "EAPI" => ErrorCategory::Api,
                "EGeneral" => ErrorCategory::General,
                "EService" => ErrorCategory::Service,
                "EOrder" => ErrorCategory::Order,
                "EFunding" => ErrorCategory::Funding,
                "EQuery" => ErrorCategory::Query,
                "ETrade" => ErrorCategory::Trade,
                _ => ErrorCategory::Unknown,
            };

            let code = KrakenErrorCode::from_str(error);

            (category, code, message)
        } else {
            (
                ErrorCategory::Unknown,
                KrakenErrorCode::from_str(error),
                error.to_string(),
            )
        }
    }

    /// Get the recovery strategy for this error
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        self.code
            .map(|c| c.recovery_strategy())
            .unwrap_or(RecoveryStrategy::Manual)
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        self.recovery_strategy().allows_retry()
    }

    /// Check if this error requires re-authentication
    pub fn requires_reauth(&self) -> bool {
        matches!(self.recovery_strategy(), RecoveryStrategy::Reauthenticate)
    }

    /// Check if this is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        matches!(
            self.code,
            Some(KrakenErrorCode::RateLimitExceeded)
                | Some(KrakenErrorCode::TooManyRequests)
                | Some(KrakenErrorCode::OrderRateLimitExceeded)
        )
    }

    /// Check if this is a fatal error that cannot be recovered
    pub fn is_fatal(&self) -> bool {
        matches!(self.recovery_strategy(), RecoveryStrategy::Fatal)
    }
}

/// All known Kraken API error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KrakenErrorCode {
    // === EAPI (API-level) Errors ===
    /// EAPI:Rate limit exceeded
    RateLimitExceeded,
    /// EAPI:Invalid key
    InvalidKey,
    /// EAPI:Invalid signature
    InvalidSignature,
    /// EAPI:Invalid nonce
    InvalidNonce,
    /// EAPI:Bad request
    BadRequest,
    /// EAPI:Invalid session
    InvalidSession,
    /// EAPI:Feature disabled
    FeatureDisabled,

    // === EGeneral (General) Errors ===
    /// EGeneral:Invalid arguments
    InvalidArguments,
    /// EGeneral:Invalid arguments:Index unavailable
    IndexUnavailable,
    /// EGeneral:Permission denied
    PermissionDenied,
    /// EGeneral:Unknown asset pair
    UnknownAssetPair,
    /// EGeneral:Unknown asset
    UnknownAsset,
    /// EGeneral:Too many requests
    TooManyRequests,
    /// EGeneral:Temporary lockout
    TemporaryLockout,
    /// EGeneral:Unknown method
    UnknownMethod,
    /// EGeneral:Internal error
    InternalError,

    // === EService (Service) Errors ===
    /// EService:Unavailable
    ServiceUnavailable,
    /// EService:Busy
    ServiceBusy,
    /// EService:Market in cancel_only mode
    MarketCancelOnly,
    /// EService:Market in post_only mode
    MarketPostOnly,
    /// EService:Deadline elapsed
    DeadlineElapsed,
    /// EService:Timeout
    ServiceTimeout,

    // === EOrder (Order) Errors ===
    /// EOrder:Cannot open position
    CannotOpenPosition,
    /// EOrder:Cannot open opposing position
    CannotOpenOpposingPosition,
    /// EOrder:Margin allowance exceeded
    MarginAllowanceExceeded,
    /// EOrder:Insufficient margin
    InsufficientMargin,
    /// EOrder:Insufficient funds
    InsufficientFunds,
    /// EOrder:Order minimum not met
    OrderMinimumNotMet,
    /// EOrder:Cost minimum not met
    CostMinimumNotMet,
    /// EOrder:Tick size check failed
    TickSizeCheckFailed,
    /// EOrder:Orders limit exceeded
    OrdersLimitExceeded,
    /// EOrder:Rate limit exceeded
    OrderRateLimitExceeded,
    /// EOrder:Domain rate limit exceeded
    DomainRateLimitExceeded,
    /// EOrder:Positions limit exceeded
    PositionsLimitExceeded,
    /// EOrder:Position size exceeded
    PositionSizeExceeded,
    /// EOrder:Unknown order
    UnknownOrder,
    /// EOrder:Unknown position
    UnknownPosition,
    /// EOrder:Invalid price
    InvalidPrice,
    /// EOrder:Invalid volume
    InvalidVolume,
    /// EOrder:Invalid order type
    InvalidOrderType,
    /// EOrder:Market is closed
    MarketClosed,
    /// EOrder:Trading disabled
    TradingDisabled,
    /// EOrder:Scheduled orders disabled
    ScheduledOrdersDisabled,
    /// EOrder:Post only order
    PostOnlyOrder,

    // === EFunding (Funding) Errors ===
    /// EFunding:Unknown withdraw key
    UnknownWithdrawKey,
    /// EFunding:Invalid amount
    InvalidFundingAmount,
    /// EFunding:Unknown asset
    UnknownFundingAsset,
    /// EFunding:Too small
    FundingTooSmall,
    /// EFunding:Too large
    FundingTooLarge,

    // === EQuery (Query) Errors ===
    /// EQuery:Unknown asset pair
    QueryUnknownAssetPair,

    // === ETrade (Trade) Errors ===
    /// ETrade:Unknown position
    TradeUnknownPosition,

    // === WebSocket Specific ===
    /// Invalid subscription
    InvalidSubscription,
    /// Subscription limit exceeded
    SubscriptionLimitExceeded,
}

impl KrakenErrorCode {
    /// Parse error code from Kraken error string
    pub fn from_str(error: &str) -> Option<Self> {
        // Normalize for matching
        let normalized = error.to_lowercase();

        // Match against known patterns
        Some(match error {
            // EAPI errors
            "EAPI:Rate limit exceeded" => Self::RateLimitExceeded,
            "EAPI:Invalid key" => Self::InvalidKey,
            "EAPI:Invalid signature" => Self::InvalidSignature,
            "EAPI:Invalid nonce" => Self::InvalidNonce,
            "EAPI:Bad request" => Self::BadRequest,
            "EAPI:Invalid session" => Self::InvalidSession,
            "EAPI:Feature disabled" => Self::FeatureDisabled,

            // EGeneral errors
            "EGeneral:Invalid arguments" => Self::InvalidArguments,
            "EGeneral:Permission denied" => Self::PermissionDenied,
            "EGeneral:Unknown asset pair" => Self::UnknownAssetPair,
            "EGeneral:Unknown asset" => Self::UnknownAsset,
            "EGeneral:Too many requests" => Self::TooManyRequests,
            "EGeneral:Temporary lockout" => Self::TemporaryLockout,
            "EGeneral:Unknown method" => Self::UnknownMethod,
            "EGeneral:Internal error" => Self::InternalError,

            // EService errors
            "EService:Unavailable" => Self::ServiceUnavailable,
            "EService:Busy" => Self::ServiceBusy,
            "EService:Market in cancel_only mode" => Self::MarketCancelOnly,
            "EService:Market in post_only mode" => Self::MarketPostOnly,
            "EService:Deadline elapsed" => Self::DeadlineElapsed,
            "EService:Timeout" => Self::ServiceTimeout,

            // EOrder errors
            "EOrder:Cannot open position" => Self::CannotOpenPosition,
            "EOrder:Cannot open opposing position" => Self::CannotOpenOpposingPosition,
            "EOrder:Margin allowance exceeded" => Self::MarginAllowanceExceeded,
            "EOrder:Insufficient margin" => Self::InsufficientMargin,
            "EOrder:Insufficient funds" => Self::InsufficientFunds,
            "EOrder:Order minimum not met" => Self::OrderMinimumNotMet,
            "EOrder:Cost minimum not met" => Self::CostMinimumNotMet,
            "EOrder:Tick size check failed" => Self::TickSizeCheckFailed,
            "EOrder:Orders limit exceeded" => Self::OrdersLimitExceeded,
            "EOrder:Rate limit exceeded" => Self::OrderRateLimitExceeded,
            "EOrder:Domain rate limit exceeded" => Self::DomainRateLimitExceeded,
            "EOrder:Positions limit exceeded" => Self::PositionsLimitExceeded,
            "EOrder:Position size exceeded" => Self::PositionSizeExceeded,
            "EOrder:Unknown order" => Self::UnknownOrder,
            "EOrder:Unknown position" => Self::UnknownPosition,
            "EOrder:Invalid price" => Self::InvalidPrice,
            "EOrder:Invalid volume" => Self::InvalidVolume,
            "EOrder:Invalid order type" => Self::InvalidOrderType,
            "EOrder:Market is closed" => Self::MarketClosed,
            "EOrder:Trading disabled" => Self::TradingDisabled,
            "EOrder:Scheduled orders disabled" => Self::ScheduledOrdersDisabled,
            "EOrder:Post only order" => Self::PostOnlyOrder,

            // EFunding errors
            "EFunding:Unknown withdraw key" => Self::UnknownWithdrawKey,
            "EFunding:Invalid amount" => Self::InvalidFundingAmount,
            "EFunding:Unknown asset" => Self::UnknownFundingAsset,
            "EFunding:Too small" => Self::FundingTooSmall,
            "EFunding:Too large" => Self::FundingTooLarge,

            // EQuery errors
            "EQuery:Unknown asset pair" => Self::QueryUnknownAssetPair,

            // ETrade errors
            "ETrade:Unknown position" => Self::TradeUnknownPosition,

            // Partial matches for variations
            _ => {
                // Handle partial matches and variations
                if normalized.contains("rate limit") {
                    Self::RateLimitExceeded
                } else if normalized.contains("invalid key") {
                    Self::InvalidKey
                } else if normalized.contains("invalid signature") {
                    Self::InvalidSignature
                } else if normalized.contains("invalid nonce") {
                    Self::InvalidNonce
                } else if normalized.contains("insufficient funds") {
                    Self::InsufficientFunds
                } else if normalized.contains("insufficient margin") {
                    Self::InsufficientMargin
                } else if normalized.contains("unknown asset pair")
                    || normalized.contains("unknown pair")
                {
                    Self::UnknownAssetPair
                } else if normalized.contains("permission denied") {
                    Self::PermissionDenied
                } else if normalized.contains("unavailable") {
                    Self::ServiceUnavailable
                } else if normalized.contains("timeout") {
                    Self::ServiceTimeout
                } else if normalized.contains("market") && normalized.contains("closed") {
                    Self::MarketClosed
                } else if normalized.contains("cancel_only") || normalized.contains("cancel only") {
                    Self::MarketCancelOnly
                } else if normalized.contains("post_only") || normalized.contains("post only") {
                    Self::MarketPostOnly
                } else if normalized.contains("trading disabled") {
                    Self::TradingDisabled
                } else if normalized.contains("lockout") {
                    Self::TemporaryLockout
                } else {
                    return None;
                }
            }
        })
    }

    /// Get the recovery strategy for this error code
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            // Rate limiting - backoff
            Self::RateLimitExceeded | Self::TooManyRequests | Self::OrderRateLimitExceeded => {
                RecoveryStrategy::rate_limit_backoff()
            }
            Self::DomainRateLimitExceeded => RecoveryStrategy::Backoff {
                initial_ms: 5000,
                max_ms: 120000,
                multiplier: 2,
            },

            // Authentication - re-auth
            Self::InvalidKey | Self::InvalidSignature | Self::InvalidNonce => {
                RecoveryStrategy::Reauthenticate
            }
            Self::InvalidSession => RecoveryStrategy::Reauthenticate,

            // Service issues - retry
            Self::ServiceUnavailable | Self::ServiceBusy | Self::ServiceTimeout => {
                RecoveryStrategy::service_retry()
            }
            Self::DeadlineElapsed => RecoveryStrategy::Retry {
                delay_ms: 1000,
                max_attempts: 5,
            },

            // Trading issues - user action
            Self::InsufficientFunds => RecoveryStrategy::UserAction {
                message: "Insufficient funds - deposit more or reduce order size",
            },
            Self::InsufficientMargin => RecoveryStrategy::UserAction {
                message: "Insufficient margin - add collateral or reduce position",
            },
            Self::OrderMinimumNotMet | Self::CostMinimumNotMet => RecoveryStrategy::UserAction {
                message: "Order size too small - increase quantity",
            },
            Self::OrdersLimitExceeded | Self::PositionsLimitExceeded => RecoveryStrategy::UserAction {
                message: "Too many open orders/positions - close some first",
            },
            Self::PositionSizeExceeded | Self::MarginAllowanceExceeded => {
                RecoveryStrategy::UserAction {
                    message: "Position too large - reduce size",
                }
            }

            // Market state - wait or skip
            Self::MarketClosed => RecoveryStrategy::UserAction {
                message: "Market is closed - wait for market hours",
            },
            Self::MarketCancelOnly => RecoveryStrategy::UserAction {
                message: "Market in cancel-only mode - can only cancel orders",
            },
            Self::MarketPostOnly => RecoveryStrategy::UserAction {
                message: "Market in post-only mode - use limit orders with post-only flag",
            },
            Self::TradingDisabled | Self::ScheduledOrdersDisabled => RecoveryStrategy::UserAction {
                message: "Trading is temporarily disabled",
            },

            // Validation errors - skip/manual
            Self::InvalidArguments | Self::BadRequest | Self::InvalidOrderType => {
                RecoveryStrategy::Skip
            }
            Self::InvalidPrice | Self::InvalidVolume | Self::TickSizeCheckFailed => {
                RecoveryStrategy::Skip
            }
            Self::UnknownAssetPair | Self::UnknownAsset | Self::QueryUnknownAssetPair => {
                RecoveryStrategy::Skip
            }
            Self::UnknownOrder | Self::UnknownPosition | Self::TradeUnknownPosition => {
                RecoveryStrategy::Skip
            }
            Self::PostOnlyOrder => RecoveryStrategy::Skip,

            // Permission/Config - fatal
            Self::PermissionDenied | Self::FeatureDisabled => RecoveryStrategy::Fatal,

            // Lockout - backoff
            Self::TemporaryLockout => RecoveryStrategy::Backoff {
                initial_ms: 60000,
                max_ms: 600000,
                multiplier: 2,
            },

            // Position errors - user action
            Self::CannotOpenPosition | Self::CannotOpenOpposingPosition => {
                RecoveryStrategy::UserAction {
                    message: "Cannot open this position - check existing positions",
                }
            }

            // Funding errors
            Self::UnknownWithdrawKey | Self::UnknownFundingAsset => RecoveryStrategy::UserAction {
                message: "Unknown withdraw key or asset - verify withdrawal settings",
            },
            Self::InvalidFundingAmount | Self::FundingTooSmall | Self::FundingTooLarge => {
                RecoveryStrategy::UserAction {
                    message: "Invalid funding amount - adjust to within limits",
                }
            }

            // WebSocket specific
            Self::InvalidSubscription => RecoveryStrategy::Skip,
            Self::SubscriptionLimitExceeded => RecoveryStrategy::UserAction {
                message: "Too many subscriptions - unsubscribe from some channels",
            },

            // Unknown/fallback
            Self::InternalError | Self::UnknownMethod | Self::IndexUnavailable => {
                RecoveryStrategy::Manual
            }
        }
    }

    /// Get a human-readable description of this error
    pub fn description(&self) -> &'static str {
        match self {
            Self::RateLimitExceeded => "API rate limit exceeded",
            Self::InvalidKey => "Invalid API key",
            Self::InvalidSignature => "Invalid request signature",
            Self::InvalidNonce => "Invalid nonce value",
            Self::BadRequest => "Malformed request",
            Self::InvalidSession => "Session has expired or is invalid",
            Self::FeatureDisabled => "This feature is not available for your account",
            Self::InvalidArguments => "Invalid arguments provided",
            Self::IndexUnavailable => "Requested index is not available",
            Self::PermissionDenied => "Permission denied for this operation",
            Self::UnknownAssetPair => "Trading pair not found",
            Self::UnknownAsset => "Asset not found",
            Self::TooManyRequests => "Too many requests",
            Self::TemporaryLockout => "Account temporarily locked",
            Self::UnknownMethod => "Unknown API method",
            Self::InternalError => "Internal server error",
            Self::ServiceUnavailable => "Service temporarily unavailable",
            Self::ServiceBusy => "Service is busy, try again",
            Self::MarketCancelOnly => "Market only accepts cancellations",
            Self::MarketPostOnly => "Market only accepts post-only orders",
            Self::DeadlineElapsed => "Request deadline exceeded",
            Self::ServiceTimeout => "Service request timed out",
            Self::CannotOpenPosition => "Cannot open position",
            Self::CannotOpenOpposingPosition => "Cannot open opposing position",
            Self::MarginAllowanceExceeded => "Margin allowance exceeded",
            Self::InsufficientMargin => "Insufficient margin",
            Self::InsufficientFunds => "Insufficient funds",
            Self::OrderMinimumNotMet => "Order minimum not met",
            Self::CostMinimumNotMet => "Cost minimum not met",
            Self::TickSizeCheckFailed => "Price tick size check failed",
            Self::OrdersLimitExceeded => "Maximum open orders exceeded",
            Self::OrderRateLimitExceeded => "Order rate limit exceeded",
            Self::DomainRateLimitExceeded => "Domain rate limit exceeded",
            Self::PositionsLimitExceeded => "Maximum positions exceeded",
            Self::PositionSizeExceeded => "Maximum position size exceeded",
            Self::UnknownOrder => "Order not found",
            Self::UnknownPosition => "Position not found",
            Self::InvalidPrice => "Invalid price",
            Self::InvalidVolume => "Invalid volume",
            Self::InvalidOrderType => "Invalid order type",
            Self::MarketClosed => "Market is closed",
            Self::TradingDisabled => "Trading is disabled",
            Self::ScheduledOrdersDisabled => "Scheduled orders are disabled",
            Self::PostOnlyOrder => "Post-only order would have executed immediately",
            Self::UnknownWithdrawKey => "Withdrawal key not found",
            Self::InvalidFundingAmount => "Invalid funding amount",
            Self::UnknownFundingAsset => "Funding asset not found",
            Self::FundingTooSmall => "Funding amount too small",
            Self::FundingTooLarge => "Funding amount too large",
            Self::QueryUnknownAssetPair => "Queried asset pair not found",
            Self::TradeUnknownPosition => "Trade position not found",
            Self::InvalidSubscription => "Invalid subscription request",
            Self::SubscriptionLimitExceeded => "Maximum subscriptions exceeded",
        }
    }

    /// Check if this is an authentication-related error
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidKey
                | Self::InvalidSignature
                | Self::InvalidNonce
                | Self::InvalidSession
                | Self::PermissionDenied
        )
    }

    /// Check if this is a rate limit error
    pub fn is_rate_limit(&self) -> bool {
        matches!(
            self,
            Self::RateLimitExceeded
                | Self::TooManyRequests
                | Self::OrderRateLimitExceeded
                | Self::DomainRateLimitExceeded
        )
    }

    /// Check if this is a trading-related error
    pub fn is_trading_error(&self) -> bool {
        matches!(
            self,
            Self::InsufficientFunds
                | Self::InsufficientMargin
                | Self::OrderMinimumNotMet
                | Self::CostMinimumNotMet
                | Self::TickSizeCheckFailed
                | Self::OrdersLimitExceeded
                | Self::PositionsLimitExceeded
                | Self::PositionSizeExceeded
                | Self::InvalidPrice
                | Self::InvalidVolume
                | Self::MarketClosed
                | Self::TradingDisabled
                | Self::MarketCancelOnly
                | Self::MarketPostOnly
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rate_limit_error() {
        let error = KrakenApiError::parse("EAPI:Rate limit exceeded");
        assert_eq!(error.code, Some(KrakenErrorCode::RateLimitExceeded));
        assert_eq!(error.category, ErrorCategory::Api);
        assert!(error.is_rate_limit());
        assert!(error.is_retryable());
    }

    #[test]
    fn test_parse_auth_error() {
        let error = KrakenApiError::parse("EAPI:Invalid key");
        assert_eq!(error.code, Some(KrakenErrorCode::InvalidKey));
        assert!(error.requires_reauth());
    }

    #[test]
    fn test_parse_order_error() {
        let error = KrakenApiError::parse("EOrder:Insufficient funds");
        assert_eq!(error.code, Some(KrakenErrorCode::InsufficientFunds));
        assert_eq!(error.category, ErrorCategory::Order);
        assert!(!error.is_retryable());
        assert!(matches!(
            error.recovery_strategy(),
            RecoveryStrategy::UserAction { .. }
        ));
    }

    #[test]
    fn test_parse_service_error() {
        let error = KrakenApiError::parse("EService:Unavailable");
        assert_eq!(error.code, Some(KrakenErrorCode::ServiceUnavailable));
        assert!(error.is_retryable());
    }

    #[test]
    fn test_parse_unknown_error() {
        let error = KrakenApiError::parse("ESomething:Unknown error format");
        assert_eq!(error.code, None);
        assert_eq!(error.category, ErrorCategory::Unknown);
    }

    #[test]
    fn test_partial_match() {
        let error = KrakenApiError::parse("Some rate limit error occurred");
        assert_eq!(error.code, Some(KrakenErrorCode::RateLimitExceeded));
    }

    #[test]
    fn test_recovery_strategies() {
        assert!(matches!(
            KrakenErrorCode::RateLimitExceeded.recovery_strategy(),
            RecoveryStrategy::Backoff { .. }
        ));

        assert!(matches!(
            KrakenErrorCode::InvalidKey.recovery_strategy(),
            RecoveryStrategy::Reauthenticate
        ));

        assert!(matches!(
            KrakenErrorCode::ServiceUnavailable.recovery_strategy(),
            RecoveryStrategy::Retry { .. }
        ));

        assert!(matches!(
            KrakenErrorCode::PermissionDenied.recovery_strategy(),
            RecoveryStrategy::Fatal
        ));
    }

    #[test]
    fn test_error_categories() {
        assert!(KrakenErrorCode::InvalidKey.is_auth_error());
        assert!(KrakenErrorCode::RateLimitExceeded.is_rate_limit());
        assert!(KrakenErrorCode::InsufficientFunds.is_trading_error());
    }
}
