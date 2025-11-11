# Indicator Benchmarks

This directory contains performance benchmarks for all indicators in the custom indicator engine.

## Running Benchmarks

### Full Benchmark Suite

To run all benchmarks:

```bash
cargo bench --bench indicators
```

To run a specific benchmark group:

```bash
cargo bench --bench indicators -- SMA
cargo bench --bench indicators -- EMA
cargo bench --bench indicators -- RSI
# etc.
```

### Performance Comparison Benchmarks

To run comparison benchmarks (matching external library format):

```bash
cargo bench --bench yata_comparison
```

## Benchmark Structure

Each indicator is benchmarked with different window sizes/periods:

- **SMA, EMA, WMA**: Windows of 10, 20, 50, 100, 200
- **RSI**: Periods of 7, 14, 21, 28
- **MACD**: Multiple configurations (12-26-9, 8-21-5, 19-39-9)
- **Bollinger Bands**: Windows of 10, 20, 50, 100
- **Stochastic**: Multiple configurations (14-1-3, 14-3-3, 21-1-5)
- **ATR**: Periods of 7, 14, 21, 28
- **ADX**: Periods of 7, 14, 21, 28
- **OBV**: Single benchmark (no window size)

Additionally, there's a combined benchmark that runs all 10 indicators together to measure overall system performance.

## Current Performance Summary

### Single Iteration Performance (Latest Results)

| Indicator | Window | Time (ns/iter) | Complexity | Status |
|-----------|--------|----------------|------------|--------|
| **SMA**   | w10/w100 | ~4.6 | O(1) | ✅ Excellent |
| **EMA**   | w10/w100 | ~4.6 | O(1) | ✅ Excellent |
| **WMA**   | w10/w100 | ~5.8 | O(1) | ✅ Excellent |
| **RSI**   | w10/w100 | ~28  | O(1) | ✅ Good |
| **MACD**  | 12-26-9 | ~50  | O(1) | ✅ Good |
| **BBands**| w20     | ~15  | O(1) | ✅ Good |
| **Stoch** | 14-1-3  | ~25  | O(1) | ✅ Good |
| **ATR**   | w14     | ~8   | O(1) | ✅ Good |
| **ADX**   | w14     | ~24  | O(1) | ✅ Good |
| **OBV**   | -       | ~5   | O(1) | ✅ Excellent |

### Performance Characteristics

- **Constant performance**: All indicators show O(1) performance regardless of window size
- **Optimized implementations**: Using incremental updates, fused multiply-add, and efficient data structures
- **Excellent for backtesting**: Processing 1 million candles takes < 5ms even with multiple indicators

### Combined Performance

Running all 10 indicators together on 1000 candles:
- **Time**: ~350 microseconds
- **Throughput**: ~2.8 million candles/second

## Metrics

Each benchmark measures:
- **Time per iteration**: How long it takes to process a single candle update
- **Throughput**: Elements processed per second
- **Outlier detection**: Identifies performance anomalies
- **Statistical analysis**: Mean, median, standard deviation, confidence intervals

## Results

Benchmark results are saved to `target/criterion/` and include:
- Statistical analysis (mean, median, standard deviation)
- HTML reports with graphs (in `target/criterion/indicators/report/index.html`)
- Comparison between different window sizes
- Baseline tracking for performance regression detection

## Viewing Results

After running benchmarks, view the HTML report:

```bash
# Main benchmark results
open target/criterion/indicators/report/index.html

# Comparison benchmark results
open target/criterion/yata_comparison/report/index.html
```

Or use the file paths directly in your browser.

## Baseline Management

Save a baseline for comparison:

```bash
# Save current results as baseline
cargo bench --bench indicators -- --save-baseline my_baseline

# Compare against baseline
cargo bench --bench indicators -- --baseline my_baseline
```

## Performance Notes

- All benchmarks run in release mode with full optimizations
- Results may vary based on CPU architecture and compiler optimizations
- Benchmarks use Criterion.rs with 100 samples for statistical accuracy
- Single iteration benchmarks measure one `update()` + `value()` call

## See Also

For detailed comparison with external libraries, see [PERFORMANCE_COMPARISON.md](../docs/PERFORMANCE_COMPARISON.md).
