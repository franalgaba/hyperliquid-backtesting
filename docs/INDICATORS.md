# Indicators Reference

Complete reference for all available technical indicators.

## Overview

Indicators are defined in the `indicators` array of your strategy:

```json
{
  "id": "my_indicator",      // Unique identifier
  "type": "RSI",             // Indicator type
  "params": { "period": 14 }, // Parameters
  "outputs": ["value"]       // Output names
}
```

---

## Trend Indicators

### SMA (Simple Moving Average)

Average of prices over a period.

```json
{
  "id": "sma_20",
  "type": "SMA",
  "params": { "period": 20 },
  "outputs": ["value"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| period | int | - | Lookback period |

**Output**: `value` - The moving average

---

### EMA (Exponential Moving Average)

Weighted average giving more weight to recent prices.

```json
{
  "id": "ema_12",
  "type": "EMA",
  "params": { "period": 12 },
  "outputs": ["value"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| period | int | - | Lookback period |

**Output**: `value` - The moving average

---

### MACD (Moving Average Convergence Divergence)

Trend-following momentum indicator.

```json
{
  "id": "macd",
  "type": "MACD",
  "params": { "fast": 12, "slow": 26, "signal": 9 },
  "outputs": ["value", "signal", "histogram"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| fast | int | 12 | Fast EMA period |
| slow | int | 26 | Slow EMA period |
| signal | int | 9 | Signal line period |

**Outputs**:
- `value` - MACD line (fast EMA - slow EMA)
- `signal` - Signal line (EMA of MACD)
- `histogram` - MACD - Signal

---

### ADX (Average Directional Index)

Measures trend strength (not direction).

```json
{
  "id": "adx",
  "type": "ADX",
  "params": { "period": 14 },
  "outputs": ["value"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| period | int | 14 | Lookback period |

**Output**: `value` - ADX value (0-100)
- < 20: Weak trend
- 20-40: Moderate trend
- > 40: Strong trend

---

## Momentum Indicators

### RSI (Relative Strength Index)

Measures overbought/oversold conditions.

```json
{
  "id": "rsi",
  "type": "RSI",
  "params": { "period": 14 },
  "outputs": ["value"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| period | int | 14 | Lookback period |

**Output**: `value` - RSI value (0-100)
- < 30: Oversold
- > 70: Overbought

---

### Stochastic

Compares closing price to price range.

```json
{
  "id": "stoch",
  "type": "Stochastic",
  "params": { "k_period": 14, "d_period": 3 },
  "outputs": ["k", "d"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| k_period | int | 14 | %K period |
| d_period | int | 3 | %D smoothing period |

**Outputs**:
- `k` - Fast stochastic (0-100)
- `d` - Slow stochastic (smoothed)

---

## Volatility Indicators

### Bollinger Bands

Price channels based on standard deviation.

```json
{
  "id": "bbands",
  "type": "BollingerBands",
  "params": { "period": 20, "std_dev": 2 },
  "outputs": ["upper", "middle", "lower"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| period | int | 20 | SMA period |
| std_dev | float | 2.0 | Standard deviation multiplier |

**Outputs**:
- `upper` - Upper band (middle + std_dev * σ)
- `middle` - Middle band (SMA)
- `lower` - Lower band (middle - std_dev * σ)

---

### ATR (Average True Range)

Measures volatility.

```json
{
  "id": "atr",
  "type": "ATR",
  "params": { "period": 14 },
  "outputs": ["value"]
}
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| period | int | 14 | Lookback period |

**Output**: `value` - Average true range in price units

---

## Volume Indicators

### OBV (On-Balance Volume)

Cumulative volume based on price direction.

```json
{
  "id": "obv",
  "type": "OBV",
  "params": {},
  "outputs": ["value"]
}
```

No parameters required.

**Output**: `value` - Cumulative OBV

---

## Using Indicator Outputs

### Single Output

```json
{ "indicator": "rsi", "op": "lt", "value": 30 }
```

### Named Output

```json
{ "indicator": "macd.histogram", "op": "gt", "value": 0 }
{ "indicator": "bbands.upper", "op": "gt", "value": 50000 }
```

### Crossover

```json
{
  "type": "crossover",
  "fast": "ema_12",
  "slow": "ema_26",
  "direction": "above"
}
```

---

## Indicator Combinations

### RSI + MACD

```json
{
  "indicators": [
    { "id": "rsi", "type": "RSI", "params": { "period": 14 }, "outputs": ["value"] },
    { "id": "macd", "type": "MACD", "params": { "fast": 12, "slow": 26, "signal": 9 }, "outputs": ["histogram"] }
  ],
  "entry": {
    "condition": {
      "type": "and",
      "conditions": [
        { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 30 },
        { "type": "threshold", "indicator": "macd.histogram", "op": "gt", "value": 0 }
      ]
    },
    "action": { "type": "buy", "size_pct": 100 }
  }
}
```

### Triple EMA

```json
{
  "indicators": [
    { "id": "ema_5", "type": "EMA", "params": { "period": 5 }, "outputs": ["value"] },
    { "id": "ema_13", "type": "EMA", "params": { "period": 13 }, "outputs": ["value"] },
    { "id": "ema_21", "type": "EMA", "params": { "period": 21 }, "outputs": ["value"] }
  ]
}
```

---

## Lookback Requirements

Each indicator requires historical data for calculation:

| Indicator | Minimum Lookback |
|-----------|------------------|
| SMA | period |
| EMA | period |
| RSI | period + 1 |
| MACD | slow + signal |
| Bollinger Bands | period |
| Stochastic | k_period |
| ATR | period |
| ADX | period * 2 |
| OBV | 1 |

Ensure your data range covers the required lookback before your trading period.
