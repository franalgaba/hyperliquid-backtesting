#!/bin/bash
# Test script for the backtester

set -e

echo "=== Hyperliquid Backtester Test Script ==="
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ASSET="ETH"
INTERVAL="1h"
START_DATE="2024-01-01"
END_DATE="2024-06-30"
INITIAL_CAPITAL=10000

echo -e "${BLUE}Step 1: Fetching historical data...${NC}"
hl-backtest fetch \
  --asset "$ASSET" \
  --interval "$INTERVAL" \
  --start "$START_DATE" \
  --end "$END_DATE"

echo ""
echo -e "${BLUE}Step 2: Running RSI Strategy backtest...${NC}"
hl-backtest run \
  --ir examples/sample_strategy.json \
  --asset "$ASSET" \
  --interval "$INTERVAL" \
  --start "$START_DATE" \
  --end "$END_DATE" \
  --initial-capital "$INITIAL_CAPITAL" \
  --out rsi_results.json

echo ""
echo -e "${GREEN}✓ RSI Strategy backtest complete!${NC}"
echo "Results written to:"
echo "  - rsi_results.json"
echo "  - rsi_results_trades.csv"
echo "  - rsi_results_equity.csv"

echo ""
echo -e "${BLUE}Step 3: Running SMA Crossover Strategy backtest...${NC}"
hl-backtest run \
  --ir examples/sma_crossover_strategy.json \
  --asset BTC \
  --interval "$INTERVAL" \
  --start "$START_DATE" \
  --end "$END_DATE" \
  --initial-capital "$INITIAL_CAPITAL" \
  --out sma_results.json

echo ""
echo -e "${GREEN}✓ SMA Crossover Strategy backtest complete!${NC}"
echo "Results written to:"
echo "  - sma_results.json"
echo "  - sma_results_trades.csv"
echo "  - sma_results_equity.csv"

echo ""
echo -e "${GREEN}=== All tests complete! ===${NC}"

