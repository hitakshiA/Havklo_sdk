//! Authentication for Kraken private WebSocket channels
//!
//! This module provides token management for accessing private channels
//! like executions, balances, and open orders.
//!
//! # Usage
//!
//! ```no_run
//! use kraken_sdk::auth::TokenManager;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create from environment variables
//!     let manager = TokenManager::from_env()?;
//!
//!     // Or provide credentials directly
//!     let manager = TokenManager::new("api_key", "private_key");
//!
//!     // Get a WebSocket token
//!     let token = manager.get_token().await?;
//!     println!("Token: {}", token);
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
use reqwest::Client;
use sha2::{Digest, Sha256, Sha512};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, instrument};

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
#[derive(Clone)]
pub struct TokenManager {
    api_key: String,
    private_key: String,
    client: Client,
}

impl TokenManager {
    /// Create a new TokenManager with API credentials
    pub fn new(api_key: impl Into<String>, private_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            private_key: private_key.into(),
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
        let nonce = self.generate_nonce();
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
    fn generate_nonce(&self) -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .to_string()
    }

    /// Sign an API request using HMAC-SHA512
    fn sign_request(
        &self,
        path: &str,
        nonce: &str,
        post_data: &str,
    ) -> Result<String, AuthError> {
        // Decode the private key
        let decoded_key = BASE64
            .decode(&self.private_key)
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
        let nonce1 = manager.generate_nonce();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let nonce2 = manager.generate_nonce();
        assert_ne!(nonce1, nonce2);
    }
}
