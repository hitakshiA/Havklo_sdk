//! Public market data endpoints
//!
//! These endpoints don't require authentication.

use crate::error::{RestError, RestResult};
use crate::types::{ApiResponse, AssetPairInfo, OrderbookData, TickerInfo};
use reqwest::Client;
use std::collections::HashMap;
use tracing::{debug, instrument};

const BASE_URL: &str = "https://api.kraken.com";

/// Public market data endpoints
pub struct MarketEndpoints<'a> {
    client: &'a Client,
}

impl<'a> MarketEndpoints<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self { client }
    }

    /// Get server time
    #[instrument(skip(self))]
    pub async fn get_server_time(&self) -> RestResult<ServerTime> {
        let url = format!("{}/0/public/Time", BASE_URL);
        debug!("Fetching server time");

        let response: ApiResponse<ServerTime> = self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get system status
    #[instrument(skip(self))]
    pub async fn get_system_status(&self) -> RestResult<SystemStatus> {
        let url = format!("{}/0/public/SystemStatus", BASE_URL);
        debug!("Fetching system status");

        let response: ApiResponse<SystemStatus> = self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get asset info
    ///
    /// # Arguments
    /// * `assets` - Optional list of assets to get info for (e.g., ["XBT", "ETH"])
    #[instrument(skip(self))]
    pub async fn get_assets(&self, assets: Option<&[&str]>) -> RestResult<HashMap<String, AssetInfo>> {
        let mut url = format!("{}/0/public/Assets", BASE_URL);

        if let Some(assets) = assets {
            url.push_str(&format!("?asset={}", assets.join(",")));
        }

        debug!("Fetching asset info");

        let response: ApiResponse<HashMap<String, AssetInfo>> =
            self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get tradable asset pairs
    ///
    /// # Arguments
    /// * `pairs` - Optional list of pairs to get info for (e.g., ["XBTUSD", "ETHUSD"])
    #[instrument(skip(self))]
    pub async fn get_asset_pairs(
        &self,
        pairs: Option<&[&str]>,
    ) -> RestResult<HashMap<String, AssetPairInfo>> {
        let mut url = format!("{}/0/public/AssetPairs", BASE_URL);

        if let Some(pairs) = pairs {
            url.push_str(&format!("?pair={}", pairs.join(",")));
        }

        debug!("Fetching asset pairs");

        let response: ApiResponse<HashMap<String, AssetPairInfo>> =
            self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get ticker information
    ///
    /// # Arguments
    /// * `pair` - Trading pair (e.g., "XBTUSD")
    #[instrument(skip(self))]
    pub async fn get_ticker(&self, pair: &str) -> RestResult<HashMap<String, TickerInfo>> {
        let url = format!("{}/0/public/Ticker?pair={}", BASE_URL, pair);
        debug!("Fetching ticker for {}", pair);

        let response: ApiResponse<HashMap<String, TickerInfo>> =
            self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get ticker information for multiple pairs
    #[instrument(skip(self))]
    pub async fn get_tickers(&self, pairs: &[&str]) -> RestResult<HashMap<String, TickerInfo>> {
        let url = format!("{}/0/public/Ticker?pair={}", BASE_URL, pairs.join(","));
        debug!("Fetching tickers for {} pairs", pairs.len());

        let response: ApiResponse<HashMap<String, TickerInfo>> =
            self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get orderbook depth
    ///
    /// # Arguments
    /// * `pair` - Trading pair (e.g., "XBTUSD")
    /// * `count` - Maximum number of asks/bids (1-500, default 100)
    #[instrument(skip(self))]
    pub async fn get_orderbook(
        &self,
        pair: &str,
        count: Option<u16>,
    ) -> RestResult<HashMap<String, OrderbookData>> {
        let mut url = format!("{}/0/public/Depth?pair={}", BASE_URL, pair);

        if let Some(count) = count {
            url.push_str(&format!("&count={}", count.min(500)));
        }

        debug!("Fetching orderbook for {}", pair);

        let response: ApiResponse<HashMap<String, OrderbookData>> =
            self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get recent trades
    ///
    /// # Arguments
    /// * `pair` - Trading pair (e.g., "XBTUSD")
    /// * `since` - Return trades since this timestamp (optional)
    /// * `count` - Number of trades to return (optional, max 1000)
    #[instrument(skip(self))]
    pub async fn get_recent_trades(
        &self,
        pair: &str,
        since: Option<u64>,
        count: Option<u16>,
    ) -> RestResult<RecentTradesResult> {
        let mut url = format!("{}/0/public/Trades?pair={}", BASE_URL, pair);

        if let Some(since) = since {
            url.push_str(&format!("&since={}", since));
        }
        if let Some(count) = count {
            url.push_str(&format!("&count={}", count.min(1000)));
        }

        debug!("Fetching recent trades for {}", pair);

        let response: ApiResponse<RecentTradesResult> =
            self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get recent spread data
    ///
    /// # Arguments
    /// * `pair` - Trading pair (e.g., "XBTUSD")
    /// * `since` - Return spreads since this timestamp (optional)
    #[instrument(skip(self))]
    pub async fn get_recent_spreads(
        &self,
        pair: &str,
        since: Option<u64>,
    ) -> RestResult<RecentSpreadsResult> {
        let mut url = format!("{}/0/public/Spread?pair={}", BASE_URL, pair);

        if let Some(since) = since {
            url.push_str(&format!("&since={}", since));
        }

        debug!("Fetching recent spreads for {}", pair);

        let response: ApiResponse<RecentSpreadsResult> =
            self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get OHLC data
    ///
    /// # Arguments
    /// * `pair` - Trading pair (e.g., "XBTUSD")
    /// * `interval` - Time frame interval in minutes (1, 5, 15, 30, 60, 240, 1440, 10080, 21600)
    /// * `since` - Return candles since this timestamp (optional)
    #[instrument(skip(self))]
    pub async fn get_ohlc(
        &self,
        pair: &str,
        interval: Option<u32>,
        since: Option<u64>,
    ) -> RestResult<OhlcResult> {
        let mut url = format!("{}/0/public/OHLC?pair={}", BASE_URL, pair);

        if let Some(interval) = interval {
            url.push_str(&format!("&interval={}", interval));
        }
        if let Some(since) = since {
            url.push_str(&format!("&since={}", since));
        }

        debug!("Fetching OHLC for {}", pair);

        let response: ApiResponse<OhlcResult> = self.client.get(&url).send().await?.json().await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }
}

// Response types specific to market endpoints

use serde::Deserialize;

/// Server time response
#[derive(Debug, Clone, Deserialize)]
pub struct ServerTime {
    /// Unix timestamp
    pub unixtime: u64,
    /// RFC 1123 time string
    pub rfc1123: String,
}

/// System status response
#[derive(Debug, Clone, Deserialize)]
pub struct SystemStatus {
    /// System status (online, maintenance, cancel_only, post_only)
    pub status: String,
    /// Timestamp
    pub timestamp: String,
}

/// Asset information
#[derive(Debug, Clone, Deserialize)]
pub struct AssetInfo {
    /// Asset class
    pub aclass: String,
    /// Alternate name
    pub altname: String,
    /// Decimals
    pub decimals: u32,
    /// Display decimals
    pub display_decimals: u32,
}

/// Recent trades result
#[derive(Debug, Clone, Deserialize)]
pub struct RecentTradesResult {
    /// Trades data (pair -> array of trades)
    #[serde(flatten)]
    pub trades: HashMap<String, Vec<Vec<serde_json::Value>>>,
    /// Last trade ID for pagination
    pub last: Option<String>,
}

/// Recent spreads result
#[derive(Debug, Clone, Deserialize)]
pub struct RecentSpreadsResult {
    /// Spread data (pair -> array of spreads)
    #[serde(flatten)]
    pub spreads: HashMap<String, Vec<Vec<serde_json::Value>>>,
    /// Last timestamp for pagination
    pub last: Option<u64>,
}

/// OHLC result
#[derive(Debug, Clone, Deserialize)]
pub struct OhlcResult {
    /// OHLC data (pair -> array of candles)
    #[serde(flatten)]
    pub ohlc: HashMap<String, Vec<Vec<serde_json::Value>>>,
    /// Last timestamp for pagination
    pub last: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_construction() {
        // Verify URL patterns are correct
        assert!(format!("{}/0/public/Time", BASE_URL).contains("api.kraken.com"));
    }
}
