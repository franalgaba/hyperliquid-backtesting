#!/usr/bin/env python3
"""Generate test L2 order book data for perps backtesting"""

import json
import os
from datetime import datetime, timedelta
from pathlib import Path

def generate_l2_events(num_events=100, start_date_str="2023-09-16", start_hour=9, coin="BTC"):
    """Generate mock L2 order book events"""
    start_date = datetime.strptime(start_date_str, "%Y-%m-%d")
    start_ts = int((start_date.replace(hour=start_hour, minute=0, second=0)).timestamp() * 1000)
    
    events = []
    base_price = 25000.0
    
    for i in range(num_events):
        # Simulate price movement
        price_change = (i % 20 - 10) * 0.5  # Oscillating price
        current_price = base_price + price_change
        
        # Create order book levels
        # Bids (below current price)
        bids = []
        for j in range(5):
            bid_price = current_price - (j + 1) * 0.5
            bid_size = 1.0 + (j * 0.5)
            bids.append({
                "px": str(round(bid_price, 2)),
                "sz": str(round(bid_size, 2)),
                "n": j + 1
            })
        
        # Asks (above current price)
        asks = []
        for j in range(5):
            ask_price = current_price + (j + 1) * 0.5
            ask_size = 1.0 + (j * 0.5)
            asks.append({
                "px": str(round(ask_price, 2)),
                "sz": str(round(ask_size, 2)),
                "n": j + 1
            })
        
        # Create event (every second)
        event_ts = start_ts + (i * 1000)
        event = {
            "time": datetime.fromtimestamp(event_ts / 1000).isoformat() + "Z",
            "raw": {
                "data": {
                    "time": event_ts,
                    "levels": [bids, asks]
                }
            }
        }
        events.append(event)
    
    return events

if __name__ == "__main__":
    coin = "BTC"
    date_str = "2023-09-16"
    hour = 9
    
    # Generate events
    events = generate_l2_events(
        num_events=3600,  # 1 hour of data (1 event per second)
        start_date_str=date_str,
        start_hour=hour,
        coin=coin
    )
    
    # Create output directory
    output_dir = Path("data/events") / coin
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Write JSONL file
    output_file = output_dir / f"{date_str.replace('-', '')}-{hour}.jsonl"
    with open(output_file, 'w') as f:
        for event in events:
            f.write(json.dumps(event) + '\n')
    
    print(f"Generated {len(events)} L2 events in {output_file}")
    print(f"Date range: {datetime.fromtimestamp(events[0]['raw']['data']['time'] / 1000)} to {datetime.fromtimestamp(events[-1]['raw']['data']['time'] / 1000)}")
    print(f"\nYou can now run:")
    print(f"  ./target/release/hl-backtest run-perps \\")
    print(f"    --ir examples/sample_strategy.json \\")
    print(f"    --coin {coin} \\")
    print(f"    --events data/events \\")
    print(f"    --start {date_str.replace('-', '')}-{hour} \\")
    print(f"    --end {date_str.replace('-', '')}-{hour} \\")
    print(f"    --maker-fee-bps=-1 \\")
    print(f"    --taker-fee-bps=10 \\")
    print(f"    --out perps_results.json")

