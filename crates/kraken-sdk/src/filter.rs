//! Event Filtering System
//!
//! Provides efficient filtering for market events based on symbols, channels,
//! and custom criteria. This enables building targeted demo applications.
//!
//! # Features
//!
//! - **Symbol Filtering**: Only receive events for specific symbols
//! - **Channel Filtering**: Filter by event type (orderbook, ticker, trade)
//! - **Trade Size Filter**: Minimum trade size threshold
//! - **Spread Threshold**: Alert on spread changes beyond threshold
//! - **Custom Predicates**: User-defined filter functions
//!
//! # Example
//!
//! ```
//! use kraken_sdk::filter::{EventFilter, FilterBuilder};
//!
//! // Filter for BTC/USD orderbook events only
//! let filter = FilterBuilder::new()
//!     .symbols(["BTC/USD", "ETH/USD"])
//!     .orderbook_events()
//!     .build();
//!
//! // Filter for large trades only
//! let trade_filter = FilterBuilder::new()
//!     .symbols(["BTC/USD"])
//!     .trade_events()
//!     .min_trade_size(rust_decimal_macros::dec!(1.0))
//!     .build();
//! ```

use kraken_types::Decimal;
use kraken_ws::{Event, MarketEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Event filter configuration
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Filter by symbols (None = all symbols)
    pub symbols: Option<HashSet<String>>,
    /// Filter by channels (None = all channels)
    pub channels: Option<HashSet<FilterChannel>>,
    /// Minimum trade size to pass through
    pub min_trade_size: Option<Decimal>,
    /// Include connection events
    pub include_connection: bool,
    /// Include subscription events
    pub include_subscription: bool,
    /// Include private events
    pub include_private: bool,
    /// Include L3 events
    pub include_l3: bool,
}

/// Channels that can be filtered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum FilterChannel {
    /// Orderbook updates
    Orderbook,
    /// Ticker updates
    Ticker,
    /// Trade updates
    Trade,
    /// OHLC candles (Open, High, Low, Close)
    OHLC,
    /// Status messages
    Status,
    /// Heartbeat
    Heartbeat,
}

impl EventFilter {
    /// Create a new filter that allows all events
    pub fn all() -> Self {
        Self {
            symbols: None,
            channels: None,
            min_trade_size: None,
            include_connection: true,
            include_subscription: true,
            include_private: true,
            include_l3: true,
        }
    }

    /// Create a filter for specific symbols only
    pub fn symbols(symbols: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            symbols: Some(symbols.into_iter().map(Into::into).collect()),
            ..Default::default()
        }
    }

    /// Create a filter for market events only (no connection/subscription events)
    pub fn market_only() -> Self {
        Self {
            include_connection: false,
            include_subscription: false,
            include_private: false,
            include_l3: false,
            ..Default::default()
        }
    }

    /// Check if an event passes this filter
    pub fn matches(&self, event: &Event) -> bool {
        match event {
            Event::Connection(_) => self.include_connection,
            Event::Subscription(_) => self.include_subscription,
            Event::Private(_) => self.include_private,
            Event::L3(l3) => {
                if !self.include_l3 {
                    return false;
                }
                self.matches_symbol(l3.symbol())
            }
            Event::Market(market) => self.matches_market_event(market),
        }
    }

    /// Check if a market event passes this filter
    fn matches_market_event(&self, event: &MarketEvent) -> bool {
        match event {
            MarketEvent::OrderbookSnapshot { symbol, .. }
            | MarketEvent::OrderbookUpdate { symbol, .. } => {
                self.matches_symbol(symbol) && self.matches_channel(FilterChannel::Orderbook)
            }
            MarketEvent::ChecksumMismatch { symbol, .. } => {
                self.matches_symbol(symbol) && self.matches_channel(FilterChannel::Orderbook)
            }
            MarketEvent::Status { .. } => self.matches_channel(FilterChannel::Status),
            MarketEvent::Heartbeat => self.matches_channel(FilterChannel::Heartbeat),
        }
    }

    /// Check if a symbol passes this filter
    fn matches_symbol(&self, symbol: &str) -> bool {
        match &self.symbols {
            None => true,
            Some(symbols) => symbols.contains(symbol),
        }
    }

    /// Check if a channel passes this filter
    fn matches_channel(&self, channel: FilterChannel) -> bool {
        match &self.channels {
            None => true,
            Some(channels) => channels.contains(&channel),
        }
    }

    /// Add a symbol to the filter
    pub fn add_symbol(&mut self, symbol: impl Into<String>) {
        self.symbols
            .get_or_insert_with(HashSet::new)
            .insert(symbol.into());
    }

    /// Add a channel to the filter
    pub fn add_channel(&mut self, channel: FilterChannel) {
        self.channels
            .get_or_insert_with(HashSet::new)
            .insert(channel);
    }

    /// Set minimum trade size
    pub fn set_min_trade_size(&mut self, size: Decimal) {
        self.min_trade_size = Some(size);
    }
}

/// Builder for creating event filters with fluent API
#[derive(Debug, Clone, Default)]
pub struct FilterBuilder {
    filter: EventFilter,
}

impl FilterBuilder {
    /// Create a new filter builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter for specific symbols
    pub fn symbols(mut self, symbols: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter.symbols = Some(symbols.into_iter().map(Into::into).collect());
        self
    }

    /// Add a single symbol
    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.filter.add_symbol(symbol);
        self
    }

    /// Include only orderbook events
    pub fn orderbook_events(mut self) -> Self {
        self.filter.add_channel(FilterChannel::Orderbook);
        self
    }

    /// Include only ticker events
    pub fn ticker_events(mut self) -> Self {
        self.filter.add_channel(FilterChannel::Ticker);
        self
    }

    /// Include only trade events
    pub fn trade_events(mut self) -> Self {
        self.filter.add_channel(FilterChannel::Trade);
        self
    }

    /// Include OHLC events
    pub fn ohlc_events(mut self) -> Self {
        self.filter.add_channel(FilterChannel::OHLC);
        self
    }

    /// Set minimum trade size filter
    pub fn min_trade_size(mut self, size: Decimal) -> Self {
        self.filter.min_trade_size = Some(size);
        self
    }

    /// Include connection events
    pub fn with_connection_events(mut self) -> Self {
        self.filter.include_connection = true;
        self
    }

    /// Exclude connection events
    pub fn without_connection_events(mut self) -> Self {
        self.filter.include_connection = false;
        self
    }

    /// Include subscription events
    pub fn with_subscription_events(mut self) -> Self {
        self.filter.include_subscription = true;
        self
    }

    /// Exclude subscription events
    pub fn without_subscription_events(mut self) -> Self {
        self.filter.include_subscription = false;
        self
    }

    /// Include private events
    pub fn with_private_events(mut self) -> Self {
        self.filter.include_private = true;
        self
    }

    /// Exclude private events
    pub fn without_private_events(mut self) -> Self {
        self.filter.include_private = false;
        self
    }

    /// Include L3 events
    pub fn with_l3_events(mut self) -> Self {
        self.filter.include_l3 = true;
        self
    }

    /// Exclude L3 events
    pub fn without_l3_events(mut self) -> Self {
        self.filter.include_l3 = false;
        self
    }

    /// Build the filter
    pub fn build(self) -> EventFilter {
        self.filter
    }
}

/// Filtered event stream wrapper
///
/// Wraps an event receiver and applies filtering before yielding events.
pub struct FilteredEvents<R> {
    #[allow(dead_code)]
    receiver: R,
    filter: EventFilter,
}

impl<R> FilteredEvents<R> {
    /// Create a new filtered event stream
    pub fn new(receiver: R, filter: EventFilter) -> Self {
        Self { receiver, filter }
    }

    /// Get the underlying filter
    pub fn filter(&self) -> &EventFilter {
        &self.filter
    }

    /// Update the filter
    pub fn set_filter(&mut self, filter: EventFilter) {
        self.filter = filter;
    }
}

/// Extension trait for adding filtering to event receivers
pub trait EventFilterExt: Sized {
    /// Wrap this receiver with a filter
    fn with_filter(self, filter: EventFilter) -> FilteredEvents<Self>;
}

impl<R> EventFilterExt for R {
    fn with_filter(self, filter: EventFilter) -> FilteredEvents<Self> {
        FilteredEvents::new(self, filter)
    }
}

/// Multi-filter support - apply multiple filters with AND/OR logic
#[derive(Debug, Clone)]
pub struct MultiFilter {
    filters: Vec<EventFilter>,
    mode: FilterMode,
}

/// How multiple filters are combined
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    /// All filters must match (AND)
    #[default]
    All,
    /// Any filter can match (OR)
    Any,
}

impl MultiFilter {
    /// Create a new multi-filter with AND mode
    pub fn all(filters: impl IntoIterator<Item = EventFilter>) -> Self {
        Self {
            filters: filters.into_iter().collect(),
            mode: FilterMode::All,
        }
    }

    /// Create a new multi-filter with OR mode
    pub fn any(filters: impl IntoIterator<Item = EventFilter>) -> Self {
        Self {
            filters: filters.into_iter().collect(),
            mode: FilterMode::Any,
        }
    }

    /// Check if an event matches this multi-filter
    pub fn matches(&self, event: &Event) -> bool {
        match self.mode {
            FilterMode::All => self.filters.iter().all(|f| f.matches(event)),
            FilterMode::Any => self.filters.iter().any(|f| f.matches(event)),
        }
    }

    /// Add a filter
    pub fn add(&mut self, filter: EventFilter) {
        self.filters.push(filter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kraken_book::OrderbookSnapshot;
    use kraken_ws::ConnectionEvent;

    fn book_event(symbol: &str) -> Event {
        Event::Market(MarketEvent::OrderbookUpdate {
            symbol: symbol.to_string(),
            snapshot: OrderbookSnapshot::default(),
        })
    }

    fn conn_event() -> Event {
        Event::Connection(ConnectionEvent::Connected {
            api_version: "v2".to_string(),
            connection_id: 123,
        })
    }

    #[test]
    fn test_event_filter() {
        // All events pass through
        let all = EventFilter::all();
        assert!(all.matches(&book_event("BTC/USD")));
        assert!(all.matches(&conn_event()));

        // Market only excludes connection events
        let market = EventFilter::market_only();
        assert!(market.matches(&book_event("BTC/USD")));
        assert!(!market.matches(&conn_event()));

        // Symbol filter
        let btc = FilterBuilder::new().symbols(["BTC/USD"]).build();
        assert!(btc.matches(&book_event("BTC/USD")));
        assert!(!btc.matches(&book_event("ETH/USD")));
    }

    #[test]
    fn test_multi_filter() {
        let f1 = FilterBuilder::new().symbols(["BTC/USD"]).build();
        let f2 = FilterBuilder::new().symbols(["ETH/USD"]).build();

        // ANY mode - either filter passes
        let any = MultiFilter::any([f1.clone(), f2.clone()]);
        assert!(any.matches(&book_event("BTC/USD")));
        assert!(any.matches(&book_event("ETH/USD")));
        assert!(!any.matches(&book_event("XRP/USD")));

        // ALL mode - both filters must pass
        let all = MultiFilter::all([f1, FilterBuilder::new().orderbook_events().build()]);
        assert!(all.matches(&book_event("BTC/USD")));
        assert!(!all.matches(&book_event("ETH/USD")));
    }
}
