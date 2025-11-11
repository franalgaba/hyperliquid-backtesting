# Documentation Index

Complete guide to all documentation for the Hyperliquid backtester.

## Getting Started

- **[Quick Start Guide](QUICK_START.md)** - Get up and running in minutes
- **[Main README](../README.md)** - Overview and basic usage

## Core Documentation

### API Reference

- **[API Documentation](API.md)** - Complete API reference with examples
  - PerpsEngine API
  - PerpsExecution API
  - Trade Utilities
  - FundingSchedule API
  - OrderBook API

### Architecture & Design

- **[Architecture Overview](ARCHITECTURE.md)** - System design and component details
  - Component architecture
  - Data flow diagrams
  - Performance optimizations
  - Error handling patterns

### Performance

- **[Performance Optimizations](PERFORMANCE_OPTIMIZATIONS.md)** - Performance improvements and benchmarks
- **[Performance Comparison](PERFORMANCE_COMPARISON.md)** - Benchmark results

### Setup & Configuration

- **[S3 Setup Guide](S3_SETUP.md)** - Instructions for downloading L2 data from S3
- **[Main README](../README.md)** - Installation and basic usage

## Reference Documentation

### Code Documentation

- **Rust Docs**: Generate with `cargo doc --open`
- **Inline Comments**: Comprehensive documentation in source code
- **Examples**: See `examples/` directory

### Security

- **[Security Documentation](SECURITY.md)** - Comprehensive security documentation, audit findings, and fixes

### Development

- **[Test Coverage](TEST_COVERAGE.md)** - Comprehensive test coverage information

## Documentation by Topic

### For Users

1. Start with [Quick Start Guide](QUICK_START.md)
2. Read [Main README](../README.md) for basic usage
3. Check [API Documentation](API.md) for detailed function references
4. See [S3 Setup Guide](S3_SETUP.md) for data setup

### For Developers

1. Read [Architecture Overview](ARCHITECTURE.md) for system design
2. Review [API Documentation](API.md) for implementation details
3. Check [Performance Optimizations](PERFORMANCE_OPTIMIZATIONS.md) for optimization strategies
4. Review git history for code patterns and changes

### For Contributors

1. Review [Architecture Overview](ARCHITECTURE.md) for system understanding
2. Check [Security Documentation](SECURITY.md) for security guidelines
3. See [Test Coverage](TEST_COVERAGE.md) for testing standards
4. Review git history for change history

## Key Concepts

### Order Execution

- **Market Orders**: Execute immediately, sweep the book
- **Limit Orders**: Wait for price to cross, fill when crossed
- **Partial Fills**: Tracked correctly, orders remain active until fully filled

### Strategy Evaluation

- **Graph-based**: Strategies are compiled to execution graphs
- **Throttled**: Only evaluates on significant price changes (0.01%)
- **Indicators**: Updated on every event to maintain rolling windows

### Funding Payments

- **Frequency**: Applied every 8 hours
- **Direction**: Longs pay, shorts receive
- **Calculation**: payment = notional * rate

## Examples

### Code Examples

- **Complete Backtest**: See [API Documentation - Examples](API.md#examples)
- **Order Execution**: See [API Documentation - PerpsExecution](API.md#perpsexecution)
- **Strategy Creation**: See [Quick Start Guide](QUICK_START.md#common-patterns)

### Strategy Examples

- **SMA Crossover**: `examples/strategies/sma_crossover.json`
- **RSI Mean Reversion**: `examples/strategies/rsi_mean_reversion.json`
- **MACD Crossover**: `examples/strategies/macd_crossover.json`
- **Bollinger Bands**: `examples/strategies/bollinger_bands.json`

## Troubleshooting

### Common Issues

- **"No events found"**: Check [Quick Start Guide - Troubleshooting](QUICK_START.md#troubleshooting)
- **Performance issues**: See [Performance Optimizations](PERFORMANCE_OPTIMIZATIONS.md)
- **API errors**: Check [API Documentation - Error Handling](API.md#error-handling)

### Best Practices

- **Order Execution**: See [API Documentation - Best Practices](API.md#best-practices)
- **Common Pitfalls**: See [API Documentation - Common Pitfalls](API.md#common-pitfalls)
- **Performance**: See [Performance Optimizations](PERFORMANCE_OPTIMIZATIONS.md)

## Additional Resources

- **Hyperliquid Docs**: <https://hyperliquid.gitbook.io/hyperliquid-docs/>
- **Rust Book**: <https://doc.rust-lang.org/book/>
- **Examples Directory**: `examples/` in the repository

## Documentation Standards

All documentation follows these standards:

- **Rust Doc Comments**: Functions use `///` doc comments with examples
- **Markdown Format**: Documentation files use Markdown
- **Code Examples**: All examples are tested and compile
- **Cross-references**: Links between related documentation
- **Version Info**: Documentation matches current codebase

## Contributing to Documentation

When adding new features:

1. Add inline documentation to code (`///` comments)
2. Update relevant documentation files
3. Add examples to [Quick Start Guide](QUICK_START.md)
4. Update [API Documentation](API.md) if API changes
5. Update [Architecture Overview](ARCHITECTURE.md) if architecture changes

## Questions?

- Check the [API Documentation](API.md) for function details
- Review [Common Pitfalls](API.md#common-pitfalls) for known issues
- Look at example strategies in `examples/strategies/`
- Review error messages - they include helpful context

