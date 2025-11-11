# Strategy Examples

This directory contains example trading strategies in IR (Intermediate Representation) format for testing and validation.

## Strategies

### 1. SMA Crossover (`sma_crossover.json`)
- **Description**: Buy when fast SMA (10) crosses above slow SMA (20), sell when it crosses below
- **Indicators**: SMA(10), SMA(20)
- **Entry**: Fast SMA > Slow SMA
- **Exit**: Fast SMA < Slow SMA
- **Order Type**: Market orders

### 2. RSI Mean Reversion (`rsi_mean_reversion.json`)
- **Description**: Buy when RSI < 30 (oversold), sell when RSI > 70 (overbought)
- **Indicators**: RSI(14)
- **Entry**: RSI < 30
- **Exit**: RSI > 70
- **Order Type**: Market orders

### 3. MACD Crossover (`macd_crossover.json`)
- **Description**: Buy when MACD line crosses above signal line, sell when it crosses below
- **Indicators**: MACD(12, 26, 9)
- **Entry**: MACD > Signal
- **Exit**: MACD < Signal
- **Order Type**: Market orders

### 4. Bollinger Bands (`bollinger_bands.json`)
- **Description**: Buy when price touches lower band, sell when price touches upper band
- **Indicators**: BBands(20, 2.0)
- **Entry**: Close <= Lower Band
- **Exit**: Close >= Upper Band
- **Order Type**: Market orders

### 5. Simple Price Momentum (`simple_price_momentum.json`)
- **Description**: Simple momentum strategy using short-term SMA comparison
- **Indicators**: SMA(5), SMA(10)
- **Entry**: SMA(5) > SMA(10)
- **Exit**: SMA(5) < SMA(10)
- **Order Type**: Market orders
- **Note**: More likely to trigger frequently

## Testing

Run the E2E test script to validate all strategies:

```bash
./scripts/test_strategies_e2e.sh
```

Or test individual strategies:

```bash
./target/release/hl-backtest run-perps \
  --ir examples/strategies/sma_crossover.json \
  --coin BTC \
  --events data/events \
  --start 20251001-0 \
  --end 20251001-23 \
  --initial-capital 10000 \
  --maker-fee-bps=-1 \
  --taker-fee-bps=10 \
  --out scripts/results/sma_test.json
```

## Strategy Format

All strategies follow the IR (Intermediate Representation) format:
- **version**: IR version (currently "1.0")
- **scopes**: Array of trading scopes (instruments + indicators + graph)
- **graph**: Decision tree with conditions and actions
- **indicators**: Technical indicators used in the strategy

See the main [README.md](../../README.md) for more details on the IR format.

