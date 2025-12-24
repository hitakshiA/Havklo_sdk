//! Example: Observability Hooks for Connection Monitoring
//!
//! This example demonstrates setting up the Hooks API to monitor connection
//! lifecycle events. The hooks are automatically invoked by the SDK during
//! actual WebSocket operations.
//!
//! Use cases:
//! - Logging connection events to external systems
//! - Metrics collection (Prometheus, StatsD, etc.)
//! - Alerting on disconnections or errors
//! - Debugging connection issues
//!
//! Run with: cargo run --example observability_hooks

use kraken_ws::hooks::Hooks;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn main() {
    println!("=== Observability Hooks Example ===\n");
    println!("Demonstrating connection lifecycle monitoring hooks\n");

    // Create metrics counters (in real app, these would be Prometheus gauges/counters)
    let connect_count = Arc::new(AtomicU64::new(0));
    let disconnect_count = Arc::new(AtomicU64::new(0));
    let message_bytes = Arc::new(AtomicU64::new(0));
    let error_count = Arc::new(AtomicU64::new(0));
    let checksum_failures = Arc::new(AtomicU64::new(0));
    let reconnect_attempts = Arc::new(AtomicU64::new(0));

    // Clone for use in hooks
    let connect_count_clone = connect_count.clone();
    let disconnect_count_clone = disconnect_count.clone();
    let message_bytes_clone = message_bytes.clone();
    let error_count_clone = error_count.clone();
    let checksum_failures_clone = checksum_failures.clone();
    let reconnect_attempts_clone = reconnect_attempts.clone();

    // Build hooks with callbacks
    let hooks = Hooks::new()
        // Connection established
        .on_connect(move |info| {
            connect_count_clone.fetch_add(1, Ordering::Relaxed);
            let status = if info.is_reconnection { "RECONNECTED" } else { "CONNECTED" };
            println!(
                "[HOOK] {} - API: {}, Connection ID: {}",
                status, info.api_version, info.connection_id
            );
        })
        // Connection lost
        .on_disconnect(move |info| {
            disconnect_count_clone.fetch_add(1, Ordering::Relaxed);
            println!("[HOOK] DISCONNECTED - {:?}", info);
        })
        // Reconnection attempt
        .on_reconnect_attempt(move |attempt, delay| {
            reconnect_attempts_clone.fetch_add(1, Ordering::Relaxed);
            println!(
                "[HOOK] RECONNECTING - Attempt {}, waiting {:?}",
                attempt, delay
            );
        })
        // Subscription status
        .on_subscription(move |info| {
            let status = if info.accepted { "SUBSCRIBED" } else { "REJECTED" };
            println!(
                "[HOOK] {} - Channel: {}, Symbols: {:?}",
                status, info.channel, info.symbols
            );
        })
        // Checksum mismatch
        .on_checksum_mismatch(move |info| {
            checksum_failures_clone.fetch_add(1, Ordering::Relaxed);
            println!(
                "[HOOK] CHECKSUM MISMATCH - {}: expected {}, got {}",
                info.symbol, info.expected, info.computed
            );
        })
        // Message received (for bandwidth monitoring)
        .on_message(move |size| {
            message_bytes_clone.fetch_add(size as u64, Ordering::Relaxed);
        })
        // Error occurred
        .on_error(move |msg| {
            error_count_clone.fetch_add(1, Ordering::Relaxed);
            println!("[HOOK] ERROR - {}", msg);
        });

    // Display configured hooks
    println!("Configured Hooks:");
    println!("{:#?}\n", hooks);

    println!("--- Hook Callbacks ---\n");
    println!("on_connect:           Track connection events");
    println!("on_disconnect:        Track disconnection reasons");
    println!("on_reconnect_attempt: Monitor reconnection attempts");
    println!("on_subscription:      Track subscription status");
    println!("on_checksum_mismatch: Alert on data integrity issues");
    println!("on_message:           Track bandwidth usage");
    println!("on_error:             Collect error metrics");

    println!("\n--- Integration Example ---\n");
    println!("To use hooks with the SDK:");
    println!();
    println!("```rust");
    println!("use kraken_ws::{{ConnectionConfig, Hooks}};");
    println!();
    println!("let hooks = Hooks::new()");
    println!("    .on_connect(|info| {{");
    println!("        metrics::counter!(\"kraken.connections\").increment(1);");
    println!("        info!(\"Connected to Kraken API {{}}\", info.api_version);");
    println!("    }})");
    println!("    .on_disconnect(|reason| {{");
    println!("        warn!(\"Disconnected: {{:?}}\", reason);");
    println!("    }})");
    println!("    .on_error(|msg| {{");
    println!("        error!(\"Kraken error: {{}}\", msg);");
    println!("    }});");
    println!();
    println!("// Hooks are passed to ConnectionConfig");
    println!("// (integration with builder coming in future release)");
    println!("```");

    println!("\n--- Metrics Example with Prometheus ---\n");
    println!("```rust");
    println!("use prometheus::{{Counter, Gauge}};");
    println!();
    println!("lazy_static! {{");
    println!("    static ref CONNECTIONS: Counter = Counter::new(");
    println!("        \"kraken_connections_total\",");
    println!("        \"Total connections established\"");
    println!("    ).unwrap();");
    println!("    static ref BYTES_RECEIVED: Counter = Counter::new(");
    println!("        \"kraken_bytes_received_total\",");
    println!("        \"Total bytes received\"");
    println!("    ).unwrap();");
    println!("}}");
    println!();
    println!("let hooks = Hooks::new()");
    println!("    .on_connect(|_| CONNECTIONS.inc())");
    println!("    .on_message(|bytes| BYTES_RECEIVED.inc_by(bytes as f64));");
    println!("```");

    // Show current (simulated) metrics
    println!("\n--- Metrics Summary ---\n");
    println!("Connections:        {}", connect_count.load(Ordering::Relaxed));
    println!("Disconnections:     {}", disconnect_count.load(Ordering::Relaxed));
    println!("Reconnect Attempts: {}", reconnect_attempts.load(Ordering::Relaxed));
    println!("Messages Received:  {} bytes", message_bytes.load(Ordering::Relaxed));
    println!("Checksum Failures:  {}", checksum_failures.load(Ordering::Relaxed));
    println!("Errors:             {}", error_count.load(Ordering::Relaxed));

    println!("\n=== Example Complete ===");
}
