//! Authentication for Kraken Futures WebSocket
//!
//! Kraken Futures uses a different authentication mechanism than Spot:
//! - Challenge-response based authentication
//! - Uses API key directly (no WS token)
//! - HMAC-SHA256 signing (not SHA512)

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{FuturesError, FuturesResult};

type HmacSha256 = Hmac<Sha256>;

/// Credentials for Futures API authentication
#[derive(Clone)]
pub struct FuturesCredentials {
    /// API key
    api_key: String,
    /// API secret (base64 encoded)
    api_secret: Vec<u8>,
}

impl FuturesCredentials {
    /// Create new credentials
    pub fn new(api_key: impl Into<String>, api_secret: impl AsRef<str>) -> FuturesResult<Self> {
        let api_key = api_key.into();
        let secret_str = api_secret.as_ref();

        let decoded = BASE64
            .decode(secret_str)
            .map_err(|e| FuturesError::InvalidCredentials(format!("Invalid base64 secret: {}", e)))?;

        Ok(Self {
            api_key,
            api_secret: decoded,
        })
    }

    /// Create credentials from environment variables
    ///
    /// Reads `KRAKEN_FUTURES_API_KEY` and `KRAKEN_FUTURES_API_SECRET`
    pub fn from_env() -> FuturesResult<Self> {
        let api_key = std::env::var("KRAKEN_FUTURES_API_KEY")
            .map_err(|_| FuturesError::EnvVarNotSet("KRAKEN_FUTURES_API_KEY".to_string()))?;
        let api_secret = std::env::var("KRAKEN_FUTURES_API_SECRET")
            .map_err(|_| FuturesError::EnvVarNotSet("KRAKEN_FUTURES_API_SECRET".to_string()))?;

        Self::new(api_key, api_secret)
    }

    /// Get the API key
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Sign a challenge for authentication
    ///
    /// Kraken Futures auth flow:
    /// 1. Server sends challenge string
    /// 2. Client signs: HMAC-SHA256(api_secret, challenge)
    /// 3. Client sends: api_key + signed_challenge
    pub fn sign_challenge(&self, challenge: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.api_secret)
            .expect("HMAC can take key of any size");
        mac.update(challenge.as_bytes());
        let result = mac.finalize();

        BASE64.encode(result.into_bytes())
    }

    /// Create the authentication message for the challenge
    pub fn auth_message(&self, challenge: &str) -> serde_json::Value {
        let signed = self.sign_challenge(challenge);

        serde_json::json!({
            "event": "challenge",
            "api_key": self.api_key,
            "signed_challenge": signed
        })
    }
}

impl std::fmt::Debug for FuturesCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuturesCredentials")
            .field("api_key", &format!("{}...", &self.api_key[..8.min(self.api_key.len())]))
            .field("api_secret", &"[REDACTED]")
            .finish()
    }
}

/// Authentication state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthState {
    /// Not authenticated
    Unauthenticated,
    /// Waiting for challenge
    WaitingForChallenge,
    /// Challenge received, signing
    Signing,
    /// Authenticated
    Authenticated,
    /// Authentication failed
    Failed(String),
}

impl Default for AuthState {
    fn default() -> Self {
        Self::Unauthenticated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_debug_redacts_secret() {
        let creds = FuturesCredentials::new("test_api_key", "dGVzdF9zZWNyZXQ=").unwrap();
        let debug = format!("{:?}", creds);
        assert!(!debug.contains("test_secret"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn test_sign_challenge() {
        let creds = FuturesCredentials::new(
            "test_key",
            "dGVzdF9zZWNyZXQ=", // base64 of "test_secret"
        ).unwrap();

        let signed = creds.sign_challenge("test_challenge");
        assert!(!signed.is_empty());

        // Signing same challenge should produce same result
        let signed2 = creds.sign_challenge("test_challenge");
        assert_eq!(signed, signed2);

        // Different challenge should produce different result
        let signed3 = creds.sign_challenge("different_challenge");
        assert_ne!(signed, signed3);
    }

    #[test]
    fn test_auth_message() {
        let creds = FuturesCredentials::new("test_key", "dGVzdF9zZWNyZXQ=").unwrap();
        let msg = creds.auth_message("challenge123");

        assert_eq!(msg["event"], "challenge");
        assert_eq!(msg["api_key"], "test_key");
        assert!(msg["signed_challenge"].is_string());
    }
}
