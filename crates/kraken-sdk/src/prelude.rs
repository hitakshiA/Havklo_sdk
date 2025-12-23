//! Re-exports for convenience
//!
//! Import everything you need with:
//! ```
//! use kraken_sdk::prelude::*;
//! ```

// Client
pub use crate::client::KrakenClient;
pub use crate::builder::{KrakenClientBuilder, ConfigError, OhlcInterval};

// Types from kraken-types
pub use kraken_types::{
    Channel, Depth, KrakenError, Level, Side, Symbol,
    BookData, SubscribeParams, SubscribeRequest,
    // Trading types
    AddOrderRequest, AddOrderParams, AmendOrderRequest, AmendOrderParams,
    CancelOrderRequest, CancelOrderParams, CancelAllRequest,
    BatchAddRequest, BatchCancelRequest, TimeInForce,
    // L3 types
    L3Data, L3Order, L3EventType,
};

// WebSocket types
pub use kraken_ws::{
    ConnectionConfig, ConnectionState, Endpoint, Event,
    ConnectionEvent, MarketEvent, SubscriptionEvent,
    ReconnectConfig,
    // Private channel events
    PrivateEvent, OrderStatus, TrackedOrder, OrderFill, ExecutionType, OrderChange, BalanceInfo,
    // L3 events
    L3Event,
    // Trading client
    TradingClient,
};

// Orderbook types
pub use kraken_book::{
    Orderbook, OrderbookSnapshot, OrderbookState,
    ApplyResult, ChecksumMismatch,
    // L3 orderbook
    L3Book,
};

// Market state types
pub use crate::market::{
    MarketState, Spread, BBO, BookImbalance, ImbalanceSignal, TradeRecord,
};

// Event filtering
pub use crate::filter::{
    EventFilter, FilterBuilder, FilterChannel, FilterMode, MultiFilter,
    FilteredEvents, EventFilterExt,
};

// Decimal for prices/quantities
pub use rust_decimal::Decimal;
