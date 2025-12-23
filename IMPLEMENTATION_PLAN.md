# Havklo SDK - WebSocket-Only Refactor Plan

## Executive Summary

Refactor the SDK to focus exclusively on WebSocket APIs, removing REST endpoints except for the essential `GetWebSocketsToken` needed for private channel authentication.

---

## Phase 1: Crate Restructuring

### 1.1 Remove kraken-rest (Partial)

**Current state:** Full REST client with Market, Account, Trading, Funding, Earn endpoints

**Target state:** Minimal auth-only module

**Actions:**
- Delete: `endpoints/market.rs`, `endpoints/account.rs`, `endpoints/trading.rs`, `endpoints/funding.rs`, `endpoints/earn.rs`
- Keep: `auth.rs` (for token generation)
- Create: `token.rs` - Single endpoint for `GetWebSocketsToken`
- Update: `lib.rs` to only expose token functionality

**New API:**
```rust
use kraken_auth::{Credentials, TokenProvider};

let creds = Credentials::from_env()?;
let provider = TokenProvider::new(creds);
let token = provider.get_ws_token().await?;
```

### 1.2 Rename Crate

- Rename `kraken-rest` → `kraken-auth`
- Update all workspace references

---

## Phase 2: Spot WebSocket v2 Completion (`kraken-ws`)

### 2.1 Add L3 Orders Channel

**Endpoint:** `wss://ws-l3.kraken.com/v2`

**New files:**
- `src/l3_connection.rs` - Separate connection for L3 endpoint
- `src/channels/l3_orders.rs` - L3 channel handler

**Types needed:**
```rust
pub enum L3Event {
    Add { order_id: String, side: Side, price: Decimal, qty: Decimal },
    Modify { order_id: String, new_qty: Decimal },
    Delete { order_id: String },
}

pub struct L3Subscription {
    symbols: Vec<String>,
    depth: L3Depth,  // 10, 100, 1000
    snapshot: bool,
}
```

**Integration with kraken-book:**
- Feed L3 events directly to `L3Book` from `kraken-book`

### 2.2 Add Trading Methods

**New file:** `src/trading.rs`

**Methods to implement:**

| Method | Request Params | Response |
|--------|---------------|----------|
| `add_order` | order_type, side, pair, price, volume, leverage, etc. | txid, order description |
| `amend_order` | order_id, new params | amended order info |
| `cancel_order` | order_id or cl_ord_id | cancelled count |
| `cancel_all` | - | cancelled count |
| `cancel_on_disconnect` | timeout | status |
| `batch_add` | list of orders | list of results |
| `batch_cancel` | list of order_ids | cancelled count |
| `edit_order` | order_id, new params | new txid |

**Request/Response format:**
```rust
// Request
pub struct AddOrderRequest {
    pub method: &'static str,  // "add_order"
    pub params: AddOrderParams,
    pub req_id: Option<u64>,
}

pub struct AddOrderParams {
    pub order_type: OrderType,
    pub side: Side,
    pub symbol: String,
    pub limit_price: Option<Decimal>,
    pub volume: Decimal,
    pub time_in_force: Option<TimeInForce>,
    pub post_only: Option<bool>,
    pub reduce_only: Option<bool>,
    // ... more params
}

// Response
pub struct AddOrderResult {
    pub order_id: String,
    pub cl_ord_id: Option<String>,
    pub order_userref: Option<i64>,
    pub warning: Option<Vec<String>>,
}
```

### 2.3 Add Admin Methods

**New file:** `src/admin.rs`

```rust
// Ping/Pong for keepalive
pub async fn ping(&self) -> Result<(), WsError>;

// Subscribe to heartbeat
pub async fn subscribe_heartbeat(&self) -> Result<(), WsError>;
```

### 2.4 Update Channel Enum

```rust
pub enum Channel {
    // Existing
    Ticker,
    Book,       // L2
    Trade,
    Ohlc,
    Instrument,
    Executions,
    Balances,
    Status,

    // New
    Level3,     // L3 Orders
    Heartbeat,
}
```

### 2.5 Update Event Enum

```rust
pub enum Event {
    // Existing...

    // New
    L3Update(L3Event),

    // Trading responses
    OrderAdded(AddOrderResult),
    OrderAmended(AmendOrderResult),
    OrderCancelled(CancelResult),
    OrderEdited(EditOrderResult),
    BatchResult(Vec<BatchOrderResult>),

    // Admin
    Pong,
    HeartbeatAck,
}
```

---

## Phase 3: Futures WebSocket Completion (`kraken-futures-ws`)

### 3.1 Add Open Orders Verbose Feed

**New file:** `src/channels/orders_verbose.rs`

```rust
pub struct VerboseOrder {
    pub order_id: String,
    pub cl_ord_id: Option<String>,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub qty: Decimal,
    pub filled_qty: Decimal,
    pub reduce_only: bool,
    pub last_update_time: String,
    pub status: OrderStatus,
    // ... additional fields
}
```

### 3.2 Add Account Log Feed

**New file:** `src/channels/account_log.rs`

```rust
pub struct AccountLogEntry {
    pub id: u64,
    pub timestamp: String,
    pub event_type: AccountEventType,
    pub asset: String,
    pub amount: Decimal,
    pub balance: Decimal,
    pub info: Option<String>,
}

pub enum AccountEventType {
    Trade,
    Deposit,
    Withdrawal,
    Transfer,
    FundingPayment,
    Liquidation,
    // ...
}
```

### 3.3 Update Channel Constants

```rust
pub mod channels {
    // Existing...

    // New
    pub const OPEN_ORDERS_VERBOSE: &str = "open_orders_verbose";
    pub const ACCOUNT_LOG: &str = "account_log";
}
```

---

## Phase 4: SDK Integration (`kraken-sdk`)

### 4.1 Update Dependencies

```toml
[dependencies]
kraken-types = { workspace = true }
kraken-book = { workspace = true }
kraken-ws = { workspace = true }
kraken-futures-ws = { workspace = true }
kraken-auth = { workspace = true }  # Renamed from kraken-rest
```

### 4.2 New Unified API

```rust
use kraken_sdk::prelude::*;

// Spot WebSocket with trading
let client = KrakenClient::builder()
    .with_credentials(Credentials::from_env()?)
    .with_symbols(["BTC/USD", "ETH/USD"])
    .with_depth(Depth::D25)
    .with_l3(true)  // Enable L3 orderbook
    .with_trading(true)  // Enable trading methods
    .connect()
    .await?;

// Place order via WebSocket
let order = client.add_order(AddOrderParams {
    symbol: "BTC/USD",
    side: Side::Buy,
    order_type: OrderType::Limit,
    volume: dec!(0.001),
    limit_price: Some(dec!(50000)),
    ..Default::default()
}).await?;

// Cancel order
client.cancel_order(&order.order_id).await?;

// Futures WebSocket
let futures = FuturesClient::builder()
    .with_credentials(FuturesCredentials::from_env()?)
    .with_products(["PI_XBTUSD"])
    .connect()
    .await?;
```

---

## Phase 5: WASM Updates (`kraken-wasm`)

### 5.1 No Changes Required

The WASM crate only exposes the orderbook engine from `kraken-book`, which doesn't depend on REST. No changes needed.

---

## Phase 6: Documentation Updates

### 6.1 Update README.md

- Remove all REST API references
- Update examples to use WS trading
- Update crate table

### 6.2 Update docs/

- Remove `REST_API.md`
- Create `WS_TRADING.md` - WebSocket trading guide
- Update `QUICKSTART.md` - WS-only examples
- Update `INTEGRATION.md` - Remove REST patterns

### 6.3 Update Examples

Remove/update:
- `rest_trading.rs` → Delete
- Add `ws_trading.rs` - WebSocket trading example
- Add `l3_orderbook.rs` - L3 channel example

---

## File Changes Summary

### Delete
```
crates/kraken-rest/src/endpoints/market.rs
crates/kraken-rest/src/endpoints/account.rs
crates/kraken-rest/src/endpoints/trading.rs
crates/kraken-rest/src/endpoints/funding.rs
crates/kraken-rest/src/endpoints/earn.rs
crates/kraken-rest/src/endpoints/mod.rs
crates/kraken-rest/src/client.rs
crates/kraken-rest/src/rate_limiter.rs
crates/kraken-rest/src/types.rs
crates/kraken-rest/tests/integration_tests.rs
crates/kraken-sdk/examples/rest_trading.rs
docs/REST_API.md
```

### Create
```
crates/kraken-auth/src/lib.rs           # Renamed crate
crates/kraken-auth/src/token.rs         # GetWebSocketsToken endpoint
crates/kraken-ws/src/trading.rs         # WS trading methods
crates/kraken-ws/src/admin.rs           # Ping/heartbeat
crates/kraken-ws/src/channels/l3.rs     # L3 orders channel
crates/kraken-ws/src/l3_connection.rs   # L3 endpoint connection
crates/kraken-futures-ws/src/channels/orders_verbose.rs
crates/kraken-futures-ws/src/channels/account_log.rs
crates/kraken-sdk/examples/ws_trading.rs
crates/kraken-sdk/examples/l3_orderbook.rs
docs/WS_TRADING.md
```

### Modify
```
Cargo.toml                              # Remove kraken-rest, add kraken-auth
crates/kraken-ws/src/lib.rs             # Add trading, L3
crates/kraken-ws/src/events.rs          # Add trading events
crates/kraken-ws/src/subscription.rs    # Add L3 subscription
crates/kraken-futures-ws/src/lib.rs     # Add new channels
crates/kraken-futures-ws/src/channels/mod.rs
crates/kraken-sdk/src/lib.rs            # WS-only API
crates/kraken-sdk/src/client.rs         # Add trading methods
crates/kraken-sdk/Cargo.toml            # Update deps
README.md
docs/QUICKSTART.md
docs/INTEGRATION.md
```

---

## Implementation Order

1. **Phase 1** - Restructure kraken-rest → kraken-auth (keep token only)
2. **Phase 2.1** - Add L3 channel to kraken-ws
3. **Phase 2.2** - Add trading methods to kraken-ws
4. **Phase 3** - Complete kraken-futures-ws feeds
5. **Phase 4** - Update kraken-sdk
6. **Phase 6** - Update documentation
7. **Testing** - Verify all changes

---

## Estimated Scope

| Phase | New Code | Modified | Deleted |
|-------|----------|----------|---------|
| 1 | ~100 lines | ~50 lines | ~2000 lines |
| 2 | ~800 lines | ~200 lines | 0 |
| 3 | ~200 lines | ~50 lines | 0 |
| 4 | ~150 lines | ~300 lines | 0 |
| 6 | ~200 lines | ~400 lines | ~400 lines |

**Net change:** ~1450 new lines, ~1400 deleted = cleaner, focused codebase

---

## API Compatibility After Refactor

| API | Version | Coverage |
|-----|---------|----------|
| Spot WebSocket v2 | Latest | 100% |
| Futures WebSocket | v1 | 100% |
| REST | N/A | Token only |

---

*Plan created: 2024-12-24*
*Status: READY FOR APPROVAL*
