//! WebSocket token provider
//!
//! Provides functionality to obtain authentication tokens for Kraken's private WebSocket channels.

use crate::credentials::{Credentials, RequestSigner};
use crate::error::{AuthError, AuthResult};
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, instrument};

const BASE_URL: &str = "https://api.kraken.com";

/// WebSocket authentication token
#[derive(Debug, Clone)]
pub struct WsToken {
    /// The authentication token
    pub token: String,
    /// Token expiration in seconds (typically 900 = 15 minutes)
    pub expires: u64,
}

/// Response from GetWebSocketsToken endpoint
#[derive(Debug, Deserialize)]
struct TokenResponse {
    error: Vec<String>,
    result: Option<TokenResult>,
}

#[derive(Debug, Deserialize)]
struct TokenResult {
    token: String,
    expires: u64,
}

/// Provider for WebSocket authentication tokens
///
/// Handles obtaining and refreshing tokens for private WebSocket channels.
///
/// # Example
///
/// ```no_run
/// use kraken_auth::{Credentials, TokenProvider};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let creds = Credentials::from_env()?;
/// let provider = TokenProvider::new(creds);
///
/// // Get token for private WebSocket channels
/// let token = provider.get_ws_token().await?;
/// println!("Use this token for private subscriptions: {}", token.token);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct TokenProvider {
    credentials: Credentials,
    client: Client,
}

impl TokenProvider {
    /// Create a new token provider with the given credentials
    pub fn new(credentials: Credentials) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("kraken-auth/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            credentials,
            client,
        }
    }

    /// Create a new token provider from environment variables
    ///
    /// Reads `KRAKEN_API_KEY` and `KRAKEN_PRIVATE_KEY` from the environment.
    pub fn from_env() -> AuthResult<Self> {
        let credentials = Credentials::from_env()?;
        Ok(Self::new(credentials))
    }

    /// Get a WebSocket authentication token
    ///
    /// This token is required for subscribing to private WebSocket channels
    /// such as `executions` and `balances`.
    ///
    /// # Returns
    /// A token valid for approximately 15 minutes.
    ///
    /// # Errors
    /// Returns an error if the request fails or credentials are invalid.
    #[instrument(skip(self))]
    pub async fn get_ws_token(&self) -> AuthResult<WsToken> {
        let path = "/0/private/GetWebSocketsToken";
        let signer = RequestSigner::new(&self.credentials, path);
        let nonce = signer.nonce();

        // Build POST data with nonce
        let post_data =
            serde_urlencoded::to_string([("nonce", nonce)]).map_err(|e| AuthError::Parse(e.to_string()))?;

        let signature = signer.sign(&post_data);
        let url = format!("{}{}", BASE_URL, path);

        debug!("Requesting WebSocket token");

        let response: TokenResponse = self
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

        // Check for API errors
        if !response.error.is_empty() {
            return Err(AuthError::Api(response.error.join(", ")));
        }

        // Extract token
        let result = response
            .result
            .ok_or_else(|| AuthError::Parse("Missing result in response".to_string()))?;

        debug!("Got WebSocket token, expires in {} seconds", result.expires);

        Ok(WsToken {
            token: result.token,
            expires: result.expires,
        })
    }

    /// Get the credentials used by this provider
    pub fn credentials(&self) -> &Credentials {
        &self.credentials
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_provider_creation() {
        let creds = Credentials::new("test_key", "dGVzdF9wcml2YXRlX2tleQ==").unwrap();
        let provider = TokenProvider::new(creds);
        assert!(provider.credentials().api_key().starts_with("test"));
    }
}
