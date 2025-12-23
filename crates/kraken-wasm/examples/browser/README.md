# Havklo SDK Browser Demo

This example demonstrates using the Kraken WASM SDK in a browser environment, featuring:

- **L2 Orderbook**: Real-time price levels from Kraken WebSocket
- **L3 Orderbook**: Order-level tracking with queue position (simulated)
- **Market Analytics**: Spread, imbalance, VWAP calculations

## Prerequisites

1. [Rust](https://rustup.rs/) installed
2. [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) installed:
   ```bash
   cargo install wasm-pack
   ```

## Building

From the repository root:

```bash
cd crates/kraken-wasm
wasm-pack build --target web
```

This creates the `pkg/` directory with:
- `kraken_wasm.js` - JavaScript bindings
- `kraken_wasm.d.ts` - TypeScript definitions
- `kraken_wasm_bg.wasm` - WebAssembly binary

## Running

You need a local HTTP server (WASM modules require proper CORS headers):

```bash
# Using Python
cd crates/kraken-wasm
python3 -m http.server 8080

# Using Node.js
npx serve crates/kraken-wasm

# Using Rust
cargo install miniserve
miniserve crates/kraken-wasm --port 8080
```

Then open: http://localhost:8080/examples/browser/

## Features Demonstrated

### L2 Orderbook
- Connects to `wss://ws.kraken.com/v2`
- Subscribes to BTC/USD orderbook (depth=10)
- Displays real-time bid/ask levels
- Shows spread in USD and basis points

### L3 Orderbook (Simulated)
- Creates order-level book from L2 data
- Tracks individual order queue positions
- Calculates fill probability
- Computes VWAP for market orders

### Analytics
- **Imbalance**: Buy/sell pressure indicator (-1 to +1)
- **VWAP**: Volume-weighted average price for slippage estimation
- **Queue Position**: Where your order sits in the book

## TypeScript Usage

The `types.ts` file provides full type definitions:

```typescript
import init, { WasmOrderbook, WasmL3Book } from '../../pkg/kraken_wasm.js';

// Initialize WASM
await init();

// Create L2 orderbook
const l2Book = new WasmOrderbook('BTC/USD');
l2Book.enable_history(100);

// Process messages
ws.onmessage = (event) => {
    l2Book.apply_message(event.data);

    if (l2Book.is_synced()) {
        console.log('Mid price:', l2Book.get_mid_price());
        console.log('Spread:', l2Book.get_spread());
    }
};

// Create L3 orderbook
const l3Book = new WasmL3Book('BTC/USD', 100);

// Add orders
l3Book.add_order('order_1', 'bid', '50000.00', '1.5');

// Check queue position
const pos = l3Book.get_queue_position('order_1');
if (pos) {
    console.log('Position:', pos.position, 'of', pos.total_orders);
    console.log('Fill probability:', pos.fill_probability * 100, '%');
}

// Analytics
console.log('Imbalance:', l3Book.get_imbalance());
console.log('VWAP to buy 10 BTC:', l3Book.get_vwap_ask('10.0'));
```

## Production Considerations

1. **L3 Data**: This demo simulates L3 from L2 data. In production, subscribe to Kraken's actual L3 feed (requires authentication).

2. **Error Handling**: Add try/catch around WASM calls as they may throw on invalid input.

3. **Memory**: The WASM module manages its own memory. Call `clear()` on orderbooks when switching symbols.

4. **Reconnection**: Implement reconnection logic for production use. The SDK supports history replay for gap detection.

## File Structure

```
browser/
├── index.html     # Main demo page
├── types.ts       # TypeScript definitions
└── README.md      # This file
```
