#!/usr/bin/env python3
"""Generate test candle data for backtesting"""

import csv
import random
from datetime import datetime, timedelta

# Generate 100 hours of candle data
start_time = datetime(2024, 1, 1, 0, 0, 0)
base_price = 2500.0  # Starting ETH price

candles = []
current_price = base_price

for i in range(100):
    time_open = start_time + timedelta(hours=i)
    time_close = time_open + timedelta(hours=1) - timedelta(seconds=1)
    
    # Random walk price movement
    change = random.uniform(-0.02, 0.02)  # ±2% per hour
    open_price = current_price
    high_price = open_price * (1 + abs(random.uniform(0, 0.01)))
    low_price = open_price * (1 - abs(random.uniform(0, 0.01)))
    close_price = open_price * (1 + change)
    current_price = close_price
    
    volume = random.uniform(1000, 10000)
    num_trades = random.randint(50, 500)
    
    candles.append({
        'time_open': int(time_open.timestamp() * 1000),
        'time_close': int(time_close.timestamp() * 1000),
        'coin': 'ETH',
        'interval': '1h',
        'open': f'{open_price:.2f}',
        'close': f'{close_price:.2f}',
        'high': f'{high_price:.2f}',
        'low': f'{low_price:.2f}',
        'volume': f'{volume:.2f}',
        'num_trades': num_trades,
    })

# Write to CSV
import os
os.makedirs('data/hyperliquid/ETH', exist_ok=True)

with open('data/hyperliquid/ETH/1h.csv', 'w', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=candles[0].keys())
    writer.writeheader()
    writer.writerows(candles)

print(f"Generated {len(candles)} candles in data/hyperliquid/ETH/1h.csv")
print(f"Date range: {start_time} to {time_close}")

