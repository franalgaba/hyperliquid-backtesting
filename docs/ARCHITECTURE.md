# Architecture Overview

## System Architecture

The Hyperliquid backtester is designed as a high-performance, event-driven simulation engine for testing trading strategies against historical market data.

```
Backtester CLI
    |
    v
Strategy Compiler
    |
    v
PerpsEngine
    ├── OrderBook (L2 state)
    ├── Portfolio (Positions)
    ├── Indicators (RSI, SMA...)
    ├── FundingSchedule
    └── FeeCalculator
    |
    v
PerpsExecution
    (Order execution logic)
```

## Component Details

### PerpsEngine

**Purpose**: Main simulation engine that orchestrates the backtest.

**Responsibilities**:
- Loading and processing L2 events
- Maintaining order book state
- Evaluating strategies (entry and exit graphs)
- Executing orders
- Tracking portfolio state
- Recording results

**Key Design Decisions**:
- **Event-driven**: Processes events sequentially to maintain temporal accuracy
- **Order book reconstruction**: Rebuilds book from snapshots (doesn't modify after execution)
- **Dual-graph evaluation**: 
  - Entry graph evaluated when flat (no position) - subject to cooldown
  - Exit graph evaluated when in position - bypasses cooldown for prompt exits
- **Strategy throttling**: Only evaluates on significant price changes (0.01% threshold)
- **Parallel I/O**: File loading is parallelized for performance

### OrderBook

**Purpose**: Maintains current bid/ask levels from L2 snapshots.

**Data Structure**:
- `BTreeMap<u64, f64>` for bids (price scaled to integer → size)
- `BTreeMap<u64, f64>` for asks (price scaled to integer → size)

**Key Operations**:
- `apply_snapshot()`: Replaces entire book with new snapshot
- `best_bid()` / `best_ask()`: Get best prices
- `mid_price()`: Calculate mid price
- `sweep_market_buy()` / `sweep_market_sell()`: Execute market orders

**Design Notes**:
- Uses scaled integers for price keys (1e8 scale) for efficient BTreeMap operations
- Book state is NOT modified after execution (correct for backtesting)

### PerpsExecution

**Purpose**: Handles order execution logic.

**Key Functions**:
- `execute_market()`: Execute market orders immediately
- `check_limit_fill()`: Check if limit orders should fill
- `can_place_limit()`: Validate limit order placement

**Execution Flow**:
1. Market orders: Execute immediately, sweep book
2. Limit orders: Check on each event, fill when price crosses
3. Partial fills: Track remaining size, update order state

### Portfolio

**Purpose**: Tracks positions, cash, and equity.

**State**:
- `cash`: Available cash balance
- `positions`: Map of coin → position (size, entry price)
- `fee_calc`: Fee calculator reference

**Key Operations**:
- `execute_trade()`: Update cash and positions
- `total_equity()`: Calculate total portfolio value
- `get_position_value()`: Get position notional value

### Strategy Evaluation

**Purpose**: Executes strategy graphs based on market conditions and position state.

**Graph Types**:
- **Entry Graph**: Evaluated when portfolio is flat (no position)
  - Creates BUY/SELL orders to open positions
  - Subject to trade cooldown (configurable, default 15 minutes)
  - Only evaluated when price changes exceed threshold (0.01%)
- **Exit Graph**: Evaluated when portfolio has a position
  - Creates CLOSE orders or opposite-side orders to exit positions
  - Bypasses cooldown for prompt exit execution
  - Evaluated on every price change (no throttling)

**Evaluation Flow**:
1. Check current position size
2. If flat → evaluate entry graph (if cooldown allows)
3. If in position → evaluate exit graph (immediate, no cooldown)
4. Execute any orders created by graph evaluation

**Design Rationale**:
- Entry cooldown prevents excessive trading on small price movements
- Exit bypass ensures positions can be closed promptly when conditions are met
- Position-based evaluation ensures correct graph is used at the right time

### FundingSchedule

**Purpose**: Manages historical funding rates.

**Data Structure**:
- `Vec<FundingPoint>`: Sorted by timestamp
- Each point contains: timestamp, rate

**Key Operations**:
- `from_api()`: Fetch from Hyperliquid API
- `rate_at()`: Lookup rate by timestamp
- `calculate_payment()`: Calculate funding payment

**Design Notes**:
- Rates are fetched once at start of backtest
- Lookup is O(log n) using binary search
- Validates inputs for security

## Data Flow

### Backtest Execution Flow

```
1. Load Events
   └─> Read JSONL files from directory
   └─> Parse L2 snapshots
   └─> Filter by timestamp range
   └─> Sort by timestamp

2. Initialize Engine
   └─> Create OrderBook
   └─> Fetch FundingSchedule
   └─> Initialize Portfolio
   └─> Create Indicators

3. Process Events (for each event)
   ├─> Update OrderBook (apply snapshot)
   ├─> Update Indicators (synthetic candle)
   ├─> Evaluate Strategy (if price changed)
   │   └─> Create new orders
   ├─> Execute Market Orders (immediately)
   ├─> Check Limit Orders (for fills)
   ├─> Apply Funding (every 8 hours)
   └─> Record Equity (every minute)

4. Calculate Results
   └─> Final equity
   └─> Total return
   └─> Performance metrics
```

### Order Execution Flow

```
Market Order:
  Order Created → Execute Immediately → Fill (or retry) → Update Portfolio → Remove Order

Limit Order:
  Order Created → Add to Active Orders → Check Each Event → Fill When Price Crosses → 
  Update Portfolio → Remove When Fully Filled
```

## Performance Optimizations

### Memory Management
- Pre-allocated vectors with estimated capacity
- Reused synthetic candle (updated in-place)
- Reused coin strings to avoid allocations

### Computation
- Strategy evaluation throttled to price changes
- Parallel file I/O for loading events
- Parallel indicator updates (when multiple indicators)
- Efficient order removal (swap_remove pattern)

### I/O
- Streaming file parsing (doesn't load entire file)
- Parallel file reading with bounded concurrency
- Efficient JSONL parsing

## Error Handling

### Error Propagation
- Uses `anyhow::Result` for error handling
- Context-aware errors with `.with_context()`
- Early returns on errors

### Validation
- Input validation (coin names, dates, timestamps)
- Order validation (sizes, prices)
- Range checks (funding intervals, equity recording)

## Security Considerations

### Input Validation
- Coin names validated (alphanumeric only, no path traversal)
- Date formats validated (YYYYMMDD, 8 digits)
- Timestamp ranges validated (max 1 year)

### API Security
- HTTPS with certificate validation
- Request timeouts (30 seconds)
- Parameter sanitization

## Testing Strategy

### Unit Tests
- Individual component tests
- Edge case coverage
- Error handling tests

### Integration Tests
- End-to-end backtest scenarios
- Order execution scenarios
- Funding payment scenarios

## Future Improvements

### Potential Enhancements
1. **Caching**: Cache compiled strategies
2. **Streaming**: Stream events instead of loading all
3. **Metrics**: More detailed performance metrics
4. **Visualization**: Real-time equity curve visualization
5. **Multi-asset**: Support multiple assets simultaneously

### Performance Opportunities
1. **SIMD**: Use SIMD for indicator calculations
2. **GPU**: Offload indicator updates to GPU
3. **Distributed**: Distribute across multiple machines

## See Also

- [API Documentation](API.md)
- [Performance Optimizations](PERFORMANCE_OPTIMIZATIONS.md)
- [Security Documentation](SECURITY.md)

