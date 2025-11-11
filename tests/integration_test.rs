#[cfg(test)]
mod tests {
    use hl_backtest::data::types::Candle;
    use hl_backtest::orders::fills::process_order_fill;
    use hl_backtest::orders::types::{Action, Order, OrderStatus, Side, Tif};
    use hl_backtest::fees::FeeCalculator;
    use hl_backtest::portfolio::Portfolio;

    fn create_test_candle(time: u64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            time_open: time,
            time_close: time + 60000,
            coin: "ETH".to_string(),
            interval: "1m".to_string(),
            open,
            close,
            high,
            low,
            volume: 1000.0,
            num_trades: 100,
        }
    }

    #[test]
    fn test_market_order_fill() {
        let candle = create_test_candle(1000, 100.0, 105.0, 95.0, 102.0);
        let fee_calc = FeeCalculator::new(-1, 10, 5);
        let portfolio = Portfolio::new(10000.0, fee_calc.clone());

        let mut order = Order {
            id: 1,
            action: Action::Market {
                side: Side::Buy,
                sz: 1.0,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = process_order_fill(&mut order, &candle, &portfolio, &fee_calc);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 1.0);
        assert!(!fill.is_maker); // Market orders are always taker
        assert_eq!(fill.order_status, OrderStatus::Filled);
    }

    #[test]
    fn test_limit_order_fill_buy() {
        let candle = create_test_candle(1000, 100.0, 105.0, 95.0, 102.0);
        let fee_calc = FeeCalculator::new(-1, 10, 5);
        let portfolio = Portfolio::new(10000.0, fee_calc.clone());

        // Buy limit at 98, should fill since low (95) <= 98
        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 98.0,
                sz: 1.0,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = process_order_fill(&mut order, &candle, &portfolio, &fee_calc);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.filled_sz, 1.0);
        assert_eq!(fill.fill_price, 98.0); // Fills at limit price
    }

    #[test]
    fn test_limit_order_no_fill() {
        let candle = create_test_candle(1000, 100.0, 105.0, 95.0, 102.0);
        let fee_calc = FeeCalculator::new(-1, 10, 5);
        let portfolio = Portfolio::new(10000.0, fee_calc.clone());

        // Buy limit at 90, should not fill since low (95) > 90
        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 90.0,
                sz: 1.0,
                tif: Tif::Gtc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = process_order_fill(&mut order, &candle, &portfolio, &fee_calc);
        assert!(result.is_none()); // Should not fill
    }

    #[test]
    fn test_ioc_order_cancel() {
        let candle = create_test_candle(1000, 100.0, 105.0, 95.0, 102.0);
        let fee_calc = FeeCalculator::new(-1, 10, 5);
        let portfolio = Portfolio::new(10000.0, fee_calc.clone());

        // IOC limit at 90, should cancel since not filled
        let mut order = Order {
            id: 1,
            action: Action::Limit {
                side: Side::Buy,
                px: 90.0,
                sz: 1.0,
                tif: Tif::Ioc,
                post_only: false,
                reduce_only: false,
            },
            created_at: 1000,
            filled_sz: 0.0,
            status: OrderStatus::Pending,
        };

        let result = process_order_fill(&mut order, &candle, &portfolio, &fee_calc);
        assert!(result.is_some());
        let fill = result.unwrap();
        assert_eq!(fill.order_status, OrderStatus::Canceled);
    }
}

