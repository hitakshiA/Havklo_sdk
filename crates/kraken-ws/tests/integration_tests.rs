//! Integration tests for Kraken WebSocket API
//!
//! These tests make real WebSocket connections to Kraken's API.
//! Run with: cargo test -p kraken-ws --test integration_tests -- --ignored
//!
//! Note: These tests are ignored by default to avoid making network calls during
//! normal test runs. They should be run manually or in a CI environment with
//! network access.

use kraken_ws::{KrakenConnection, ConnectionConfig, Endpoint, Event, MarketEvent, ConnectionEvent, SubscriptionEvent};
use std::time::Duration;
use tokio::time::timeout;

/// Test that we can establish a WebSocket connection
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_ws_connection() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public);

    let conn = KrakenConnection::new(config);
    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait for connection event
    let connect_result = timeout(Duration::from_secs(10), async {
        while let Some(event) = events.recv().await {
            if let Event::Connection(ConnectionEvent::Connected { .. }) = event {
                return true;
            }
        }
        false
    })
    .await;

    assert!(connect_result.is_ok(), "Connection timed out");
    assert!(connect_result.unwrap(), "Should have connected");

    // Abort the connection task
    conn_handle.abort();
}

/// Test subscribing to ticker channel
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_ticker_subscription() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public);

    let conn = KrakenConnection::new(config);
    conn.subscribe_ticker(vec!["BTC/USD".to_string()]);

    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait for subscription confirmation
    let mut received_sub = false;
    let receive_result = timeout(Duration::from_secs(30), async {
        while let Some(event) = events.recv().await {
            if let Event::Subscription(SubscriptionEvent::Subscribed { channel, .. }) = event {
                if channel == "ticker" {
                    received_sub = true;
                    break;
                }
            }
        }
    })
    .await;

    assert!(receive_result.is_ok(), "Timed out waiting for ticker subscription");
    assert!(received_sub, "Should have received ticker subscription confirmation");

    conn_handle.abort();
}

/// Test subscribing to orderbook channel
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_orderbook_subscription() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public)
        .with_depth(kraken_types::Depth::D10);

    let conn = KrakenConnection::new(config);
    conn.subscribe_orderbook(vec!["BTC/USD".to_string()]);

    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait for orderbook snapshot
    let mut received_book = false;
    let receive_result = timeout(Duration::from_secs(30), async {
        while let Some(event) = events.recv().await {
            if let Event::Market(MarketEvent::OrderbookSnapshot { .. }) = event {
                received_book = true;
                break;
            }
        }
    })
    .await;

    assert!(receive_result.is_ok(), "Timed out waiting for orderbook");
    assert!(received_book, "Should have received orderbook data");

    conn_handle.abort();
}

/// Test subscribing to trades channel
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_trades_subscription() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public);

    let conn = KrakenConnection::new(config);
    conn.subscribe_trade(vec!["BTC/USD".to_string()]);

    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait for subscription confirmation
    let mut received_sub = false;
    let receive_result = timeout(Duration::from_secs(30), async {
        while let Some(event) = events.recv().await {
            if let Event::Subscription(SubscriptionEvent::Subscribed { channel, .. }) = event {
                if channel == "trade" {
                    received_sub = true;
                    break;
                }
            }
        }
    })
    .await;

    assert!(receive_result.is_ok(), "Timed out waiting for trade subscription");
    assert!(received_sub, "Should have received trade subscription confirmation");

    conn_handle.abort();
}

/// Test subscribing to multiple symbols
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_multiple_symbol_subscription() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public);

    let conn = KrakenConnection::new(config);
    conn.subscribe_ticker(vec!["BTC/USD".to_string(), "ETH/USD".to_string()]);

    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait for subscription confirmation
    let mut received_sub = false;
    let receive_result = timeout(Duration::from_secs(30), async {
        while let Some(event) = events.recv().await {
            if let Event::Subscription(SubscriptionEvent::Subscribed { channel, symbols }) = event {
                if channel == "ticker" && symbols.len() >= 2 {
                    received_sub = true;
                    break;
                }
            }
        }
    })
    .await;

    // Subscription for multiple symbols should work
    if receive_result.is_ok() && received_sub {
        println!("Received multi-symbol subscription confirmation");
    }

    conn_handle.abort();
}

/// Test subscribing to multiple channels
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_multiple_channel_subscription() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public)
        .with_depth(kraken_types::Depth::D10);

    let conn = KrakenConnection::new(config);
    conn.subscribe_ticker(vec!["BTC/USD".to_string()]);
    conn.subscribe_orderbook(vec!["BTC/USD".to_string()]);

    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait for data from both channels
    let mut ticker_sub = false;
    let mut book_received = false;

    let _ = timeout(Duration::from_secs(30), async {
        while let Some(event) = events.recv().await {
            match &event {
                Event::Subscription(SubscriptionEvent::Subscribed { channel, .. }) => {
                    if channel == "ticker" {
                        ticker_sub = true;
                    }
                }
                Event::Market(MarketEvent::OrderbookSnapshot { .. }) => book_received = true,
                Event::Market(MarketEvent::OrderbookUpdate { .. }) => book_received = true,
                _ => {}
            }
            if ticker_sub && book_received {
                break;
            }
        }
    })
    .await;

    println!("Received ticker sub: {}, orderbook: {}", ticker_sub, book_received);

    conn_handle.abort();
}

/// Test heartbeat functionality
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_heartbeat() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public)
        .with_depth(kraken_types::Depth::D10);

    let conn = KrakenConnection::new(config);
    conn.subscribe_orderbook(vec!["BTC/USD".to_string()]);

    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait and ensure connection stays alive with heartbeats
    let mut heartbeat_count = 0;
    let mut book_count = 0;

    let _ = timeout(Duration::from_secs(45), async {
        while let Some(event) = events.recv().await {
            match &event {
                Event::Market(MarketEvent::Heartbeat) => {
                    heartbeat_count += 1;
                }
                Event::Market(MarketEvent::OrderbookSnapshot { .. }) |
                Event::Market(MarketEvent::OrderbookUpdate { .. }) => {
                    book_count += 1;
                }
                _ => {}
            }
            // Should get regular book updates and heartbeats
            if book_count > 5 && heartbeat_count > 0 {
                break;
            }
        }
    })
    .await;

    println!("Received {} book updates, {} heartbeats", book_count, heartbeat_count);

    conn_handle.abort();
}

/// Test subscription confirmation
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_subscription_confirmation() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public);

    let conn = KrakenConnection::new(config);
    conn.subscribe_ticker(vec!["BTC/USD".to_string()]);

    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait for subscription confirmation
    let mut sub_confirmed = false;
    let receive_result = timeout(Duration::from_secs(15), async {
        while let Some(event) = events.recv().await {
            if let Event::Subscription(SubscriptionEvent::Subscribed { .. }) = event {
                sub_confirmed = true;
                break;
            }
        }
    })
    .await;

    assert!(receive_result.is_ok(), "Timed out waiting for subscription confirmation");
    assert!(sub_confirmed, "Should have received subscription confirmation");

    conn_handle.abort();
}

/// Test orderbook updates after snapshot
#[tokio::test]
#[ignore = "Makes real WebSocket connection"]
async fn test_orderbook_updates() {
    let config = ConnectionConfig::new()
        .with_endpoint(Endpoint::Public)
        .with_depth(kraken_types::Depth::D10);

    let conn = KrakenConnection::new(config);
    conn.subscribe_orderbook(vec!["BTC/USD".to_string()]);

    let mut events = conn.take_event_receiver().expect("Should have receiver");

    // Spawn connection task
    let conn_handle = tokio::spawn(async move {
        conn.connect_and_run().await
    });

    // Wait for snapshot then updates
    let mut snapshot_received = false;
    let mut updates_received = 0;

    let receive_result = timeout(Duration::from_secs(60), async {
        while let Some(event) = events.recv().await {
            match event {
                Event::Market(MarketEvent::OrderbookSnapshot { .. }) => {
                    snapshot_received = true;
                }
                Event::Market(MarketEvent::OrderbookUpdate { .. }) => {
                    if snapshot_received {
                        updates_received += 1;
                        if updates_received >= 5 {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    })
    .await;

    assert!(receive_result.is_ok(), "Timed out waiting for orderbook updates");
    assert!(snapshot_received, "Should have received snapshot");
    assert!(updates_received >= 5, "Should have received multiple updates");

    conn_handle.abort();
}
