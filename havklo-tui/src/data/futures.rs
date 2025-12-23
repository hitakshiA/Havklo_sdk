//! Futures market data handler

#![allow(dead_code)]

use kraken_futures_ws::{FuturesConfig, FuturesConnection};
use anyhow::Result;

pub struct FuturesDataHandler {
    conn: Option<FuturesConnection>,
    products: Vec<String>,
}

impl FuturesDataHandler {
    pub fn new(products: Vec<String>) -> Self {
        Self {
            conn: None,
            products,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let config = FuturesConfig::new()
            .with_products(self.products.clone());

        let conn = FuturesConnection::new(config);
        self.conn = Some(conn);
        Ok(())
    }

    pub fn connection(&self) -> Option<&FuturesConnection> {
        self.conn.as_ref()
    }
}
