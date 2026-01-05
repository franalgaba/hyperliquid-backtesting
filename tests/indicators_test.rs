//! Tests for the indicators module

use hl_backtest::data::types::Candle;
use hl_backtest::indicators2::{create_indicator, IndicatorEvaluator, IndicatorRegistry};
use std::collections::HashMap;

fn create_test_candle(close: f64, high: f64, low: f64, volume: f64) -> Candle {
    Candle {
        time_open: 1704067200000,
        time_close: 1704070800000,
        coin: "BTC".to_string(),
        interval: "1h".to_string(),
        open: close - 10.0,
        high,
        low,
        close,
        volume,
        num_trades: 100,
    }
}

fn create_candle_series(prices: &[f64]) -> Vec<Candle> {
    prices
        .iter()
        .enumerate()
        .map(|(i, &price)| Candle {
            time_open: 1704067200000 + (i as u64 * 3600000),
            time_close: 1704070800000 + (i as u64 * 3600000),
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: price - 5.0,
            high: price + 10.0,
            low: price - 10.0,
            close: price,
            volume: 1000.0,
            num_trades: 100,
        })
        .collect()
}

#[test]
fn test_sma_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 20.0);

    let indicator = create_indicator("SMA", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_sma_indicator_calculation() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 5.0);

    let mut indicator = create_indicator("SMA", &params).unwrap();

    // Feed 5 candles with prices 10, 20, 30, 40, 50
    let prices = [10.0, 20.0, 30.0, 40.0, 50.0];
    let candles = create_candle_series(&prices);

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    // SMA of [10, 20, 30, 40, 50] = 150 / 5 = 30
    let value = indicator.value("value").unwrap();
    assert!((value - 30.0).abs() < 0.001);
}

#[test]
fn test_ema_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 12.0);

    let indicator = create_indicator("EMA", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_ema_indicator_calculation() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 5.0);

    let mut indicator = create_indicator("EMA", &params).unwrap();

    // Feed some candles
    let prices = [100.0, 102.0, 104.0, 103.0, 105.0, 107.0, 106.0, 108.0];
    let candles = create_candle_series(&prices);

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    let value = indicator.value("value").unwrap();
    // EMA should be close to recent prices, less than or around 107-108
    assert!(value > 100.0 && value < 110.0);
}

#[test]
fn test_wma_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 10.0);

    let indicator = create_indicator("WMA", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_rsi_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 14.0);

    let indicator = create_indicator("RSI", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_rsi_indicator_range() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 14.0);

    let mut indicator = create_indicator("RSI", &params).unwrap();

    // Create an uptrend followed by downtrend
    let mut prices: Vec<f64> = (0..20).map(|i| 100.0 + i as f64 * 2.0).collect();
    prices.extend((0..10).map(|i| 140.0 - i as f64 * 2.0));

    let candles = create_candle_series(&prices);

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    let value = indicator.value("value").unwrap();
    // RSI should always be between 0 and 100
    assert!(value >= 0.0 && value <= 100.0);
}

#[test]
fn test_macd_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("fast".to_string(), 12.0);
    params.insert("slow".to_string(), 26.0);
    params.insert("signal".to_string(), 9.0);

    let indicator = create_indicator("MACD", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_macd_indicator_outputs() {
    let mut params = HashMap::new();
    params.insert("fast".to_string(), 12.0);
    params.insert("slow".to_string(), 26.0);
    params.insert("signal".to_string(), 9.0);

    let mut indicator = create_indicator("MACD", &params).unwrap();

    // Need enough candles for MACD warmup
    let prices: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.5)).collect();
    let candles = create_candle_series(&prices);

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    // MACD should have macd, signal and histogram outputs
    let macd = indicator.value("macd");
    let signal = indicator.value("signal");
    let histogram = indicator.value("histogram");

    assert!(macd.is_ok());
    assert!(signal.is_ok());
    assert!(histogram.is_ok());
}

#[test]
fn test_bbands_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 20.0);
    params.insert("std".to_string(), 2.0);

    let indicator = create_indicator("BBANDS", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_bbands_indicator_outputs() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 20.0);
    params.insert("std".to_string(), 2.0);

    let mut indicator = create_indicator("BBANDS", &params).unwrap();

    let prices: Vec<f64> = (0..30).map(|i| 100.0 + (i % 5) as f64).collect();
    let candles = create_candle_series(&prices);

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    // BBands should have upper, middle, lower outputs
    let upper = indicator.value("upper");
    let middle = indicator.value("middle");
    let lower = indicator.value("lower");

    assert!(upper.is_ok());
    assert!(middle.is_ok());
    assert!(lower.is_ok());

    // Upper > Middle > Lower
    let upper_val = upper.unwrap();
    let middle_val = middle.unwrap();
    let lower_val = lower.unwrap();
    assert!(upper_val >= middle_val);
    assert!(middle_val >= lower_val);
}

#[test]
fn test_stoch_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("k_period".to_string(), 14.0);
    params.insert("d_period".to_string(), 3.0);

    let indicator = create_indicator("STOCH", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_stoch_indicator_range() {
    let mut params = HashMap::new();
    params.insert("k_period".to_string(), 14.0);
    params.insert("k_smooth".to_string(), 1.0);
    params.insert("d_period".to_string(), 3.0);

    let mut indicator = create_indicator("STOCH", &params).unwrap();

    let prices: Vec<f64> = (0..30)
        .map(|i| 100.0 + 10.0 * (i as f64 * 0.5).sin())
        .collect();
    let candles = create_candle_series(&prices);

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    // Stochastic should be between 0 and 100
    let k = indicator.value("value").unwrap();
    let d = indicator.value("d").unwrap();

    assert!(k >= 0.0 && k <= 100.0);
    assert!(d >= 0.0 && d <= 100.0);
}

#[test]
fn test_atr_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("period".to_string(), 14.0);

    let indicator = create_indicator("ATR", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_atr_indicator_positive() {
    let mut params = HashMap::new();
    params.insert("period".to_string(), 14.0);

    let mut indicator = create_indicator("ATR", &params).unwrap();

    // Create candles with known volatility
    let candles: Vec<Candle> = (0..20)
        .map(|i| Candle {
            time_open: 1704067200000 + (i as u64 * 3600000),
            time_close: 1704070800000 + (i as u64 * 3600000),
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 100.0,
            high: 105.0, // $5 range
            low: 95.0,
            close: 100.0 + (i % 3) as f64,
            volume: 1000.0,
            num_trades: 100,
        })
        .collect();

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    let value = indicator.value("value").unwrap();
    // ATR should be positive
    assert!(value > 0.0);
}

#[test]
fn test_adx_indicator_creation() {
    let mut params = HashMap::new();
    params.insert("period".to_string(), 14.0);

    let indicator = create_indicator("ADX", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_adx_indicator_range() {
    let mut params = HashMap::new();
    params.insert("period".to_string(), 14.0);

    let mut indicator = create_indicator("ADX", &params).unwrap();

    // Create trending candles
    let candles: Vec<Candle> = (0..40)
        .map(|i| Candle {
            time_open: 1704067200000 + (i as u64 * 3600000),
            time_close: 1704070800000 + (i as u64 * 3600000),
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 100.0 + i as f64,
            high: 105.0 + i as f64,
            low: 95.0 + i as f64,
            close: 102.0 + i as f64,
            volume: 1000.0,
            num_trades: 100,
        })
        .collect();

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    let value = indicator.value("value").unwrap();
    // ADX should be between 0 and 100
    assert!(value >= 0.0 && value <= 100.0);
}

#[test]
fn test_obv_indicator_creation() {
    let params = HashMap::new();

    let indicator = create_indicator("OBV", &params);
    assert!(indicator.is_ok());
}

#[test]
fn test_obv_indicator_calculation() {
    let params = HashMap::new();
    let mut indicator = create_indicator("OBV", &params).unwrap();

    // Up candle followed by down candle
    let candles = vec![
        Candle {
            time_open: 1704067200000,
            time_close: 1704070800000,
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 102.0, // Up
            volume: 1000.0,
            num_trades: 100,
        },
        Candle {
            time_open: 1704070800000,
            time_close: 1704074400000,
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 102.0,
            high: 103.0,
            low: 99.0,
            close: 100.0, // Down
            volume: 500.0,
            num_trades: 50,
        },
    ];

    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    // OBV should be: +1000 (up) - 500 (down) = 500
    let value = indicator.value("value").unwrap();
    assert!((value - 500.0).abs() < 0.001);
}

#[test]
fn test_unknown_indicator_fails() {
    let params = HashMap::new();
    let indicator = create_indicator("UNKNOWN_INDICATOR", &params);
    assert!(indicator.is_err());
}

#[test]
fn test_indicator_registry_lookback() {
    let registry = IndicatorRegistry::new();

    // Test SMA lookback
    let mut params = HashMap::new();
    params.insert("length".to_string(), 20.0);
    let lookback = registry.get_lookback("SMA", &params).unwrap();
    assert_eq!(lookback, 20);

    // Test RSI lookback
    params.clear();
    params.insert("length".to_string(), 14.0);
    let lookback = registry.get_lookback("RSI", &params).unwrap();
    assert_eq!(lookback, 15); // period + 1

    // Test MACD lookback
    params.clear();
    params.insert("fast".to_string(), 12.0);
    params.insert("slow".to_string(), 26.0);
    params.insert("signal".to_string(), 9.0);
    let lookback = registry.get_lookback("MACD", &params).unwrap();
    assert_eq!(lookback, 26 + 9 * 3); // slow + signal * 3
}

#[test]
fn test_indicator_reset() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 5.0);

    let mut indicator = create_indicator("SMA", &params).unwrap();

    let candles = create_candle_series(&[10.0, 20.0, 30.0, 40.0, 50.0]);
    for candle in &candles {
        indicator.update(candle).unwrap();
    }

    let value_before = indicator.value("value").unwrap();
    assert!((value_before - 30.0).abs() < 0.001);

    // Reset the indicator
    indicator.reset();

    // Feed new candles
    let new_candles = create_candle_series(&[100.0, 100.0, 100.0, 100.0, 100.0]);
    for candle in &new_candles {
        indicator.update(candle).unwrap();
    }

    let value_after = indicator.value("value").unwrap();
    assert!((value_after - 100.0).abs() < 0.001);
}

#[test]
fn test_indicator_warmup() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 20.0);

    let indicator = create_indicator("SMA", &params).unwrap();
    assert_eq!(indicator.warmup(), 20);
}

#[test]
fn test_case_insensitive_indicator_type() {
    let params = HashMap::new();

    // All should work
    assert!(create_indicator("SMA", &params).is_ok());
    assert!(create_indicator("sma", &params).is_ok());
    assert!(create_indicator("Sma", &params).is_ok());

    assert!(create_indicator("RSI", &params).is_ok());
    assert!(create_indicator("rsi", &params).is_ok());

    assert!(create_indicator("MACD", &params).is_ok());
    assert!(create_indicator("macd", &params).is_ok());
}

#[test]
fn test_indicator_default_params() {
    let params = HashMap::new();

    // SMA with default length (20)
    let indicator = create_indicator("SMA", &params).unwrap();
    assert_eq!(indicator.warmup(), 20);

    // RSI with default length (14)
    let indicator = create_indicator("RSI", &params).unwrap();
    assert_eq!(indicator.warmup(), 15); // 14 + 1
}

#[test]
fn test_bb_alias() {
    let mut params = HashMap::new();
    params.insert("length".to_string(), 20.0);
    params.insert("std".to_string(), 2.0);

    // Both "BBANDS" and "BB" should work
    assert!(create_indicator("BBANDS", &params).is_ok());
    assert!(create_indicator("BB", &params).is_ok());
}

#[test]
fn test_stochastic_alias() {
    let mut params = HashMap::new();
    params.insert("k_period".to_string(), 14.0);

    // Both "STOCH" and "STOCHASTIC" should work
    assert!(create_indicator("STOCH", &params).is_ok());
    assert!(create_indicator("STOCHASTIC", &params).is_ok());
}
