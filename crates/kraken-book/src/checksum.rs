//! CRC32 checksum validation for orderbook integrity
//!
//! Implements Kraken's orderbook checksum algorithm to detect data corruption.
//!
//! # Algorithm
//!
//! 1. Use top 10 levels only (regardless of subscribed depth)
//! 2. Process asks first (sorted low→high), then bids (sorted high→low)
//! 3. For each level: remove decimal point, strip leading zeros
//! 4. Concatenate all: asks_string + bids_string
//! 5. Apply standard CRC32 (ISO 3309, polynomial 0xEDB88320)

use crc32fast::Hasher;
use kraken_types::Level;
use rust_decimal::Decimal;

/// Compute Kraken's CRC32 checksum for orderbook validation
///
/// # Arguments
///
/// * `bids` - Bid levels sorted high to low (best bid first)
/// * `asks` - Ask levels sorted low to high (best ask first)
///
/// # Returns
///
/// The CRC32 checksum as a u32
pub fn compute_checksum(bids: &[Level], asks: &[Level]) -> u32 {
    let mut hasher = Hasher::new();

    // Take top 10 of each side
    let top_asks: Vec<_> = asks.iter().take(10).collect();
    let top_bids: Vec<_> = bids.iter().take(10).collect();

    // Process asks first (already sorted low to high)
    for ask in &top_asks {
        let price_str = format_for_checksum(&ask.price);
        let qty_str = format_for_checksum(&ask.qty);
        hasher.update(price_str.as_bytes());
        hasher.update(qty_str.as_bytes());
    }

    // Then bids (already sorted high to low)
    for bid in &top_bids {
        let price_str = format_for_checksum(&bid.price);
        let qty_str = format_for_checksum(&bid.qty);
        hasher.update(price_str.as_bytes());
        hasher.update(qty_str.as_bytes());
    }

    hasher.finalize()
}

/// Format a decimal for checksum: remove decimal point, strip leading zeros
///
/// Examples:
/// - 45285.2 → "452852"
/// - 0.00100000 → "100000"
/// - 0.05005 → "5005"
fn format_for_checksum(value: &Decimal) -> String {
    let s = value.to_string();

    // Remove the decimal point
    let without_decimal = s.replace('.', "");

    // Strip leading zeros
    let trimmed = without_decimal.trim_start_matches('0');

    // If all zeros, return "0"
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Checksum result with computed and expected values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChecksumResult {
    /// The computed checksum
    pub computed: u32,
    /// The expected checksum from the server
    pub expected: u32,
}

impl ChecksumResult {
    /// Create a new checksum result
    pub fn new(computed: u32, expected: u32) -> Self {
        Self { computed, expected }
    }

    /// Check if the checksum matches
    pub fn is_valid(&self) -> bool {
        self.computed == self.expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_format_for_checksum() {
        assert_eq!(format_for_checksum(&dec!(45285.2)), "452852");
        assert_eq!(format_for_checksum(&dec!(0.00100000)), "100000");
        assert_eq!(format_for_checksum(&dec!(0.05005)), "5005");
        assert_eq!(format_for_checksum(&dec!(1.5)), "15");
        assert_eq!(format_for_checksum(&dec!(100)), "100");
    }

    #[test]
    fn test_checksum_computation() {
        // Simple test case
        let asks = vec![
            Level::new(dec!(100.5), dec!(1.0)),
            Level::new(dec!(101.0), dec!(2.0)),
        ];
        let bids = vec![
            Level::new(dec!(100.0), dec!(1.5)),
            Level::new(dec!(99.5), dec!(2.5)),
        ];

        let checksum = compute_checksum(&bids, &asks);
        // The checksum is deterministic
        assert!(checksum > 0);

        // Same input should give same output
        let checksum2 = compute_checksum(&bids, &asks);
        assert_eq!(checksum, checksum2);
    }

    #[test]
    fn test_checksum_order_matters() {
        let level1 = Level::new(dec!(100), dec!(1));
        let level2 = Level::new(dec!(101), dec!(2));

        let checksum1 = compute_checksum(&[level1.clone()], &[level2.clone()]);
        let checksum2 = compute_checksum(&[level2], &[level1]);

        // Different order should give different checksum
        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_checksum_uses_top_10() {
        // Create 15 levels on each side
        let mut asks: Vec<Level> = (1..=15)
            .map(|i| Level::new(Decimal::from(100 + i), dec!(1)))
            .collect();
        let mut bids: Vec<Level> = (1..=15)
            .map(|i| Level::new(Decimal::from(100 - i), dec!(1)))
            .collect();

        let checksum1 = compute_checksum(&bids, &asks);

        // Add more levels beyond top 10
        asks.push(Level::new(dec!(200), dec!(1)));
        bids.push(Level::new(dec!(1), dec!(1)));

        let checksum2 = compute_checksum(&bids, &asks);

        // Checksum should be the same (only uses top 10)
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_checksum_result() {
        let result = ChecksumResult::new(12345, 12345);
        assert!(result.is_valid());

        let result = ChecksumResult::new(12345, 54321);
        assert!(!result.is_valid());
    }
}
