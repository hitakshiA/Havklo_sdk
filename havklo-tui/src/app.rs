//! Application state and business logic

#![allow(dead_code)]

use anyhow::Result;
use kraken_sdk::prelude::*;
use ratatui::style::Color;
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

/// Beautiful color theme inspired by Bloomberg terminal
pub struct Theme;

impl Theme {
    pub const BG: Color = Color::Rgb(10, 14, 20);           // Deep blue-black
    pub const FG: Color = Color::Rgb(179, 177, 173);        // Warm gray
    pub const ACCENT: Color = Color::Rgb(0, 217, 255);      // Cyan
    pub const BID: Color = Color::Rgb(0, 255, 136);         // Green
    pub const ASK: Color = Color::Rgb(255, 68, 68);         // Red
    pub const HIGHLIGHT: Color = Color::Rgb(255, 215, 0);   // Gold
    pub const MUTED: Color = Color::Rgb(74, 74, 74);        // Dim gray
    pub const BORDER: Color = Color::Rgb(42, 46, 56);       // Subtle border
    pub const SUCCESS: Color = Color::Rgb(0, 255, 136);     // Same as bid
    pub const WARNING: Color = Color::Rgb(255, 200, 0);     // Amber
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Orderbook,
    Dashboard,
    Imbalance,
    Futures,
    Alerts,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Orderbook => "ORDERBOOK",
            Tab::Dashboard => "Dashboard",
            Tab::Imbalance => "Imbalance",
            Tab::Futures => "Futures",
            Tab::Alerts => "Alerts",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[Tab::Orderbook, Tab::Dashboard, Tab::Imbalance, Tab::Futures, Tab::Alerts]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error,
}

#[derive(Debug, Clone)]
pub struct OrderbookData {
    pub bids: Vec<(Decimal, Decimal)>,  // (price, qty)
    pub asks: Vec<(Decimal, Decimal)>,
    pub spread: Option<Decimal>,
    pub mid_price: Option<Decimal>,
    pub checksum_valid: bool,
    pub update_count: u64,
}

impl Default for OrderbookData {
    fn default() -> Self {
        Self {
            bids: Vec::new(),
            asks: Vec::new(),
            spread: None,
            mid_price: None,
            checksum_valid: true,
            update_count: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SymbolData {
    pub symbol: String,
    pub price: Option<Decimal>,
    pub change_pct: f64,
    pub spread: Option<Decimal>,
    pub synced: bool,
    pub price_history: VecDeque<Decimal>,
}

impl SymbolData {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
            price: None,
            change_pct: 0.0,
            spread: None,
            synced: false,
            price_history: VecDeque::with_capacity(30),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuturesData {
    pub product: String,
    pub mark_price: Option<Decimal>,
    pub funding_rate: Option<Decimal>,
    pub annual_rate: Option<Decimal>,
    pub premium: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct Alert {
    pub symbol: String,
    pub condition: String,
    pub triggered: bool,
}

#[derive(Debug, Clone)]
pub struct AlertEvent {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub message: String,
}

pub struct App {
    // UI State
    pub current_tab: Tab,
    pub selected_symbol_idx: usize,
    pub show_splash: bool,
    pub paused: bool,
    pub splash_progress: f64,
    pub frame_count: u64,
    pub fps: f64,

    // Connection
    pub connection_state: ConnectionState,
    pub client: Option<KrakenClient>,
    pub start_time: Instant,
    pub reconnect_count: u32,

    // Data
    pub symbols: Vec<String>,
    pub orderbooks: HashMap<String, OrderbookData>,
    pub symbol_data: HashMap<String, SymbolData>,
    pub futures_data: Vec<FuturesData>,
    pub imbalance: f64,
    pub imbalance_history: VecDeque<f64>,

    // Alerts
    pub alerts: Vec<Alert>,
    pub alert_history: VecDeque<AlertEvent>,

    // Stats
    pub update_count: u64,
    pub updates_per_second: f64,
    last_fps_update: Instant,
    fps_frame_count: u64,
}

impl App {
    pub fn new() -> Self {
        let symbols = vec![
            "BTC/USD".to_string(),
            "ETH/USD".to_string(),
            "SOL/USD".to_string(),
            "XRP/USD".to_string(),
            "DOT/USD".to_string(),
            "LINK/USD".to_string(),
        ];

        let mut symbol_data = HashMap::new();
        for s in &symbols {
            symbol_data.insert(s.clone(), SymbolData::new(s));
        }

        let futures_data = vec![
            FuturesData {
                product: "PI_XBTUSD".to_string(),
                mark_price: None,
                funding_rate: None,
                annual_rate: None,
                premium: None,
            },
            FuturesData {
                product: "PI_ETHUSD".to_string(),
                mark_price: None,
                funding_rate: None,
                annual_rate: None,
                premium: None,
            },
            FuturesData {
                product: "PF_SOLUSD".to_string(),
                mark_price: None,
                funding_rate: None,
                annual_rate: None,
                premium: None,
            },
        ];

        Self {
            current_tab: Tab::Orderbook,
            selected_symbol_idx: 0,
            show_splash: true,
            paused: false,
            splash_progress: 0.0,
            frame_count: 0,
            fps: 60.0,

            connection_state: ConnectionState::Disconnected,
            client: None,
            start_time: Instant::now(),
            reconnect_count: 0,

            symbols,
            orderbooks: HashMap::new(),
            symbol_data,
            futures_data,
            imbalance: 0.0,
            imbalance_history: VecDeque::with_capacity(60),

            alerts: vec![
                Alert {
                    symbol: "BTC/USD".to_string(),
                    condition: "above $100,000".to_string(),
                    triggered: false,
                },
                Alert {
                    symbol: "ETH/USD".to_string(),
                    condition: "below $3,000".to_string(),
                    triggered: false,
                },
            ],
            alert_history: VecDeque::with_capacity(50),

            update_count: 0,
            updates_per_second: 0.0,
            last_fps_update: Instant::now(),
            fps_frame_count: 0,
        }
    }

    pub fn selected_symbol(&self) -> &str {
        &self.symbols[self.selected_symbol_idx]
    }

    pub fn next_tab(&mut self) {
        let tabs = Tab::all();
        let idx = tabs.iter().position(|t| *t == self.current_tab).unwrap_or(0);
        self.current_tab = tabs[(idx + 1) % tabs.len()];
    }

    pub fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let idx = tabs.iter().position(|t| *t == self.current_tab).unwrap_or(0);
        self.current_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
    }

    pub fn next_symbol(&mut self) {
        self.selected_symbol_idx = (self.selected_symbol_idx + 1) % self.symbols.len();
    }

    pub fn prev_symbol(&mut self) {
        self.selected_symbol_idx = (self.selected_symbol_idx + self.symbols.len() - 1) % self.symbols.len();
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn reconnect(&mut self) {
        self.connection_state = ConnectionState::Reconnecting;
        self.reconnect_count += 1;
    }

    pub fn uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn tick(&mut self) {
        self.frame_count += 1;
        self.fps_frame_count += 1;

        // Update FPS every second
        if self.last_fps_update.elapsed() >= std::time::Duration::from_secs(1) {
            self.fps = self.fps_frame_count as f64;
            self.fps_frame_count = 0;
            self.last_fps_update = Instant::now();
        }

        // Update splash progress
        if self.show_splash {
            self.splash_progress = (self.splash_progress + 0.02).min(1.0);
        }

        // Update data from client
        self.update_from_client();
    }

    fn update_from_client(&mut self) {
        let client = match &self.client {
            Some(c) => c,
            None => return,
        };
        for symbol in &self.symbols {
            // Update orderbook data
            if let Some(orderbook) = client.orderbook(symbol) {
                let bids = orderbook.bids_vec();
                let asks = orderbook.asks_vec();

                let ob_data = self.orderbooks.entry(symbol.clone()).or_default();
                ob_data.bids = bids.iter().map(|l| (l.price, l.qty)).collect();
                ob_data.asks = asks.iter().map(|l| (l.price, l.qty)).collect();
                ob_data.spread = client.spread(symbol);
                ob_data.mid_price = client.mid_price(symbol);
                ob_data.checksum_valid = true;
                ob_data.update_count += 1;
            }

            // Update symbol data
            if let Some(data) = self.symbol_data.get_mut(symbol) {
                if let Some(mid) = client.mid_price(symbol) {
                    // Track price history
                    if data.price_history.len() >= 30 {
                        data.price_history.pop_front();
                    }
                    data.price_history.push_back(mid);

                    // Calculate change
                    if let Some(first) = data.price_history.front() {
                        if !first.is_zero() {
                            let change = (mid - *first) / *first;
                            data.change_pct = change.to_string().parse().unwrap_or(0.0) * 100.0;
                        }
                    }

                    data.price = Some(mid);
                }
                data.spread = client.spread(symbol);
                data.synced = client.is_synced(symbol);
            }
        }

        self.update_count += 1;
        self.connection_state = ConnectionState::Connected;
    }

    pub async fn start_connection(&mut self) -> Result<()> {
        self.connection_state = ConnectionState::Connecting;

        let client = KrakenClient::builder(&self.symbols)
            .with_depth(Depth::D25)
            .with_book(true)
            .connect()
            .await?;

        self.client = Some(client);
        self.connection_state = ConnectionState::Connected;
        self.start_time = Instant::now();

        // Log alert
        self.alert_history.push_front(AlertEvent {
            timestamp: chrono::Local::now(),
            message: "Session started - Connected to Kraken".to_string(),
        });

        Ok(())
    }
}
