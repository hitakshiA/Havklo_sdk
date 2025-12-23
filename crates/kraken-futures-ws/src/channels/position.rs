//! Position channel handler

use crate::types::{FuturesEvent, MarginInfo, Position, PositionSide, PositionUpdate};
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{debug, info};

/// Position channel handler
pub struct PositionChannel {
    /// Current positions by product ID
    positions: HashMap<String, Position>,
    /// Margin info
    margin: Option<MarginInfo>,
}

impl PositionChannel {
    /// Create a new position channel handler
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
            margin: None,
        }
    }

    /// Process a position update
    pub fn process_positions(&mut self, update: PositionUpdate) -> FuturesEvent {
        for pos in &update.positions {
            debug!(
                "Position update for {}: {:?} {} @ {}",
                pos.product_id, pos.side, pos.size, pos.entry_price
            );

            if pos.size.is_zero() {
                // Position closed
                self.positions.remove(&pos.product_id);
                info!("Position closed for {}", pos.product_id);
            } else {
                self.positions.insert(pos.product_id.clone(), pos.clone());
            }
        }

        FuturesEvent::Position(update)
    }

    /// Process a margin update
    pub fn process_margin(&mut self, margin: MarginInfo) -> FuturesEvent {
        debug!(
            "Margin update: available={}, pnl={}",
            margin.available_margin, margin.unrealized_pnl
        );
        self.margin = Some(margin.clone());
        FuturesEvent::Margin(margin)
    }

    /// Get position for a product
    pub fn position(&self, product_id: &str) -> Option<&Position> {
        self.positions.get(product_id)
    }

    /// Get all open positions
    pub fn all_positions(&self) -> Vec<&Position> {
        self.positions.values().collect()
    }

    /// Check if we have a position in a product
    pub fn has_position(&self, product_id: &str) -> bool {
        self.positions.contains_key(product_id)
    }

    /// Get position size for a product (negative for short)
    pub fn position_size(&self, product_id: &str) -> Decimal {
        self.positions
            .get(product_id)
            .map(|p| match p.side {
                PositionSide::Long => p.size,
                PositionSide::Short => -p.size,
            })
            .unwrap_or(Decimal::ZERO)
    }

    /// Get total unrealized PnL across all positions
    pub fn total_unrealized_pnl(&self) -> Decimal {
        self.positions.values().map(|p| p.unrealized_pnl).sum()
    }

    /// Get total position value across all products
    pub fn total_position_value(&self) -> Decimal {
        self.positions.values().map(|p| p.value()).sum()
    }

    /// Get current margin info
    pub fn margin_info(&self) -> Option<&MarginInfo> {
        self.margin.as_ref()
    }

    /// Get available margin
    pub fn available_margin(&self) -> Option<Decimal> {
        self.margin.as_ref().map(|m| m.available_margin)
    }

    /// Get margin level percentage
    pub fn margin_level(&self) -> Option<Decimal> {
        self.margin.as_ref().map(|m| m.margin_level)
    }

    /// Check if margin is low (below 150%)
    pub fn is_margin_low(&self) -> bool {
        self.margin
            .as_ref()
            .map(|m| m.margin_level < Decimal::from(150))
            .unwrap_or(false)
    }

    /// Check if we're approaching liquidation (below 110%)
    pub fn is_margin_critical(&self) -> bool {
        self.margin
            .as_ref()
            .map(|m| m.margin_level < Decimal::from(110))
            .unwrap_or(false)
    }
}

impl Default for PositionChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_position() -> Position {
        Position {
            product_id: "PI_XBTUSD".to_string(),
            side: PositionSide::Long,
            size: Decimal::from(1),
            entry_price: Decimal::from(50000),
            mark_price: Decimal::from(51000),
            liq_price: Some(Decimal::from(40000)),
            unrealized_pnl: Decimal::from(1000),
            realized_pnl: Decimal::ZERO,
            margin: Decimal::from(5000),
            leverage: Decimal::from(10),
        }
    }

    #[test]
    fn test_position_channel() {
        let mut channel = PositionChannel::new();

        let update = PositionUpdate {
            positions: vec![create_test_position()],
            account: Some("test".to_string()),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        channel.process_positions(update);

        assert!(channel.has_position("PI_XBTUSD"));
        assert_eq!(channel.position_size("PI_XBTUSD"), Decimal::from(1));
        assert_eq!(channel.total_unrealized_pnl(), Decimal::from(1000));
    }

    #[test]
    fn test_position_close() {
        let mut channel = PositionChannel::new();

        // Open position
        let update = PositionUpdate {
            positions: vec![create_test_position()],
            account: None,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };
        channel.process_positions(update);
        assert!(channel.has_position("PI_XBTUSD"));

        // Close position (size = 0)
        let mut closed_pos = create_test_position();
        closed_pos.size = Decimal::ZERO;
        let close_update = PositionUpdate {
            positions: vec![closed_pos],
            account: None,
            timestamp: "2024-01-01T00:01:00Z".to_string(),
        };
        channel.process_positions(close_update);

        assert!(!channel.has_position("PI_XBTUSD"));
    }

    #[test]
    fn test_margin_monitoring() {
        let mut channel = PositionChannel::new();

        // Normal margin
        let margin = MarginInfo {
            available_margin: Decimal::from(10000),
            initial_margin: Decimal::from(5000),
            maintenance_margin: Decimal::from(2500),
            portfolio_value: Decimal::from(20000),
            unrealized_pnl: Decimal::from(1000),
            margin_level: Decimal::from(400), // 400% - healthy
        };
        channel.process_margin(margin);

        assert!(!channel.is_margin_low());
        assert!(!channel.is_margin_critical());

        // Low margin
        let low_margin = MarginInfo {
            available_margin: Decimal::from(1000),
            initial_margin: Decimal::from(5000),
            maintenance_margin: Decimal::from(2500),
            portfolio_value: Decimal::from(6000),
            unrealized_pnl: Decimal::from(-4000),
            margin_level: Decimal::from(120), // 120% - low but not critical
        };
        channel.process_margin(low_margin);

        assert!(channel.is_margin_low());
        assert!(!channel.is_margin_critical());

        // Critical margin
        let critical_margin = MarginInfo {
            available_margin: Decimal::from(100),
            initial_margin: Decimal::from(5000),
            maintenance_margin: Decimal::from(2500),
            portfolio_value: Decimal::from(2700),
            unrealized_pnl: Decimal::from(-7300),
            margin_level: Decimal::from(105), // 105% - approaching liquidation
        };
        channel.process_margin(critical_margin);

        assert!(channel.is_margin_low());
        assert!(channel.is_margin_critical());
    }
}
