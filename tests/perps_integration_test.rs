#[cfg(test)]
mod tests {
    use hl_backtest::ingest::{parse_l2_jsonl, L2Event, OrderLevel};
    use hl_backtest::orderbook::OrderBook;
    use hl_backtest::perps::execution::PerpsExecution;
    use hl_backtest::orders::types::{Action, Order, OrderStatus, Side, Tif};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_perps_playback_flow() {
        // Create mock L2 events
        let jsonl = r#"{"time":"2023-09-16T09:00:00Z","raw":{"data":{"time":1694858400000,"levels":[[{"px":"25000","sz":"1.5","n":1},{"px":"24999","sz":"2.0","n":2}],[{"px":"25001","sz":"1.0","n":3},{"px":"25002","sz":"1.5","n":4}]]}}}
{"time":"2023-09-16T09:00:01Z","raw":{"data":{"time":1694858401000,"levels":[[{"px":"25000.5","sz":"1.2","n":1}],[{"px":"25001.5","sz":"0.8","n":3}]]}}}
{"time":"2023-09-16T09:00:02Z","raw":{"data":{"time":1694858402000,"levels":[[{"px":"25001","sz":"1.0","n":1}],[{"px":"25002","sz":"1.0","n":2}]]}}}"#;

        let events = parse_l2_jsonl(jsonl).unwrap();
        assert_eq!(events.len(), 3);

        // Create temp directory for events
        let temp_dir = TempDir::new().unwrap();
        let events_dir = temp_dir.path().join("BTC");
        fs::create_dir_all(&events_dir).unwrap();

        // Write events to file
        let events_file = events_dir.join("20230916-09.jsonl");
        fs::write(&events_file, jsonl).unwrap();

        // Test order book reconstruction
        let mut book = OrderBook::new();
        for event in &events {
            book.apply_snapshot(&event.levels);
            assert!(book.best_bid().is_some());
            assert!(book.best_ask().is_some());
        }

        // Test limit order execution
        // Apply final snapshot first
        book.apply_snapshot(&events[2].levels);
        
        // Best ask is 25002.0, so limit buy at 25002.0 or above should fill
        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 25002.0, // At best ask
                sz: 0.5,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1694858400000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        // Order should fill at 25002.0 (best ask)
        let fill_result = PerpsExecution::check_limit_fill(&mut order, &book);
        assert!(fill_result.is_some());
        let fill = fill_result.unwrap();
        assert_eq!(fill.fill_price, 25002.0);
        assert_eq!(fill.filled_sz, 0.5);
    }

    #[test]
    fn test_market_order_sweep_integration() {
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

        // Market buy order
        let mut order = Order {
            id: 1,
            action: Action::Market { side: Side::Buy, sz: 2.0 },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let fill_result = PerpsExecution::execute_market(&mut order, &book);
        assert!(fill_result.is_some());
        let fill = fill_result.unwrap();
        assert_eq!(fill.filled_sz, 2.0);
        // Should fill 1.5 at 25001.0 and 0.5 at 25002.0
        assert!(fill.fill_price >= 25001.0 && fill.fill_price <= 25002.0);
        assert!(!fill.is_maker); // Market orders are taker
    }
}

