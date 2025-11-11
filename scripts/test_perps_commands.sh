#!/bin/bash
# Test commands for perps backtesting with funding history

# Change to backtester root directory
cd "$(dirname "$0")/.."

# First, download some L2 data (example for BTC, 1 hour)
./target/release/hl-backtest ingest s3 \
  --coin BTC \
  --start 20230916 \
  --start-hour 9 \
  --end 20230916 \
  --end-hour 10 \
  --out data/s3

# Build events
./target/release/hl-backtest ingest build-events \
  --coin BTC \
  --input data/s3 \
  --out data/events

# Run perps backtest (this will fetch funding history automatically)
./target/release/hl-backtest run-perps \
  --ir examples/sample_strategy.json \
  --coin BTC \
  --events data/events \
  --start 20230916-09 \
  --end 20230916-10 \
  --initial-capital 10000 \
  --maker-fee-bps=-1 \
  --taker-fee-bps=10 \
  --out scripts/results/perps_results.json

