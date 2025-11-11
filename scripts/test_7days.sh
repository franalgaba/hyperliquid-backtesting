#!/bin/bash
set -euo pipefail

# Change to backtester root directory
cd "$(dirname "$0")/.."

# Use October 2025 - first 7 days
START_DATE="20251001"
END_DATE="20251007"

COIN="BTC"  # Change to ETH, SOL, or HYPE as needed
IR_FILE="examples/sample_strategy.json"
EVENTS_DIR="data/events"
INITIAL_CAPITAL=10000
RESULTS_FILE="scripts/results/results_oct2025.json"

echo "=== Running 7-Day Backtest ==="
echo "Coin: $COIN"
echo "Date range: ${START_DATE}-00 to ${END_DATE}-23"
echo "Strategy: $IR_FILE"
echo ""

# Check if events directory exists
if [ ! -d "$EVENTS_DIR/$COIN" ]; then
  echo "Error: Events directory not found: $EVENTS_DIR/$COIN"
  echo ""
  echo "First, download and build events:"
  echo "  ./target/release/hl-backtest ingest s3 \\"
  echo "    --coin $COIN \\"
  echo "    --start $START_DATE \\"
  echo "    --start-hour 0 \\"
  echo "    --end $END_DATE \\"
  echo "    --end-hour 23 \\"
  echo "    --out data/s3"
  echo ""
  echo "  ./target/release/hl-backtest ingest build-events \\"
  echo "    --coin $COIN \\"
  echo "    --input data/s3 \\"
  echo "    --out $EVENTS_DIR"
  exit 1
fi

# Run backtest
./target/release/hl-backtest run-perps \
  --ir "$IR_FILE" \
  --coin "$COIN" \
  --events "$EVENTS_DIR" \
  --start "${START_DATE}-00" \
  --end "${END_DATE}-23" \
  --initial-capital "$INITIAL_CAPITAL" \
  --maker-fee-bps=-1 \
  --taker-fee-bps=10 \
  --out "$RESULTS_FILE"

echo ""
echo "=== Results ==="
echo "Results written to: $RESULTS_FILE"
echo "Trades CSV: ${RESULTS_FILE%.json}_trades.csv"
echo "Equity CSV: ${RESULTS_FILE%.json}_equity.csv"
