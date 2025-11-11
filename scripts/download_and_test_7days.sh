#!/bin/bash
set -euo pipefail

# Change to backtester root directory
cd "$(dirname "$0")/.."

# Use October 2025 - first 7 days
START_DATE="20251001"
END_DATE="20251007"

COIN="BTC"
S3_DIR="data/s3"
EVENTS_DIR="data/events"
IR_FILE="examples/sample_strategy.json"
RESULTS_FILE="scripts/results/results_oct2025.json"

echo "=== Downloading and Testing 7-Day Backtest ==="
echo "Coin: $COIN"
echo "Date range: ${START_DATE}-00 to ${END_DATE}-23"
echo ""

# Step 1: Download L2 data from S3
echo "Step 1: Downloading L2 data from S3..."
./target/release/hl-backtest ingest s3 \
  --coin "$COIN" \
  --start "$START_DATE" \
  --start-hour 0 \
  --end "$END_DATE" \
  --end-hour 23 \
  --out "$S3_DIR"

if [ $? -ne 0 ]; then
  echo "Error: Failed to download S3 data"
  exit 1
fi

# Step 2: Build events from downloaded files
echo ""
echo "Step 2: Building events from downloaded files..."
./target/release/hl-backtest ingest build-events \
  --coin "$COIN" \
  --input "$S3_DIR" \
  --out "$EVENTS_DIR"

if [ $? -ne 0 ]; then
  echo "Error: Failed to build events"
  exit 1
fi

# Step 3: Run backtest
echo ""
echo "Step 3: Running backtest..."
./target/release/hl-backtest run-perps \
  --ir "$IR_FILE" \
  --coin "$COIN" \
  --events "$EVENTS_DIR" \
  --start "${START_DATE}-00" \
  --end "${END_DATE}-23" \
  --initial-capital 10000 \
  --maker-fee-bps=-1 \
  --taker-fee-bps=10 \
  --out "$RESULTS_FILE"

echo ""
echo "=== Complete ==="
echo "Results written to: $RESULTS_FILE"
