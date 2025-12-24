//! Example: Structured Error Handling and Recovery
//!
//! This example demonstrates the SDK's error handling capabilities:
//! - Actionable error types with recovery strategies
//! - Automatic retry logic with backoff
//! - Distinguishing between error categories
//!
//! Run with: cargo run --example error_handling

use kraken_types::{KrakenError, error_codes::RecoveryStrategy};
use std::time::Duration;

fn main() {
    println!("=== Structured Error Handling Example ===\n");

    // Demonstrate different error types and their recovery strategies
    demonstrate_error_categories();

    // Show how to use recovery strategies
    demonstrate_recovery_strategies();

    // Show API error parsing
    demonstrate_api_error_parsing();
}

fn demonstrate_error_categories() {
    println!("--- Error Categories ---\n");

    // Connection errors
    let conn_err = KrakenError::ConnectionFailed {
        url: "wss://ws.kraken.com".to_string(),
        reason: "Connection refused".to_string(),
    };
    print_error_info("Connection", &conn_err);

    // Rate limit errors
    let rate_err = KrakenError::RateLimited {
        retry_after: Duration::from_secs(5),
    };
    print_error_info("Rate Limit", &rate_err);

    // Checksum errors (data integrity)
    let checksum_err = KrakenError::ChecksumMismatch {
        symbol: "BTC/USD".to_string(),
        expected: 12345678,
        computed: 87654321,
    };
    print_error_info("Checksum", &checksum_err);

    // Auth errors
    let auth_err = KrakenError::AuthenticationFailed {
        reason: "Invalid API key".to_string(),
    };
    print_error_info("Auth", &auth_err);

    println!();
}

fn print_error_info(category: &str, error: &KrakenError) {
    println!("{}:", category);
    println!("  Message:           {}", error);
    println!("  Retryable:         {}", error.is_retryable());
    println!("  Requires Reconnect: {}", error.requires_reconnect());
    println!("  Requires Reauth:   {}", error.requires_reauth());
    println!("  Is Rate Limit:     {}", error.is_rate_limit());
    if let Some(delay) = error.retry_after() {
        println!("  Suggested Delay:   {:?}", delay);
    }
    println!();
}

fn demonstrate_recovery_strategies() {
    println!("--- Recovery Strategies ---\n");

    let errors: Vec<(&str, KrakenError)> = vec![
        ("Rate Limited", KrakenError::RateLimited { retry_after: Duration::from_secs(5) }),
        ("Connection Failed", KrakenError::ConnectionFailed {
            url: "wss://ws.kraken.com".to_string(),
            reason: "Network unreachable".to_string(),
        }),
        ("Checksum Mismatch", KrakenError::ChecksumMismatch {
            symbol: "ETH/USD".to_string(),
            expected: 1111,
            computed: 2222,
        }),
        ("Token Expired", KrakenError::TokenExpired),
        ("Channel Closed", KrakenError::ChannelClosed),
    ];

    for (name, error) in errors {
        let strategy = error.recovery_strategy();
        println!("{}: {:?}", name, strategy);

        // Show how to handle each strategy
        match strategy {
            RecoveryStrategy::Backoff { initial_ms, max_ms, multiplier } => {
                println!("  -> Retry with exponential backoff");
                println!("     Initial: {}ms, Max: {}ms, Mult: {}x", initial_ms, max_ms, multiplier);
            }
            RecoveryStrategy::RequestSnapshot => {
                println!("  -> Request fresh orderbook snapshot");
            }
            RecoveryStrategy::Reauthenticate => {
                println!("  -> Refresh authentication token");
            }
            RecoveryStrategy::Skip => {
                println!("  -> Skip this message, continue processing");
            }
            RecoveryStrategy::Fatal => {
                println!("  -> Cannot recover, shutdown gracefully");
            }
            _ => {
                println!("  -> Handle accordingly");
            }
        }
        println!();
    }
}

fn demonstrate_api_error_parsing() {
    println!("--- API Error Parsing ---\n");
    println!("Kraken returns specific error codes that we parse automatically:\n");

    let api_errors = vec![
        "EAPI:Rate limit exceeded",
        "EAPI:Invalid key",
        "EOrder:Insufficient funds",
        "EOrder:Order minimum not met",
        "EGeneral:Invalid arguments",
    ];

    for error_str in api_errors {
        let error = KrakenError::from_api_error(error_str);
        println!("Raw: \"{}\"", error_str);
        println!("  Parsed Code: {:?}", error.error_code());
        println!("  Recovery:    {:?}", error.recovery_strategy());
        println!("  Retryable:   {}", error.is_retryable());
        println!();
    }

    println!("--- Practical Error Handling Pattern ---\n");
    println!("```rust");
    println!("match result {{");
    println!("    Ok(data) => process(data),");
    println!("    Err(e) if e.is_retryable() => {{");
    println!("        let delay = e.retry_after().unwrap_or(Duration::from_secs(1));");
    println!("        tokio::time::sleep(delay).await;");
    println!("        retry();");
    println!("    }}");
    println!("    Err(e) if e.requires_reconnect() => {{");
    println!("        connection.reconnect().await;");
    println!("    }}");
    println!("    Err(e) if e.requires_reauth() => {{");
    println!("        refresh_token().await;");
    println!("    }}");
    println!("    Err(e) => {{");
    println!("        log::error!(\"Unrecoverable: {{}}\", e);");
    println!("        shutdown();");
    println!("    }}");
    println!("}}");
    println!("```");

    println!("\n=== Example Complete ===");
}
