//! Error types for authentication operations

/// Errors that can occur during authentication
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Invalid API credentials
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    /// API returned an error
    #[error("API error: {0}")]
    Api(String),

    /// Failed to parse response
    #[error("Parse error: {0}")]
    Parse(String),

    /// Environment variable not set
    #[error("Environment variable not set: {0}")]
    EnvVarNotSet(String),

    /// Token expired
    #[error("WebSocket token expired")]
    TokenExpired,
}

/// Result type for authentication operations
pub type AuthResult<T> = Result<T, AuthError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AuthError::EnvVarNotSet("KRAKEN_API_KEY".to_string());
        assert!(err.to_string().contains("KRAKEN_API_KEY"));
    }
}
