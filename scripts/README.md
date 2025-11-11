# Scripts Directory

This directory contains utility scripts and test results for the Hyperliquid backtester.

## Scripts

- `download_and_test_7days.sh` - Downloads S3 data, builds events, and runs a 7-day backtest
- `test_7days.sh` - Runs a 7-day backtest (assumes events already exist)
- `test_perps_commands.sh` - Example commands for perps backtesting workflow
- `test_s3_size.sh` - Tests S3 download for 1 day to estimate sizes and costs

## Results

The `results/` subdirectory contains CSV and JSON output files from backtest runs:
- `*_trades.csv` - Trade execution logs
- `*_equity.csv` - Equity curve over time
- `*.json` - Complete backtest results with metrics

## Usage

All scripts should be run from the backtester root directory. They will automatically change to the correct directory.

Example:
```bash
./scripts/test_7days.sh
```
