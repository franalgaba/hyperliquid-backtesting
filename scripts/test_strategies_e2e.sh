#!/bin/bash
set -euo pipefail

# Change to backtester root directory
cd "$(dirname "$0")/.."

COIN="BTC"
EVENTS_DIR="data/events"
RESULTS_DIR="scripts/results/e2e_tests"

# Create results directory
mkdir -p "$RESULTS_DIR"

# Test data availability
if [ ! -d "$EVENTS_DIR/$COIN" ]; then
  echo "Error: Events directory not found: $EVENTS_DIR/$COIN"
  echo ""
  echo "Please download events first:"
  echo "  ./target/release/hl-backtest ingest s3 \\"
  echo "    --coin $COIN \\"
  echo "    --start 20251001 \\"
  echo "    --start-hour 0 \\"
  echo "    --end 20251031 \\"
  echo "    --end-hour 23 \\"
  echo "    --out data/s3"
  echo ""
  echo "  ./target/release/hl-backtest ingest build-events \\"
  echo "    --coin $COIN \\"
  echo "    --input data/s3 \\"
  echo "    --out $EVENTS_DIR"
  exit 1
fi

# Detect available date range from events files
EVENT_FILES=$(ls -1 "$EVENTS_DIR/$COIN"/*.jsonl 2>/dev/null | sort)
if [ -z "$EVENT_FILES" ]; then
  echo "Error: No event files found in $EVENTS_DIR/$COIN"
  exit 1
fi

# Extract dates from filenames (format: YYYYMMDD-HH.jsonl)
FIRST_FILE=$(echo "$EVENT_FILES" | head -1)
LAST_FILE=$(echo "$EVENT_FILES" | tail -1)
START_DATE=$(basename "$FIRST_FILE" .jsonl | cut -d'-' -f1)
END_DATE=$(basename "$LAST_FILE" .jsonl | cut -d'-' -f1)

# Calculate number of days
START_TS=$(date -j -f "%Y%m%d" "$START_DATE" "+%s" 2>/dev/null || date -d "$START_DATE" "+%s" 2>/dev/null)
END_TS=$(date -j -f "%Y%m%d" "$END_DATE" "+%s" 2>/dev/null || date -d "$END_DATE" "+%s" 2>/dev/null)
DAYS=$(( (END_TS - START_TS) / 86400 + 1 ))

echo "=== End-to-End Strategy Testing ==="
echo "Testing period: ${START_DATE} to ${END_DATE} (${DAYS} days)"
echo "Available event files: $(echo "$EVENT_FILES" | wc -l | tr -d ' ')"
echo ""

# Test each strategy
STRATEGIES=(
  "examples/strategies/simple_price_momentum.json:Simple Price Momentum"
  "examples/strategies/sma_crossover.json:SMA Crossover"
  "examples/strategies/rsi_mean_reversion.json:RSI Mean Reversion"
  "examples/strategies/macd_crossover.json:MACD Crossover"
  "examples/strategies/bollinger_bands.json:Bollinger Bands"
)

SUCCESS_COUNT=0
FAIL_COUNT=0

for strategy_entry in "${STRATEGIES[@]}"; do
  IFS=':' read -r strategy_file strategy_name <<< "$strategy_entry"
  
  if [ ! -f "$strategy_file" ]; then
    echo "⚠️  Strategy file not found: $strategy_file"
    ((FAIL_COUNT++))
    continue
  fi
  
  echo "Testing: $strategy_name"
  echo "  File: $strategy_file"
  
  # Generate output filename from strategy name
  output_name=$(echo "$strategy_name" | tr '[:upper:]' '[:lower:]' | tr ' ' '_')
  output_file="$RESULTS_DIR/${output_name}_results.json"
  
  # Run backtest using detected date range
  if ./target/release/hl-backtest run-perps \
    --ir "$strategy_file" \
    --coin "$COIN" \
    --events "$EVENTS_DIR" \
    --start "${START_DATE}-00" \
    --end "${END_DATE}-23" \
    --initial-capital 10000 \
    --maker-fee-bps=-1 \
    --taker-fee-bps=10 \
    --out "$output_file" 2>&1; then
    
    # Check if results file was created and has trades
    if [ -f "$output_file" ]; then
      num_trades=$(jq -r '.num_trades // 0' "$output_file" 2>/dev/null || echo "0")
      final_equity=$(jq -r '.final_equity // 0' "$output_file" 2>/dev/null || echo "0")
      
      if [ "$num_trades" -gt 0 ]; then
        echo "  ✅ SUCCESS: $num_trades trades executed, Final equity: \$$(printf "%.2f" $final_equity)"
        ((SUCCESS_COUNT++))
      else
        echo "  ⚠️  WARNING: Backtest completed but no trades executed"
        ((FAIL_COUNT++))
      fi
    else
      echo "  ❌ FAILED: Results file not created"
      ((FAIL_COUNT++))
    fi
  else
    echo "  ❌ FAILED: Backtest execution error"
    ((FAIL_COUNT++))
  fi
  echo ""
done

echo "=== Summary ==="
echo "✅ Successful: $SUCCESS_COUNT"
echo "❌ Failed: $FAIL_COUNT"
echo "Total: $((SUCCESS_COUNT + FAIL_COUNT))"
echo ""
echo "Results saved to: $RESULTS_DIR"

if [ $FAIL_COUNT -eq 0 ]; then
  echo ""
  echo "🎉 All strategies tested successfully!"
  exit 0
else
  echo ""
  echo "⚠️  Some strategies failed. Check the output above for details."
  exit 1
fi

