# Hyperliquid Backtester

High-performance Rust backtester for Hyperliquid trading strategies.

**This is a Rust Cargo package** - see the [main README](../README.md) for project structure.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/hl-backtest`.

## Usage

### Fetch Historical Data

```bash
hl-backtest fetch \
  --asset ETH \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30
```

This downloads and caches candle data to `data/hyperliquid/{asset}/{interval}.csv`.

**Note**: Hyperliquid does not provide historical candle data for spot markets via their API (see [their documentation](https://hyperliquid.gitbook.io/hyperliquid-docs/historical-data)). The `fetch` command will attempt to query the API, but may return empty results for older dates. You may need to:
- Use the API to continuously record data yourself
- Use mock/test data (see `examples/generate_test_data.py`)
- Query only recent dates where data is available

### Run Backtest

```bash
hl-backtest run \
  --ir /path/to/strategy.ir.json \
  --asset ETH \
  --interval 1h \
  --start 2024-01-01 \
  --end 2024-06-30 \
  --initial-capital 10000 \
  --maker-fee-bps -1 \
  --taker-fee-bps 10 \
  --slippage-bps 5 \
  --out results.json
```

### Output Files

- `results.json`: Complete backtest results with metrics
- `results_trades.csv`: Trade log
- `results_equity.csv`: Equity curve over time

## Perps L2 Playback (Beta)

The backtester supports event-driven perps backtesting using Hyperliquid's historical L2 order book data from S3. This provides realistic fill simulation based on actual order book depth.

### Ingest L2 Data from S3

Download historical L2 order book snapshots:

```bash
hl-backtest ingest s3 \
  --coin BTC \
  --start 20230916 \
  --start-hour 9 \
  --end 20230916 \
  --end-hour 11 \
  --out data/s3
```

This downloads `.lz4` files from `s3://hyperliquid-archive/market_data/`.

**Note**: Based on the Python download script pattern (using `--request-payer requester`), 
the bucket appears to require:
- **AWS credentials** (configure via `aws configure` or environment variables)
- **Request-payer=requester header** (you pay for data transfer, ~$0.09/GB)

If downloads fail, check AWS credentials are configured.

See `docs/S3_SETUP.md` for detailed setup instructions.

### Build Events

Convert downloaded L2 files to event format:

```bash
hl-backtest ingest build-events \
  --coin BTC \
  --input data/s3 \
  --out data/events
```

This decompresses LZ4 files, parses JSONL, and creates normalized event files.

### Run Perps Backtest

Run a backtest using L2 events:

```bash
hl-backtest run-perps \
  --ir strategy.json \
  --coin BTC \
  --events data/events \
  --start 20230916-09 \
  --end 20230916-11 \
  --initial-capital 10000 \
  --maker-fee-bps -1 \
  --taker-fee-bps 10 \
  --out perps_results.json
```

### How It Works

1. **Order Book Reconstruction**: Each L2 snapshot updates a full order book (bids/asks)
2. **Realistic Fills**: Market orders sweep the book; limit orders fill when price crosses
3. **Funding Payments**: Applied every 8 hours based on position notional and funding rate
4. **Maker/Taker Fees**: Correctly applied based on order type and execution

### Limitations

- S3 data may have gaps (Hyperliquid docs warn: "no guarantee of timely updates")
- Large date ranges may require significant S3 transfer costs
- Funding history is fetched from Hyperliquid API (see [API docs](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/perpetuals#retrieve-historical-funding-rates))

## Features

- Supports all Hyperliquid order types (Market, Limit, Stop, Take, etc.)
- Configurable maker/taker fees and slippage
- Portfolio accounting with position tracking
- Comprehensive performance metrics (Sharpe, Sortino, drawdown, etc.)
- High-performance custom indicators (RSI, SMA, EMA, MACD, BBands, Stochastic, ATR, ADX, OBV)
- **Perps L2 playback** with realistic order book fills and funding payments

## Testing

```bash
cargo test
```

## Documentation

Comprehensive documentation is available in the `docs/` directory:

- **[Documentation Index](docs/DOCUMENTATION_INDEX.md)** - Complete documentation catalog
- **[Quick Start Guide](docs/QUICK_START.md)** - Get started in minutes
- **[API Documentation](docs/API.md)** - Complete API reference with examples
- **[Architecture Overview](docs/ARCHITECTURE.md)** - System design and component details
- **[Performance Optimizations](docs/PERFORMANCE_OPTIMIZATIONS.md)** - Performance improvements and benchmarks
- **[S3 Setup Guide](docs/S3_SETUP.md)** - Instructions for downloading L2 data from S3
- **[Security Documentation](docs/SECURITY.md)** - Security practices and audit findings
- **[Test Coverage](docs/TEST_COVERAGE.md)** - Test suite information

### Quick Links

- **Getting Started**: See [Quick Start Guide](docs/QUICK_START.md)
- **API Reference**: See [API Documentation](docs/API.md)
- **Understanding the Engine**: See [Architecture Overview](docs/ARCHITECTURE.md)

## Development

### Building Documentation

```bash
# Generate Rust documentation
cargo doc --open

# View documentation index
cat docs/DOCUMENTATION_INDEX.md
```

### Code Quality

- All code follows Rust best practices
- Comprehensive error handling with `anyhow`
- Security-focused input validation
- Performance-optimized for large datasets

