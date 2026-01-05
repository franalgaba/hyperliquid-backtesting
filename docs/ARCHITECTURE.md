# Architecture Overview

## System Architecture

The Hyperliquid Data Ingestor & Backtester is designed as a high-performance, event-driven simulation engine for testing trading strategies against historical market data.

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI (cli.rs)                            │
│  Commands: fetch, export, run, run-perps, ingest                │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│  Data Module  │    │   Strategy    │    │    Ingest     │
│               │    │    Module     │    │    Module     │
│ • loader.rs   │    │ • types.rs    │    │ • s3.rs       │
│ • cache.rs    │    │ • compile.rs  │    │ • l2_parser   │
│ • parquet.rs  │    │ • eval.rs     │    │               │
└───────────────┘    └───────────────┘    └───────────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│ OrdersEngine  │    │  PerpsEngine  │    │  Indicators   │
│ (Candle-based)│    │ (L2 Events)   │    │  (Technical)  │
└───────────────┘    └───────────────┘    └───────────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
                              ▼
                    ┌───────────────┐
                    │   Portfolio   │
                    │  & Execution  │
                    └───────────────┘
```

## Module Overview

### Data Module (`src/data/`)

Handles market data fetching, caching, and export.

| File | Purpose |
|------|---------|
| `loader.rs` | Fetch candles from Hyperliquid API |
| `cache.rs` | Local CSV caching |
| `parquet.rs` | Parquet export (candles, trades, equity) |
| `types.rs` | Candle data structures |

### Strategy Module (`src/strategy/`)

Simplified strategy definition and evaluation.

| File | Purpose |
|------|---------|
| `types.rs` | Strategy, Condition, Action types |
| `compile.rs` | Compile strategy (resolve indicator lookbacks) |
| `eval.rs` | Evaluate conditions against indicator values |

### Ingest Module (`src/ingest/`)

L2 order book data ingestion from S3.

| File | Purpose |
|------|---------|
| `s3.rs` | Download from Hyperliquid S3 archive |
| `l2_parser.rs` | Parse LZ4-compressed L2 snapshots |

### Orders Module (`src/orders/`)

Candle-based backtesting engine.

| File | Purpose |
|------|---------|
| `engine.rs` | Main simulation loop |
| `types.rs` | Order, Trade, SimResult types |
| `fills.rs` | Order fill processing |

### Perps Module (`src/perps/`)

L2 event-driven perpetuals backtesting.

| File | Purpose |
|------|---------|
| `engine.rs` | Event-driven simulation |
| `execution.rs` | Order execution against order book |
| `funding.rs` | Funding rate handling |
| `trade_utils.rs` | Trade utilities |

---

## Strategy System

### Strategy Definition

Strategies are defined in JSON with:
- **Indicators**: Technical indicators to compute
- **Entry Rule**: Condition + Action for opening positions
- **Exit Rule**: Condition + Action for closing positions

### Condition Types

```rust
enum Condition {
    Threshold { indicator, op, value },  // Compare to constant
    Crossover { fast, slow, direction }, // Indicator crossover
    And { conditions },                  // Logical AND
    Or { conditions },                   // Logical OR
}
```

### Evaluation Flow

```
Load Strategy JSON
       │
       ▼
Compile (resolve lookbacks)
       │
       ▼
For each candle/event:
       │
       ├─> Update indicators
       │
       ├─> If FLAT position:
       │   └─> Evaluate entry condition
       │       └─> If true: create order
       │
       └─> If IN position:
           └─> Evaluate exit condition
               └─> If true: close position
```

---

## Backtesting Engines

### OrdersEngine (Candle-based)

Simple backtesting on OHLC data:
- Fills at candle close price
- No order book simulation
- Fast execution

### PerpsEngine (L2 Events)

Realistic backtesting on order book snapshots:
- Real order book reconstruction
- Market orders sweep book
- Limit orders fill on price cross
- Funding payments every 8 hours

---

## Data Flow

### OHLC Data Pipeline

```
Hyperliquid API → loader.rs → cache.rs (CSV) → parquet.rs (Parquet)
                                    │
                                    ▼
                              OrdersEngine
```

### L2 Data Pipeline

```
S3 Archive → s3.rs → l2_parser.rs → events (JSONL)
                                         │
                                         ▼
                                   PerpsEngine
```

---

## Key Components

### OrderBook (`src/orderbook/`)

BTreeMap-based limit order book:
- `apply_snapshot()`: Update from L2 data
- `best_bid()` / `best_ask()`: Get best prices
- `sweep_market_buy()`: Execute market orders

### Portfolio (`src/portfolio.rs`)

Position and cash tracking:
- `cash`: Available balance
- `positions`: Map of symbol → position
- `execute_trade()`: Update on fills
- `total_equity()`: Calculate portfolio value

### FeeCalculator (`src/fees.rs`)

Fee and slippage calculation:
- Maker/taker fees (basis points)
- Slippage simulation

---

## Performance Optimizations

### Memory
- Pre-allocated vectors
- Reused synthetic candles
- Efficient string handling

### Computation
- Strategy evaluation throttled (0.01% price change)
- Parallel indicator updates
- Efficient order removal (swap_remove)

### I/O
- Parallel file loading
- Streaming JSONL parsing
- Snappy-compressed Parquet

---

## Error Handling

- `anyhow::Result` for error propagation
- Context-aware errors with `.with_context()`
- Input validation (assets, dates, parameters)

---

## Security

### Input Validation
- Asset names: alphanumeric only
- Dates: format validation
- Paths: no traversal allowed

### API Security
- HTTPS with certificate validation
- Request timeouts
- Parameter sanitization

---

## See Also

- [CLI Reference](CLI.md)
- [Strategies Guide](STRATEGIES.md)
- [Indicators Reference](INDICATORS.md)
- [Data Ingestion](DATA_INGESTION.md)
