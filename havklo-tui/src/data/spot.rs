//! Spot market data handler

#![allow(dead_code)]

use kraken_sdk::prelude::*;
use anyhow::Result;

pub struct SpotDataHandler {
    client: Option<KrakenClient>,
    symbols: Vec<String>,
}

impl SpotDataHandler {
    pub fn new(symbols: Vec<String>) -> Self {
        Self {
            client: None,
            symbols,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let client = KrakenClient::builder(&self.symbols)
            .with_depth(Depth::D25)
            .with_book(true)
            .connect()
            .await?;

        self.client = Some(client);
        Ok(())
    }

    pub fn client(&self) -> Option<&KrakenClient> {
        self.client.as_ref()
    }
}
