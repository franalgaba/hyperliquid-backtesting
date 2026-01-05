//! Tests for the Parquet export module

use hl_backtest::data::parquet::{
    export_candles_to_parquet, export_equity_to_parquet, export_funding_to_parquet,
    export_trades_to_parquet, read_candles_from_parquet, FundingPayment,
};
use hl_backtest::data::types::Candle;
use hl_backtest::orders::types::{EquityPoint, Trade};
use tempfile::tempdir;

fn create_test_candles() -> Vec<Candle> {
    vec![
        Candle {
            time_open: 1704067200000, // 2024-01-01 00:00:00
            time_close: 1704070800000,
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 42000.0,
            high: 42500.0,
            low: 41800.0,
            close: 42300.0,
            volume: 1500.0,
            num_trades: 1200,
        },
        Candle {
            time_open: 1704070800000,
            time_close: 1704074400000,
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 42300.0,
            high: 42800.0,
            low: 42100.0,
            close: 42600.0,
            volume: 1800.0,
            num_trades: 1500,
        },
        Candle {
            time_open: 1704074400000,
            time_close: 1704078000000,
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 42600.0,
            high: 43000.0,
            low: 42400.0,
            close: 42900.0,
            volume: 2000.0,
            num_trades: 1800,
        },
    ]
}

fn create_test_trades() -> Vec<Trade> {
    vec![
        Trade {
            timestamp: 1704067200000,
            symbol: "BTC".to_string(),
            side: "BUY".to_string(),
            size: 0.5,
            price: 42000.0,
            fee: 2.1,
            order_id: 1,
        },
        Trade {
            timestamp: 1704074400000,
            symbol: "BTC".to_string(),
            side: "SELL".to_string(),
            size: 0.5,
            price: 42900.0,
            fee: 2.145,
            order_id: 2,
        },
    ]
}

fn create_test_equity_curve() -> Vec<EquityPoint> {
    vec![
        EquityPoint {
            timestamp: 1704067200000,
            equity: 10000.0,
            cash: 10000.0,
            position_value: 0.0,
        },
        EquityPoint {
            timestamp: 1704067200000,
            equity: 9997.9, // After buy fee
            cash: 9997.9 - 21000.0,
            position_value: 21000.0,
        },
        EquityPoint {
            timestamp: 1704074400000,
            equity: 10445.0, // After sell
            cash: 10445.0,
            position_value: 0.0,
        },
    ]
}

fn create_test_funding_payments() -> Vec<FundingPayment> {
    vec![
        FundingPayment {
            timestamp: 1704096000000, // 8 hours after start
            coin: "BTC".to_string(),
            rate: 0.0001,
            payment: -2.1, // Paid by long
            position_size: 0.5,
        },
        FundingPayment {
            timestamp: 1704124800000, // 16 hours after start
            coin: "BTC".to_string(),
            rate: -0.00005,
            payment: 1.05, // Received by long
            position_size: 0.5,
        },
    ]
}

#[test]
fn test_candles_parquet_roundtrip() {
    let candles = create_test_candles();
    let dir = tempdir().unwrap();
    let path = dir.path().join("candles.parquet");

    // Export
    export_candles_to_parquet(&candles, &path).unwrap();
    assert!(path.exists());

    // Import
    let loaded = read_candles_from_parquet(&path).unwrap();

    // Verify
    assert_eq!(loaded.len(), candles.len());
    for (original, loaded) in candles.iter().zip(loaded.iter()) {
        assert_eq!(loaded.time_open, original.time_open);
        assert_eq!(loaded.time_close, original.time_close);
        assert_eq!(loaded.coin, original.coin);
        assert_eq!(loaded.interval, original.interval);
        assert!((loaded.open - original.open).abs() < 0.001);
        assert!((loaded.high - original.high).abs() < 0.001);
        assert!((loaded.low - original.low).abs() < 0.001);
        assert!((loaded.close - original.close).abs() < 0.001);
        assert!((loaded.volume - original.volume).abs() < 0.001);
        assert_eq!(loaded.num_trades, original.num_trades);
    }
}

#[test]
fn test_candles_parquet_empty() {
    let candles: Vec<Candle> = vec![];
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty_candles.parquet");

    export_candles_to_parquet(&candles, &path).unwrap();
    let loaded = read_candles_from_parquet(&path).unwrap();

    assert!(loaded.is_empty());
}

#[test]
fn test_candles_parquet_creates_parent_dirs() {
    let candles = create_test_candles();
    let dir = tempdir().unwrap();
    let path = dir.path().join("nested").join("deep").join("candles.parquet");

    export_candles_to_parquet(&candles, &path).unwrap();
    assert!(path.exists());
}

#[test]
fn test_trades_parquet_export() {
    let trades = create_test_trades();
    let dir = tempdir().unwrap();
    let path = dir.path().join("trades.parquet");

    export_trades_to_parquet(&trades, &path).unwrap();
    assert!(path.exists());

    // Verify file size is reasonable (should be > 0 bytes)
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
fn test_trades_parquet_empty() {
    let trades: Vec<Trade> = vec![];
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty_trades.parquet");

    export_trades_to_parquet(&trades, &path).unwrap();
    assert!(path.exists());
}

#[test]
fn test_equity_parquet_export() {
    let equity = create_test_equity_curve();
    let dir = tempdir().unwrap();
    let path = dir.path().join("equity.parquet");

    export_equity_to_parquet(&equity, &path).unwrap();
    assert!(path.exists());

    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
fn test_equity_parquet_empty() {
    let equity: Vec<EquityPoint> = vec![];
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty_equity.parquet");

    export_equity_to_parquet(&equity, &path).unwrap();
    assert!(path.exists());
}

#[test]
fn test_funding_parquet_export() {
    let payments = create_test_funding_payments();
    let dir = tempdir().unwrap();
    let path = dir.path().join("funding.parquet");

    export_funding_to_parquet(&payments, &path).unwrap();
    assert!(path.exists());

    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
fn test_funding_parquet_empty() {
    let payments: Vec<FundingPayment> = vec![];
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty_funding.parquet");

    export_funding_to_parquet(&payments, &path).unwrap();
    assert!(path.exists());
}

#[test]
fn test_multiple_exports_same_dir() {
    let dir = tempdir().unwrap();

    let candles = create_test_candles();
    let trades = create_test_trades();
    let equity = create_test_equity_curve();
    let funding = create_test_funding_payments();

    export_candles_to_parquet(&candles, dir.path().join("candles.parquet")).unwrap();
    export_trades_to_parquet(&trades, dir.path().join("trades.parquet")).unwrap();
    export_equity_to_parquet(&equity, dir.path().join("equity.parquet")).unwrap();
    export_funding_to_parquet(&funding, dir.path().join("funding.parquet")).unwrap();

    assert!(dir.path().join("candles.parquet").exists());
    assert!(dir.path().join("trades.parquet").exists());
    assert!(dir.path().join("equity.parquet").exists());
    assert!(dir.path().join("funding.parquet").exists());
}

#[test]
fn test_candles_large_dataset() {
    // Create 1000 candles
    let candles: Vec<Candle> = (0..1000)
        .map(|i| Candle {
            time_open: 1704067200000 + (i as u64 * 3600000),
            time_close: 1704070800000 + (i as u64 * 3600000),
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 42000.0 + (i as f64 * 10.0),
            high: 42500.0 + (i as f64 * 10.0),
            low: 41800.0 + (i as f64 * 10.0),
            close: 42300.0 + (i as f64 * 10.0),
            volume: 1500.0 + (i as f64),
            num_trades: 1200 + i as i64,
        })
        .collect();

    let dir = tempdir().unwrap();
    let path = dir.path().join("large_candles.parquet");

    export_candles_to_parquet(&candles, &path).unwrap();
    let loaded = read_candles_from_parquet(&path).unwrap();

    assert_eq!(loaded.len(), 1000);
    assert_eq!(loaded[0].time_open, candles[0].time_open);
    assert_eq!(loaded[999].time_open, candles[999].time_open);
}

#[test]
fn test_parquet_compression() {
    // Verify that Parquet files are compressed (smaller than equivalent CSV)
    let candles: Vec<Candle> = (0..100)
        .map(|i| Candle {
            time_open: 1704067200000 + (i as u64 * 3600000),
            time_close: 1704070800000 + (i as u64 * 3600000),
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 42000.0,
            high: 42500.0,
            low: 41800.0,
            close: 42300.0,
            volume: 1500.0,
            num_trades: 1200,
        })
        .collect();

    let dir = tempdir().unwrap();
    let path = dir.path().join("compressed.parquet");

    export_candles_to_parquet(&candles, &path).unwrap();

    let metadata = std::fs::metadata(&path).unwrap();
    // 100 candles * ~100 bytes each = ~10KB uncompressed
    // Parquet with Snappy should be significantly smaller
    assert!(metadata.len() < 5000, "File should be compressed");
}

#[test]
fn test_different_coins() {
    let candles = vec![
        Candle {
            time_open: 1704067200000,
            time_close: 1704070800000,
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 42000.0,
            high: 42500.0,
            low: 41800.0,
            close: 42300.0,
            volume: 1500.0,
            num_trades: 1200,
        },
        Candle {
            time_open: 1704067200000,
            time_close: 1704070800000,
            coin: "ETH".to_string(),
            interval: "1h".to_string(),
            open: 2200.0,
            high: 2250.0,
            low: 2180.0,
            close: 2230.0,
            volume: 5000.0,
            num_trades: 3000,
        },
        Candle {
            time_open: 1704067200000,
            time_close: 1704070800000,
            coin: "SOL".to_string(),
            interval: "1h".to_string(),
            open: 100.0,
            high: 102.0,
            low: 99.0,
            close: 101.0,
            volume: 10000.0,
            num_trades: 5000,
        },
    ];

    let dir = tempdir().unwrap();
    let path = dir.path().join("multi_coin.parquet");

    export_candles_to_parquet(&candles, &path).unwrap();
    let loaded = read_candles_from_parquet(&path).unwrap();

    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0].coin, "BTC");
    assert_eq!(loaded[1].coin, "ETH");
    assert_eq!(loaded[2].coin, "SOL");
}

#[test]
fn test_different_intervals() {
    let candles = vec![
        Candle {
            time_open: 1704067200000,
            time_close: 1704067260000,
            coin: "BTC".to_string(),
            interval: "1m".to_string(),
            open: 42000.0,
            high: 42050.0,
            low: 41980.0,
            close: 42030.0,
            volume: 50.0,
            num_trades: 30,
        },
        Candle {
            time_open: 1704067200000,
            time_close: 1704070800000,
            coin: "BTC".to_string(),
            interval: "1h".to_string(),
            open: 42000.0,
            high: 42500.0,
            low: 41800.0,
            close: 42300.0,
            volume: 1500.0,
            num_trades: 1200,
        },
        Candle {
            time_open: 1704067200000,
            time_close: 1704153600000,
            coin: "BTC".to_string(),
            interval: "1d".to_string(),
            open: 42000.0,
            high: 43000.0,
            low: 41500.0,
            close: 42800.0,
            volume: 50000.0,
            num_trades: 40000,
        },
    ];

    let dir = tempdir().unwrap();
    let path = dir.path().join("multi_interval.parquet");

    export_candles_to_parquet(&candles, &path).unwrap();
    let loaded = read_candles_from_parquet(&path).unwrap();

    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded[0].interval, "1m");
    assert_eq!(loaded[1].interval, "1h");
    assert_eq!(loaded[2].interval, "1d");
}
