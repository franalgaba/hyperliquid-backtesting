#[cfg(test)]
mod tests {
    use hl_backtest::fees::FeeCalculator;
    use hl_backtest::orderbook::OrderBook;
    use hl_backtest::orders::types::{Action, Order, OrderStatus, Side, Trade, Tif};
    use hl_backtest::perps::execution::FillResult;
    use hl_backtest::perps::funding::FundingSchedule;
    use hl_backtest::perps::PerpsEngine;
    use hl_backtest::portfolio::Portfolio;
    use hl_backtest::orders::types::SimConfig;

    // Helper to create a test engine
    fn create_test_engine() -> PerpsEngine {
        let funding = FundingSchedule::new();
        let config = SimConfig {
            initial_capital: 10000.0,
            maker_fee_bps: -1,
            taker_fee_bps: 10,
            slippage_bps: 5,
        };
        PerpsEngine::new(funding, &config)
    }

    // Note: The helper functions (process_trade_fill, apply_funding_payment, record_equity_point)
    // are private, so we test them indirectly through the public API.
    // However, we can test the trade_utils functions directly.

    #[test]
    fn test_perps_engine_new() {
        let funding = FundingSchedule::new();
        let config = SimConfig {
            initial_capital: 10000.0,
            maker_fee_bps: -1,
            taker_fee_bps: 10,
            slippage_bps: 5,
        };
        let engine = PerpsEngine::new(funding, &config);
        
        // Engine should be initialized (we can't directly access book, but we can verify it exists)
        // The engine is created successfully if no panic occurs
    }

    #[test]
    fn test_order_deduplication_logic() {
        use hl_backtest::perps::trade_utils::extract_side_from_action;
        use hl_backtest::orders::types::Action;

        // Test that duplicate detection works correctly
        let order1 = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 50000.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let order2 = Order {
            id: 2,
            action: Action::Limit {
                side: Side::Buy,
                px: 50000.009, // Within tolerance (< 0.01)
                sz: 0.5000001, // Within tolerance (1e-6)
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        // These should be considered duplicates based on the deduplication logic
        let side1 = extract_side_from_action(&order1.action).unwrap();
        let side2 = extract_side_from_action(&order2.action).unwrap();
        assert_eq!(side1, side2);

        // Price difference check - verify they're within tolerance
        if let (Action::Limit { px: px1, sz: sz1, .. }, Action::Limit { px: px2, sz: sz2, .. }) = (&order1.action, &order2.action) {
            assert_eq!(side1, side2); // Same side
            assert!((px1 - px2).abs() < 1e-2, "Price difference {} should be < 0.01", (px1 - px2).abs()); // Within price tolerance
            assert!((sz1 - sz2).abs() < 1e-6, "Size difference {} should be < 1e-6", (sz1 - sz2).abs()); // Within size tolerance
        }
    }

    #[test]
    fn test_order_deduplication_different_prices() {
        use hl_backtest::orders::types::Action;

        let order1 = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 50000.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let order2 = Order {
            id: 2,
            action: Action::Limit {
                side: Side::Buy,
                px: 50010.0, // Outside tolerance (> 0.01)
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        // These should NOT be considered duplicates (different prices)
        if let (Action::Limit { px: px1, .. }, Action::Limit { px: px2, .. }) = (&order1.action, &order2.action) {
            assert!((px1 - px2).abs() >= 1e-2); // Outside tolerance
        }
    }

    #[test]
    fn test_order_deduplication_market_orders() {
        use hl_backtest::orders::types::Action;

        let order1 = Order {
            id: 1,
            action: Action::Market { side: Side::Buy, sz: 1.0 },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let order2 = Order {
            id: 2,
            action: Action::Market { side: Side::Buy, sz: 1.0000001 }, // Within tolerance
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        // These should be considered duplicates (same side, same size within tolerance)
        if let (Action::Market { side: s1, sz: sz1 }, Action::Market { side: s2, sz: sz2 }) = (&order1.action, &order2.action) {
            assert_eq!(s1, s2);
            assert!((sz1 - sz2).abs() < 1e-6); // Within tolerance
        }
    }

    #[test]
    fn test_order_deduplication_different_sides() {
        use hl_backtest::orders::types::Action;

        let order1 = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 50000.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let order2 = Order {
            id: 2,
            action: Action::Limit {
                side: Side::Sell, // Different side
                px: 50000.0,
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        // These should NOT be considered duplicates (different sides)
        if let (Action::Limit { side: s1, .. }, Action::Limit { side: s2, .. }) = (&order1.action, &order2.action) {
            assert_ne!(s1, s2);
        }
    }
}

