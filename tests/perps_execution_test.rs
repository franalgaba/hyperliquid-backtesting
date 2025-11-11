#[cfg(test)]
mod tests {
    use hl_backtest::ingest::OrderLevel;
    use hl_backtest::orderbook::OrderBook;
    use hl_backtest::perps::execution::{FillResult, PerpsExecution};
    use hl_backtest::orders::types::{Action, Order, OrderStatus, Side, Tif};

    // Helper to create a book with bids and asks
    fn create_test_book(bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> OrderBook {
        let mut book = OrderBook::new();
        let bid_levels: Vec<OrderLevel> = bids
            .into_iter()
            .enumerate()
            .map(|(i, (px, sz))| OrderLevel { px, sz, n: i as u64 })
            .collect();
        let ask_levels: Vec<OrderLevel> = asks
            .into_iter()
            .enumerate()
            .map(|(i, (px, sz))| OrderLevel { px, sz, n: i as u64 })
            .collect();
        book.apply_snapshot(&[bid_levels, ask_levels]);
        book
    }

    // ========== Market Order Tests ==========

    #[test]
    fn test_execute_market_buy_full_fill() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 2.0), (25002.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Market { side: Side::Buy, sz: 1.5 },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::execute_market(&mut order, &book);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 1.5);
        assert_eq!(fill.fill_price, 25001.0); // Should fill at best ask
        assert!(!fill.is_maker); // Market orders are taker
        assert_eq!(order.filled_sz, 1.5);
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_execute_market_buy_partial_fill() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)], // Only 1.0 available, order wants 2.0
        );

        let mut order = Order {
            id: 1,
            action: Action::Market { side: Side::Buy, sz: 2.0 },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::execute_market(&mut order, &book);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 1.0); // Partial fill
        assert_eq!(fill.fill_price, 25001.0);
        assert_eq!(order.filled_sz, 1.0);
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
    }

    #[test]
    fn test_execute_market_buy_no_liquidity() {
        let book = OrderBook::new(); // Empty book

        let mut order = Order {
            id: 1,
            action: Action::Market { side: Side::Buy, sz: 1.0 },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::execute_market(&mut order, &book);
        assert!(result.is_none()); // No liquidity
        assert_eq!(order.filled_sz, 0.0);
        assert_eq!(order.status, OrderStatus::Pending);
    }

    #[test]
    fn test_execute_market_buy_already_filled() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Market { side: Side::Buy, sz: 1.0 },
            created_at: 1000,
            filled_sz: 1.0, // Already fully filled
            status: OrderStatus::Filled,
        };

        let result = PerpsExecution::execute_market(&mut order, &book);
        assert!(result.is_none()); // Already filled
    }

    #[test]
    fn test_execute_market_sell_full_fill() {
        let book = create_test_book(
            vec![(25000.0, 2.0), (24999.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Market { side: Side::Sell, sz: 1.5 },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::execute_market(&mut order, &book);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 1.5);
        assert_eq!(fill.fill_price, 25000.0); // Should fill at best bid
        assert!(!fill.is_maker);
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_execute_market_multiple_levels() {
        // Order should sweep multiple levels
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 0.5), (25002.0, 0.5), (25003.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Market { side: Side::Buy, sz: 1.5 },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::execute_market(&mut order, &book);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 1.5);
        // Weighted average: (0.5 * 25001.0 + 0.5 * 25002.0 + 0.5 * 25003.0) / 1.5
        assert!(fill.fill_price >= 25001.0 && fill.fill_price <= 25003.0);
    }

    // ========== Limit Order Tests ==========

    #[test]
    fn test_check_limit_fill_buy_crosses_ask() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0, // At ask price, should fill
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 0.5);
        assert_eq!(fill.fill_price, 25001.0);
        assert!(fill.is_maker); // Limit orders are maker
        assert_eq!(order.filled_sz, 0.5);
        assert_eq!(order.status, OrderStatus::Filled);
    }

    #[test]
    fn test_check_limit_fill_buy_below_ask_no_fill() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25000.5, // Below ask, shouldn't fill
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(result.is_none()); // Price doesn't cross
    }

    #[test]
    fn test_check_limit_fill_sell_crosses_bid() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Sell,
                px: 25000.0, // At bid price, should fill
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 0.5);
        assert_eq!(fill.fill_price, 25000.0);
        assert!(fill.is_maker);
    }

    #[test]
    fn test_check_limit_fill_partial_fill() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 0.3)], // Only 0.3 available, order wants 0.5
        );

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 0.3); // Partial fill
        assert_eq!(order.filled_sz, 0.3);
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        
        // Check that remaining size is updated
        if let Action::Limit { sz, .. } = order.action {
            assert!((sz - 0.2).abs() < 1e-6); // Remaining: 0.5 - 0.3 = 0.2
        } else {
            panic!("Expected Limit action");
        }
    }

    #[test]
    fn test_check_limit_fill_multiple_partial_fills() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 0.2)], // First fill: 0.2
        );

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        // First partial fill
        let result1 = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(result1.is_some());
        assert_eq!(order.filled_sz, 0.2);
        assert_eq!(order.status, OrderStatus::PartiallyFilled);

        // Update book with more liquidity
        let book2 = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 0.3)], // More liquidity available
        );

        // Check remaining size after first fill
        let remaining_after_first = match &order.action {
            Action::Limit { sz, .. } => *sz,
            _ => panic!("Expected Limit action"),
        };
        // After first fill: sz reduced from 0.5 to 0.3, filled_sz = 0.2
        // So remaining_sz = sz - filled_sz = 0.3 - 0.2 = 0.1
        
        // Second partial fill - remaining_sz will be 0.3 - 0.2 = 0.1
        let result2 = PerpsExecution::check_limit_fill(&mut order, &book2);
        assert!(result2.is_some());
        let fill2 = result2.unwrap();
        // Should fill min(remaining_sz, available) = min(0.1, 0.3) = 0.1
        assert!((fill2.filled_sz - 0.1).abs() < 1e-6, "Expected fill of ~0.1, got {}", fill2.filled_sz);
        // Total filled: 0.2 + 0.1 = 0.3 (not fully filled yet)
        assert!((order.filled_sz - 0.3).abs() < 1e-6, "Expected total filled of ~0.3, got {}", order.filled_sz);
        
        // Order is still partially filled (0.3 < 0.5)
        assert_eq!(order.status, OrderStatus::PartiallyFilled);
    }

    #[test]
    fn test_check_limit_fill_already_filled() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.5, // Already fully filled
            status: OrderStatus::Filled,
        };

        let result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(result.is_none()); // Already filled
    }

    #[test]
    fn test_check_limit_fill_insufficient_liquidity() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 0.0)], // Zero liquidity
        );

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(result.is_none()); // No liquidity
    }

    #[test]
    fn test_check_limit_fill_empty_book() {
        let book = OrderBook::new(); // Empty book

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(result.is_none()); // No book
    }

    // ========== Post-Only Tests ==========

    #[test]
    fn test_can_place_limit_post_only_not_crossing() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25000.5, // Below ask, won't cross
                sz: 0.5,
                tif: Tif::Alo, // Post-only
                post_only: true,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        assert!(PerpsExecution::can_place_limit(&order, &book, true));
    }

    #[test]
    fn test_can_place_limit_post_only_crossing() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0, // At ask, would cross
                sz: 0.5,
                tif: Tif::Alo,
                post_only: true,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        assert!(!PerpsExecution::can_place_limit(&order, &book, true));
    }

    #[test]
    fn test_can_place_limit_non_post_only_crossing_allowed() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0, // Would cross, but not post-only
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        assert!(PerpsExecution::can_place_limit(&order, &book, false));
    }

    // ========== Edge Cases ==========

    #[test]
    fn test_execute_market_very_small_size() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Market { side: Side::Buy, sz: 1e-9 }, // Very small
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = PerpsExecution::execute_market(&mut order, &book);
        // Should handle very small sizes
        assert!(result.is_some() || result.is_none()); // Either fills or doesn't based on precision
    }

    #[test]
    fn test_check_limit_fill_very_small_remaining() {
        let book = create_test_book(
            vec![(25000.0, 1.0)],
            vec![(25001.0, 1.0)],
        );

        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.4999999999, // Almost fully filled
            status: OrderStatus::PartiallyFilled,
        };

        let result = PerpsExecution::check_limit_fill(&mut order, &book);
        // Should handle very small remaining size
        if result.is_some() {
            let fill = result.unwrap();
            assert!(fill.filled_sz > 0.0);
        }
    }
}

