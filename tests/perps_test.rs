#[cfg(test)]
mod tests {
    use hl_backtest::ingest::{parse_l2_jsonl, OrderLevel};
    use hl_backtest::orderbook::OrderBook;
    use hl_backtest::perps::funding::FundingSchedule;
    use hl_backtest::perps::execution::PerpsExecution;
    use hl_backtest::orders::types::{Action, Order, OrderStatus, Side, Tif};

    #[test]
    fn test_orderbook_reconstruction() {
        let jsonl = r#"{"time":"2023-09-16T09:00:00Z","raw":{"data":{"time":1694858400000,"levels":[[{"px":"25000","sz":"1.5","n":1},{"px":"24999","sz":"2.0","n":2}],[{"px":"25001","sz":"1.0","n":3},{"px":"25002","sz":"1.5","n":4}]]}}}
{"time":"2023-09-16T09:00:01Z","raw":{"data":{"time":1694858401000,"levels":[[{"px":"25000.5","sz":"1.2","n":1}],[{"px":"25001.5","sz":"0.8","n":3}]]}}}"#;

        let events = parse_l2_jsonl(jsonl).unwrap();
        assert_eq!(events.len(), 2);

        let mut book = OrderBook::new();
        book.apply_snapshot(&events[0].levels);

        let best_bid = book.best_bid().unwrap();
        assert_eq!(best_bid.0, 25000.0);
        assert_eq!(best_bid.1, 1.5);

        let best_ask = book.best_ask().unwrap();
        assert_eq!(best_ask.0, 25001.0);
        assert_eq!(best_ask.1, 1.0);

        // Apply second snapshot
        book.apply_snapshot(&events[1].levels);
        let best_bid2 = book.best_bid().unwrap();
        assert_eq!(best_bid2.0, 25000.5);
    }

    #[test]
    fn test_funding_schedule() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);
        schedule.add_point(2000, 0.0002);
        schedule.add_point(3000, 0.00015);

        assert_eq!(schedule.rate_at(1500), Some(0.0001));
        assert_eq!(schedule.rate_at(2500), Some(0.0002));
        assert_eq!(schedule.rate_at(500), None);

        let payment = schedule.calculate_payment(10000.0, 1500);
        assert_eq!(payment, 1.0); // 10000 * 0.0001

        let timestamps = schedule.timestamps_in_range(1500, 2500);
        assert_eq!(timestamps.len(), 1);
        assert_eq!(timestamps[0], 2000);
    }

    #[test]
    fn test_limit_order_execution() {
        let mut book = OrderBook::new();
        let levels = vec![
            vec![OrderLevel { px: 25000.0, sz: 1.0, n: 1 }],
            vec![OrderLevel { px: 25001.0, sz: 2.0, n: 2 }],
        ];
        book.apply_snapshot(&levels);

        // Create a limit buy order at 25001.0 (crosses ask)
        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0,
                sz: 1.0,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        // Should fill immediately since limit price crosses ask
        let fill_result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(fill_result.is_some());
        let fill = fill_result.unwrap();
        assert_eq!(fill.filled_sz, 1.0);
        assert_eq!(fill.fill_price, 25001.0);
        assert!(fill.is_maker);
    }

    #[test]
    fn test_post_only_rejection() {
        let mut book = OrderBook::new();
        let levels = vec![
            vec![OrderLevel { px: 25000.0, sz: 1.0, n: 1 }],
            vec![OrderLevel { px: 25001.0, sz: 1.0, n: 2 }],
        ];
        book.apply_snapshot(&levels);

        let order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25001.0, // Would cross
                sz: 1.0,
                tif: Tif::Alo,
                post_only: true,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        // Post-only order should be rejected if crossing
        assert!(!PerpsExecution::can_place_limit(&order, &book, true));
    }
}

