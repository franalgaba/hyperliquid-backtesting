# Backtesting Guide

How to run backtests and interpret results.

## Overview

The backtester supports two modes:

| Mode | Data Source | Use Case |
|------|-------------|----------|
| **Candle-based** | OHLC candles | Fast testing |
| **L2 Event-driven** | Order book snapshots | Realistic fills |

---

## Candle-Based Backtesting

Uses OHLC candle data for fast strategy testing.

### How It Works

```
Load Strategy → Compile Indicators → Load Candles
                                         ↓
                                   Warm-up Phase
                                         ↓
┌─────────────────────────────────────────────────┐
│              Main Loop (per candle)              │
│                                                  │
│  1. Update indicators with candle data           │
│  2. Get current indicator values                 │
│  3. If FLAT: evaluate entry condition            │
│     → If true: create BUY order                  │
│  4. If IN POSITION: evaluate exit condition      │
│     → If true: create CLOSE order                │
│  5. Execute pending orders                       │
│  6. Update portfolio                             │
│  7. Record equity                                │
└─────────────────────────────────────────────────┘
                                         ↓
                                   Output Results
```

### Running

```bash
hl-backtest run \
  --strategy strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --initial-capital 10000 \
  --out results.json
```

### Limitations

- Fills at candle close price
- No order book depth simulation
- No partial fills

---

## L2 Event-Driven Backtesting

Uses actual order book snapshots for realistic simulation.

### How It Works

1. **Order Book Reconstruction**: Each L2 snapshot updates a full order book
2. **Realistic Fills**: Market orders sweep the book; limit orders fill when price crosses
3. **Funding Payments**: Applied every 8 hours based on position and funding rate
4. **Maker/Taker Fees**: Correctly applied based on order type

### Running

```bash
# First, get L2 data
hl-backtest ingest s3 --coin BTC --start 20240101 --end 20240107
hl-backtest ingest build-events --coin BTC --input data/s3

# Then run backtest
hl-backtest run-perps \
  --strategy strategy.json \
  --coin BTC \
  --events data/events \
  --start 20240101-00 \
  --end 20240107-23 \
  --initial-capital 10000
```

### Order Execution

| Order Type | Execution |
|------------|-----------|
| Market | Sweeps order book, may have slippage |
| Limit | Fills when price crosses limit price |

---

## Configuration

### Fees

| Parameter | Description | Default |
|-----------|-------------|---------|
| `--maker-fee-bps` | Maker fee (negative = rebate) | -1 |
| `--taker-fee-bps` | Taker fee | 10 |
| `--slippage-bps` | Slippage simulation | 5 |

**Note**: 1 basis point = 0.01%

### Trade Cooldown

Prevents excessive trading:

```bash
--trade-cooldown-min 30  # 30 minutes between trades
```

Default: 15 minutes for entry signals (exits bypass cooldown)

---

## Results

### Output Files

| File | Content |
|------|---------|
| `results.json` | Complete metrics |
| `results_trades.csv` | Trade log |
| `results_equity.csv` | Equity curve |

### Parquet Export

```bash
--parquet-results ./results/
```

Creates:
- `./results/trades.parquet`
- `./results/equity.parquet`

---

## Metrics

### Performance Metrics

| Metric | Description |
|--------|-------------|
| `final_equity` | Final portfolio value |
| `total_return` | Absolute return ($) |
| `total_return_pct` | Percentage return |
| `num_trades` | Number of trades |

### Risk Metrics

| Metric | Description |
|--------|-------------|
| `win_rate` | % of profitable trades |
| `avg_win` | Average winning trade |
| `avg_loss` | Average losing trade |
| `max_drawdown` | Maximum peak-to-trough decline ($) |
| `max_drawdown_pct` | Maximum drawdown (%) |
| `sharpe_ratio` | Risk-adjusted return (annualized) |
| `sortino_ratio` | Downside risk-adjusted return |

### Interpreting Metrics

| Metric | Good | Bad |
|--------|------|-----|
| Sharpe | > 1.5 | < 0.5 |
| Sortino | > 2.0 | < 1.0 |
| Win Rate | > 50% | < 40% |
| Max Drawdown | < 20% | > 40% |

---

## Best Practices

### Data

1. **Use sufficient history**: Cover multiple market conditions
2. **Check for gaps**: Ensure continuous data
3. **Validate data quality**: Check for outliers

### Strategy

1. **Start simple**: Add complexity gradually
2. **Avoid overfitting**: Don't over-optimize parameters
3. **Test out-of-sample**: Use walk-forward analysis

### Execution

1. **Account for fees**: Use realistic fee assumptions
2. **Consider slippage**: Especially for large orders
3. **Include cooldowns**: Prevent excessive trading

---

## Troubleshooting

### "No candles found"

- Run `fetch` first
- Check date range
- Verify asset symbol

### "Not enough candles"

- Need enough data for indicator warmup
- Increase date range

### "No events found in range"

- Check events directory exists
- Verify date-hour format (YYYYMMDD-HH)

### Poor Performance

1. Check fee assumptions
2. Review indicator parameters
3. Analyze trade log for patterns
4. Check if strategy matches market conditions

---

## Example Analysis

### Python Analysis

```python
import pandas as pd
import matplotlib.pyplot as plt

# Load results
trades = pd.read_parquet("results/trades.parquet")
equity = pd.read_parquet("results/equity.parquet")

# Convert timestamps
equity['datetime'] = pd.to_datetime(equity['timestamp'], unit='ms')

# Plot equity curve
plt.figure(figsize=(12, 6))
plt.plot(equity['datetime'], equity['equity'])
plt.title('Equity Curve')
plt.xlabel('Date')
plt.ylabel('Equity ($)')
plt.grid(True)
plt.show()

# Trade analysis
print(f"Total trades: {len(trades)}")
print(f"Win rate: {(trades['side'] == 'SELL').mean():.2%}")
print(f"Average trade size: ${trades['size'].mean() * trades['price'].mean():,.2f}")
```
