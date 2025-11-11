# Sample Strategies

This directory contains sample strategy IR JSON files for testing the backtester.

## Strategies

### 1. RSI Strategy (`sample_strategy.json`)

A simple RSI-based mean reversion strategy:
- **Buy**: When RSI(14) < 30 (oversold)
- **Sell**: When RSI(14) > 70 (overbought)
- **Order Type**: Market orders
- **Sizing**: $1000 cash for buys, 100% of position for sells

**Usage:**
```bash
hl-backtest run \
  --ir examples/sample_strategy.json \
  --asset ETH \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --initial-capital 10000 \
  --out rsi_results.json
```

### 2. SMA Crossover Strategy (`sma_crossover_strategy.json`)

A classic moving average crossover strategy:
- **Buy**: When SMA(10) crosses above SMA(20) (golden cross)
- **Sell**: When SMA(10) crosses below SMA(20) (death cross)
- **Order Type**: Limit orders for buys, Market for sells
- **Sizing**: $2000 cash for buys, 100% of position for sells

**Usage:**
```bash
hl-backtest run \
  --ir examples/sma_crossover_strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --initial-capital 10000 \
  --out sma_results.json
```

## Testing Workflow

1. **Fetch data first** (or use test data):
```bash
# Option 1: Try fetching from API (may not work for historical dates)
hl-backtest fetch --asset ETH --interval 1h --start 2024-01-01 --end 2024-06-30

# Option 2: Generate test data (recommended for testing)
python3 examples/generate_test_data.py
```

**Note**: Hyperliquid doesn't provide historical candle data for spot markets. See [their docs](https://hyperliquid.gitbook.io/hyperliquid-docs/historical-data). For testing, use the test data generator.

2. **Run backtest:**
```bash
hl-backtest run \
  --ir examples/sample_strategy.json \
  --asset ETH \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --initial-capital 10000 \
  --out results.json
```

3. **Check results:**
- `results.json`: Full metrics and summary
- `results_trades.csv`: All executed trades
- `results_equity.csv`: Equity curve over time

## Notes

- Make sure the asset symbol in the strategy matches the `--asset` parameter
- The timeframe in the strategy should match the `--interval` parameter
- For limit orders, if the limit price is set to 0.0 or not provided, it will use the current candle close price
- The strategies use simple logic for demonstration - you can modify them to add more sophisticated conditions

## Quick Test

Run the test script to execute both strategies:

```bash
cd backtester
./examples/test_script.sh
```

This will:
1. Fetch historical data for ETH and BTC
2. Run both strategy backtests
3. Generate result files for analysis

