//! Authentication for Kraken private WebSocket channels
//!
//! This module provides token management for accessing private channels
//! like executions, balances, and open orders.
//!
//! # Features
//!
//! - Automatic token caching with expiry tracking
//! - Background token refresh before expiry
//! - Thread-safe token access for async contexts
//! - Token state observation via callbacks
//!
//! # Usage
//!
//! ```no_run
//! use kraken_sdk::auth::{TokenManager, AutoRefreshTokenManager};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Simple usage - manual refresh
//!     let manager = TokenManager::from_env()?;
//!     let token = manager.get_token().await?;
//!
//!     // Auto-refresh usage - handles expiry automatically
//!     let auto_manager = AutoRefreshTokenManager::from_env()?;
//!     auto_manager.start_auto_refresh().await;
//!
//!     // Get cached token (refreshes if needed)
//!     let token = auto_manager.get_valid_token().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Environment Variables
//!
//! - `KRAKEN_API_KEY` - Your Kraken API key
//! - `KRAKEN_PRIVATE_KEY` - Your Kraken private key (base64 encoded)

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use parking_lot::RwLock;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256, Sha512};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;
use tracing::{debug, error, info, instrument, warn};

type HmacSha512 = Hmac<Sha512>;

const KRAKEN_API_URL: &str = "https://api.kraken.com";
const GET_WS_TOKEN_PATH: &str = "/0/private/GetWebSocketsToken";

/// Error types for authentication
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Missing API key")]
    MissingApiKey,

    #[error("Missing private key")]
    MissingPrivateKey,

    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Environment variable not set: {0}")]
    EnvVarNotSet(String),

    #[error("System clock error: time went backwards")]
    SystemClockError,
}

/// Response from GetWebSocketsToken endpoint
#[derive(Debug, serde::Deserialize)]
struct TokenResponse {
    error: Vec<String>,
    result: Option<TokenResult>,
}

#[derive(Debug, serde::Deserialize)]
struct TokenResult {
    token: String,
    #[allow(dead_code)]
    expires: Option<u64>,
}

/// Manages authentication tokens for Kraken private channels
///
/// # Security
///
/// Private keys are stored using the `secrecy` crate which zeroizes
/// memory on drop, preventing sensitive data from remaining in memory.
pub struct TokenManager {
    api_key: String,
    /// Private key stored securely (zeroized on drop)
    private_key: SecretString,
    client: Client,
}

impl Clone for TokenManager {
    fn clone(&self) -> Self {
        Self {
            api_key: self.api_key.clone(),
            private_key: SecretString::from(self.private_key.expose_secret().to_string()),
            client: self.client.clone(),
        }
    }
}

impl TokenManager {
    /// Create a new TokenManager with API credentials
    ///
    /// The private key is immediately wrapped in SecretString for secure storage.
    pub fn new(api_key: impl Into<String>, private_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            private_key: SecretString::from(private_key.into()),
            client: Client::new(),
        }
    }

    /// Create a TokenManager from environment variables
    ///
    /// Reads `KRAKEN_API_KEY` and `KRAKEN_PRIVATE_KEY` from environment.
    pub fn from_env() -> Result<Self, AuthError> {
        let api_key = std::env::var("KRAKEN_API_KEY")
            .map_err(|_| AuthError::EnvVarNotSet("KRAKEN_API_KEY".to_string()))?;
        let private_key = std::env::var("KRAKEN_PRIVATE_KEY")
            .map_err(|_| AuthError::EnvVarNotSet("KRAKEN_PRIVATE_KEY".to_string()))?;

        Ok(Self::new(api_key, private_key))
    }

    /// Get a WebSocket authentication token
    ///
    /// Tokens are valid for 15 minutes. Call this method periodically
    /// to refresh the token before it expires.
    #[instrument(skip(self))]
    pub async fn get_token(&self) -> Result<String, AuthError> {
        let nonce = self.generate_nonce()?;
        let post_data = format!("nonce={}", nonce);

        let signature = self.sign_request(GET_WS_TOKEN_PATH, &nonce, &post_data)?;

        debug!("Requesting WebSocket token from Kraken API");

        let response = self
            .client
            .post(format!("{}{}", KRAKEN_API_URL, GET_WS_TOKEN_PATH))
            .header("API-Key", &self.api_key)
            .header("API-Sign", &signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(post_data)
            .send()
            .await?;

        let token_response: TokenResponse = response.json().await?;

        if !token_response.error.is_empty() {
            return Err(AuthError::ApiError(token_response.error.join(", ")));
        }

        token_response
            .result
            .map(|r| r.token)
            .ok_or_else(|| AuthError::ApiError("No token in response".to_string()))
    }

    /// Generate a nonce for API requests
    fn generate_nonce(&self) -> Result<String, AuthError> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AuthError::SystemClockError)?
            .as_millis()
            .to_string())
    }

    /// Sign an API request using HMAC-SHA512
    fn sign_request(
        &self,
        path: &str,
        nonce: &str,
        post_data: &str,
    ) -> Result<String, AuthError> {
        // Decode the private key (expose_secret provides controlled access)
        let decoded_key = BASE64
            .decode(self.private_key.expose_secret())
            .map_err(|e| AuthError::InvalidPrivateKey(e.to_string()))?;

        // Create SHA256 hash of nonce + post_data
        let mut sha256 = Sha256::new();
        sha256.update(nonce.as_bytes());
        sha256.update(post_data.as_bytes());
        let sha256_result = sha256.finalize();

        // Create message: path + SHA256 hash
        let mut message = path.as_bytes().to_vec();
        message.extend_from_slice(&sha256_result);

        // Create HMAC-SHA512 signature
        let mut mac = HmacSha512::new_from_slice(&decoded_key)
            .map_err(|e| AuthError::InvalidPrivateKey(e.to_string()))?;
        mac.update(&message);
        let result = mac.finalize();

        // Encode signature as base64
        Ok(BASE64.encode(result.into_bytes()))
    }
}

// ============================================================================
// Token Auto-Refresh Implementation
// ============================================================================

/// Token validity duration (15 minutes as per Kraken docs)
const TOKEN_VALIDITY_SECS: u64 = 15 * 60;

/// Refresh buffer - refresh this many seconds before expiry
const REFRESH_BUFFER_SECS: u64 = 60;

/// Minimum refresh interval to prevent rapid refresh loops
const MIN_REFRESH_INTERVAL_SECS: u64 = 30;

/// State of the authentication token
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenState {
    /// No token has been fetched yet
    NotInitialized,
    /// Token is valid and ready to use
    Valid,
    /// Token is being refreshed
    Refreshing,
    /// Token has expired
    Expired,
    /// Token refresh failed
    Error(String),
}

impl Default for TokenState {
    fn default() -> Self {
        Self::NotInitialized
    }
}

/// Cached token with metadata
#[derive(Debug, Clone)]
struct CachedToken {
    /// The token string
    token: String,
    /// When the token was fetched
    #[allow(dead_code)]
    fetched_at: Instant,
    /// When the token expires (server-reported or estimated)
    expires_at: Instant,
}

impl CachedToken {
    fn new(token: String, expires_in_secs: u64) -> Self {
        let now = Instant::now();
        Self {
            token,
            fetched_at: now,
            expires_at: now + Duration::from_secs(expires_in_secs),
        }
    }

    /// Check if token is expired
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Check if token should be refreshed (before expiry buffer)
    fn should_refresh(&self) -> bool {
        let refresh_threshold = self.expires_at - Duration::from_secs(REFRESH_BUFFER_SECS);
        Instant::now() >= refresh_threshold
    }

    /// Time until token expires
    fn time_until_expiry(&self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }

    /// Time until refresh should happen
    fn time_until_refresh(&self) -> Duration {
        let refresh_at = self.expires_at - Duration::from_secs(REFRESH_BUFFER_SECS);
        refresh_at.saturating_duration_since(Instant::now())
    }
}

/// Configuration for auto-refresh behavior
#[derive(Debug, Clone)]
pub struct AutoRefreshConfig {
    /// How many seconds before expiry to refresh (default: 60)
    pub refresh_buffer_secs: u64,
    /// Maximum retry attempts on refresh failure (default: 3)
    pub max_retries: u32,
    /// Initial retry delay in milliseconds (default: 1000)
    pub retry_delay_ms: u64,
    /// Enable/disable automatic background refresh (default: true)
    pub auto_refresh_enabled: bool,
}

impl Default for AutoRefreshConfig {
    fn default() -> Self {
        Self {
            refresh_buffer_secs: REFRESH_BUFFER_SECS,
            max_retries: 3,
            retry_delay_ms: 1000,
            auto_refresh_enabled: true,
        }
    }
}

/// Token manager with automatic refresh capability
///
/// This wraps the basic `TokenManager` and adds:
/// - Token caching with expiry tracking
/// - Automatic background refresh before expiry
/// - Thread-safe token access
/// - Observable token state
///
/// # Example
///
/// ```no_run
/// use kraken_sdk::auth::AutoRefreshTokenManager;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let manager = AutoRefreshTokenManager::from_env()?;
///
///     // Start background refresh task
///     manager.start_auto_refresh().await;
///
///     // Get the current valid token (waits for initial fetch)
///     let token = manager.get_valid_token().await?;
///
///     // Subscribe to token state changes
///     let mut state_rx = manager.subscribe_state();
///     tokio::spawn(async move {
///         while state_rx.changed().await.is_ok() {
///             println!("Token state: {:?}", *state_rx.borrow());
///         }
///     });
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct AutoRefreshTokenManager {
    inner: Arc<AutoRefreshInner>,
}

struct AutoRefreshInner {
    /// The underlying token manager
    token_manager: TokenManager,
    /// Cached token (if any)
    cached_token: RwLock<Option<CachedToken>>,
    /// Current token state
    state: RwLock<TokenState>,
    /// State change broadcaster
    state_tx: watch::Sender<TokenState>,
    /// Configuration
    config: AutoRefreshConfig,
    /// Shutdown flag
    shutdown: RwLock<bool>,
}

impl std::fmt::Debug for AutoRefreshTokenManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutoRefreshTokenManager")
            .field("state", &self.state())
            .field("has_token", &self.has_valid_token())
            .finish()
    }
}

impl AutoRefreshTokenManager {
    /// Create a new auto-refresh token manager with credentials
    pub fn new(api_key: impl Into<String>, private_key: impl Into<String>) -> Self {
        Self::with_config(api_key, private_key, AutoRefreshConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(
        api_key: impl Into<String>,
        private_key: impl Into<String>,
        config: AutoRefreshConfig,
    ) -> Self {
        let (state_tx, _) = watch::channel(TokenState::NotInitialized);

        Self {
            inner: Arc::new(AutoRefreshInner {
                token_manager: TokenManager::new(api_key, private_key),
                cached_token: RwLock::new(None),
                state: RwLock::new(TokenState::NotInitialized),
                state_tx,
                config,
                shutdown: RwLock::new(false),
            }),
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, AuthError> {
        let token_manager = TokenManager::from_env()?;
        let (state_tx, _) = watch::channel(TokenState::NotInitialized);

        Ok(Self {
            inner: Arc::new(AutoRefreshInner {
                token_manager,
                cached_token: RwLock::new(None),
                state: RwLock::new(TokenState::NotInitialized),
                state_tx,
                config: AutoRefreshConfig::default(),
                shutdown: RwLock::new(false),
            }),
        })
    }

    /// Create from environment variables with custom config
    pub fn from_env_with_config(config: AutoRefreshConfig) -> Result<Self, AuthError> {
        let token_manager = TokenManager::from_env()?;
        let (state_tx, _) = watch::channel(TokenState::NotInitialized);

        Ok(Self {
            inner: Arc::new(AutoRefreshInner {
                token_manager,
                cached_token: RwLock::new(None),
                state: RwLock::new(TokenState::NotInitialized),
                state_tx,
                config,
                shutdown: RwLock::new(false),
            }),
        })
    }

    /// Get the current token state
    pub fn state(&self) -> TokenState {
        self.inner.state.read().clone()
    }

    /// Check if we have a valid (non-expired) token
    pub fn has_valid_token(&self) -> bool {
        self.inner
            .cached_token
            .read()
            .as_ref()
            .map(|t: &CachedToken| !t.is_expired())
            .unwrap_or(false)
    }

    /// Get the cached token if valid, otherwise returns None
    pub fn get_cached_token(&self) -> Option<String> {
        let cached = self.inner.cached_token.read();
        cached.as_ref().filter(|t: &&CachedToken| !t.is_expired()).map(|t: &CachedToken| t.token.clone())
    }

    /// Get time until token expires (None if no token or expired)
    pub fn time_until_expiry(&self) -> Option<Duration> {
        let cached = self.inner.cached_token.read();
        cached
            .as_ref()
            .filter(|t: &&CachedToken| !t.is_expired())
            .map(|t: &CachedToken| t.time_until_expiry())
    }

    /// Subscribe to token state changes
    pub fn subscribe_state(&self) -> watch::Receiver<TokenState> {
        self.inner.state_tx.subscribe()
    }

    /// Get a valid token, fetching/refreshing if necessary
    ///
    /// This method will:
    /// 1. Return cached token if still valid
    /// 2. Fetch a new token if no cached token exists
    /// 3. Refresh the token if it's about to expire
    #[instrument(skip(self))]
    pub async fn get_valid_token(&self) -> Result<String, AuthError> {
        // Check if we have a valid cached token
        {
            let cached = self.inner.cached_token.read();
            if let Some(ref token) = *cached {
                if !token.is_expired() {
                    debug!("Using cached token (expires in {:?})", token.time_until_expiry());
                    return Ok(token.token.clone());
                }
            }
        }

        // Need to fetch/refresh
        debug!("No valid cached token, fetching new token");
        self.refresh_token().await
    }

    /// Force a token refresh
    #[instrument(skip(self))]
    pub async fn refresh_token(&self) -> Result<String, AuthError> {
        self.set_state(TokenState::Refreshing);

        let mut last_error = None;
        let mut delay = Duration::from_millis(self.inner.config.retry_delay_ms);

        for attempt in 0..=self.inner.config.max_retries {
            if attempt > 0 {
                warn!("Token refresh attempt {} after {:?} delay", attempt + 1, delay);
                tokio::time::sleep(delay).await;
                delay *= 2; // Exponential backoff
            }

            match self.inner.token_manager.get_token().await {
                Ok(token) => {
                    info!("Token refreshed successfully");

                    // Cache the token
                    let cached = CachedToken::new(token.clone(), TOKEN_VALIDITY_SECS);
                    *self.inner.cached_token.write() = Some(cached);

                    self.set_state(TokenState::Valid);
                    return Ok(token);
                }
                Err(e) => {
                    error!("Token refresh failed: {}", e);
                    last_error = Some(e);
                }
            }
        }

        let error = last_error.unwrap_or_else(|| AuthError::ApiError("Unknown error".into()));
        self.set_state(TokenState::Error(error.to_string()));
        Err(error)
    }

    /// Start the automatic background refresh task
    ///
    /// This spawns a background task that:
    /// 1. Fetches an initial token
    /// 2. Refreshes the token before it expires
    /// 3. Handles refresh failures with retries
    ///
    /// The task runs until `stop_auto_refresh()` is called or the manager is dropped.
    #[instrument(skip(self))]
    pub async fn start_auto_refresh(&self) {
        if !self.inner.config.auto_refresh_enabled {
            debug!("Auto-refresh disabled in config");
            return;
        }

        // Reset shutdown flag
        *self.inner.shutdown.write() = false;

        let manager = self.clone();
        tokio::spawn(async move {
            manager.auto_refresh_loop().await;
        });
    }

    /// Stop the automatic background refresh task
    pub fn stop_auto_refresh(&self) {
        *self.inner.shutdown.write() = true;
    }

    /// The background refresh loop
    async fn auto_refresh_loop(&self) {
        info!("Starting auto-refresh loop");

        // Fetch initial token
        if let Err(e) = self.refresh_token().await {
            error!("Initial token fetch failed: {}", e);
            // Continue loop - will retry
        }

        loop {
            // Check shutdown
            if *self.inner.shutdown.read() {
                info!("Auto-refresh loop shutting down");
                break;
            }

            // Calculate sleep duration
            let sleep_duration = {
                let cached = self.inner.cached_token.read();
                match &*cached {
                    Some(token) if !token.is_expired() => {
                        // Sleep until refresh time
                        let until_refresh = token.time_until_refresh();
                        until_refresh.max(Duration::from_secs(MIN_REFRESH_INTERVAL_SECS))
                    }
                    _ => {
                        // No valid token - retry soon
                        Duration::from_secs(MIN_REFRESH_INTERVAL_SECS)
                    }
                }
            };

            debug!("Auto-refresh sleeping for {:?}", sleep_duration);
            tokio::time::sleep(sleep_duration).await;

            // Check shutdown again after sleep
            if *self.inner.shutdown.read() {
                info!("Auto-refresh loop shutting down");
                break;
            }

            // Check if refresh is needed
            let should_refresh = {
                let cached = self.inner.cached_token.read();
                cached.as_ref().map(|t: &CachedToken| t.should_refresh()).unwrap_or(true)
            };

            if should_refresh {
                debug!("Token refresh triggered");
                if let Err(e) = self.refresh_token().await {
                    error!("Auto-refresh failed: {}", e);
                    // State is already set to Error in refresh_token
                }
            }
        }
    }

    /// Set the token state and notify subscribers
    fn set_state(&self, state: TokenState) {
        *self.inner.state.write() = state.clone();
        // Ignore send error (no subscribers)
        let _ = self.inner.state_tx.send(state);
    }

    /// Get the underlying TokenManager for direct access
    pub fn token_manager(&self) -> &TokenManager {
        &self.inner.token_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_manager_creation() {
        let manager = TokenManager::new("test_key", "dGVzdF9zZWNyZXQ=");
        assert_eq!(manager.api_key, "test_key");
    }

    #[test]
    fn test_nonce_generation() {
        let manager = TokenManager::new("key", "c2VjcmV0");
        let nonce1 = manager.generate_nonce().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let nonce2 = manager.generate_nonce().unwrap();
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_cached_token_expiry() {
        let token = CachedToken::new("test_token".to_string(), 1);

        // Should not be expired immediately
        assert!(!token.is_expired());

        // Wait for expiry
        std::thread::sleep(Duration::from_secs(2));
        assert!(token.is_expired());
    }

    #[test]
    fn test_cached_token_should_refresh() {
        // Create token with 2 second validity (refresh buffer is 60s, but we'll test the logic)
        let token = CachedToken::new("test".to_string(), 65); // 65 seconds

        // Should not need refresh immediately (60s buffer, 65s validity)
        assert!(!token.should_refresh());
    }

    #[test]
    fn test_auto_refresh_config_default() {
        let config = AutoRefreshConfig::default();
        assert_eq!(config.refresh_buffer_secs, 60);
        assert_eq!(config.max_retries, 3);
        assert!(config.auto_refresh_enabled);
    }

    #[test]
    fn test_auto_refresh_token_manager_creation() {
        let manager = AutoRefreshTokenManager::new("api_key", "c2VjcmV0");
        assert_eq!(manager.state(), TokenState::NotInitialized);
        assert!(!manager.has_valid_token());
    }

    #[test]
    fn test_token_state_default() {
        assert_eq!(TokenState::default(), TokenState::NotInitialized);
    }

    #[test]
    fn test_token_state_equality() {
        assert_eq!(TokenState::Valid, TokenState::Valid);
        assert_ne!(TokenState::Valid, TokenState::Expired);
        assert_eq!(
            TokenState::Error("test".to_string()),
            TokenState::Error("test".to_string())
        );
    }

    #[test]
    fn test_cached_token_time_tracking() {
        let token = CachedToken::new("test".to_string(), 100);

        // Time until expiry should be close to 100 seconds
        let until_expiry = token.time_until_expiry();
        assert!(until_expiry > Duration::from_secs(95));
        assert!(until_expiry <= Duration::from_secs(100));

        // Time until refresh should be ~40 seconds (100 - 60 buffer)
        let until_refresh = token.time_until_refresh();
        assert!(until_refresh > Duration::from_secs(35));
        assert!(until_refresh <= Duration::from_secs(40));
    }

    #[test]
    fn test_auto_refresh_manager_debug() {
        let manager = AutoRefreshTokenManager::new("key", "c2VjcmV0");
        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("AutoRefreshTokenManager"));
        assert!(debug_str.contains("NotInitialized"));
    }

    #[tokio::test]
    async fn test_auto_refresh_state_subscription() {
        let manager = AutoRefreshTokenManager::new("key", "c2VjcmV0");
        let rx = manager.subscribe_state();

        // Initial state
        assert_eq!(*rx.borrow(), TokenState::NotInitialized);
    }
}
