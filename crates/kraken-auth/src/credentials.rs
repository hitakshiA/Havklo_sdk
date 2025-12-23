//! Authentication credentials for Kraken API
//!
//! Implements HMAC-SHA512 signing as required by Kraken's private endpoints.
//!
//! # Security
//!
//! Private keys are stored using the `secrecy` crate which:
//! - Zeroizes memory on drop (prevents memory scanning)
//! - Prevents accidental logging via Debug impl
//! - Provides explicit access via `expose_secret()`

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretBox};
use sha2::{Digest, Sha256, Sha512};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{AuthError, AuthResult};

type HmacSha512 = Hmac<Sha512>;

/// Atomic nonce counter to ensure unique nonces even with rapid requests
static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// API credentials for authenticated requests
///
/// Private keys are automatically zeroized when the Credentials are dropped,
/// preventing sensitive data from remaining in memory.
pub struct Credentials {
    /// API key (public)
    api_key: String,
    /// Private key (decoded from base64, zeroized on drop)
    private_key: SecretBox<Vec<u8>>,
}

impl Credentials {
    /// Create new credentials from API key and private key
    ///
    /// # Arguments
    /// * `api_key` - Your Kraken API key
    /// * `private_key` - Your private key (base64 encoded string)
    ///
    /// # Returns
    /// Result containing Credentials or error if private key is invalid
    ///
    /// # Security
    /// The private key is immediately converted to a SecretVec which will
    /// be zeroized when dropped.
    pub fn new(api_key: impl Into<String>, private_key: impl AsRef<str>) -> AuthResult<Self> {
        let api_key = api_key.into();
        let private_key_str = private_key.as_ref();

        let decoded = BASE64.decode(private_key_str).map_err(|e| {
            AuthError::InvalidCredentials(format!("Invalid base64 private key: {}", e))
        })?;

        Ok(Self {
            api_key,
            private_key: SecretBox::new(Box::new(decoded)),
        })
    }

    /// Create credentials from environment variables
    ///
    /// Reads `KRAKEN_API_KEY` and `KRAKEN_PRIVATE_KEY` from the environment.
    pub fn from_env() -> AuthResult<Self> {
        let api_key = std::env::var("KRAKEN_API_KEY")
            .map_err(|_| AuthError::EnvVarNotSet("KRAKEN_API_KEY".to_string()))?;
        let private_key = std::env::var("KRAKEN_PRIVATE_KEY")
            .map_err(|_| AuthError::EnvVarNotSet("KRAKEN_PRIVATE_KEY".to_string()))?;

        Self::new(api_key, private_key)
    }

    /// Get the API key
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Generate a unique nonce for this request
    ///
    /// Nonces must be strictly increasing. We use millisecond timestamp
    /// plus an atomic counter to handle rapid successive requests.
    pub fn generate_nonce() -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as u64;

        // Combine timestamp with counter for uniqueness
        let counter = NONCE_COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("{}{:06}", timestamp, counter % 1_000_000)
    }

    /// Sign a request for Kraken's API
    ///
    /// Kraken signature algorithm:
    /// 1. SHA256(nonce + POST_data)
    /// 2. HMAC-SHA512(private_key, uri_path + SHA256_result)
    /// 3. Base64 encode result
    ///
    /// # Arguments
    /// * `path` - API endpoint path (e.g., "/0/private/GetWebSocketsToken")
    /// * `nonce` - Unique nonce for this request
    /// * `post_data` - URL-encoded POST body
    ///
    /// # Returns
    /// Base64-encoded signature
    pub fn sign(&self, path: &str, nonce: &str, post_data: &str) -> String {
        // Step 1: SHA256(nonce + post_data)
        let mut sha256 = Sha256::new();
        sha256.update(nonce.as_bytes());
        sha256.update(post_data.as_bytes());
        let sha256_result = sha256.finalize();

        // Step 2: message = path + SHA256_result
        let mut message = path.as_bytes().to_vec();
        message.extend_from_slice(&sha256_result);

        // Step 3: HMAC-SHA512(private_key, message)
        // expose_secret() provides controlled access to the key
        let mut mac = HmacSha512::new_from_slice(self.private_key.expose_secret())
            .expect("HMAC can take key of any size");
        mac.update(&message);
        let result = mac.finalize();

        // Step 4: Base64 encode
        BASE64.encode(result.into_bytes())
    }
}

impl Clone for Credentials {
    /// Clone credentials (creates new SecretBox with same content)
    fn clone(&self) -> Self {
        Self {
            api_key: self.api_key.clone(),
            private_key: SecretBox::new(Box::new(self.private_key.expose_secret().clone())),
        }
    }
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field(
                "api_key",
                &format!("{}...", &self.api_key[..8.min(self.api_key.len())]),
            )
            .field("private_key", &"[REDACTED]")
            .finish()
    }
}

/// Request signer for building authenticated requests
#[derive(Debug)]
pub struct RequestSigner<'a> {
    credentials: &'a Credentials,
    path: String,
    nonce: String,
}

impl<'a> RequestSigner<'a> {
    /// Create a new request signer
    pub fn new(credentials: &'a Credentials, path: impl Into<String>) -> Self {
        Self {
            credentials,
            path: path.into(),
            nonce: Credentials::generate_nonce(),
        }
    }

    /// Get the nonce for this request
    pub fn nonce(&self) -> &str {
        &self.nonce
    }

    /// Get the API key
    pub fn api_key(&self) -> &str {
        self.credentials.api_key()
    }

    /// Sign the request with the given POST data
    pub fn sign(&self, post_data: &str) -> String {
        self.credentials.sign(&self.path, &self.nonce, post_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_generation() {
        let nonce1 = Credentials::generate_nonce();
        let nonce2 = Credentials::generate_nonce();
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_nonce_is_numeric() {
        let nonce = Credentials::generate_nonce();
        assert!(nonce.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_credentials_debug_redacts_key() {
        let creds = Credentials::new("test_api_key", "dGVzdF9wcml2YXRlX2tleQ==").unwrap();
        let debug = format!("{:?}", creds);
        assert!(!debug.contains("test_private_key"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn test_signing_consistency() {
        let creds = Credentials::new(
            "API_KEY",
            "kQH5HW/8p1uGOVjbgWA7FunAmGO8lsSUXNsu3eow76sz84Q18fWxnyRzBHCd3pd5nE9qa99HAZtuZuj6F1huXg==",
        )
        .unwrap();

        let signature = creds.sign(
            "/0/private/GetWebSocketsToken",
            "1616492376594",
            "nonce=1616492376594",
        );

        // Signature should be base64 encoded
        assert!(BASE64.decode(&signature).is_ok());

        // And consistent
        let signature2 = creds.sign(
            "/0/private/GetWebSocketsToken",
            "1616492376594",
            "nonce=1616492376594",
        );
        assert_eq!(signature, signature2);
    }
}
