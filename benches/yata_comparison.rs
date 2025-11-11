use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use hl_backtest::data::types::Candle;
use hl_backtest::indicators2::create_indicator;
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

// Single candle for per-iteration benchmarks (matching yata format)
fn create_single_candle() -> Candle {
    create_test_candle(1000, 100.0, 102.0, 99.0, 101.0, 1000.0)
}

fn bench_sma_w10(c: &mut Criterion) {
    c.bench_function("sma_w10", |b| {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 10.0);
        let mut indicator = create_indicator("SMA", &params).unwrap();
        let candle = create_single_candle();
        
        b.iter(|| {
            indicator.update(black_box(&candle)).unwrap();
            black_box(indicator.value("value").unwrap());
        });
    });
}

fn bench_sma_w100(c: &mut Criterion) {
    c.bench_function("sma_w100", |b| {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 100.0);
        let mut indicator = create_indicator("SMA", &params).unwrap();
        let candle = create_single_candle();
        
        b.iter(|| {
            indicator.update(black_box(&candle)).unwrap();
            black_box(indicator.value("value").unwrap());
        });
    });
}

fn bench_ema_w10(c: &mut Criterion) {
    c.bench_function("ema_w10", |b| {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 10.0);
        let mut indicator = create_indicator("EMA", &params).unwrap();
        let candle = create_single_candle();
        
        b.iter(|| {
            indicator.update(black_box(&candle)).unwrap();
            black_box(indicator.value("value").unwrap());
        });
    });
}

fn bench_ema_w100(c: &mut Criterion) {
    c.bench_function("ema_w100", |b| {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 100.0);
        let mut indicator = create_indicator("EMA", &params).unwrap();
        let candle = create_single_candle();
        
        b.iter(|| {
            indicator.update(black_box(&candle)).unwrap();
            black_box(indicator.value("value").unwrap());
        });
    });
}

fn bench_wma_w10(c: &mut Criterion) {
    c.bench_function("wma_w10", |b| {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 10.0);
        let mut indicator = create_indicator("WMA", &params).unwrap();
        let candle = create_single_candle();
        
        b.iter(|| {
            indicator.update(black_box(&candle)).unwrap();
            black_box(indicator.value("value").unwrap());
        });
    });
}

fn bench_wma_w100(c: &mut Criterion) {
    c.bench_function("wma_w100", |b| {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 100.0);
        let mut indicator = create_indicator("WMA", &params).unwrap();
        let candle = create_single_candle();
        
        b.iter(|| {
            indicator.update(black_box(&candle)).unwrap();
            black_box(indicator.value("value").unwrap());
        });
    });
}

fn bench_rsi_w10(c: &mut Criterion) {
    c.bench_function("rsi_w10", |b| {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 10.0);
        let mut indicator = create_indicator("RSI", &params).unwrap();
        let candle = create_single_candle();
        
        b.iter(|| {
            indicator.update(black_box(&candle)).unwrap();
            black_box(indicator.value("value").unwrap());
        });
    });
}

fn bench_rsi_w100(c: &mut Criterion) {
    c.bench_function("rsi_w100", |b| {
        let mut params = HashMap::new();
        params.insert("length".to_string(), 100.0);
        let mut indicator = create_indicator("RSI", &params).unwrap();
        let candle = create_single_candle();
        
        b.iter(|| {
            indicator.update(black_box(&candle)).unwrap();
            black_box(indicator.value("value").unwrap());
        });
    });
}

criterion_group!(
    benches,
    bench_sma_w10,
    bench_sma_w100,
    bench_ema_w10,
    bench_ema_w100,
    bench_wma_w10,
    bench_wma_w100,
    bench_rsi_w10,
    bench_rsi_w100
);
criterion_main!(benches);

