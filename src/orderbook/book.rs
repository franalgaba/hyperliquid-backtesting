use crate::ingest::OrderLevel;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct OrderBook {
    bids: BTreeMap<u64, f64>, // price (scaled) -> total size
    asks: BTreeMap<u64, f64>, // price (scaled) -> total size
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// Apply a full snapshot (replace current book)
    /// levels[0] = bids, levels[1] = asks
    /// Optimized: clear and rebuild efficiently
    pub fn apply_snapshot(&mut self, levels: &[Vec<OrderLevel>]) {
        self.bids.clear();
        self.asks.clear();

        if levels.is_empty() {
            return;
        }

        // Bids (level 0) - sorted descending by price
        if let Some(bid_levels) = levels.get(0) {
            for level in bid_levels {
                let price_scaled = (level.px * 1e8) as u64; // Scale for integer key
                *self.bids.entry(price_scaled).or_insert(0.0) += level.sz;
            }
        }

        // Asks (level 1) - sorted ascending by price
        if let Some(ask_levels) = levels.get(1) {
            for level in ask_levels {
                let price_scaled = (level.px * 1e8) as u64;
                *self.asks.entry(price_scaled).or_insert(0.0) += level.sz;
            }
        }
    }

    pub fn best_bid(&self) -> Option<(f64, f64)> {
        // Bids are in descending order, so last is highest
        self.bids.iter().next_back().map(|(p, s)| (*p as f64 / 1e8, *s))
    }

    pub fn best_ask(&self) -> Option<(f64, f64)> {
        // Asks are in ascending order, so first is lowest
        self.asks.iter().next().map(|(p, s)| (*p as f64 / 1e8, *s))
    }

    pub fn mid_price(&self) -> Option<f64> {
        let bid = self.best_bid()?;
        let ask = self.best_ask()?;
        Some((bid.0 + ask.0) / 2.0)
    }

    /// Get cumulative depth up to a price level
    /// For bids: returns depth at prices >= price (better or equal)
    /// For asks: returns depth at prices <= price (better or equal)
    pub fn bid_depth_to(&self, price: f64) -> f64 {
        let price_scaled = (price * 1e8) as u64;
        self.bids
            .iter()
            .filter(|(p, _)| **p >= price_scaled)
            .map(|(_, s)| s)
            .sum()
    }

    pub fn ask_depth_to(&self, price: f64) -> f64 {
        let price_scaled = (price * 1e8) as u64;
        self.asks
            .iter()
            .filter(|(p, _)| **p <= price_scaled)
            .map(|(_, s)| s)
            .sum()
    }

    /// Sweep market order: fill size by walking the book
    /// Returns (filled_size, avg_fill_price, is_maker)
    pub fn sweep_market_buy(&self, size: f64) -> Option<(f64, f64, bool)> {
        let mut remaining = size;
        let mut total_cost = 0.0;
        let mut total_filled = 0.0;

        for (price_scaled, level_size) in self.asks.iter() {
            if remaining <= 0.0 {
                break;
            }
            let price = *price_scaled as f64 / 1e8;
            let fill_size = remaining.min(*level_size);
            total_cost += fill_size * price;
            total_filled += fill_size;
            remaining -= fill_size;
        }

        if total_filled > 0.0 {
            Some((total_filled, total_cost / total_filled, false)) // Market orders are taker
        } else {
            None
        }
    }

    pub fn sweep_market_sell(&self, size: f64) -> Option<(f64, f64, bool)> {
        let mut remaining = size;
        let mut total_cost = 0.0;
        let mut total_filled = 0.0;

        for (price_scaled, level_size) in self.bids.iter().rev() {
            if remaining <= 0.0 {
                break;
            }
            let price = *price_scaled as f64 / 1e8;
            let fill_size = remaining.min(*level_size);
            total_cost += fill_size * price;
            total_filled += fill_size;
            remaining -= fill_size;
        }

        if total_filled > 0.0 {
            Some((total_filled, total_cost / total_filled, false))
        } else {
            None
        }
    }

    /// Check if a limit order would cross the book
    pub fn would_cross_buy(&self, limit_price: f64) -> bool {
        if let Some((best_ask, _)) = self.best_ask() {
            limit_price >= best_ask
        } else {
            false
        }
    }

    pub fn would_cross_sell(&self, limit_price: f64) -> bool {
        if let Some((best_bid, _)) = self.best_bid() {
            limit_price <= best_bid
        } else {
            false
        }
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_book_snapshot() {
        let mut book = OrderBook::new();
        
        let levels = vec![
            vec![
                OrderLevel { px: 25000.0, sz: 1.5, n: 1 },
                OrderLevel { px: 24999.0, sz: 2.0, n: 2 },
            ],
            vec![
                OrderLevel { px: 25001.0, sz: 1.0, n: 3 },
                OrderLevel { px: 25002.0, sz: 1.5, n: 4 },
            ],
        ];

        book.apply_snapshot(&levels);

        let best_bid = book.best_bid().unwrap();
        assert_eq!(best_bid.0, 25000.0);
        assert_eq!(best_bid.1, 1.5);

        let best_ask = book.best_ask().unwrap();
        assert_eq!(best_ask.0, 25001.0);
        assert_eq!(best_ask.1, 1.0);

        let mid = book.mid_price().unwrap();
        assert_eq!(mid, 25000.5);
    }

    #[test]
    fn test_market_sweep() {
        let mut book = OrderBook::new();
        let levels = vec![
            vec![OrderLevel { px: 25000.0, sz: 1.0, n: 1 }],
            vec![OrderLevel { px: 25001.0, sz: 2.0, n: 2 }],
        ];
        book.apply_snapshot(&levels);

        // Buy 1.5 size - should fill 1.5 at 25001.0 (there's 2.0 available)
        let (filled, avg_price, is_maker) = book.sweep_market_buy(1.5).unwrap();
        assert_eq!(filled, 1.5);
        assert_eq!(avg_price, 25001.0);
        assert!(!is_maker);
    }

    #[test]
    fn test_market_sweep_multiple_levels() {
        let mut book = OrderBook::new();
        let levels = vec![
            vec![
                OrderLevel { px: 25000.0, sz: 1.0, n: 1 },
                OrderLevel { px: 24999.0, sz: 2.0, n: 2 },
            ],
            vec![
                OrderLevel { px: 25001.0, sz: 1.5, n: 3 },
                OrderLevel { px: 25002.0, sz: 2.0, n: 4 },
            ],
        ];
        book.apply_snapshot(&levels);

        // Buy 3.0 size - should fill 1.5 at 25001.0 and 1.5 at 25002.0
        let (filled, avg_price, _) = book.sweep_market_buy(3.0).unwrap();
        assert_eq!(filled, 3.0);
        // Weighted average: (1.5 * 25001.0 + 1.5 * 25002.0) / 3.0 = 25001.5
        assert!((avg_price - 25001.5).abs() < 0.01);
    }

    #[test]
    fn test_would_cross() {
        let mut book = OrderBook::new();
        let levels = vec![
            vec![OrderLevel { px: 25000.0, sz: 1.0, n: 1 }],
            vec![OrderLevel { px: 25001.0, sz: 1.0, n: 2 }],
        ];
        book.apply_snapshot(&levels);

        assert!(book.would_cross_buy(25001.0)); // At ask
        assert!(book.would_cross_buy(25002.0)); // Above ask
        assert!(!book.would_cross_buy(25000.0)); // Below ask

        assert!(book.would_cross_sell(25000.0)); // At bid
        assert!(book.would_cross_sell(24999.0)); // Below bid
        assert!(!book.would_cross_sell(25001.0)); // Above bid
    }

    #[test]
    fn test_depth_queries() {
        let mut book = OrderBook::new();
        let levels = vec![
            vec![
                OrderLevel { px: 25000.0, sz: 1.0, n: 1 },
                OrderLevel { px: 24999.0, sz: 2.0, n: 2 },
            ],
            vec![
                OrderLevel { px: 25001.0, sz: 1.5, n: 3 },
                OrderLevel { px: 25002.0, sz: 2.0, n: 4 },
            ],
        ];
        book.apply_snapshot(&levels);

        // Bid depth to 25000.0: bids >= 25000.0 (better or equal)
        let bid_depth = book.bid_depth_to(25000.0);
        assert_eq!(bid_depth, 1.0); // Only the bid at 25000.0

        // Bid depth to 24999.0: bids >= 24999.0 (includes both)
        let bid_depth_lower = book.bid_depth_to(24999.0);
        assert_eq!(bid_depth_lower, 3.0); // 1.0 + 2.0

        // Ask depth to 25002.0: asks <= 25002.0 (better or equal)
        let ask_depth = book.ask_depth_to(25002.0);
        assert_eq!(ask_depth, 3.5); // 1.5 + 2.0

        // Ask depth to 25001.0: asks <= 25001.0
        let ask_depth_lower = book.ask_depth_to(25001.0);
        assert_eq!(ask_depth_lower, 1.5); // Only the ask at 25001.0
    }
}

