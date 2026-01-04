#!/bin/bash
set -euo pipefail

# Change to backtester root directory
cd "$(dirname "$0")"

# Configuration
COIN="BTC"
START_DATE="20251011"
END_DATE="20251111"
IR_FILE="/Users/franalgaba/Projects/strategy-compiler/.strategy_store/8b0f0317fc854ee415bbc7c2917ffbde76c78a5c4c3013dba8ce78d4ebf3afea/ir.json"
S3_DIR="data/s3"
EVENTS_DIR="data/events"
INITIAL_CAPITAL=10000
RESULTS_FILE="backtest_results.json"

echo "=== Running Backtest ==="
echo "Coin: $COIN"
echo "Date range: ${START_DATE}-00 to ${END_DATE}-23"
echo "Strategy: $IR_FILE"
echo ""

# Step 1: Download L2 data from S3
echo "Step 1: Downloading L2 data from S3..."
echo "  Source: s3://hyperliquid-archive/market_data/"
echo "  Output: $S3_DIR/$COIN/"
echo "  Date range: ${START_DATE} 00:00 to ${END_DATE} 23:59"
echo ""

# Count files before
FILES_BEFORE=$(find "$S3_DIR/$COIN" -name "*.lz4" 2>/dev/null | wc -l | tr -d ' ')
echo "  Files before download: $FILES_BEFORE"

# Run download with timestamp and capture warnings
START_TIME=$(date +%s)
WARNINGS_FILE=$(mktemp)
OUTPUT_FILE=$(mktemp)

./target/release/hl-backtest ingest s3 \
  --coin "$COIN" \
  --start "$START_DATE" \
  --start-hour 0 \
  --end "$END_DATE" \
  --end-hour 23 \
  --out "$S3_DIR" > "$OUTPUT_FILE" 2>&1

DOWNLOAD_EXIT=$?
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Process output with timestamps and capture warnings
while IFS= read -r line; do
  echo "  [$(date +%H:%M:%S)] $line"
  if echo "$line" | grep -q "Warning:"; then
    echo "$line" >> "$WARNINGS_FILE"
  fi
done < "$OUTPUT_FILE"

rm -f "$OUTPUT_FILE"

# Count files after and show stats
FILES_AFTER=$(find "$S3_DIR/$COIN" -name "*.lz4" 2>/dev/null | wc -l | tr -d ' ')
FILES_DOWNLOADED=$((FILES_AFTER - FILES_BEFORE))
TOTAL_SIZE=$(du -sh "$S3_DIR/$COIN" 2>/dev/null | cut -f1 || echo "unknown")

# Count actual warnings
ACTUAL_WARNINGS=$(wc -l < "$WARNINGS_FILE" 2>/dev/null | tr -d ' ' || echo "0")
rm -f "$WARNINGS_FILE"

echo ""
if [ $DOWNLOAD_EXIT -eq 0 ] || [ $FILES_DOWNLOADED -gt 0 ]; then
  echo "  ✓ Download complete in ${DURATION}s"
  echo "  Files downloaded: $FILES_DOWNLOADED (total: $FILES_AFTER)"
  if [ "$ACTUAL_WARNINGS" -gt 0 ]; then
    echo "  Warnings: $ACTUAL_WARNINGS (some files may not exist in S3 for future dates)"
  fi
  echo "  Total size: $TOTAL_SIZE"
  
  # Warn if no files were downloaded
  if [ $FILES_DOWNLOADED -eq 0 ]; then
    echo ""
    echo "  ⚠️  Warning: No files were downloaded. Check:"
    echo "     - Date range (future dates may not exist in S3)"
    echo "     - AWS credentials are configured"
    echo "     - Network connectivity"
  fi
else
  echo "  ❌ Download failed (exit code: $DOWNLOAD_EXIT)"
  exit 1
fi

# Step 2: Build events from downloaded files
echo ""
echo "Step 2: Building events from downloaded files..."
echo "  Input: $S3_DIR/$COIN/"
echo "  Output: $EVENTS_DIR/$COIN/"
echo ""

EVENTS_BEFORE=$(find "$EVENTS_DIR/$COIN" -name "*.jsonl" 2>/dev/null | wc -l | tr -d ' ')
echo "  Event files before: $EVENTS_BEFORE"

START_TIME=$(date +%s)
./target/release/hl-backtest ingest build-events \
  --coin "$COIN" \
  --input "$S3_DIR" \
  --out "$EVENTS_DIR" 2>&1 | while IFS= read -r line; do
    echo "  [$(date +%H:%M:%S)] $line"
  done

BUILD_EXIT=${PIPESTATUS[0]}
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

if [ $BUILD_EXIT -ne 0 ]; then
  echo ""
  echo "Error: Failed to build events (exit code: $BUILD_EXIT)"
  exit 1
fi

EVENTS_AFTER=$(find "$EVENTS_DIR/$COIN" -name "*.jsonl" 2>/dev/null | wc -l | tr -d ' ')
EVENTS_BUILT=$((EVENTS_AFTER - EVENTS_BEFORE))
EVENTS_SIZE=$(du -sh "$EVENTS_DIR/$COIN" 2>/dev/null | cut -f1 || echo "unknown")

echo ""
echo "  ✓ Events built in ${DURATION}s"
echo "  Event files created: $EVENTS_BUILT (total: $EVENTS_AFTER)"
echo "  Total size: $EVENTS_SIZE"

# Step 3: Run backtest
echo ""
echo "Step 3: Running backtest..."
echo "  Strategy: $(basename "$IR_FILE")"
echo "  Events: $EVENTS_DIR/$COIN/"
echo "  Period: ${START_DATE}-00 to ${END_DATE}-23"
echo "  Initial capital: \$$INITIAL_CAPITAL"
echo ""

START_TIME=$(date +%s)
./target/release/hl-backtest run-perps \
  --ir "$IR_FILE" \
  --coin "$COIN" \
  --events "$EVENTS_DIR" \
  --start "${START_DATE}-00" \
  --end "${END_DATE}-23" \
  --initial-capital "$INITIAL_CAPITAL" \
  --maker-fee-bps=-1 \
  --taker-fee-bps=10 \
  --trade-cooldown-min 120 \
  --out "$RESULTS_FILE" 2>&1 | while IFS= read -r line; do
    echo "  [$(date +%H:%M:%S)] $line"
  done

BACKTEST_EXIT=${PIPESTATUS[0]}
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

if [ $BACKTEST_EXIT -ne 0 ]; then
  echo ""
  echo "Error: Backtest failed (exit code: $BACKTEST_EXIT)"
  exit 1
fi

echo ""
echo "  ✓ Backtest completed in ${DURATION}s"

echo ""
echo "=== Complete ==="
echo "Results written to: $RESULTS_FILE"
echo "Trades CSV: ${RESULTS_FILE%.json}_trades.csv"
echo "Equity CSV: ${RESULTS_FILE%.json}_equity.csv"

