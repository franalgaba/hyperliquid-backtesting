use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
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

fn generate_candles(count: usize) -> Vec<Candle> {
    let mut candles = Vec::with_capacity(count);
    for i in 0..count {
        let base_price = 100.0 + (i as f64) * 0.1;
        let noise = (i as f64 % 10.0) * 0.5;
        candles.push(create_test_candle(
            (i as u64) * 1000,
            base_price - 1.0 + noise,
            base_price + 1.0 + noise,
            base_price - 2.0 + noise,
            base_price + noise,
            1000.0 + (i as f64) * 10.0,
        ));
    }
    candles
}

fn bench_sma(c: &mut Criterion) {
    let mut group = c.benchmark_group("SMA");
    
    let windows = vec![10, 20, 50, 100, 200];
    let candles = generate_candles(1000);
    
    for window in windows {
        let mut params = HashMap::new();
        params.insert("length".to_string(), window as f64);
        
        group.throughput(Throughput::Elements(window as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(window),
            &window,
            |b, &window| {
                let mut params = HashMap::new();
                params.insert("length".to_string(), window as f64);
                let mut indicator = create_indicator("SMA", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("value").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_ema(c: &mut Criterion) {
    let mut group = c.benchmark_group("EMA");
    
    let windows = vec![10, 20, 50, 100, 200];
    let candles = generate_candles(1000);
    
    for window in windows {
        group.throughput(Throughput::Elements(window as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(window),
            &window,
            |b, &window| {
                let mut params = HashMap::new();
                params.insert("length".to_string(), window as f64);
                let mut indicator = create_indicator("EMA", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("value").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_wma(c: &mut Criterion) {
    let mut group = c.benchmark_group("WMA");
    
    let windows = vec![10, 20, 50, 100, 200];
    let candles = generate_candles(1000);
    
    for window in windows {
        group.throughput(Throughput::Elements(window as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(window),
            &window,
            |b, &window| {
                let mut params = HashMap::new();
                params.insert("length".to_string(), window as f64);
                let mut indicator = create_indicator("WMA", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("value").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_rsi(c: &mut Criterion) {
    let mut group = c.benchmark_group("RSI");
    
    let windows = vec![7, 14, 21, 28];
    let candles = generate_candles(1000);
    
    for window in windows {
        group.throughput(Throughput::Elements(window as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(window),
            &window,
            |b, &window| {
                let mut params = HashMap::new();
                params.insert("length".to_string(), window as f64);
                let mut indicator = create_indicator("RSI", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("value").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_macd(c: &mut Criterion) {
    let mut group = c.benchmark_group("MACD");
    
    let configs = vec![
        (12, 26, 9),
        (8, 21, 5),
        (19, 39, 9),
    ];
    let candles = generate_candles(1000);
    
    for (fast, slow, signal) in configs {
        let id = format!("{}-{}-{}", fast, slow, signal);
        group.bench_with_input(
            BenchmarkId::from_parameter(id),
            &(fast, slow, signal),
            |b, &(fast, slow, signal)| {
                let mut params = HashMap::new();
                params.insert("fast".to_string(), fast as f64);
                params.insert("slow".to_string(), slow as f64);
                params.insert("signal".to_string(), signal as f64);
                let mut indicator = create_indicator("MACD", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("macd").unwrap());
                        black_box(indicator.value("signal").unwrap());
                        black_box(indicator.value("histogram").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_bbands(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bollinger Bands");
    
    let windows = vec![10, 20, 50, 100];
    let candles = generate_candles(1000);
    
    for window in windows {
        group.throughput(Throughput::Elements(window as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(window),
            &window,
            |b, &window| {
                let mut params = HashMap::new();
                params.insert("length".to_string(), window as f64);
                params.insert("std".to_string(), 2.0);
                let mut indicator = create_indicator("BBANDS", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("upper").unwrap());
                        black_box(indicator.value("middle").unwrap());
                        black_box(indicator.value("lower").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_stoch(c: &mut Criterion) {
    let mut group = c.benchmark_group("Stochastic");
    
    let configs = vec![
        (14, 1, 3),
        (14, 3, 3),
        (21, 1, 5),
    ];
    let candles = generate_candles(1000);
    
    for (k_period, k_smooth, d_period) in configs {
        let id = format!("{}-{}-{}", k_period, k_smooth, d_period);
        group.bench_with_input(
            BenchmarkId::from_parameter(id),
            &(k_period, k_smooth, d_period),
            |b, &(k_period, k_smooth, d_period)| {
                let mut params = HashMap::new();
                params.insert("k_period".to_string(), k_period as f64);
                params.insert("k_smooth".to_string(), k_smooth as f64);
                params.insert("d_period".to_string(), d_period as f64);
                let mut indicator = create_indicator("STOCH", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("k").unwrap());
                        black_box(indicator.value("d").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_atr(c: &mut Criterion) {
    let mut group = c.benchmark_group("ATR");
    
    let windows = vec![7, 14, 21, 28];
    let candles = generate_candles(1000);
    
    for window in windows {
        group.throughput(Throughput::Elements(window as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(window),
            &window,
            |b, &window| {
                let mut params = HashMap::new();
                params.insert("period".to_string(), window as f64);
                let mut indicator = create_indicator("ATR", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("value").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_adx(c: &mut Criterion) {
    let mut group = c.benchmark_group("ADX");
    
    let windows = vec![7, 14, 21, 28];
    let candles = generate_candles(1000);
    
    for window in windows {
        group.throughput(Throughput::Elements(window as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(window),
            &window,
            |b, &window| {
                let mut params = HashMap::new();
                params.insert("period".to_string(), window as f64);
                let mut indicator = create_indicator("ADX", &params).unwrap();
                
                b.iter(|| {
                    for candle in &candles {
                        indicator.update(black_box(candle)).unwrap();
                        black_box(indicator.value("adx").unwrap());
                        black_box(indicator.value("plus_di").unwrap());
                        black_box(indicator.value("minus_di").unwrap());
                    }
                    indicator.reset();
                });
            },
        );
    }
    group.finish();
}

fn bench_obv(c: &mut Criterion) {
    let mut group = c.benchmark_group("OBV");
    
    let candles = generate_candles(1000);
    
    group.bench_function("OBV", |b| {
        let mut indicator = create_indicator("OBV", &HashMap::new()).unwrap();
        
        b.iter(|| {
            for candle in &candles {
                indicator.update(black_box(candle)).unwrap();
                black_box(indicator.value("value").unwrap());
            }
            indicator.reset();
        });
    });
    
    group.finish();
}

fn bench_all_indicators_combined(c: &mut Criterion) {
    let mut group = c.benchmark_group("All Indicators Combined");
    
    let candles = generate_candles(1000);
    
    group.bench_function("10 indicators", |b| {
        let mut params_sma = HashMap::new();
        params_sma.insert("length".to_string(), 20.0);
        let mut sma = create_indicator("SMA", &params_sma).unwrap();
        
        let mut params_ema = HashMap::new();
        params_ema.insert("length".to_string(), 20.0);
        let mut ema = create_indicator("EMA", &params_ema).unwrap();
        
        let mut params_rsi = HashMap::new();
        params_rsi.insert("length".to_string(), 14.0);
        let mut rsi = create_indicator("RSI", &params_rsi).unwrap();
        
        let mut params_macd = HashMap::new();
        params_macd.insert("fast".to_string(), 12.0);
        params_macd.insert("slow".to_string(), 26.0);
        params_macd.insert("signal".to_string(), 9.0);
        let mut macd = create_indicator("MACD", &params_macd).unwrap();
        
        let mut params_bb = HashMap::new();
        params_bb.insert("length".to_string(), 20.0);
        params_bb.insert("std".to_string(), 2.0);
        let mut bb = create_indicator("BBANDS", &params_bb).unwrap();
        
        let mut params_stoch = HashMap::new();
        params_stoch.insert("k_period".to_string(), 14.0);
        params_stoch.insert("k_smooth".to_string(), 1.0);
        params_stoch.insert("d_period".to_string(), 3.0);
        let mut stoch = create_indicator("STOCH", &params_stoch).unwrap();
        
        let mut params_atr = HashMap::new();
        params_atr.insert("period".to_string(), 14.0);
        let mut atr = create_indicator("ATR", &params_atr).unwrap();
        
        let mut params_adx = HashMap::new();
        params_adx.insert("period".to_string(), 14.0);
        let mut adx = create_indicator("ADX", &params_adx).unwrap();
        
        let mut obv = create_indicator("OBV", &HashMap::new()).unwrap();
        
        b.iter(|| {
            for candle in &candles {
                sma.update(black_box(candle)).unwrap();
                ema.update(black_box(candle)).unwrap();
                rsi.update(black_box(candle)).unwrap();
                macd.update(black_box(candle)).unwrap();
                bb.update(black_box(candle)).unwrap();
                stoch.update(black_box(candle)).unwrap();
                atr.update(black_box(candle)).unwrap();
                adx.update(black_box(candle)).unwrap();
                obv.update(black_box(candle)).unwrap();
                
                // Read values
                black_box(sma.value("value").unwrap());
                black_box(ema.value("value").unwrap());
                black_box(rsi.value("value").unwrap());
                black_box(macd.value("macd").unwrap());
                black_box(bb.value("upper").unwrap());
                black_box(stoch.value("k").unwrap());
                black_box(atr.value("value").unwrap());
                black_box(adx.value("adx").unwrap());
                black_box(obv.value("value").unwrap());
            }
            
            sma.reset();
            ema.reset();
            rsi.reset();
            macd.reset();
            bb.reset();
            stoch.reset();
            atr.reset();
            adx.reset();
            obv.reset();
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_sma,
    bench_ema,
    bench_wma,
    bench_rsi,
    bench_macd,
    bench_bbands,
    bench_stoch,
    bench_atr,
    bench_adx,
    bench_obv,
    bench_all_indicators_combined
);
criterion_main!(benches);

