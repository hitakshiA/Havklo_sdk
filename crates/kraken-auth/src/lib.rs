//! Authentication and WebSocket token provider for Kraken API
//!
//! This crate provides authentication utilities for Kraken's WebSocket APIs.
//! The primary use case is obtaining WebSocket tokens for private channel subscriptions.
//!
//! # Example
//!
//! ```no_run
//! use kraken_auth::{Credentials, TokenProvider};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load credentials from environment
//!     let creds = Credentials::from_env()?;
//!
//!     // Create token provider
//!     let provider = TokenProvider::new(creds);
//!
//!     // Get WebSocket token for private channels
//!     let token = provider.get_ws_token().await?;
//!     println!("Token: {}", token.token);
//!
//!     Ok(())
//! }
//! ```

mod credentials;
mod error;
mod token;

pub use credentials::{Credentials, RequestSigner};
pub use error::{AuthError, AuthResult};
pub use token::{TokenProvider, WsToken};
