# Documentation

Welcome to the Hyperliquid Data Ingestor & Backtester documentation.

## Table of Contents

| Document | Description |
|----------|-------------|
| [Quick Start](QUICK_START.md) | Get up and running in 5 minutes |
| [Data Ingestion](DATA_INGESTION.md) | Fetching OHLC and L2 data from Hyperliquid |
| [Strategies](STRATEGIES.md) | Writing and configuring trading strategies |
| [Indicators](INDICATORS.md) | Available technical indicators and parameters |
| [Backtesting](BACKTESTING.md) | Running backtests and understanding results |
| [Parquet Export](PARQUET.md) | Exporting data to Parquet format |
| [CLI Reference](CLI.md) | Complete command-line interface reference |
| [Architecture](ARCHITECTURE.md) | System design and code structure |

## Quick Links

### I want to...

- **Fetch historical data** → [Data Ingestion](DATA_INGESTION.md)
- **Write a strategy** → [Strategies](STRATEGIES.md)
- **Run a backtest** → [Backtesting](BACKTESTING.md)
- **Export to Parquet** → [Parquet Export](PARQUET.md)
- **Use indicators** → [Indicators](INDICATORS.md)

## Installation

```bash
# Clone the repository
git clone https://github.com/your-org/hyperliquid-backtesting.git
cd hyperliquid-backtesting

# Build
cargo build --release

# Binary location
./target/release/hl-backtest --help
```

## Support

- [GitHub Issues](https://github.com/your-org/hyperliquid-backtesting/issues)
- [Hyperliquid Documentation](https://hyperliquid.gitbook.io/hyperliquid-docs/)
