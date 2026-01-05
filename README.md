# Hyperliquid Data Ingestor & Backtester

High-performance Rust tool for fetching OHLC data from Hyperliquid and backtesting trading strategies.

## Features

- Fetch OHLC candle data for any Hyperliquid token pair
- Export data to **Parquet** format for efficient data analysis
- L2 order book data ingestion from Hyperliquid S3 archive
- Strategy backtesting with realistic order book fills
- Built-in technical indicators (RSI, SMA, EMA, MACD, Bollinger Bands, etc.)
- Configurable fees, slippage, and funding payments

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/hl-backtest`.

## Quick Start

### Fetch OHLC Data

```bash
# Fetch and cache BTC 1-hour candles
hl-backtest fetch \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-12-31

# Fetch and export directly to Parquet
hl-backtest fetch \
  --asset ETH \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --parquet data/eth_1h.parquet
```

### Export to Parquet

```bash
hl-backtest export \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-12-31 \
  --out data/btc_1h.parquet
```

### Run Backtest

```bash
hl-backtest run \
  --strategy examples/strategies/rsi_strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --initial-capital 10000 \
  --out results.json \
  --parquet-results ./results/   # Optional: export trades & equity to Parquet
```

## Strategy Format

Strategies are defined in a simple JSON format:

```json
{
  "name": "RSI Oversold Strategy",
  "instrument": {
    "symbol": "BTCUSD",
    "coin": "BTC",
    "venue": "HL",
    "timeframe": "1h"
  },
  "indicators": [
    {
      "id": "rsi_14",
      "type": "RSI",
      "params": { "period": 14 },
      "outputs": ["value"]
    }
  ],
  "entry": {
    "condition": {
      "type": "threshold",
      "indicator": "rsi_14",
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
      "indicator": "rsi_14",
      "op": "gt",
      "value": 70.0
    },
    "action": {
      "type": "close"
    }
  }
}
```

### Condition Types

- **threshold**: Compare indicator to a value (`lt`, `lte`, `eq`, `ne`, `gte`, `gt`)
- **crossover**: Detect when fast indicator crosses above/below slow indicator
- **and**: Logical AND of multiple conditions
- **or**: Logical OR of multiple conditions

### Actions

- **buy**: Buy with percentage of capital (`size_pct: 100.0`)
- **sell**: Sell percentage of position
- **close**: Close entire position

## L2 Order Book Data

For realistic backtesting with order book fills:

### Download L2 Data from S3

```bash
hl-backtest ingest s3 \
  --coin BTC \
  --start 20240101 \
  --end 20240131 \
  --out data/s3
```

**Note**: Requires AWS credentials and pays for data transfer (~$0.09/GB).

### Build Events

```bash
hl-backtest ingest build-events \
  --coin BTC \
  --input data/s3 \
  --out data/events
```

### Run Perps Backtest

```bash
hl-backtest run-perps \
  --strategy examples/strategies/rsi_strategy.json \
  --coin BTC \
  --events data/events \
  --start 20240101-00 \
  --end 20240131-23 \
  --initial-capital 10000 \
  --out results.json
```

## Available Indicators

| Indicator | Type | Parameters |
|-----------|------|------------|
| RSI | Momentum | `period` |
| SMA | Trend | `period` |
| EMA | Trend | `period` |
| MACD | Trend | `fast`, `slow`, `signal` |
| Bollinger Bands | Volatility | `period`, `std_dev` |
| Stochastic | Momentum | `k_period`, `d_period` |
| ATR | Volatility | `period` |
| ADX | Trend | `period` |
| OBV | Volume | - |

## Output Files

### JSON/CSV (default)
- `results.json`: Complete backtest results with metrics
- `results_trades.csv`: Trade log
- `results_equity.csv`: Equity curve

### Parquet (with `--parquet-results`)
- `trades.parquet`: Trade fills (timestamp, symbol, side, size, price, fee, order_id)
- `equity.parquet`: Equity curve (timestamp, equity, cash, position_value)

Use Parquet for efficient data analysis with pandas, polars, or DuckDB:

```python
import pandas as pd

trades = pd.read_parquet("results/trades.parquet")
equity = pd.read_parquet("results/equity.parquet")
```

## Supported Assets

Currently supported: BTC, ETH, SOL, HYPE

## Development

```bash
# Run tests
cargo test

# Generate documentation
cargo doc --open

# Build release
cargo build --release
```

## License

MIT
