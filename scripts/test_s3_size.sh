#!/bin/bash
set -euo pipefail

# Change to backtester root directory
cd "$(dirname "$0")/.."

COIN="BTC"
DATE="20230916"
OUT_DIR="data/s3_test"

echo "=== Testing S3 Download for 1 Day ==="
echo "Coin: $COIN | Date: $DATE | Hours: 0-23"
echo ""

rm -rf "$OUT_DIR"

echo "Downloading..."
./target/release/hl-backtest ingest s3 \
  --coin "$COIN" \
  --start "$DATE" \
  --start-hour 0 \
  --end "$DATE" \
  --end-hour 23 \
  --out "$OUT_DIR"

echo ""
echo "=== Results ==="

# Count files
FILE_COUNT=$(find "$OUT_DIR" -name "*.lz4" -type f 2>/dev/null | wc -l | tr -d " ")
echo "Files downloaded: $FILE_COUNT"

# Calculate total size (works on both macOS and Linux)
if command -v gdu >/dev/null 2>&1; then
  TOTAL_SIZE_BYTES=$(gdu -sb "$OUT_DIR" 2>/dev/null | cut -f1)
elif command -v du >/dev/null 2>&1; then
  TOTAL_SIZE_BYTES=$(du -sb "$OUT_DIR" 2>/dev/null | cut -f1)
else
  TOTAL_SIZE_BYTES=$(find "$OUT_DIR" -type f -exec stat -f%z {} + 2>/dev/null | awk "{sum+=\$1} END {print sum}" || find "$OUT_DIR" -type f -exec stat -c%s {} + 2>/dev/null | awk "{sum+=\$1} END {print sum}")
fi

TOTAL_SIZE_MB=$(echo "scale=2; $TOTAL_SIZE_BYTES / 1024 / 1024" | bc 2>/dev/null || echo "scale=2; $TOTAL_SIZE_BYTES / 1024 / 1024" | awk "{printf \"%.2f\", \$1}")
TOTAL_SIZE_GB=$(echo "scale=3; $TOTAL_SIZE_BYTES / 1024 / 1024 / 1024" | bc 2>/dev/null || echo "scale=3; $TOTAL_SIZE_BYTES / 1024 / 1024 / 1024" | awk "{printf \"%.3f\", \$1}")

echo "Total size: ${TOTAL_SIZE_MB} MB (${TOTAL_SIZE_GB} GB)"

# Estimate cost
ESTIMATED_COST=$(echo "scale=4; $TOTAL_SIZE_GB * 0.09" | bc 2>/dev/null || echo "$TOTAL_SIZE_GB * 0.09" | awk "{printf \"%.4f\", \$1}")
echo "Estimated cost: \$${ESTIMATED_COST} USD (at \$0.09/GB)"
echo ""
echo "Per-file sizes:"
find "$OUT_DIR" -name "*.lz4" -type f -exec ls -lh {} \; 2>/dev/null | awk "{print \$5, \$9}" | sort -k2
