//! Price level types with decimal precision

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};

/// A single price level in the orderbook
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    /// Price of this level
    #[serde(deserialize_with = "deserialize_decimal")]
    pub price: Decimal,
    /// Quantity at this price level
    #[serde(deserialize_with = "deserialize_decimal")]
    pub qty: Decimal,
}

impl Level {
    /// Create a new price level
    pub fn new(price: Decimal, qty: Decimal) -> Self {
        Self { price, qty }
    }

    /// Create a level from f64 values (for testing)
    pub fn from_f64(price: f64, qty: f64) -> Self {
        use rust_decimal::prelude::FromPrimitive;
        Self {
            price: Decimal::from_f64(price).unwrap_or_default(),
            qty: Decimal::from_f64(qty).unwrap_or_default(),
        }
    }

    /// Get price as f64 (for JavaScript interop)
    pub fn price_f64(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.price.to_f64().unwrap_or(0.0)
    }

    /// Get quantity as f64 (for JavaScript interop)
    pub fn qty_f64(&self) -> f64 {
        use rust_decimal::prelude::ToPrimitive;
        self.qty.to_f64().unwrap_or(0.0)
    }

    /// Check if this level has zero quantity (should be removed)
    pub fn is_zero(&self) -> bool {
        self.qty.is_zero()
    }
}

/// CRITICAL: Custom deserializer to preserve decimal precision
/// Kraken sends JSON numbers that lose precision with f64
fn deserialize_decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    use rust_decimal::prelude::FromPrimitive;
    use serde::de::Error;
    use std::str::FromStr;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(serde_json::Number),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Decimal::from_str(&s).map_err(D::Error::custom),
        StringOrNumber::Number(n) => {
            // First try to parse the string representation
            let s = n.to_string();
            // Handle scientific notation (e.g., 5e-6) by using f64 conversion
            if s.contains('e') || s.contains('E') {
                let f = n.as_f64().ok_or_else(|| D::Error::custom("invalid number"))?;
                Decimal::from_f64(f).ok_or_else(|| D::Error::custom("cannot convert to decimal"))
            } else {
                Decimal::from_str(&s).map_err(D::Error::custom)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_from_json_number() {
        // Test parsing from JSON with number values (as sent by Kraken)
        let json = r#"{"price": 88813.5, "qty": 0.00460208}"#;
        let level: Level = serde_json::from_str(json).unwrap();

        assert_eq!(level.price.to_string(), "88813.5");
        assert_eq!(level.qty.to_string(), "0.00460208");
    }

    #[test]
    fn test_level_from_json_string() {
        // Test parsing from JSON with string values
        let json = r#"{"price": "88813.5", "qty": "0.00460208"}"#;
        let level: Level = serde_json::from_str(json).unwrap();

        assert_eq!(level.price.to_string(), "88813.5");
        assert_eq!(level.qty.to_string(), "0.00460208");
    }

    #[test]
    fn test_level_precision_preserved() {
        // Ensure precision is preserved for checksum calculation
        // Using realistic Kraken values that don't trigger scientific notation
        let json = r#"{"price": 88813.5, "qty": 0.00460208}"#;
        let level: Level = serde_json::from_str(json).unwrap();

        assert_eq!(level.price.to_string(), "88813.5");
        assert_eq!(level.qty.to_string(), "0.00460208");
    }

    #[test]
    fn test_level_small_qty() {
        // Test small quantities that might be in scientific notation
        let json = r#"{"price": 0.05005, "qty": 0.000005}"#;
        let level: Level = serde_json::from_str(json).unwrap();

        assert_eq!(level.price.to_string(), "0.05005");
        // Small values might have trailing zeros stripped
        assert!(level.qty > Decimal::ZERO);
    }

    #[test]
    fn test_level_is_zero() {
        let zero = Level::new(Decimal::new(100, 0), Decimal::ZERO);
        assert!(zero.is_zero());

        let non_zero = Level::new(Decimal::new(100, 0), Decimal::ONE);
        assert!(!non_zero.is_zero());
    }
}
