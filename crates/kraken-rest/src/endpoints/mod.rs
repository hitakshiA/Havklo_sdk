//! API endpoint implementations

pub mod market;
pub mod account;
pub mod trading;
pub mod funding;
pub mod earn;

pub use market::MarketEndpoints;
pub use account::AccountEndpoints;
pub use trading::TradingEndpoints;
pub use funding::FundingEndpoints;
pub use earn::EarnEndpoints;
