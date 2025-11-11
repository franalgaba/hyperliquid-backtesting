#[cfg(test)]
mod tests {
    use crate::data::types::Candle;
    use crate::indicators2::create_indicator;
    use std::collections::HashMap;

    fn create_test_candle(time: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle {
            time_open: time,
            time_close: time + 60000,
            coin: "ETH".to_string(),
            interval: "1m".to_string(),
            open,
            close,
            high,
            low,
            volume,
            num_trades: 100,
        }
    }

    #[test]
    fn test_sma() {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 3.0);
        let mut sma = create_indicator("SMA", &params).unwrap();
        
        let candles = vec![
            create_test_candle(1000, 10.0, 12.0, 9.0, 11.0, 1000.0),
            create_test_candle(2000, 11.0, 13.0, 10.0, 12.0, 1100.0),
            create_test_candle(3000, 12.0, 14.0, 11.0, 13.0, 1200.0),
        ];

        for candle in &candles {
            sma.update(candle).unwrap();
        }

        // SMA of [11, 12, 13] = 12.0
        assert!((sma.value("value").unwrap() - 12.0).abs() < 0.001);
    }

    #[test]
    fn test_ema() {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 3.0);
        let mut ema = create_indicator("EMA", &params).unwrap();
        
        let candles = vec![
            create_test_candle(1000, 10.0, 12.0, 9.0, 11.0, 1000.0),
            create_test_candle(2000, 11.0, 13.0, 10.0, 12.0, 1100.0),
            create_test_candle(3000, 12.0, 14.0, 11.0, 13.0, 1200.0),
        ];

        for candle in &candles {
            ema.update(candle).unwrap();
        }

        // EMA should be close to recent values
        let value = ema.value("value").unwrap();
        assert!(value > 11.0 && value < 13.5);
    }

    #[test]
    fn test_rsi() {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 14.0);
        let mut rsi = create_indicator("RSI", &params).unwrap();
        
        // Create upward trending candles
        let mut candles = Vec::new();
        for i in 0..20 {
            let price = 100.0 + i as f64;
            candles.push(create_test_candle(
                (i as u64) * 1000,
                price - 1.0,
                price + 1.0,
                price - 2.0,
                price,
                1000.0,
            ));
        }

        for candle in &candles {
            rsi.update(candle).unwrap();
        }

        // RSI should be high (above 50) for upward trend
        let value = rsi.value("value").unwrap();
        assert!(value > 50.0 && value <= 100.0);
    }

    #[test]
    fn test_macd() {
        let mut params = HashMap::new();
        params.insert("fast".to_string(), 12.0);
        params.insert("slow".to_string(), 26.0);
        params.insert("signal".to_string(), 9.0);
        let mut macd = create_indicator("MACD", &params).unwrap();

        // Create enough candles for MACD warmup
        let mut candles = Vec::new();
        for i in 0..50 {
            let price = 100.0 + (i as f64) * 0.5;
            candles.push(create_test_candle(
                (i as u64) * 1000,
                price - 1.0,
                price + 1.0,
                price - 2.0,
                price,
                1000.0,
            ));
        }

        for candle in &candles {
            macd.update(candle).unwrap();
        }

        // Check MACD outputs exist
        let macd_value = macd.value("macd").unwrap();
        let signal_value = macd.value("signal").unwrap();
        let histogram_value = macd.value("histogram").unwrap();

        assert!((histogram_value - (macd_value - signal_value)).abs() < 0.001);
    }

    #[test]
    fn test_bbands() {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 20.0);
        params.insert("std".to_string(), 2.0);
        let mut bb = create_indicator("BBANDS", &params).unwrap();

        // Create enough candles
        let mut candles = Vec::new();
        for i in 0..30 {
            let price = 100.0 + (i % 5) as f64;
            candles.push(create_test_candle(
                (i as u64) * 1000,
                price - 1.0,
                price + 1.0,
                price - 2.0,
                price,
                1000.0,
            ));
        }

        for candle in &candles {
            bb.update(candle).unwrap();
        }

        let upper = bb.value("upper").unwrap();
        let middle = bb.value("middle").unwrap();
        let lower = bb.value("lower").unwrap();

        assert!(upper > middle);
        assert!(middle > lower);
    }

    #[test]
    fn test_obv() {
        let mut obv = create_indicator("OBV", &std::collections::HashMap::new()).unwrap();

        let candles = vec![
            create_test_candle(1000, 10.0, 12.0, 9.0, 11.0, 1000.0), // Price up
            create_test_candle(2000, 11.0, 13.0, 10.0, 10.0, 2000.0), // Price down
            create_test_candle(3000, 10.0, 12.0, 9.0, 12.0, 1500.0), // Price up
        ];

        for candle in &candles {
            obv.update(candle).unwrap();
        }

        // OBV should accumulate: +1000 - 2000 + 1500 = 500
        let value = obv.value("value").unwrap();
        assert!((value - 500.0).abs() < 0.001);
    }
}

