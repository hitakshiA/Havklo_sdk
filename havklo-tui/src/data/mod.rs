//! Data handling for WebSocket connections
//!
//! This module manages the connection to Kraken's WebSocket APIs
//! and provides data to the UI layer.

pub mod spot;
pub mod futures;

#[allow(unused_imports)]
pub use spot::SpotDataHandler;
#[allow(unused_imports)]
pub use futures::FuturesDataHandler;
