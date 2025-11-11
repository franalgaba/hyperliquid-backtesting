# API Documentation

## Overview

This document provides comprehensive API documentation for the Hyperliquid backtester, focusing on the perps engine and related modules.

## Table of Contents

- [PerpsEngine](#perpengine)
- [PerpsExecution](#perpsexecution)
- [Trade Utilities](#trade-utilities)
- [FundingSchedule](#fundingschedule)
- [OrderBook](#orderbook)

---

## PerpsEngine

The `PerpsEngine` is the core component for running event-driven perpetual futures backtests using historical L2 order book data.

### Purpose

The `PerpsEngine` simulates realistic trading execution by:
- Reconstructing order books from historical L2 snapshots
- Executing orders against the order book with realistic fills
- Tracking positions, funding payments, and fees
- Recording equity curves and trade history

### Architecture

```
PerpsEngine
├── OrderBook          # Current order book state
├── FundingSchedule    # Historical funding rates
├── FeeCalculator      # Fee calculation logic
└── Portfolio          # Position and cash tracking
```

### Key Concepts

- **L2 Events**: Order book snapshots containing bid/ask levels
- **Synthetic Candles**: Price data derived from order book mid-price
- **Strategy Evaluation**: Graph-based strategy execution on price changes
- **Order Execution**: Market orders execute immediately, limit orders wait for fills

### Public API

#### `new(funding: FundingSchedule, config: &SimConfig) -> Self`

Creates a new `PerpsEngine` instance.

**Parameters**:
- `funding`: Historical funding rate schedule
- `config`: Simulation configuration (fees, initial capital, etc.)

**Returns**: Initialized `PerpsEngine`

**Example**:
```rust
let funding = FundingSchedule::from_api("BTC", start_ts, end_ts).await?;
let engine = PerpsEngine::new(funding, &config);
```

#### `run(...) -> Result<SimResult>`

Runs a complete backtest from L2 events.

**Parameters**:
- `events_dir`: Directory containing JSONL event files
- `ir`: Compiled strategy IR (Intermediate Representation)
- `config`: Simulation configuration
- `coin`: Coin symbol (e.g., "BTC", "ETH")
- `start_ts`: Start timestamp (milliseconds)
- `end_ts`: End timestamp (milliseconds)
- `io_concurrency`: Optional concurrency limit for file I/O
- `indicators_parallel`: Whether to update indicators in parallel

**Returns**: `SimResult` containing trades, equity curve, and metrics

**Example**:
```rust
let result = PerpsEngine::run(
    "data/events/BTC",
    &strategy_ir,
    &config,
    "BTC",
    1694858400000,  // Start: 2023-09-16 09:00:00 UTC
    1694865600000,  // End:   2023-09-16 11:00:00 UTC
    Some(8),        // Use 8 concurrent file readers
    true,           // Parallel indicator updates
).await?;
```

**Error Handling**:
- Returns `Err` if events directory cannot be read
- Returns `Err` if strategy compilation fails
- Returns `Err` if funding history cannot be fetched
- Returns `Err` if no events found in time range

**Performance Considerations**:
- Processes events sequentially (order matters for backtesting)
- File loading is parallelized for better I/O performance
- Indicator updates can be parallelized if multiple indicators exist
- Strategy evaluation is throttled to price changes (0.01% threshold)

---

## PerpsExecution

The `PerpsExecution` module handles order execution logic against the order book.

### Purpose

Provides execution logic for:
- Market order execution (immediate fills)
- Limit order fill detection
- Partial fill tracking
- Order status management

### Public API

#### `execute_market(order: &mut Order, book: &OrderBook) -> Option<FillResult>`

Executes a market order against the current order book.

**Parameters**:
- `order`: Market order to execute (mutated in place)
- `book`: Current order book state

**Returns**: 
- `Some(FillResult)` if order executed (fully or partially)
- `None` if no liquidity available

**Behavior**:
- Sweeps the book to fill as much as possible
- Updates `order.filled_sz` and `order.status`
- Handles partial fills correctly
- Market orders are always taker (not maker)

**Example**:
```rust
let mut order = Order {
    id: 1,
    action: Action::Market { side: Side::Buy, sz: 1.0 },
    created_at: 1000,
    filled_sz: 0.0,
    status: OrderStatus::Pending,
};

if let Some(fill) = PerpsExecution::execute_market(&mut order, &book) {
    println!("Filled: {} @ {}", fill.filled_sz, fill.fill_price);
}
```

**Edge Cases**:
- Returns `None` if order already fully filled
- Returns `None` if no liquidity available
- Handles partial fills when book depth is insufficient

#### `check_limit_fill(order: &mut Order, book: &OrderBook) -> Option<FillResult>`

Checks if a limit order should fill based on current book state.

**Parameters**:
- `order`: Limit order to check (mutated in place)
- `book`: Current order book state

**Returns**:
- `Some(FillResult)` if order can fill (fully or partially)
- `None` if order cannot fill

**Behavior**:
- Checks if limit price crosses best bid/ask
- Verifies sufficient liquidity at fill price
- Updates order filled size and remaining size
- Handles partial fills correctly
- Limit orders that fill are maker orders

**Example**:
```rust
let mut order = Order {
    id: 2,
    action: Action::Limit {
        side: Side::Buy,
        px: 50000.0,
        sz: 0.5,
        tif: Tif::Gtc,
        post_only: false,
        reduce_only: false,
    },
    created_at: 1000,
    filled_sz: 0.0,
    status: OrderStatus::Pending,
};

// Check on each event
if let Some(fill) = PerpsExecution::check_limit_fill(&mut order, &book) {
    println!("Limit order filled: {} @ {}", fill.filled_sz, fill.fill_price);
}
```

**Edge Cases**:
- Returns `None` if order already fully filled
- Returns `None` if limit price doesn't cross book
- Returns `None` if insufficient liquidity
- Handles partial fills when liquidity < order size

#### `can_place_limit(order: &Order, book: &OrderBook, post_only: bool) -> bool`

Checks if a limit order can be placed without crossing the book.

**Parameters**:
- `order`: Limit order to check
- `book`: Current order book state
- `post_only`: Whether post-only mode is enforced

**Returns**: `true` if order can be placed, `false` if it would cross

**Use Case**: Validate orders before adding to active orders list

---

## Trade Utilities

The `trade_utils` module provides helper functions for trade-related operations.

### Purpose

Centralizes common trade operations:
- Side enum conversion
- Side extraction from order actions
- Trade creation utilities

### Public API

#### `side_to_string(side: Side) -> &'static str`

Converts a `Side` enum to its string representation.

**Parameters**:
- `side`: `Side::Buy` or `Side::Sell`

**Returns**: `"BUY"` or `"SELL"` (static string slice)

**Example**:
```rust
let side_str = side_to_string(Side::Buy);  // "BUY"
```

#### `extract_side_from_action(action: &Action) -> Option<Side>`

Extracts the side from an order action.

**Parameters**:
- `action`: Order action (Market, Limit, etc.)

**Returns**: 
- `Some(Side)` if action has a side
- `None` for action types without sides

**Example**:
```rust
let action = Action::Market { side: Side::Buy, sz: 1.0 };
let side = extract_side_from_action(&action);  // Some(Side::Buy)
```

**Supported Actions**:
- `Action::Market` ✅
- `Action::Limit` ✅
- Other action types return `None`

---

## FundingSchedule

The `FundingSchedule` manages historical funding rates for perpetual futures.

### Purpose

- Fetches funding history from Hyperliquid API
- Calculates funding payments based on position notional
- Provides rate lookup by timestamp

### Public API

#### `from_api(coin: &str, start_ts: u64, end_ts: u64) -> Result<Self>`

Fetches funding history from Hyperliquid API.

**Parameters**:
- `coin`: Coin symbol (e.g., "BTC")
- `start_ts`: Start timestamp (milliseconds)
- `end_ts`: End timestamp (milliseconds)

**Returns**: `FundingSchedule` with historical rates

**Example**:
```rust
let funding = FundingSchedule::from_api(
    "BTC",
    1694858400000,  // Start
    1694865600000,  // End
).await?;
```

**Error Handling**:
- Returns `Err` if API request fails
- Returns `Err` if coin parameter is invalid
- Returns `Err` if timestamp range is invalid (> 1 year)
- Returns `Err` if funding rate parsing fails

**Security**:
- Validates coin parameter (prevents injection)
- Validates timestamp range (prevents DoS)
- Uses HTTPS with certificate validation
- 30-second timeout on requests

#### `rate_at(ts_ms: u64) -> Option<f64>`

Gets the funding rate at a specific timestamp.

**Parameters**:
- `ts_ms`: Timestamp in milliseconds

**Returns**: 
- `Some(rate)` if rate found at or before timestamp
- `None` if no rate available

**Behavior**: Returns the most recent rate at or before the timestamp

#### `calculate_payment(notional: f64, ts_ms: u64) -> f64`

Calculates funding payment for a position.

**Parameters**:
- `notional`: Position notional value (size * price)
- `ts_ms`: Timestamp for rate lookup

**Returns**: Funding payment amount (positive = received, negative = paid)

**Example**:
```rust
let notional = 10000.0;  // $10,000 position
let payment = funding.calculate_payment(notional, ts_ms);
// If rate is 0.0001 (0.01%), payment = 1.0 (long pays $1)
```

---

## OrderBook

The `OrderBook` maintains the current state of bid/ask levels.

### Purpose

- Stores bid and ask levels from L2 snapshots
- Provides price and liquidity queries
- Supports market order execution (sweeping)

### Key Methods

#### `apply_snapshot(levels: &[Vec<OrderLevel>])`

Replaces the entire order book with a new snapshot.

**Parameters**:
- `levels`: Array of two vectors: `[bids, asks]`

**Behavior**: Clears existing book and rebuilds from snapshot

#### `best_bid() -> Option<(f64, f64)>`

Gets the best bid (highest price, size).

**Returns**: `Some((price, size))` or `None` if no bids

#### `best_ask() -> Option<(f64, f64)>`

Gets the best ask (lowest price, size).

**Returns**: `Some((price, size))` or `None` if no asks

#### `mid_price() -> Option<f64>`

Calculates the mid price (average of best bid and ask).

**Returns**: `Some(price)` or `None` if book is empty

#### `sweep_market_buy(size: f64) -> Option<(f64, f64, bool)>`

Executes a market buy order by sweeping asks.

**Parameters**:
- `size`: Size to buy

**Returns**: 
- `Some((filled_size, avg_price, is_maker))`
- `None` if no liquidity

**Behavior**: Walks up the ask side, filling at each level until size is filled

#### `sweep_market_sell(size: f64) -> Option<(f64, f64, bool)>`

Executes a market sell order by sweeping bids.

**Parameters**:
- `size`: Size to sell

**Returns**: 
- `Some((filled_size, avg_price, is_maker))`
- `None` if no liquidity

**Behavior**: Walks down the bid side, filling at each level until size is filled

---

## Constants

### PerpsEngine Constants

```rust
const DEFAULT_EVENTS_CAPACITY: usize = 100_000;      // Pre-allocated event capacity
const DEFAULT_ORDERS_CAPACITY: usize = 100;         // Pre-allocated order capacity
const DEFAULT_TRADES_CAPACITY: usize = 1000;        // Pre-allocated trade capacity
const DEFAULT_EQUITY_CURVE_CAPACITY: usize = 10000; // Pre-allocated equity points
const FUNDING_INTERVAL_MS: u64 = 8 * 60 * 60 * 1000; // 8 hours
const PRICE_CHANGE_THRESHOLD: f64 = 0.0001;          // 0.01% change threshold
const MIN_FILL_SIZE: f64 = 1e-10;                    // Minimum fill size
const EQUITY_RECORDING_INTERVAL_MS: u64 = 60 * 1000; // 1 minute
```

---

## Error Handling

### Common Error Types

- **`anyhow::Error`**: General errors with context
- **`Result<T>`**: Standard Rust result type

### Error Patterns

```rust
// Context-aware errors
.with_context(|| format!("Failed to read file: {}", path))?

// Bail for invalid input
anyhow::bail!("Invalid coin name: {}", coin);

// Option to Result conversion
let value = option.ok_or_else(|| anyhow::anyhow!("Missing value"))?;
```

---

## Best Practices

### 1. Order Execution

- Always check `FillResult` for partial fills
- Update order state after execution
- Handle `None` return values (no liquidity)

### 2. Strategy Evaluation

- Use price change threshold to avoid excessive evaluation
- Ensure sufficient lookback data before evaluating
- Handle strategy compilation errors gracefully

### 3. Performance

- Pre-allocate vectors with estimated capacity
- Use parallel file I/O for large datasets
- Enable parallel indicator updates for multiple indicators

### 4. Error Handling

- Always handle `Result` types
- Provide context in error messages
- Validate inputs before processing

---

## Common Pitfalls

### 1. Order Book State

⚠️ **Important**: The order book is NOT modified after execution. This is intentional for backtesting - we simulate against fixed historical data.

### 2. Partial Fills

⚠️ **Watch out**: Partial fills must be tracked correctly. Always update `order.filled_sz` and reduce `order.action.sz`.

### 3. Market Order Retries

⚠️ **Note**: Market orders that fail to execute (no liquidity) are kept for retry on the next event. This allows execution when liquidity becomes available.

### 4. Funding Payments

⚠️ **Timing**: Funding payments are applied every 8 hours. Ensure your timestamp range aligns with funding intervals.

### 5. Strategy Evaluation Frequency

⚠️ **Performance**: Strategy evaluation is throttled to price changes (0.01% threshold). Very small price movements won't trigger evaluation.

---

## Examples

### Complete Backtest Example

```rust
use crate::perps::PerpsEngine;
use crate::ir::types::StrategyIr;
use crate::orders::types::SimConfig;

// Load strategy
let ir_str = std::fs::read_to_string("strategy.json")?;
let strategy_ir: StrategyIr = serde_json::from_str(&ir_str)?;

// Configure backtest
let config = SimConfig {
    initial_capital: 10000.0,
    maker_fee_bps: -1,
    taker_fee_bps: 10,
    slippage_bps: 5,
};

// Run backtest
let result = PerpsEngine::run(
    "data/events/BTC",
    &strategy_ir,
    &config,
    "BTC",
    1694858400000,
    1694865600000,
    Some(8),
    true,
).await?;

// Process results
println!("Final equity: ${:.2}", result.final_equity);
println!("Total return: {:.2}%", result.total_return_pct);
println!("Number of trades: {}", result.num_trades);
```

### Order Execution Example

```rust
use crate::perps::execution::PerpsExecution;
use crate::orders::types::{Order, Action, Side, OrderStatus};

// Create market order
let mut order = Order {
    id: 1,
    action: Action::Market { side: Side::Buy, sz: 1.0 },
    created_at: 1000,
    filled_sz: 0.0,
    status: OrderStatus::Pending,
};

// Execute against book
if let Some(fill) = PerpsExecution::execute_market(&mut order, &book) {
    println!("Order {} filled: {} @ ${:.2}", 
        order.id, fill.filled_sz, fill.fill_price);
    
    if fill.order_status == OrderStatus::PartiallyFilled {
        println!("Warning: Partial fill, remaining: {:.6}", 
            order.action.size() - fill.filled_sz);
    }
} else {
    println!("Order {} could not execute (no liquidity)", order.id);
}
```

---

## See Also

- [Performance Optimizations](PERFORMANCE_OPTIMIZATIONS.md)
- [S3 Setup Guide](S3_SETUP.md)
- [Main README](../README.md)

