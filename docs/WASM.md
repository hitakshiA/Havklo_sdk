# WASM Browser Integration Guide

Use Havklo SDK in the browser via WebAssembly.

## Building the WASM Package

```bash
cd crates/kraken-wasm
wasm-pack build --target web
```

This creates a `pkg/` directory with:
- `kraken_wasm.js` - JavaScript bindings
- `kraken_wasm.d.ts` - TypeScript definitions
- `kraken_wasm_bg.wasm` - WebAssembly binary

## Installation

### From Local Build

```html
<script type="module">
  import init, { WasmOrderbook, WasmL3Book, WasmRestClient, WasmRateLimiter }
    from './pkg/kraken_wasm.js';

  await init();
</script>
```

### From NPM (after publishing)

```bash
npm install @kraken-forge/wasm
```

```javascript
import init, { WasmOrderbook } from '@kraken-forge/wasm';
await init();
```

## L2 Orderbook

Track price levels with aggregated quantities:

```javascript
import init, { WasmOrderbook } from './pkg/kraken_wasm.js';

await init();

const book = new WasmOrderbook('BTC/USD');
book.set_precision(1, 8);  // Price: 1 decimal, Qty: 8 decimals

// Connect to Kraken WebSocket
const ws = new WebSocket('wss://ws.kraken.com/v2');

ws.onopen = () => {
  ws.send(JSON.stringify({
    method: 'subscribe',
    params: {
      channel: 'book',
      symbol: ['BTC/USD'],
      depth: 10
    }
  }));
};

ws.onmessage = (event) => {
  const result = book.apply_message(event.data);

  if (book.is_synced()) {
    console.log('State:', book.get_state());
    console.log('Spread:', book.get_spread());
    console.log('Mid price:', book.get_mid_price());
    console.log('Best bid:', book.get_best_bid());
    console.log('Best ask:', book.get_best_ask());

    // Get all levels
    const bids = book.get_bids();  // [{price, qty}, ...]
    const asks = book.get_asks();

    // Get top N levels
    const topBids = book.get_top_bids(5);
    const topAsks = book.get_top_asks(5);
  }
};
```

### Time-Travel Feature

Enable history to replay orderbook states:

```javascript
const book = new WasmOrderbook('BTC/USD');
book.enable_history(100);  // Store last 100 snapshots

// Later: replay history
const length = book.get_history_length();
for (let i = 0; i < length; i++) {
  const snapshot = book.get_snapshot_at(i);
  console.log(`Snapshot ${i}:`, snapshot.bids, snapshot.asks);
}
```

## L3 Orderbook

Track individual orders for market making:

```javascript
import init, { WasmL3Book } from './pkg/kraken_wasm.js';

await init();

const book = new WasmL3Book('BTC/USD', 100);  // 100 levels max

// Add orders
book.add_order('order_123', 50000.00, 1.5, 'bid');
book.add_order('order_124', 50001.00, 2.0, 'ask');

// With metadata (timestamp, sequence)
book.add_order_with_metadata('order_125', 49999.00, 0.5, 'bid', Date.now() * 1000, 1);

// Check queue position
const pos = book.get_queue_position('order_123');
if (pos) {
  console.log('Position:', pos.position);
  console.log('Orders ahead:', pos.orders_ahead);
  console.log('Qty ahead:', pos.qty_ahead);
  console.log('Fill probability:', pos.fill_probability);
}

// Market analytics
console.log('Imbalance:', book.get_imbalance());  // -1 to 1
console.log('VWAP to buy 10:', book.get_vwap_ask(10.0));
console.log('VWAP to sell 10:', book.get_vwap_bid(10.0));

// Aggregated view (L2)
const aggBids = book.get_aggregated_bids();
const aggAsks = book.get_aggregated_asks();

// Modify orders
book.modify_order('order_123', 2.0);  // Change quantity

// Remove orders
const removed = book.remove_order('order_123');

// Checksum validation
const checksum = book.compute_checksum();
const isValid = book.validate_checksum(expectedChecksum);
```

## REST Client

Make API calls from the browser:

```javascript
import init, { WasmRestClient } from './pkg/kraken_wasm.js';

await init();

const client = new WasmRestClient();

// Public endpoints
const time = await client.get_server_time();
console.log('Server time:', time.unixtime);

const status = await client.get_system_status();
console.log('Status:', status.status);

const assets = await client.get_assets();
const pairs = await client.get_asset_pairs();

// Ticker data
const ticker = await client.get_ticker('XBTUSD');
console.log('BTC price:', ticker.XXBTZUSD.c[0]);

// Multiple tickers
const tickers = await client.get_ticker('XBTUSD,ETHUSD');

// Orderbook
const book = await client.get_orderbook('XBTUSD', 10);
console.log('Best bid:', book.XXBTZUSD.bids[0]);
console.log('Best ask:', book.XXBTZUSD.asks[0]);

// OHLC data
const ohlc = await client.get_ohlc('XBTUSD', 60, null);  // 60 min candles

// Recent trades
const trades = await client.get_recent_trades('XBTUSD', null, 100);

// Spread data
const spreads = await client.get_spread('XBTUSD', null);
```

## Rate Limiter

Prevent hitting API limits:

```javascript
import init, { WasmRateLimiter, WasmRestClient } from './pkg/kraken_wasm.js';

await init();

// Create limiters for different endpoint types
const publicLimiter = WasmRateLimiter.kraken_public();   // 15 req, 0.5/sec
const privateLimiter = WasmRateLimiter.kraken_private(); // 20 req, 0.33/sec

// Or custom limiter
const customLimiter = new WasmRateLimiter(10, 1.0);  // 10 tokens, 1/sec refill

const client = new WasmRestClient();

async function fetchTicker(pair) {
  // Check if we can make request
  if (publicLimiter.try_acquire()) {
    return await client.get_ticker(pair);
  } else {
    // Wait for token
    const waitMs = publicLimiter.time_until_available();
    console.log(`Rate limited, waiting ${waitMs}ms`);
    await publicLimiter.wait_for_token();
    return await client.get_ticker(pair);
  }
}

// Check limiter status
console.log('Available tokens:', publicLimiter.available());
console.log('Utilization:', publicLimiter.utilization());  // 0.0 to 1.0
console.log('Is limited:', publicLimiter.is_limited());

// Reset to full capacity
publicLimiter.reset();
```

## TypeScript Support

The package includes TypeScript definitions:

```typescript
import init, {
  WasmOrderbook,
  WasmL3Book,
  WasmRestClient,
  WasmRateLimiter
} from './pkg/kraken_wasm.js';

interface PriceLevel {
  price: number;
  qty: number;
}

interface QueuePosition {
  position: number;
  orders_ahead: number;
  qty_ahead: number;
  total_orders: number;
  total_qty: number;
  fill_probability: number;
}

async function main(): Promise<void> {
  await init();

  const book: WasmOrderbook = new WasmOrderbook('BTC/USD');
  const bids: PriceLevel[] = book.get_bids();

  const l3: WasmL3Book = new WasmL3Book('BTC/USD', 100);
  const pos: QueuePosition | null = l3.get_queue_position('order_id');
}
```

## Complete Trading UI Example

```html
<!DOCTYPE html>
<html>
<head>
  <title>Kraken Orderbook</title>
  <style>
    .orderbook { display: flex; gap: 20px; }
    .side { width: 300px; }
    .level { display: flex; justify-content: space-between; padding: 2px; }
    .bid { background: rgba(0, 255, 0, 0.1); }
    .ask { background: rgba(255, 0, 0, 0.1); }
    .spread { text-align: center; padding: 10px; font-weight: bold; }
  </style>
</head>
<body>
  <h1>BTC/USD Orderbook</h1>
  <div class="spread" id="spread"></div>
  <div class="orderbook">
    <div class="side" id="bids"><h3>Bids</h3></div>
    <div class="side" id="asks"><h3>Asks</h3></div>
  </div>

  <script type="module">
    import init, { WasmOrderbook } from './pkg/kraken_wasm.js';

    await init();

    const book = new WasmOrderbook('BTC/USD');
    book.set_precision(1, 8);

    const ws = new WebSocket('wss://ws.kraken.com/v2');

    ws.onopen = () => {
      ws.send(JSON.stringify({
        method: 'subscribe',
        params: { channel: 'book', symbol: ['BTC/USD'], depth: 10 }
      }));
    };

    ws.onmessage = (event) => {
      book.apply_message(event.data);

      if (book.is_synced()) {
        renderOrderbook();
      }
    };

    function renderOrderbook() {
      const bids = book.get_bids();
      const asks = book.get_asks();

      document.getElementById('spread').textContent =
        `Spread: $${book.get_spread().toFixed(2)} | Mid: $${book.get_mid_price().toFixed(2)}`;

      document.getElementById('bids').innerHTML = '<h3>Bids</h3>' +
        bids.map(l => `<div class="level bid">
          <span>${l.qty.toFixed(4)}</span>
          <span>$${l.price.toFixed(2)}</span>
        </div>`).join('');

      document.getElementById('asks').innerHTML = '<h3>Asks</h3>' +
        asks.map(l => `<div class="level ask">
          <span>$${l.price.toFixed(2)}</span>
          <span>${l.qty.toFixed(4)}</span>
        </div>`).join('');
    }
  </script>
</body>
</html>
```

## API Reference

### WasmOrderbook

| Method | Returns | Description |
|--------|---------|-------------|
| `new(symbol)` | WasmOrderbook | Create L2 orderbook |
| `with_depth(symbol, depth)` | WasmOrderbook | Create with specific depth |
| `apply_message(json)` | string | Process WebSocket message |
| `is_synced()` | boolean | Check if synchronized |
| `get_state()` | string | Get current state |
| `get_bids()` | Array | Get all bid levels |
| `get_asks()` | Array | Get all ask levels |
| `get_spread()` | number | Get bid-ask spread |
| `get_mid_price()` | number | Get mid price |
| `get_checksum()` | number | Get last checksum |
| `set_precision(price, qty)` | void | Set decimal precision |

### WasmL3Book

| Method | Returns | Description |
|--------|---------|-------------|
| `new(symbol, depth)` | WasmL3Book | Create L3 orderbook |
| `add_order(id, price, qty, side)` | boolean | Add order |
| `remove_order(id)` | Object/null | Remove order |
| `modify_order(id, qty)` | boolean | Modify quantity |
| `get_queue_position(id)` | Object/null | Get queue position |
| `get_imbalance()` | number | Get bid/ask imbalance |
| `get_vwap_ask(qty)` | number | VWAP to buy |
| `get_vwap_bid(qty)` | number | VWAP to sell |

### WasmRestClient

| Method | Returns | Description |
|--------|---------|-------------|
| `new()` | WasmRestClient | Create REST client |
| `get_server_time()` | Promise | Get server time |
| `get_system_status()` | Promise | Get system status |
| `get_ticker(pair)` | Promise | Get ticker data |
| `get_orderbook(pair, count)` | Promise | Get orderbook |
| `get_ohlc(pair, interval, since)` | Promise | Get OHLC data |

### WasmRateLimiter

| Method | Returns | Description |
|--------|---------|-------------|
| `new(capacity, rate)` | WasmRateLimiter | Create limiter |
| `kraken_public()` | WasmRateLimiter | Public endpoint limiter |
| `kraken_private()` | WasmRateLimiter | Private endpoint limiter |
| `try_acquire()` | boolean | Try to get token |
| `wait_for_token()` | Promise | Wait for token |
| `available()` | number | Available tokens |
| `is_limited()` | boolean | Check if rate limited |
