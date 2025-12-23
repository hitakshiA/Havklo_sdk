/**
 * TypeScript type definitions for Kraken WASM SDK
 *
 * These types match the WASM bindings exported from kraken-wasm.
 * Import the actual WASM module from the pkg directory after building.
 */

/**
 * Price level in an orderbook
 */
export interface PriceLevel {
    /** Price as a floating point number */
    price: number;
    /** Quantity at this price level */
    qty: number;
}

/**
 * Queue position information for L3 orderbook
 */
export interface QueuePosition {
    /** Position in the queue (1-indexed) */
    position: number;
    /** Total number of orders at this price level */
    total_orders: number;
    /** Total quantity ahead in the queue */
    qty_ahead: number;
    /** Estimated fill probability (0.0 to 1.0) */
    fill_probability: number;
}

/**
 * Aggregated price level (multiple orders combined)
 */
export interface AggregatedLevel {
    /** Price level */
    price: number;
    /** Total quantity at this price */
    total_qty: number;
    /** Number of orders at this price */
    order_count: number;
}

/**
 * L2 Orderbook - aggregated price levels
 *
 * Use this for standard orderbook displays showing price levels
 * without individual order information.
 */
export interface WasmOrderbook {
    /**
     * Create a new L2 orderbook
     * @param symbol Trading pair symbol (e.g., "BTC/USD")
     */
    new(symbol: string): WasmOrderbook;

    /**
     * Process a raw WebSocket message from Kraken
     * @param message JSON string from WebSocket
     */
    apply_message(message: string): void;

    /**
     * Get all bid levels sorted by price (highest first)
     */
    get_bids(): PriceLevel[];

    /**
     * Get all ask levels sorted by price (lowest first)
     */
    get_asks(): PriceLevel[];

    /**
     * Get current bid-ask spread
     * @returns Spread in price units, or null if not available
     */
    get_spread(): number | null;

    /**
     * Get current mid price
     * @returns Mid price, or null if not available
     */
    get_mid_price(): number | null;

    /**
     * Get best bid price
     * @returns Best bid price, or null if not available
     */
    get_best_bid(): number | null;

    /**
     * Get best ask price
     * @returns Best ask price, or null if not available
     */
    get_best_ask(): number | null;

    /**
     * Check if orderbook has received a snapshot and is synchronized
     */
    is_synced(): boolean;

    /**
     * Get the last validated checksum
     */
    get_checksum(): number;

    /**
     * Enable snapshot history with specified capacity
     * @param capacity Maximum number of snapshots to retain
     */
    enable_history(capacity: number): void;

    /**
     * Get a historical snapshot by index
     * @param index Snapshot index (0 = oldest)
     * @returns Snapshot data or null
     */
    get_snapshot_at(index: number): object | null;

    /**
     * Set decimal precision for checksum calculation
     * @param precision Number of decimal places
     */
    set_precision(precision: number): void;
}

/**
 * L3 Orderbook - individual order tracking
 *
 * Use this for market making and when you need to track
 * queue position of individual orders.
 */
export interface WasmL3Book {
    /**
     * Create a new L3 orderbook
     * @param symbol Trading pair symbol (e.g., "BTC/USD")
     * @param depth Maximum number of price levels to track
     */
    new(symbol: string, depth: number): WasmL3Book;

    /**
     * Add an order to the book
     * @param order_id Unique order identifier
     * @param side Order side: "bid" or "ask"
     * @param price Order price as string (for precision)
     * @param qty Order quantity as string (for precision)
     */
    add_order(order_id: string, side: string, price: string, qty: string): void;

    /**
     * Remove an order from the book
     * @param order_id Order identifier to remove
     * @returns true if order was found and removed
     */
    remove_order(order_id: string): boolean;

    /**
     * Modify an existing order's quantity
     * @param order_id Order identifier
     * @param new_qty New quantity as string
     * @returns true if order was found and modified
     */
    modify_order(order_id: string, new_qty: string): boolean;

    /**
     * Get queue position for an order
     * @param order_id Order identifier
     * @returns Queue position info or null if not found
     */
    get_queue_position(order_id: string): QueuePosition | null;

    /**
     * Get best bid price
     */
    best_bid_price(): number | null;

    /**
     * Get best ask price
     */
    best_ask_price(): number | null;

    /**
     * Get mid price
     */
    mid_price(): number | null;

    /**
     * Get bid-ask spread
     */
    spread(): number | null;

    /**
     * Get total quantity on bid side
     */
    total_bid_qty(): number;

    /**
     * Get total quantity on ask side
     */
    total_ask_qty(): number;

    /**
     * Get total number of orders in the book
     */
    order_count(): number;

    /**
     * Get number of bid price levels
     */
    bid_level_count(): number;

    /**
     * Get number of ask price levels
     */
    ask_level_count(): number;

    /**
     * Calculate order imbalance
     * @returns Value from -1.0 (all asks) to 1.0 (all bids)
     */
    get_imbalance(): number | null;

    /**
     * Calculate VWAP for buying a quantity
     * @param qty Quantity to buy as string
     * @returns VWAP price or null if insufficient liquidity
     */
    get_vwap_ask(qty: string): number | null;

    /**
     * Calculate VWAP for selling a quantity
     * @param qty Quantity to sell as string
     * @returns VWAP price or null if insufficient liquidity
     */
    get_vwap_bid(qty: string): number | null;

    /**
     * Get aggregated bid levels
     * @returns Array of aggregated bid levels
     */
    get_aggregated_bids(): AggregatedLevel[];

    /**
     * Get aggregated ask levels
     * @returns Array of aggregated ask levels
     */
    get_aggregated_asks(): AggregatedLevel[];

    /**
     * Clear all orders from the book
     */
    clear(): void;
}

/**
 * Initialize the WASM module
 * Must be called before using any other functions
 */
export function init(): Promise<void>;
