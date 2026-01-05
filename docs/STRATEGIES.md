# Strategy Guide

This guide covers how to write and configure trading strategies for the backtester.

## Strategy Structure

A strategy is defined in JSON with the following structure:

```json
{
  "name": "Strategy Name",
  "instrument": { ... },
  "indicators": [ ... ],
  "entry": { ... },
  "exit": { ... }
}
```

## Fields

### name (required)
A descriptive name for your strategy.

### instrument (required)

Defines the trading instrument:

```json
{
  "symbol": "BTCUSD",    // Trading pair
  "coin": "BTC",         // Base asset
  "venue": "HL",         // Exchange (always "HL" for Hyperliquid)
  "timeframe": "1h"      // Candle interval
}
```

**Supported timeframes**: `1m`, `5m`, `15m`, `1h`, `4h`, `1d`, `1w`

### indicators (required)

List of technical indicators to compute:

```json
{
  "id": "rsi_14",           // Unique identifier
  "type": "RSI",            // Indicator type
  "params": { "period": 14 }, // Parameters
  "outputs": ["value"]      // Output names
}
```

See [Indicators Reference](INDICATORS.md) for all available indicators.

### entry (required)

Rule for entering a position:

```json
{
  "condition": { ... },   // When to enter
  "action": { ... }       // What to do
}
```

### exit (optional)

Rule for exiting a position:

```json
{
  "condition": { ... },   // When to exit
  "action": { ... }       // What to do
}
```

If no exit rule is defined, positions are held indefinitely.

---

## Conditions

Conditions determine when actions are triggered.

### Threshold

Compare an indicator value to a constant:

```json
{
  "type": "threshold",
  "indicator": "rsi_14",   // Indicator ID
  "op": "lt",              // Comparison operator
  "value": 30.0            // Threshold value
}
```

**Operators**:
| Op | Meaning |
|----|---------|
| `lt` | Less than |
| `lte` | Less than or equal |
| `eq` | Equal |
| `ne` | Not equal |
| `gte` | Greater than or equal |
| `gt` | Greater than |

### Crossover

Detect when one indicator crosses another:

```json
{
  "type": "crossover",
  "fast": "sma_10",        // Fast indicator ID
  "slow": "sma_50",        // Slow indicator ID
  "direction": "above"     // "above" or "below"
}
```

- `above`: Fast crosses above slow (bullish)
- `below`: Fast crosses below slow (bearish)

### And (Logical AND)

All conditions must be true:

```json
{
  "type": "and",
  "conditions": [
    { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 30 },
    { "type": "threshold", "indicator": "macd.histogram", "op": "gt", "value": 0 }
  ]
}
```

### Or (Logical OR)

At least one condition must be true:

```json
{
  "type": "or",
  "conditions": [
    { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 20 },
    { "type": "threshold", "indicator": "rsi", "op": "gt", "value": 80 }
  ]
}
```

---

## Actions

Actions define what to do when conditions are met.

### Buy

Buy with a percentage of available capital:

```json
{
  "type": "buy",
  "size_pct": 100.0   // 100% of capital
}
```

### Sell

Sell a percentage of the current position:

```json
{
  "type": "sell",
  "size_pct": 50.0    // Sell 50% of position
}
```

### Close

Close the entire position:

```json
{
  "type": "close"
}
```

---

## Indicator Outputs

Some indicators produce multiple outputs. Access them with dot notation:

```json
// MACD outputs: value, signal, histogram
{ "indicator": "macd.histogram", "op": "gt", "value": 0 }

// Bollinger Bands outputs: upper, middle, lower
{ "indicator": "bbands.upper", "op": "lt", "value": 50000 }
```

---

## Example Strategies

### RSI Mean Reversion

Buy when oversold, sell when overbought:

```json
{
  "name": "RSI Mean Reversion",
  "instrument": { "symbol": "BTCUSD", "coin": "BTC", "venue": "HL", "timeframe": "1h" },
  "indicators": [
    { "id": "rsi", "type": "RSI", "params": { "period": 14 }, "outputs": ["value"] }
  ],
  "entry": {
    "condition": { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 30 },
    "action": { "type": "buy", "size_pct": 100 }
  },
  "exit": {
    "condition": { "type": "threshold", "indicator": "rsi", "op": "gt", "value": 70 },
    "action": { "type": "close" }
  }
}
```

### Moving Average Crossover

Buy on golden cross, sell on death cross:

```json
{
  "name": "MA Crossover",
  "instrument": { "symbol": "BTCUSD", "coin": "BTC", "venue": "HL", "timeframe": "4h" },
  "indicators": [
    { "id": "ema_12", "type": "EMA", "params": { "period": 12 }, "outputs": ["value"] },
    { "id": "ema_26", "type": "EMA", "params": { "period": 26 }, "outputs": ["value"] }
  ],
  "entry": {
    "condition": { "type": "crossover", "fast": "ema_12", "slow": "ema_26", "direction": "above" },
    "action": { "type": "buy", "size_pct": 100 }
  },
  "exit": {
    "condition": { "type": "crossover", "fast": "ema_12", "slow": "ema_26", "direction": "below" },
    "action": { "type": "close" }
  }
}
```

### Bollinger Band Breakout

Buy when price breaks above upper band:

```json
{
  "name": "BB Breakout",
  "instrument": { "symbol": "ETHUSD", "coin": "ETH", "venue": "HL", "timeframe": "1h" },
  "indicators": [
    { "id": "bb", "type": "BollingerBands", "params": { "period": 20, "std_dev": 2 }, "outputs": ["upper", "middle", "lower"] },
    { "id": "rsi", "type": "RSI", "params": { "period": 14 }, "outputs": ["value"] }
  ],
  "entry": {
    "condition": {
      "type": "and",
      "conditions": [
        { "type": "threshold", "indicator": "rsi", "op": "gt", "value": 50 },
        { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 70 }
      ]
    },
    "action": { "type": "buy", "size_pct": 100 }
  },
  "exit": {
    "condition": { "type": "threshold", "indicator": "rsi", "op": "gt", "value": 80 },
    "action": { "type": "close" }
  }
}
```

### Multi-Indicator Confirmation

Enter only when multiple indicators agree:

```json
{
  "name": "Multi-Indicator",
  "instrument": { "symbol": "BTCUSD", "coin": "BTC", "venue": "HL", "timeframe": "1h" },
  "indicators": [
    { "id": "rsi", "type": "RSI", "params": { "period": 14 }, "outputs": ["value"] },
    { "id": "macd", "type": "MACD", "params": { "fast": 12, "slow": 26, "signal": 9 }, "outputs": ["histogram"] },
    { "id": "adx", "type": "ADX", "params": { "period": 14 }, "outputs": ["value"] }
  ],
  "entry": {
    "condition": {
      "type": "and",
      "conditions": [
        { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 40 },
        { "type": "threshold", "indicator": "macd.histogram", "op": "gt", "value": 0 },
        { "type": "threshold", "indicator": "adx", "op": "gt", "value": 25 }
      ]
    },
    "action": { "type": "buy", "size_pct": 100 }
  },
  "exit": {
    "condition": {
      "type": "or",
      "conditions": [
        { "type": "threshold", "indicator": "rsi", "op": "gt", "value": 75 },
        { "type": "threshold", "indicator": "macd.histogram", "op": "lt", "value": 0 }
      ]
    },
    "action": { "type": "close" }
  }
}
```

---

## Tips

1. **Start simple**: Begin with one indicator and add complexity gradually
2. **Use lookback**: Ensure you have enough data for indicator warmup
3. **Test thoroughly**: Run backtests on different time periods
4. **Check fees**: Consider maker/taker fees in your strategy
5. **Avoid overfitting**: Don't optimize parameters too specifically to historical data
