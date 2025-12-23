//! Subscription management

use kraken_types::{Channel, Depth, SubscribeParams, SubscribeRequest};
use std::collections::HashSet;

/// Active subscription tracker
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Channel type
    pub channel: Channel,
    /// Subscribed symbols
    pub symbols: Vec<String>,
    /// Orderbook depth (if applicable)
    pub depth: Option<Depth>,
    /// Request snapshot on subscribe
    pub snapshot: bool,
}

impl Subscription {
    /// Create a new subscription
    pub fn new(channel: Channel, symbols: Vec<String>) -> Self {
        Self {
            channel,
            symbols,
            depth: None,
            snapshot: true,
        }
    }

    /// Create an orderbook subscription
    pub fn orderbook(symbols: Vec<String>, depth: Depth) -> Self {
        Self {
            channel: Channel::Book,
            symbols,
            depth: Some(depth),
            snapshot: true,
        }
    }

    /// Create a ticker subscription
    pub fn ticker(symbols: Vec<String>) -> Self {
        Self {
            channel: Channel::Ticker,
            symbols,
            depth: None,
            snapshot: true,
        }
    }

    /// Create a trade subscription
    pub fn trade(symbols: Vec<String>) -> Self {
        Self {
            channel: Channel::Trade,
            symbols,
            depth: None,
            snapshot: true,
        }
    }

    /// Create an L3 (Level 3) orderbook subscription
    ///
    /// Note: L3 requires connection to the Level3 endpoint (wss://ws-l3.kraken.com/v2)
    /// and special access permissions.
    pub fn level3(symbols: Vec<String>) -> Self {
        Self {
            channel: Channel::Level3,
            symbols,
            depth: None,
            snapshot: true,
        }
    }

    /// Convert to a subscribe request
    pub fn to_request(&self, req_id: Option<u64>) -> SubscribeRequest {
        let params = match self.channel {
            Channel::Book => SubscribeParams::book(self.symbols.clone(), self.depth.unwrap_or(Depth::D10)),
            Channel::Ticker => SubscribeParams::ticker(self.symbols.clone()),
            Channel::Trade => SubscribeParams::trade(self.symbols.clone()),
            _ => SubscribeParams {
                channel: self.channel,
                symbol: self.symbols.clone(),
                depth: self.depth.map(|d| d.as_u32()),
                snapshot: Some(self.snapshot),
                interval: None,
                event_trigger: None,
                token: None,
            },
        };

        SubscribeRequest {
            method: "subscribe",
            params,
            req_id,
        }
    }
}

/// Manages active subscriptions for reconnection restoration
#[derive(Debug, Default)]
pub struct SubscriptionManager {
    /// Active subscriptions keyed by channel + symbols
    subscriptions: Vec<Subscription>,
    /// Pending subscription requests
    pending: HashSet<u64>,
    /// Next request ID
    next_req_id: u64,
}

impl SubscriptionManager {
    /// Create a new subscription manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a subscription
    pub fn add(&mut self, sub: Subscription) -> u64 {
        let req_id = self.next_req_id;
        self.next_req_id += 1;
        self.pending.insert(req_id);
        self.subscriptions.push(sub);
        req_id
    }

    /// Mark a subscription as confirmed
    pub fn confirm(&mut self, req_id: u64) {
        self.pending.remove(&req_id);
    }

    /// Mark a subscription as rejected
    pub fn reject(&mut self, req_id: u64) {
        self.pending.remove(&req_id);
        // Note: we don't remove from subscriptions - let caller decide
    }

    /// Get all active subscriptions (for restoration after reconnect)
    pub fn all(&self) -> &[Subscription] {
        &self.subscriptions
    }

    /// Get number of active subscriptions
    pub fn count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Clear all subscriptions
    pub fn clear(&mut self) {
        self.subscriptions.clear();
        self.pending.clear();
    }

    /// Check if any subscriptions are pending confirmation
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Get subscribe requests for all active subscriptions (for restoration)
    pub fn restoration_requests(&mut self) -> Vec<(u64, SubscribeRequest)> {
        let mut requests = Vec::new();

        for sub in &self.subscriptions {
            let req_id = self.next_req_id;
            self.next_req_id += 1;
            self.pending.insert(req_id);
            requests.push((req_id, sub.to_request(Some(req_id))));
        }

        requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_creation() {
        let sub = Subscription::orderbook(vec!["BTC/USD".to_string()], Depth::D10);
        assert_eq!(sub.channel, Channel::Book);
        assert_eq!(sub.depth, Some(Depth::D10));
        assert!(sub.snapshot);
    }

    #[test]
    fn test_subscription_manager() {
        let mut manager = SubscriptionManager::new();

        let sub1 = Subscription::ticker(vec!["BTC/USD".to_string()]);
        let req_id1 = manager.add(sub1);

        let sub2 = Subscription::orderbook(vec!["ETH/USD".to_string()], Depth::D10);
        let req_id2 = manager.add(sub2);

        assert_eq!(manager.count(), 2);
        assert!(manager.has_pending());

        manager.confirm(req_id1);
        manager.confirm(req_id2);

        assert!(!manager.has_pending());
    }
}
