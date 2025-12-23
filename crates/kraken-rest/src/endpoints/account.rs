//! Private account endpoints
//!
//! These endpoints require authentication.

use crate::auth::{Credentials, RequestSigner};
use crate::error::{RestError, RestResult};
use crate::types::{ApiResponse, BalanceInfo, ExtendedBalance, OpenOrder, TradeHistoryEntry};
use reqwest::Client;
use std::collections::HashMap;
use tracing::{debug, instrument};

const BASE_URL: &str = "https://api.kraken.com";

/// Private account endpoints
pub struct AccountEndpoints<'a> {
    client: &'a Client,
    credentials: &'a Credentials,
}

impl<'a> AccountEndpoints<'a> {
    pub fn new(client: &'a Client, credentials: &'a Credentials) -> Self {
        Self { client, credentials }
    }

    /// Make an authenticated POST request
    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> RestResult<T> {
        let signer = RequestSigner::new(self.credentials, path);
        let nonce = signer.nonce();

        // Build POST data with nonce
        let mut post_params: Vec<(&str, &str)> = vec![("nonce", nonce)];
        post_params.extend_from_slice(params);

        let post_data = serde_urlencoded::to_string(&post_params)
            .map_err(|e| RestError::InvalidParameter(e.to_string()))?;

        let signature = signer.sign(&post_data);
        let url = format!("{}{}", BASE_URL, path);

        debug!("Making authenticated request to {}", path);

        let response: ApiResponse<T> = self
            .client
            .post(&url)
            .header("API-Key", signer.api_key())
            .header("API-Sign", signature)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(post_data)
            .send()
            .await?
            .json()
            .await?;

        response
            .into_result()
            .map_err(RestError::from_api_errors)
    }

    /// Get account balance
    #[instrument(skip(self))]
    pub async fn get_balance(&self) -> RestResult<BalanceInfo> {
        self.post("/0/private/Balance", &[]).await
    }

    /// Get extended balance with hold amounts
    #[instrument(skip(self))]
    pub async fn get_extended_balance(&self) -> RestResult<HashMap<String, ExtendedBalance>> {
        self.post("/0/private/BalanceEx", &[]).await
    }

    /// Get trade balance (margin info)
    ///
    /// # Arguments
    /// * `asset` - Base asset for calculations (default: "ZUSD")
    #[instrument(skip(self))]
    pub async fn get_trade_balance(&self, asset: Option<&str>) -> RestResult<TradeBalance> {
        let params: Vec<(&str, &str)> = if let Some(asset) = asset {
            vec![("asset", asset)]
        } else {
            vec![]
        };

        self.post("/0/private/TradeBalance", &params).await
    }

    /// Get open orders
    ///
    /// # Arguments
    /// * `trades` - Include trade info
    /// * `userref` - Filter by user reference
    #[instrument(skip(self))]
    pub async fn get_open_orders(
        &self,
        trades: Option<bool>,
        userref: Option<i32>,
    ) -> RestResult<OpenOrdersResult> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(trades) = trades {
            params.push(("trades", trades.to_string()));
        }
        if let Some(userref) = userref {
            params.push(("userref", userref.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.post("/0/private/OpenOrders", &params_ref).await
    }

    /// Get closed orders
    ///
    /// # Arguments
    /// * `trades` - Include trade info
    /// * `userref` - Filter by user reference
    /// * `start` - Start timestamp for range
    /// * `end` - End timestamp for range
    /// * `ofs` - Offset for pagination
    /// * `closetime` - Which time to use (open, close, both)
    #[instrument(skip(self))]
    pub async fn get_closed_orders(
        &self,
        trades: Option<bool>,
        userref: Option<i32>,
        start: Option<u64>,
        end: Option<u64>,
        ofs: Option<u32>,
        closetime: Option<&str>,
    ) -> RestResult<ClosedOrdersResult> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(trades) = trades {
            params.push(("trades", trades.to_string()));
        }
        if let Some(userref) = userref {
            params.push(("userref", userref.to_string()));
        }
        if let Some(start) = start {
            params.push(("start", start.to_string()));
        }
        if let Some(end) = end {
            params.push(("end", end.to_string()));
        }
        if let Some(ofs) = ofs {
            params.push(("ofs", ofs.to_string()));
        }
        if let Some(closetime) = closetime {
            params.push(("closetime", closetime.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.post("/0/private/ClosedOrders", &params_ref).await
    }

    /// Query orders by transaction ID
    ///
    /// # Arguments
    /// * `txid` - Transaction IDs (comma-separated or slice)
    /// * `trades` - Include trade info
    /// * `userref` - Filter by user reference
    #[instrument(skip(self))]
    pub async fn query_orders(
        &self,
        txid: &str,
        trades: Option<bool>,
        userref: Option<i32>,
    ) -> RestResult<HashMap<String, OpenOrder>> {
        let mut params: Vec<(&str, String)> = vec![("txid", txid.to_string())];

        if let Some(trades) = trades {
            params.push(("trades", trades.to_string()));
        }
        if let Some(userref) = userref {
            params.push(("userref", userref.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.post("/0/private/QueryOrders", &params_ref).await
    }

    /// Get trade history
    ///
    /// # Arguments
    /// * `trade_type` - Type of trades (all, any, closed, no position, etc.)
    /// * `trades` - Include trade info
    /// * `start` - Start timestamp
    /// * `end` - End timestamp
    /// * `ofs` - Offset for pagination
    #[instrument(skip(self))]
    pub async fn get_trades_history(
        &self,
        trade_type: Option<&str>,
        trades: Option<bool>,
        start: Option<u64>,
        end: Option<u64>,
        ofs: Option<u32>,
    ) -> RestResult<TradesHistoryResult> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(trade_type) = trade_type {
            params.push(("type", trade_type.to_string()));
        }
        if let Some(trades) = trades {
            params.push(("trades", trades.to_string()));
        }
        if let Some(start) = start {
            params.push(("start", start.to_string()));
        }
        if let Some(end) = end {
            params.push(("end", end.to_string()));
        }
        if let Some(ofs) = ofs {
            params.push(("ofs", ofs.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.post("/0/private/TradesHistory", &params_ref).await
    }

    /// Query specific trades
    ///
    /// # Arguments
    /// * `txid` - Trade transaction IDs
    /// * `trades` - Include trade info
    #[instrument(skip(self))]
    pub async fn query_trades(
        &self,
        txid: &str,
        trades: Option<bool>,
    ) -> RestResult<HashMap<String, TradeHistoryEntry>> {
        let mut params: Vec<(&str, String)> = vec![("txid", txid.to_string())];

        if let Some(trades) = trades {
            params.push(("trades", trades.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.post("/0/private/QueryTrades", &params_ref).await
    }

    /// Get open positions
    ///
    /// # Arguments
    /// * `txid` - Position transaction IDs (optional)
    /// * `docalcs` - Include profit/loss calculations
    /// * `consolidation` - Consolidation mode
    #[instrument(skip(self))]
    pub async fn get_open_positions(
        &self,
        txid: Option<&str>,
        docalcs: Option<bool>,
        consolidation: Option<&str>,
    ) -> RestResult<HashMap<String, OpenPosition>> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(txid) = txid {
            params.push(("txid", txid.to_string()));
        }
        if let Some(docalcs) = docalcs {
            params.push(("docalcs", docalcs.to_string()));
        }
        if let Some(consolidation) = consolidation {
            params.push(("consolidation", consolidation.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.post("/0/private/OpenPositions", &params_ref).await
    }

    /// Get ledgers
    ///
    /// # Arguments
    /// * `asset` - Filter by asset
    /// * `aclass` - Asset class
    /// * `ledger_type` - Type (all, deposit, withdrawal, trade, margin)
    /// * `start` - Start timestamp
    /// * `end` - End timestamp
    /// * `ofs` - Offset for pagination
    #[instrument(skip(self))]
    pub async fn get_ledgers(
        &self,
        asset: Option<&str>,
        aclass: Option<&str>,
        ledger_type: Option<&str>,
        start: Option<u64>,
        end: Option<u64>,
        ofs: Option<u32>,
    ) -> RestResult<LedgersResult> {
        let mut params: Vec<(&str, String)> = Vec::new();

        if let Some(asset) = asset {
            params.push(("asset", asset.to_string()));
        }
        if let Some(aclass) = aclass {
            params.push(("aclass", aclass.to_string()));
        }
        if let Some(ledger_type) = ledger_type {
            params.push(("type", ledger_type.to_string()));
        }
        if let Some(start) = start {
            params.push(("start", start.to_string()));
        }
        if let Some(end) = end {
            params.push(("end", end.to_string()));
        }
        if let Some(ofs) = ofs {
            params.push(("ofs", ofs.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.post("/0/private/Ledgers", &params_ref).await
    }

    /// Get trade volume
    ///
    /// # Arguments
    /// * `pair` - Trading pair for fee info
    #[instrument(skip(self))]
    pub async fn get_trade_volume(&self, pair: Option<&str>) -> RestResult<TradeVolume> {
        let params: Vec<(&str, &str)> = if let Some(pair) = pair {
            vec![("pair", pair)]
        } else {
            vec![]
        };

        self.post("/0/private/TradeVolume", &params).await
    }

    /// Request export report
    ///
    /// # Arguments
    /// * `report` - Report type (trades, ledgers)
    /// * `description` - Report description
    /// * `format` - Export format (CSV, TSV)
    /// * `fields` - Fields to include
    /// * `starttm` - Start timestamp
    /// * `endtm` - End timestamp
    #[instrument(skip(self))]
    pub async fn request_export_report(
        &self,
        report: &str,
        description: &str,
        format: Option<&str>,
        starttm: Option<u64>,
        endtm: Option<u64>,
    ) -> RestResult<ExportReportResponse> {
        let mut params: Vec<(&str, String)> = vec![
            ("report", report.to_string()),
            ("description", description.to_string()),
        ];

        if let Some(format) = format {
            params.push(("format", format.to_string()));
        }
        if let Some(starttm) = starttm {
            params.push(("starttm", starttm.to_string()));
        }
        if let Some(endtm) = endtm {
            params.push(("endtm", endtm.to_string()));
        }

        let params_ref: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.post("/0/private/AddExport", &params_ref).await
    }
}

// Response types specific to account endpoints

use serde::Deserialize;

/// Trade balance (margin info)
#[derive(Debug, Clone, Deserialize)]
pub struct TradeBalance {
    /// Equivalent balance (base currency)
    pub eb: String,
    /// Trade balance
    pub tb: String,
    /// Margin amount of open positions
    pub m: Option<String>,
    /// Unrealized P&L of open positions
    pub n: Option<String>,
    /// Cost basis of open positions
    pub c: Option<String>,
    /// Current floating valuation
    pub v: Option<String>,
    /// Equity
    pub e: Option<String>,
    /// Free margin
    pub mf: Option<String>,
    /// Margin level
    pub ml: Option<String>,
}

/// Open orders result
#[derive(Debug, Clone, Deserialize)]
pub struct OpenOrdersResult {
    /// Open orders keyed by transaction ID
    pub open: HashMap<String, OpenOrder>,
}

/// Closed orders result
#[derive(Debug, Clone, Deserialize)]
pub struct ClosedOrdersResult {
    /// Closed orders keyed by transaction ID
    pub closed: HashMap<String, OpenOrder>,
    /// Count of total results
    pub count: u32,
}

/// Trades history result
#[derive(Debug, Clone, Deserialize)]
pub struct TradesHistoryResult {
    /// Trades keyed by transaction ID
    pub trades: HashMap<String, TradeHistoryEntry>,
    /// Count of total results
    pub count: u32,
}

/// Open position
#[derive(Debug, Clone, Deserialize)]
pub struct OpenPosition {
    /// Order transaction ID
    pub ordertxid: String,
    /// Position status
    pub posstatus: String,
    /// Pair
    pub pair: String,
    /// Time of position
    pub time: f64,
    /// Type (buy/sell)
    #[serde(rename = "type")]
    pub side: String,
    /// Order type
    pub ordertype: String,
    /// Cost
    pub cost: String,
    /// Fee
    pub fee: String,
    /// Volume
    pub vol: String,
    /// Closed volume
    pub vol_closed: String,
    /// Margin
    pub margin: String,
    /// Current value
    pub value: Option<String>,
    /// Unrealized P&L
    pub net: Option<String>,
    /// Terms
    pub terms: Option<String>,
    /// Roll over cost
    pub rollovertm: Option<String>,
    /// Miscellaneous
    pub misc: String,
    /// Order flags
    pub oflags: String,
}

/// Ledgers result
#[derive(Debug, Clone, Deserialize)]
pub struct LedgersResult {
    /// Ledger entries keyed by ID
    pub ledger: HashMap<String, LedgerEntry>,
    /// Count of total results
    pub count: u32,
}

/// Ledger entry
#[derive(Debug, Clone, Deserialize)]
pub struct LedgerEntry {
    /// Reference ID
    pub refid: String,
    /// Time
    pub time: f64,
    /// Type
    #[serde(rename = "type")]
    pub entry_type: String,
    /// Sub-type
    pub subtype: Option<String>,
    /// Asset class
    pub aclass: String,
    /// Asset
    pub asset: String,
    /// Amount
    pub amount: String,
    /// Fee
    pub fee: String,
    /// Balance after
    pub balance: String,
}

/// Trade volume info
#[derive(Debug, Clone, Deserialize)]
pub struct TradeVolume {
    /// Currency for volume
    pub currency: String,
    /// Current 30-day volume
    pub volume: String,
    /// Fee tier info per pair
    pub fees: Option<HashMap<String, FeeInfo>>,
    /// Maker fee tier info per pair
    pub fees_maker: Option<HashMap<String, FeeInfo>>,
}

/// Fee tier info
#[derive(Debug, Clone, Deserialize)]
pub struct FeeInfo {
    /// Current fee
    pub fee: String,
    /// Minimum fee
    pub minfee: String,
    /// Maximum fee
    pub maxfee: String,
    /// Next tier volume
    pub nextvolume: Option<String>,
    /// Next tier fee
    pub nextfee: Option<String>,
    /// Tier volume
    pub tiervolume: Option<String>,
}

/// Export report response
#[derive(Debug, Clone, Deserialize)]
pub struct ExportReportResponse {
    /// Report ID
    pub id: String,
}
