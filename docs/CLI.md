# CLI Reference

Complete command-line interface reference for `hl-backtest`.

## Synopsis

```bash
hl-backtest <COMMAND> [OPTIONS]
```

## Commands

| Command | Description |
|---------|-------------|
| `fetch` | Fetch and cache historical candle data |
| `export` | Export cached data to Parquet format |
| `run` | Run a backtest on candle data |
| `run-perps` | Run a perps backtest on L2 events |
| `ingest s3` | Download L2 data from S3 |
| `ingest build-events` | Convert L2 files to events |

---

## fetch

Fetch and cache historical candle data from Hyperliquid API.

```bash
hl-backtest fetch [OPTIONS]
```

### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `--asset` | Yes | - | Asset symbol (BTC, ETH, SOL, HYPE) |
| `--interval` | Yes | - | Timeframe (1m, 5m, 15m, 1h, 4h, 1d, 1w) |
| `--start` | Yes | - | Start date (YYYY-MM-DD) |
| `--end` | Yes | - | End date (YYYY-MM-DD) |
| `--parquet` | No | - | Export to Parquet file |

### Examples

```bash
# Basic fetch
hl-backtest fetch --asset BTC --interval 1h --start 2024-01-01 --end 2024-06-30

# Fetch and export to Parquet
hl-backtest fetch --asset ETH --interval 4h --start 2024-01-01 --end 2024-06-30 --parquet eth_4h.parquet
```

---

## export

Export cached data to Parquet format.

```bash
hl-backtest export [OPTIONS]
```

### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `--asset` | Yes | - | Asset symbol |
| `--interval` | Yes | - | Timeframe |
| `--start` | Yes | - | Start date (YYYY-MM-DD) |
| `--end` | Yes | - | End date (YYYY-MM-DD) |
| `--out` | Yes | - | Output Parquet file path |

### Example

```bash
hl-backtest export --asset BTC --interval 1h --start 2024-01-01 --end 2024-06-30 --out btc_1h.parquet
```

---

## run

Run a backtest on candle data.

```bash
hl-backtest run [OPTIONS]
```

### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `--strategy` | Yes | - | Path to strategy JSON file |
| `--asset` | Yes | - | Asset symbol |
| `--interval` | Yes | - | Timeframe |
| `--start` | Yes | - | Start date (YYYY-MM-DD) |
| `--end` | Yes | - | End date (YYYY-MM-DD) |
| `--initial-capital` | No | 10000.0 | Initial capital in USDC |
| `--maker-fee-bps` | No | -1 | Maker fee in basis points |
| `--taker-fee-bps` | No | 10 | Taker fee in basis points |
| `--slippage-bps` | No | 5 | Slippage in basis points |
| `--out` | No | results.json | Output JSON file |
| `--parquet-results` | No | - | Export results to Parquet directory |

### Examples

```bash
# Basic backtest
hl-backtest run \
  --strategy strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30

# With custom fees and Parquet export
hl-backtest run \
  --strategy strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --initial-capital 50000 \
  --maker-fee-bps 0 \
  --taker-fee-bps 5 \
  --parquet-results ./results/
```

---

## run-perps

Run a perps backtest using L2 order book events.

```bash
hl-backtest run-perps [OPTIONS]
```

### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `--strategy` | Yes | - | Path to strategy JSON file |
| `--coin` | Yes | - | Coin symbol |
| `--events` | Yes | - | Path to events directory |
| `--start` | Yes | - | Start date-hour (YYYYMMDD-HH) |
| `--end` | Yes | - | End date-hour (YYYYMMDD-HH) |
| `--initial-capital` | No | 10000.0 | Initial capital in USDC |
| `--maker-fee-bps` | No | -1 | Maker fee in basis points |
| `--taker-fee-bps` | No | 10 | Taker fee in basis points |
| `--io-concurrency` | No | auto | Parallel file loading |
| `--indicators-par` | No | auto | Parallel indicator updates |
| `--trade-cooldown-min` | No | 15 | Cooldown between trades (minutes) |
| `--out` | No | results.json | Output JSON file |
| `--parquet-results` | No | - | Export results to Parquet directory |

### Example

```bash
hl-backtest run-perps \
  --strategy strategy.json \
  --coin BTC \
  --events data/events \
  --start 20240101-00 \
  --end 20240101-23 \
  --initial-capital 10000 \
  --trade-cooldown-min 30 \
  --parquet-results ./results/
```

---

## ingest s3

Download L2 order book data from Hyperliquid S3 archive.

```bash
hl-backtest ingest s3 [OPTIONS]
```

### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `--coin` | Yes | - | Coin symbol |
| `--start` | Yes | - | Start date (YYYYMMDD) |
| `--start-hour` | No | 0 | Start hour (0-23) |
| `--end` | Yes | - | End date (YYYYMMDD) |
| `--end-hour` | No | 23 | End hour (0-23) |
| `--out` | No | data/s3 | Output directory |

### Example

```bash
hl-backtest ingest s3 \
  --coin BTC \
  --start 20240101 \
  --start-hour 9 \
  --end 20240101 \
  --end-hour 17 \
  --out data/s3
```

---

## ingest build-events

Convert downloaded L2 files to event format.

```bash
hl-backtest ingest build-events [OPTIONS]
```

### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `--coin` | Yes | - | Coin symbol |
| `--input` | Yes | - | Input directory with .lz4 files |
| `--out` | No | data/events | Output directory for events |

### Example

```bash
hl-backtest ingest build-events \
  --coin BTC \
  --input data/s3 \
  --out data/events
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `AWS_ACCESS_KEY_ID` | AWS access key for S3 |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key for S3 |
| `AWS_REGION` | AWS region (default: us-east-1) |

---

## Examples

### Complete Workflow

```bash
# 1. Fetch data
hl-backtest fetch --asset BTC --interval 1h --start 2024-01-01 --end 2024-06-30

# 2. Run backtest
hl-backtest run \
  --strategy my_strategy.json \
  --asset BTC \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --parquet-results ./results/

# 3. Analyze in Python
python -c "import pandas as pd; print(pd.read_parquet('results/trades.parquet'))"
```

### L2 Workflow

```bash
# 1. Download L2 data
hl-backtest ingest s3 --coin BTC --start 20240101 --end 20240107

# 2. Build events
hl-backtest ingest build-events --coin BTC --input data/s3

# 3. Run perps backtest
hl-backtest run-perps \
  --strategy my_strategy.json \
  --coin BTC \
  --events data/events \
  --start 20240101-00 \
  --end 20240107-23
```
