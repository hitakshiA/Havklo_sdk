//! Trading pair symbols (BTC/USD format)

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Trading pair symbol (BTC/USD format - V2 API uses BTC, not XBT!)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Symbol(String);

impl Symbol {
    /// BTC/USD trading pair
    pub const BTC_USD: &'static str = "BTC/USD";
    /// ETH/USD trading pair
    pub const ETH_USD: &'static str = "ETH/USD";
    /// SOL/USD trading pair
    pub const SOL_USD: &'static str = "SOL/USD";
    /// XRP/USD trading pair
    pub const XRP_USD: &'static str = "XRP/USD";
    /// DOGE/USD trading pair
    pub const DOGE_USD: &'static str = "DOGE/USD";

    /// Create a new symbol from a string
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the symbol as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the base currency (e.g., "BTC" from "BTC/USD")
    pub fn base(&self) -> Option<&str> {
        self.0.split('/').next()
    }

    /// Get the quote currency (e.g., "USD" from "BTC/USD")
    pub fn quote(&self) -> Option<&str> {
        self.0.split('/').nth(1)
    }
}

impl FromStr for Symbol {
    type Err = SymbolParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Validate format: BASE/QUOTE
        if !s.contains('/') {
            return Err(SymbolParseError::MissingSlash(s.to_string()));
        }

        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(SymbolParseError::InvalidFormat(s.to_string()));
        }

        if parts[0].is_empty() || parts[1].is_empty() {
            return Err(SymbolParseError::EmptyPart(s.to_string()));
        }

        Ok(Self(s.to_string()))
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Symbol {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Error parsing a symbol
#[derive(Debug, Clone, thiserror::Error)]
pub enum SymbolParseError {
    #[error("Symbol must contain '/': {0}")]
    MissingSlash(String),

    #[error("Invalid symbol format: {0}")]
    InvalidFormat(String),

    #[error("Symbol has empty base or quote: {0}")]
    EmptyPart(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_parse() {
        let symbol: Symbol = "BTC/USD".parse().unwrap();
        assert_eq!(symbol.as_str(), "BTC/USD");
        assert_eq!(symbol.base(), Some("BTC"));
        assert_eq!(symbol.quote(), Some("USD"));
    }

    #[test]
    fn test_symbol_parse_error() {
        assert!("BTCUSD".parse::<Symbol>().is_err());
        assert!("/USD".parse::<Symbol>().is_err());
        assert!("BTC/".parse::<Symbol>().is_err());
    }

    #[test]
    fn test_symbol_serde() {
        let symbol = Symbol::new("ETH/USD");
        let json = serde_json::to_string(&symbol).unwrap();
        assert_eq!(json, "\"ETH/USD\"");

        let parsed: Symbol = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, symbol);
    }
}
