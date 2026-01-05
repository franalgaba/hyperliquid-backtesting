# Data Ingestion Guide

This guide covers how to fetch and manage market data from Hyperliquid.

## Overview

The backtester supports two types of data:

| Type | Source | Use Case |
|------|--------|----------|
| **OHLC Candles** | Hyperliquid API | Standard backtesting |
| **L2 Order Book** | Hyperliquid S3 Archive | Realistic fill simulation |

---

## OHLC Candle Data

### Fetching Candles

```bash
hl-backtest fetch \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30
```

### Parameters

| Parameter | Description | Example |
|-----------|-------------|---------|
| `--asset` | Asset symbol | BTC, ETH, SOL, HYPE |
| `--interval` | Candle interval | 1m, 5m, 15m, 1h, 4h, 1d, 1w |
| `--start` | Start date | 2024-01-01 |
| `--end` | End date | 2024-06-30 |
| `--parquet` | Export to Parquet (optional) | data/btc.parquet |

### Cache Location

Candles are cached at:
```
data/hyperliquid/{ASSET}/{INTERVAL}.csv
```

Example: `data/hyperliquid/BTC/1h.csv`

### Export to Parquet

```bash
# During fetch
hl-backtest fetch \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --parquet btc_1h.parquet

# From cache
hl-backtest export \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --out btc_1h.parquet
```

### Data Format

CSV columns:
```
time_open,time_close,coin,interval,open,high,low,close,volume,num_trades
1704067200000,1704070800000,BTC,1h,42000.0,42500.0,41800.0,42300.0,150.5,1234
```

Parquet schema:
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

## L2 Order Book Data

For realistic backtesting with actual order book fills.

### Prerequisites

1. **AWS Credentials**: Configure via `aws configure` or environment variables
2. **Cost**: Data transfer costs ~$0.09/GB (requester pays)

### Step 1: Download from S3

```bash
hl-backtest ingest s3 \
  --coin BTC \
  --start 20240101 \
  --start-hour 0 \
  --end 20240101 \
  --end-hour 23 \
  --out data/s3
```

This downloads LZ4-compressed files from:
```
s3://hyperliquid-archive/market_data/{YYYYMMDD}/{H}/l2Book/{COIN}.lz4
```

### Step 2: Build Events

Convert LZ4 files to JSONL events:

```bash
hl-backtest ingest build-events \
  --coin BTC \
  --input data/s3 \
  --out data/events
```

Creates files like: `data/events/BTC/20240101-00.jsonl`

### Step 3: Run Perps Backtest

```bash
hl-backtest run-perps \
  --strategy strategy.json \
  --coin BTC \
  --events data/events \
  --start 20240101-00 \
  --end 20240101-23 \
  --initial-capital 10000 \
  --out results.json
```

### L2 Event Format

Each event contains a full order book snapshot:

```json
{
  "ts_ms": 1704067200000,
  "levels": [
    [  // Bids
      {"px": "42000.0", "sz": "1.5", "n": 3},
      {"px": "41999.0", "sz": "2.0", "n": 5}
    ],
    [  // Asks
      {"px": "42001.0", "sz": "1.0", "n": 2},
      {"px": "42002.0", "sz": "3.0", "n": 4}
    ]
  ]
}
```

---

## Supported Assets

| Asset | Symbol |
|-------|--------|
| Bitcoin | BTC |
| Ethereum | ETH |
| Solana | SOL |
| Hyperliquid | HYPE |

---

## Data Availability

### OHLC Data
- Available via Hyperliquid API
- May have gaps for older dates
- Check [Hyperliquid Docs](https://hyperliquid.gitbook.io/hyperliquid-docs/historical-data)

### L2 Data
- Available from Hyperliquid S3 archive
- May have gaps (no guarantee of timely updates)
- Requires AWS credentials

---

## Troubleshooting

### "No candles found"

1. Check asset symbol is supported
2. Verify date range is valid
3. Try a more recent date range

### S3 Download Fails

1. Check AWS credentials: `aws sts get-caller-identity`
2. Verify bucket access
3. Check network connectivity

### Large Data Volumes

For large date ranges:
1. Download in chunks
2. Use `--io-concurrency` to control parallelism
3. Monitor disk space

---

## Best Practices

1. **Cache data locally**: Avoid repeated API calls
2. **Use Parquet for analysis**: More efficient than CSV
3. **Download L2 data overnight**: Large transfers take time
4. **Validate data**: Check for gaps before backtesting
