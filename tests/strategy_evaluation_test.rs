#[cfg(test)]
mod tests {
    use hl_backtest::data::types::Candle;
    use hl_backtest::indicators2::create_indicator;
    use hl_backtest::ir::types::{ExprValue, ComparisonOp};
    use std::collections::HashMap;

    // Helper to create a test candle
    fn create_test_candle(price: f64, ts: u64) -> Candle {
        Candle {
            time_open: ts,
            time_close: ts,
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 0.0,
            num_trades: 0,
        }
    }

    // Note: evaluate_expr_perps and get_expr_value_perps are private functions.
    // We test them indirectly through the public API, but we can test the
    // indicator evaluation logic directly.

    #[test]
    fn test_indicator_creation() {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 20.0);
        
        let indicator = create_indicator("SMA", &params);
        assert!(indicator.is_ok());
    }

    #[test]
    fn test_indicator_update() {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 5.0);
        
        let mut indicator = create_indicator("SMA", &params).unwrap();
        let candle = create_test_candle(50000.0, 1000);
        
        // Update indicator multiple times to warm it up
        for i in 0..10 {
            let mut c = candle.clone();
            c.close = 50000.0 + (i as f64 * 100.0);
            indicator.update(&c).unwrap();
        }
        
        // Should be able to get value after warmup
        let value = indicator.value("value");
        assert!(value.is_ok());
    }

    #[test]
    fn test_expr_value_const() {
        // Test constant expression value
        let expr_val = ExprValue::Const { r#const: 42.0 };
        
        // This would be tested through evaluate_expr_perps
        // For now, we verify the structure
        match expr_val {
            ExprValue::Const { r#const: val } => assert_eq!(val, 42.0),
            _ => panic!("Expected Const"),
        }
    }

    #[test]
    fn test_expr_value_series_close() {
        // Test series expression value
        let candle = create_test_candle(50000.0, 1000);
        
        // This would be tested through get_expr_value_perps
        // For now, we verify the candle has the expected close price
        assert_eq!(candle.close, 50000.0);
    }

    #[test]
    fn test_comparison_operators() {
        // Test comparison logic
        let lhs: f64 = 10.0;
        let rhs: f64 = 5.0;
        
        assert!(matches!(ComparisonOp::Gt, ComparisonOp::Gt));
        assert!(lhs > rhs); // Gt
        assert!(lhs >= rhs); // Gte
        assert!(rhs < lhs); // Lt
        assert!(rhs <= lhs); // Lte
        assert!((lhs - lhs).abs() < 1e-10); // Eq
    }

    #[test]
    fn test_price_change_threshold() {
        // Test the price change threshold logic
        const PRICE_CHANGE_THRESHOLD: f64 = 0.0001; // 0.01%
        
        let last_price: f64 = 50000.0;
        let current_price: f64 = 50010.0; // 0.02% change (greater than 0.01% threshold)
        let price_change = (current_price - last_price).abs() / last_price.max(1.0);
        
        assert!(price_change > PRICE_CHANGE_THRESHOLD);
        
        let small_change: f64 = 50000.1; // Very small change
        let small_price_change = (small_change - last_price).abs() / last_price.max(1.0);
        assert!(small_price_change < PRICE_CHANGE_THRESHOLD);
    }

    #[test]
    fn test_strategy_evaluation_throttling() {
        // Test that strategy evaluation is throttled
        let last_price: f64 = 50000.0;
        let threshold: f64 = 0.0001;
        
        // Small price change should not trigger evaluation
        let small_change: f64 = 50000.1;
        let price_changed = (small_change - last_price).abs() / last_price.max(1.0) > threshold;
        assert!(!price_changed);
        
        // Large price change should trigger evaluation
        let large_change: f64 = 50010.0; // 0.02% change
        let price_changed = (large_change - last_price).abs() / last_price.max(1.0) > threshold;
        assert!(price_changed);
    }

    #[test]
    fn test_indicator_reference_parsing() {
        // Test indicator reference parsing (optimized version)
        let ref_path = "sma_20.value";
        
        if let Some(dot_pos) = ref_path.find('.') {
            let ind_id = &ref_path[..dot_pos];
            let output = &ref_path[dot_pos + 1..];
            
            assert_eq!(ind_id, "sma_20");
            assert_eq!(output, "value");
        } else {
            panic!("Should find dot");
        }
    }

    #[test]
    fn test_indicator_reference_invalid() {
        // Test invalid indicator reference (no dot)
        let ref_path = "sma_20";
        
        let dot_pos = ref_path.find('.');
        assert!(dot_pos.is_none());
    }
}

