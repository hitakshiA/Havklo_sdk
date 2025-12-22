# Architecture

This document describes the architecture of the Havklo SDK.

## Overview

The SDK is designed as a layered monorepo with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                        kraken-sdk                            │
│                    (High-Level API)                          │
├─────────────────────────────────────────────────────────────┤
│                        kraken-ws                             │
│            (WebSocket Client & Reconnection)                 │
├─────────────────────────────────────────────────────────────┤
│                       kraken-book                            │
│              (Orderbook Engine - WASM OK)                    │
├─────────────────────────────────────────────────────────────┤
│                      kraken-types                            │
│              (Core Types - Minimal Deps)                     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                       kraken-wasm                            │
│               (Browser Bindings - Separate)                  │
└─────────────────────────────────────────────────────────────┘
```

## Crate Responsibilities

### kraken-types

**Purpose:** Core type definitions with minimal dependencies.

**Key Types:**
- `Symbol` - Trading pair validation (e.g., "BTC/USD")
- `Level` - Price level with `rust_decimal::Decimal` precision
- `Channel`, `Depth`, `Side` - Subscription enums
- `WsMessage` - Parsed WebSocket message wrapper
- `KrakenError` - Comprehensive error types with recovery hints

**Design Decisions:**
- No async runtime dependency
- Uses `rust_decimal` to prevent floating-point precision loss
- All errors include context for debugging

### kraken-book

**Purpose:** Stateless orderbook engine with CRC32 checksum validation.

**Key Components:**
- `Orderbook` - State machine for orderbook lifecycle
- `TreeBook` - BTreeMap-based sorted storage
- `checksum` - Kraken v2 API checksum implementation
- `history` - Ring buffer for historical snapshots

**State Machine:**
```
Uninitialized
     │
     ▼ (receive snapshot)
AwaitingSnapshot
     │
     ▼ (checksum valid)
   Synced ◄───────────────┐
     │                    │
     ▼ (checksum invalid) │
Desynchronized ───────────┘
        (receive new snapshot)
```

**Design Decisions:**
- No tokio or async dependencies (WASM-compatible)
- No `std::time::Instant` (panics in WASM)
- Precision stored per-orderbook for correct checksum calculation

### kraken-ws

**Purpose:** WebSocket connection management with auto-reconnection.

**Key Components:**
- `KrakenConnection` - Main WebSocket handler
- `ReconnectConfig` - Exponential backoff with jitter
- `SubscriptionManager` - Tracks active subscriptions
- `Event` - Event types for user consumption

**Reconnection Strategy:**
```
Initial delay: 100ms
Max delay: 30s
Multiplier: 2.0x
Jitter: 20%
Max attempts: Unlimited (configurable)
```

**Design Decisions:**
- Uses tokio-tungstenite for native WebSocket
- DashMap for thread-safe orderbook storage
- Automatic subscription restoration on reconnect

### kraken-sdk

**Purpose:** User-facing high-level API.

**Key Components:**
- `KrakenClient` - Main entry point
- `KrakenClientBuilder` - Fluent configuration
- Event streaming via `mpsc::UnboundedReceiver`

**API Design:**
```rust
let mut client = KrakenClient::builder(["BTC/USD", "ETH/USD"])
    .with_depth(Depth::D25)
    .with_reconnect_config(ReconnectConfig::default())
    .connect()
    .await?;

let mut events = client.events();
while let Some(event) = events.recv().await {
    // Handle events
}
```

### kraken-wasm

**Purpose:** Browser bindings for JavaScript integration.

**Design:**
- Wraps `kraken-book` for orderbook processing
- WebSocket connection handled by browser/JavaScript
- Messages passed through `apply_message(json)`

## Data Flow

### Message Processing

```
Kraken WebSocket
       │
       ▼
┌─────────────────┐
│   kraken-ws    │ Parse JSON
│   connection   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  kraken-types  │ Type validation
│   WsMessage    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  kraken-book   │ Apply to orderbook
│   Orderbook    │ Validate checksum
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  kraken-sdk    │ Emit event
│     Event      │
└────────┬────────┘
         │
         ▼
    User Code
```

### Checksum Validation

1. Receive update with Kraken's checksum
2. Compute checksum from local orderbook (top 10 levels)
3. Compare values
4. On mismatch: transition to `Desynchronized`, emit event, wait for new snapshot

## Thread Safety

- `DashMap<String, Orderbook>` for concurrent symbol access
- `Arc<RwLock<ConnectionState>>` for connection state
- `AtomicBool` for shutdown signaling
- Single-threaded event loop prevents most race conditions

## Error Handling

All errors include:
- `is_retryable()` - Can this operation be retried?
- `retry_after()` - Suggested wait duration
- `requires_reconnect()` - Does the connection need to be re-established?

Categories:
- **Connection**: Network failures, timeouts
- **Protocol**: Invalid messages, checksum mismatches
- **Subscription**: Rejected subscriptions, invalid symbols
- **Authentication**: Token issues (private channels)
- **RateLimit**: Throttling from exchange
- **Internal**: SDK bugs, configuration errors

## Performance Considerations

### Benchmarks (M1 MacBook Pro)

| Operation | Time |
|-----------|------|
| Parse heartbeat | ~72ns |
| Parse 10-level snapshot | ~11.8µs |
| Best bid/ask lookup | ~1.2ns |
| Apply snapshot (10 levels) | ~6.5µs |
| Checksum compute (10 levels) | ~500ns |

### Memory

- Fixed overhead per symbol (~1KB)
- ~100 bytes per price level
- Historical snapshots optional (ring buffer)

### CPU

- Parsing: serde_json with custom deserializers
- Checksum: crc32fast with SIMD acceleration
- Sorting: BTreeMap for O(log n) operations
