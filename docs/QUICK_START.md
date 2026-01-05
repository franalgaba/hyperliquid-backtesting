# Quick Start Guide

Get up and running with the Hyperliquid Data Ingestor & Backtester in 5 minutes.

## Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- AWS credentials (for L2 data only)

## Installation

```bash
# Clone and build
git clone https://github.com/your-org/hyperliquid-backtesting.git
cd hyperliquid-backtesting
cargo build --release

# Add to PATH (optional)
export PATH="$PATH:$(pwd)/target/release"
```

## Step 1: Fetch Data

```bash
# Fetch BTC 1-hour candles for 2024
hl-backtest fetch \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30
```

Data is cached locally in `data/hyperliquid/BTC/1h.csv`.

## Step 2: Create a Strategy

Create `my_strategy.json`:

```json
{
  "name": "RSI Oversold",
  "instrument": {
    "symbol": "BTCUSD",
    "coin": "BTC",
    "venue": "HL",
    "timeframe": "1h"
  },
  "indicators": [
    {
      "id": "rsi",
      "type": "RSI",
      "params": { "period": 14 },
      "outputs": ["value"]
    }
  ],
  "entry": {
    "condition": {
      "type": "threshold",
      "indicator": "rsi",
      "op": "lt",
      "value": 30.0
    },
    "action": {
      "type": "buy",
      "size_pct": 100.0
    }
  },
  "exit": {
    "condition": {
      "type": "threshold",
      "indicator": "rsi",
      "op": "gt",
      "value": 70.0
    },
    "action": {
      "type": "close"
    }
  }
}
```

## Step 3: Run Backtest

```bash
hl-backtest run \
  --strategy my_strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --initial-capital 10000 \
  --out results.json
```

## Step 4: Analyze Results

Results are written to:
- `results.json` - Complete metrics
- `results_trades.csv` - Trade log
- `results_equity.csv` - Equity curve

### Key Metrics in results.json

```json
{
  "final_equity": 12500.0,
  "total_return_pct": 25.0,
  "num_trades": 42,
  "win_rate": 0.62,
  "max_drawdown_pct": 8.5,
  "sharpe_ratio": 1.8,
  "sortino_ratio": 2.1
}
```

### View Results

```bash
# View summary
cat results.json | jq '.final_equity, .total_return_pct, .num_trades'

# View trades
head -20 results_trades.csv

# View equity curve
head -20 results_equity.csv
```

## Optional: Export to Parquet

For efficient data analysis with Python:

```bash
# Export OHLC data to Parquet
hl-backtest fetch \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --parquet btc_1h.parquet

# Export backtest results to Parquet
hl-backtest run \
  --strategy my_strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --parquet-results ./results/
```

Then in Python:

```python
import pandas as pd

# Load OHLC data
candles = pd.read_parquet("btc_1h.parquet")

# Load backtest results
trades = pd.read_parquet("results/trades.parquet")
equity = pd.read_parquet("results/equity.parquet")

# Analyze
print(trades.describe())
equity.plot(x='timestamp', y='equity')
```

## Common Strategies

### SMA Crossover

```json
{
  "name": "SMA Crossover",
  "instrument": { "symbol": "BTCUSD", "coin": "BTC", "venue": "HL", "timeframe": "1h" },
  "indicators": [
    { "id": "sma_10", "type": "SMA", "params": { "period": 10 }, "outputs": ["value"] },
    { "id": "sma_50", "type": "SMA", "params": { "period": 50 }, "outputs": ["value"] }
  ],
  "entry": {
    "condition": { "type": "crossover", "fast": "sma_10", "slow": "sma_50", "direction": "above" },
    "action": { "type": "buy", "size_pct": 100.0 }
  },
  "exit": {
    "condition": { "type": "crossover", "fast": "sma_10", "slow": "sma_50", "direction": "below" },
    "action": { "type": "close" }
  }
}
```

### RSI with MACD Confirmation

```json
{
  "name": "RSI + MACD",
  "instrument": { "symbol": "BTCUSD", "coin": "BTC", "venue": "HL", "timeframe": "1h" },
  "indicators": [
    { "id": "rsi", "type": "RSI", "params": { "period": 14 }, "outputs": ["value"] },
    { "id": "macd", "type": "MACD", "params": { "fast": 12, "slow": 26, "signal": 9 }, "outputs": ["histogram"] }
  ],
  "entry": {
    "condition": {
      "type": "and",
      "conditions": [
        { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 35.0 },
        { "type": "threshold", "indicator": "macd.histogram", "op": "gt", "value": 0.0 }
      ]
    },
    "action": { "type": "buy", "size_pct": 100.0 }
  },
  "exit": {
    "condition": { "type": "threshold", "indicator": "rsi", "op": "gt", "value": 70.0 },
    "action": { "type": "close" }
  }
}
```

## Troubleshooting

### "No candles found"

- Check that you've run `fetch` for the asset and interval
- Verify the date range is valid
- Hyperliquid may not have data for very old dates

### "Unknown indicator"

- Check indicator type spelling (RSI, SMA, EMA, MACD, etc.)
- See [Indicators Reference](INDICATORS.md) for available indicators

### "Failed to parse strategy JSON"

- Validate JSON syntax
- Ensure all required fields are present
- Check that condition/action types are valid

## Next Steps

- [Write complex strategies](STRATEGIES.md)
- [Use all available indicators](INDICATORS.md)
- [Ingest L2 order book data](DATA_INGESTION.md)
- [Export to Parquet for analysis](PARQUET.md)
