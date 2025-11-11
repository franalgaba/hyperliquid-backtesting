# Quick Start Guide

This guide will help you get started with the Hyperliquid backtester quickly.

## Prerequisites

- Rust 1.70+ installed
- AWS credentials configured (for S3 data access)
- Basic understanding of trading concepts

## Installation

```bash
# Clone the repository
git clone <repository-url>
cd hyperliquid-backtesting/backtester

# Build the project
cargo build --release

# The binary will be at target/release/hl-backtest
```

## Basic Usage

### 1. Download L2 Data

First, download historical L2 order book data from S3:

```bash
hl-backtest ingest s3 \
  --coin BTC \
  --start 20230916 \
  --start-hour 9 \
  --end 20230916 \
  --end-hour 11 \
  --out data/s3
```

**Note**: This requires AWS credentials. See [S3 Setup Guide](S3_SETUP.md) for details.

### 2. Build Events

Convert the downloaded L2 files to event format:

```bash
hl-backtest ingest build-events \
  --coin BTC \
  --input data/s3 \
  --out data/events
```

This creates JSONL files in `data/events/BTC/` that the backtester can process.

### 3. Create a Strategy

Create a simple strategy file (`strategy.json`):

```json
{
  "entry": "start",
  "nodes": {
    "start": {
      "type": "condition",
      "expr": {
        "lhs": { "ref": "sma_20.value" },
        "op": "gt",
        "rhs": { "ref": "sma_50.value" }
      },
      "true_branch": "buy",
      "false_branch": "end"
    },
    "buy": {
      "type": "action",
      "action": {
        "kind": "buy",
        "symbol": "BTC",
        "sizing": { "mode": "cash", "value": 1000 },
        "order": { "type": "MARKET" }
      },
      "next": "end"
    },
    "end": {
      "type": "terminal"
    }
  },
  "indicators": [
    {
      "id": "sma_20",
      "type": "SMA",
      "params": { "length": 20 },
      "lookback": 20
    },
    {
      "id": "sma_50",
      "type": "SMA",
      "params": { "length": 50 },
      "lookback": 50
    }
  ]
}
```

### 4. Run Backtest

Run the backtest:

```bash
hl-backtest run-perps \
  --ir strategy.json \
  --coin BTC \
  --events data/events \
  --start 20230916-09 \
  --end 20230916-11 \
  --initial-capital 10000 \
  --maker-fee-bps -1 \
  --taker-fee-bps 10 \
  --out results.json
```

### 5. View Results

Results are written to:
- `results.json` - Complete backtest results
- `results_trades.csv` - Trade log
- `results_equity.csv` - Equity curve over time

```bash
# View summary
cat results.json | jq '.final_equity, .total_return_pct, .num_trades'

# View trades
head -20 results_trades.csv

# View equity curve
head -20 results_equity.csv
```

## Common Patterns

### Market Order Strategy

```json
{
  "action": {
    "kind": "buy",
    "symbol": "BTC",
    "sizing": { "mode": "cash", "value": 1000 },
    "order": { "type": "MARKET" }
  }
}
```

### Limit Order Strategy

```json
{
  "action": {
    "kind": "buy",
    "symbol": "BTC",
    "sizing": { "mode": "qty", "value": 0.1 },
    "order": {
      "type": "LIMIT",
      "limit": 50000.0,
      "tif": "GTC"
    }
  }
}
```

### RSI Mean Reversion

```json
{
  "entry": "start",
  "nodes": {
    "start": {
      "type": "condition",
      "expr": {
        "lhs": { "ref": "rsi.value" },
        "op": "lt",
        "rhs": { "const": 30 }
      },
      "true_branch": "buy",
      "false_branch": "end"
    },
    "buy": {
      "type": "action",
      "action": {
        "kind": "buy",
        "symbol": "BTC",
        "sizing": { "mode": "pct", "value": 10 },
        "order": { "type": "MARKET" }
      },
      "next": "end"
    },
    "end": { "type": "terminal" }
  },
  "indicators": [
    {
      "id": "rsi",
      "type": "RSI",
      "params": { "length": 14 },
      "lookback": 14
    }
  ]
}
```

## Understanding Results

### Key Metrics

- **final_equity**: Final portfolio value
- **total_return**: Absolute return in dollars
- **total_return_pct**: Percentage return
- **num_trades**: Number of trades executed
- **win_rate**: Percentage of profitable trades
- **max_drawdown**: Maximum peak-to-trough decline

### Trade Log

The trade log (`*_trades.csv`) contains:
- `timestamp`: When the trade executed
- `symbol`: Coin symbol
- `side`: BUY or SELL
- `size`: Trade size
- `price`: Execution price
- `fee`: Fee paid
- `order_id`: Order identifier

### Equity Curve

The equity curve (`*_equity.csv`) contains:
- `timestamp`: Time point
- `equity`: Total portfolio value
- `cash`: Cash balance
- `position_value`: Value of open position

## Troubleshooting

### "No events found in range"

- Check that event files exist in the events directory
- Verify timestamp range matches available data
- Ensure files are named correctly (format: `YYYYMMDD-HH.jsonl`)

### "Failed to fetch funding history"

- Check internet connection
- Verify coin symbol is correct
- Ensure timestamp range is valid (max 1 year)

### "Insufficient liquidity" warnings

- This is normal for market orders when book depth is low
- Orders will retry on next event if liquidity becomes available
- Consider using limit orders for better fill control

### Performance Issues

- Use `--io-concurrency` to limit parallel file reading
- Disable parallel indicators if you have only one indicator
- Consider reducing the time range for faster testing

## Next Steps

- Read the [API Documentation](API.md) for detailed function references
- Explore the [Architecture Overview](ARCHITECTURE.md) to understand the system
- Check [Performance Optimizations](PERFORMANCE_OPTIMIZATIONS.md) for tuning tips
- Review example strategies in `examples/strategies/`

## Getting Help

- Check the [API Documentation](API.md) for function details
- Review [Common Pitfalls](API.md#common-pitfalls) section
- Look at example strategies in the repository
- Review error messages - they include context about what went wrong

