# Parquet Export Guide

Export data to Apache Parquet format for efficient analysis.

## Why Parquet?

| Feature | CSV | Parquet |
|---------|-----|---------|
| File size | Large | ~10x smaller |
| Read speed | Slow | Fast (columnar) |
| Type safety | No | Yes |
| Compression | None | Snappy/Zstd |
| Python/R support | Good | Excellent |

---

## Exporting OHLC Data

### During Fetch

```bash
hl-backtest fetch \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --parquet btc_1h.parquet
```

### From Cache

```bash
hl-backtest export \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --out btc_1h.parquet
```

### Schema

| Column | Type | Description |
|--------|------|-------------|
| time_open | UInt64 | Open timestamp (Unix ms) |
| time_close | UInt64 | Close timestamp (Unix ms) |
| coin | String | Asset symbol |
| interval | String | Timeframe |
| open | Float64 | Open price |
| high | Float64 | High price |
| low | Float64 | Low price |
| close | Float64 | Close price |
| volume | Float64 | Volume |
| num_trades | Int64 | Trade count |

---

## Exporting Backtest Results

### Enable Export

```bash
hl-backtest run \
  --strategy strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --parquet-results ./results/
```

Creates:
- `./results/trades.parquet`
- `./results/equity.parquet`

### Trades Schema

| Column | Type | Description |
|--------|------|-------------|
| timestamp | UInt64 | Fill timestamp (Unix ms) |
| symbol | String | Asset symbol |
| side | String | "BUY" or "SELL" |
| size | Float64 | Fill size |
| price | Float64 | Fill price |
| fee | Float64 | Fee paid |
| order_id | UInt64 | Order ID |

### Equity Schema

| Column | Type | Description |
|--------|------|-------------|
| timestamp | UInt64 | Timestamp (Unix ms) |
| equity | Float64 | Total equity |
| cash | Float64 | Cash balance |
| position_value | Float64 | Position value |

---

## Using in Python

### pandas

```python
import pandas as pd

# Load OHLC data
candles = pd.read_parquet("btc_1h.parquet")

# Load backtest results
trades = pd.read_parquet("results/trades.parquet")
equity = pd.read_parquet("results/equity.parquet")

# Convert timestamps
candles['datetime'] = pd.to_datetime(candles['time_open'], unit='ms')
equity['datetime'] = pd.to_datetime(equity['timestamp'], unit='ms')

# Analyze
print(candles.describe())
print(trades.groupby('side')['size'].sum())

# Plot equity curve
equity.plot(x='datetime', y='equity', figsize=(12, 6))
```

### polars

```python
import polars as pl

# Load data
candles = pl.read_parquet("btc_1h.parquet")
trades = pl.read_parquet("results/trades.parquet")

# Fast analysis
print(candles.describe())
print(trades.group_by("side").agg(pl.col("size").sum()))
```

### DuckDB

```python
import duckdb

# Query directly
result = duckdb.query("""
    SELECT
        date_trunc('day', to_timestamp(time_open/1000)) as date,
        avg(close) as avg_close,
        sum(volume) as total_volume
    FROM 'btc_1h.parquet'
    GROUP BY 1
    ORDER BY 1
""").df()
```

---

## Using in R

```r
library(arrow)

# Load data
candles <- read_parquet("btc_1h.parquet")
trades <- read_parquet("results/trades.parquet")

# Analyze
summary(candles)
```

---

## Compression

Files are compressed with Snappy by default:
- Fast compression/decompression
- Good compression ratio (~3-5x)
- Wide compatibility

---

## Best Practices

1. **Use Parquet for analysis**: More efficient than CSV
2. **Keep CSV for inspection**: Human-readable backup
3. **Partition large datasets**: By date/month for faster queries
4. **Use column projection**: Read only needed columns

```python
# Read only specific columns
df = pd.read_parquet("btc_1h.parquet", columns=['time_open', 'close'])
```
