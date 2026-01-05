//! Integration tests for the backtesting engine

use hl_backtest::data::types::Candle;
use hl_backtest::orders::engine::simulate;
use hl_backtest::orders::types::SimConfig;
use hl_backtest::strategy::{
    Action, ComparisonOp, Condition, Instrument, IndicatorSpec, Rule, Strategy,
};
use std::collections::HashMap;

fn create_mock_candles(count: usize, base_price: f64, trend: f64) -> Vec<Candle> {
    (0..count)
        .map(|i| {
            let price = base_price + (i as f64 * trend);
            Candle {
                time_open: 1704067200000 + (i as u64 * 3600000), // hourly
                time_close: 1704070800000 + (i as u64 * 3600000),
                coin: "BTC".to_string(),
                interval: "1h".to_string(),
                open: price,
                high: price + 100.0,
                low: price - 100.0,
                close: price + 50.0,
                volume: 1000.0 + (i as f64 * 10.0),
                num_trades: 500 + i as i64,
            }
        })
        .collect()
}

fn create_rsi_strategy() -> Strategy {
    Strategy {
        name: "RSI Strategy".to_string(),
        instrument: Instrument {
            symbol: "BTCUSD".to_string(),
            coin: "BTC".to_string(),
            venue: "HL".to_string(),
            timeframe: "1h".to_string(),
        },
        indicators: vec![IndicatorSpec {
            id: "rsi_14".to_string(),
            indicator_type: "RSI".to_string(),
            params: [("period".to_string(), 14.0)].into_iter().collect(),
            outputs: vec!["value".to_string()],
        }],
        entry: Rule {
            condition: Condition::Threshold {
                indicator: "rsi_14".to_string(),
                op: ComparisonOp::Lt,
                value: 30.0,
            },
            action: Action::Buy { size_pct: 100.0 },
        },
        exit: Some(Rule {
            condition: Condition::Threshold {
                indicator: "rsi_14".to_string(),
                op: ComparisonOp::Gt,
                value: 70.0,
            },
            action: Action::Close,
        }),
    }
}

fn create_sma_crossover_strategy() -> Strategy {
    let mut fast_params = HashMap::new();
    fast_params.insert("period".to_string(), 10.0);

    let mut slow_params = HashMap::new();
    slow_params.insert("period".to_string(), 20.0);

    Strategy {
        name: "SMA Crossover".to_string(),
        instrument: Instrument {
            symbol: "BTCUSD".to_string(),
            coin: "BTC".to_string(),
            venue: "HL".to_string(),
            timeframe: "1h".to_string(),
        },
        indicators: vec![
            IndicatorSpec {
                id: "sma_fast".to_string(),
                indicator_type: "SMA".to_string(),
                params: fast_params,
                outputs: vec!["value".to_string()],
            },
            IndicatorSpec {
                id: "sma_slow".to_string(),
                indicator_type: "SMA".to_string(),
                params: slow_params,
                outputs: vec!["value".to_string()],
            },
        ],
        entry: Rule {
            condition: Condition::Crossover {
                fast: "sma_fast".to_string(),
                slow: "sma_slow".to_string(),
                direction: hl_backtest::strategy::CrossDirection::Above,
            },
            action: Action::Buy { size_pct: 100.0 },
        },
        exit: Some(Rule {
            condition: Condition::Crossover {
                fast: "sma_fast".to_string(),
                slow: "sma_slow".to_string(),
                direction: hl_backtest::strategy::CrossDirection::Below,
            },
            action: Action::Close,
        }),
    }
}

fn default_sim_config() -> SimConfig {
    SimConfig {
        initial_capital: 10000.0,
        maker_fee_bps: -1,
        taker_fee_bps: 10,
        slippage_bps: 5,
        trade_cooldown_ms: None,
    }
}

#[tokio::test]
async fn test_simulate_basic() {
    let candles = create_mock_candles(100, 42000.0, 10.0);
    let strategy = create_rsi_strategy();
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await;

    assert!(result.is_ok());
    let result = result.unwrap();

    // Basic sanity checks
    assert!(result.final_equity > 0.0);
    assert!(!result.equity_curve.is_empty());
}

#[tokio::test]
async fn test_simulate_sma_crossover() {
    let candles = create_mock_candles(100, 42000.0, 10.0);
    let strategy = create_sma_crossover_strategy();
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await;

    assert!(result.is_ok());
    let result = result.unwrap();

    assert!(result.final_equity > 0.0);
}

#[tokio::test]
async fn test_simulate_insufficient_candles() {
    let candles = create_mock_candles(10, 42000.0, 10.0); // Too few for RSI-14
    let strategy = create_rsi_strategy();
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await;

    // Should fail due to insufficient candles for warmup
    assert!(result.is_err());
}

#[tokio::test]
async fn test_simulate_returns_equity_curve() {
    let candles = create_mock_candles(100, 42000.0, 10.0);
    let strategy = create_rsi_strategy();
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await.unwrap();

    // Equity curve should have entries
    assert!(!result.equity_curve.is_empty());

    // Each equity point should have valid values
    for point in &result.equity_curve {
        assert!(point.equity > 0.0);
        assert!(point.timestamp > 0);
    }
}

#[tokio::test]
async fn test_simulate_with_fees() {
    let candles = create_mock_candles(100, 42000.0, 10.0);
    let strategy = create_rsi_strategy();
    let config = SimConfig {
        initial_capital: 10000.0,
        maker_fee_bps: 0,
        taker_fee_bps: 50, // 0.5% taker fee
        slippage_bps: 0,
        trade_cooldown_ms: None,
    };

    let result = simulate(&candles, &strategy, &config).await.unwrap();

    // With high fees, trades should have fee costs
    for trade in &result.trades {
        if trade.size > 0.0 {
            assert!(trade.fee >= 0.0);
        }
    }
}

#[tokio::test]
async fn test_simulate_metrics() {
    let candles = create_mock_candles(200, 42000.0, 5.0);
    let strategy = create_rsi_strategy();
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await.unwrap();

    // Metrics should be within valid ranges
    assert!(result.win_rate >= 0.0 && result.win_rate <= 1.0);
    assert!(result.max_drawdown >= 0.0);
    assert!(result.max_drawdown_pct >= 0.0 && result.max_drawdown_pct <= 100.0);
    // Sharpe and Sortino can be any value including negative
}

#[tokio::test]
async fn test_simulate_no_exit_rule() {
    let strategy = Strategy {
        name: "Entry Only".to_string(),
        instrument: Instrument {
            symbol: "BTCUSD".to_string(),
            coin: "BTC".to_string(),
            venue: "HL".to_string(),
            timeframe: "1h".to_string(),
        },
        indicators: vec![IndicatorSpec {
            id: "rsi_14".to_string(),
            indicator_type: "RSI".to_string(),
            params: [("period".to_string(), 14.0)].into_iter().collect(),
            outputs: vec!["value".to_string()],
        }],
        entry: Rule {
            condition: Condition::Threshold {
                indicator: "rsi_14".to_string(),
                op: ComparisonOp::Lt,
                value: 30.0,
            },
            action: Action::Buy { size_pct: 100.0 },
        },
        exit: None, // No exit rule
    };

    let candles = create_mock_candles(100, 42000.0, 10.0);
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await;

    // Should still work without exit rule
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_simulate_and_condition() {
    let strategy = Strategy {
        name: "Multi Condition".to_string(),
        instrument: Instrument {
            symbol: "BTCUSD".to_string(),
            coin: "BTC".to_string(),
            venue: "HL".to_string(),
            timeframe: "1h".to_string(),
        },
        indicators: vec![
            IndicatorSpec {
                id: "rsi".to_string(),
                indicator_type: "RSI".to_string(),
                params: [("period".to_string(), 14.0)].into_iter().collect(),
                outputs: vec!["value".to_string()],
            },
            IndicatorSpec {
                id: "sma".to_string(),
                indicator_type: "SMA".to_string(),
                params: [("period".to_string(), 20.0)].into_iter().collect(),
                outputs: vec!["value".to_string()],
            },
        ],
        entry: Rule {
            condition: Condition::And {
                conditions: vec![
                    Condition::Threshold {
                        indicator: "rsi".to_string(),
                        op: ComparisonOp::Lt,
                        value: 40.0,
                    },
                    Condition::Threshold {
                        indicator: "sma".to_string(),
                        op: ComparisonOp::Gt,
                        value: 0.0, // SMA > 0 (always true for positive prices)
                    },
                ],
            },
            action: Action::Buy { size_pct: 50.0 },
        },
        exit: Some(Rule {
            condition: Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Gt,
                value: 60.0,
            },
            action: Action::Close,
        }),
    };

    let candles = create_mock_candles(100, 42000.0, 10.0);
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_simulate_or_condition() {
    let strategy = Strategy {
        name: "OR Condition".to_string(),
        instrument: Instrument {
            symbol: "BTCUSD".to_string(),
            coin: "BTC".to_string(),
            venue: "HL".to_string(),
            timeframe: "1h".to_string(),
        },
        indicators: vec![IndicatorSpec {
            id: "rsi".to_string(),
            indicator_type: "RSI".to_string(),
            params: [("period".to_string(), 14.0)].into_iter().collect(),
            outputs: vec!["value".to_string()],
        }],
        entry: Rule {
            condition: Condition::Or {
                conditions: vec![
                    Condition::Threshold {
                        indicator: "rsi".to_string(),
                        op: ComparisonOp::Lt,
                        value: 25.0,
                    },
                    Condition::Threshold {
                        indicator: "rsi".to_string(),
                        op: ComparisonOp::Gt,
                        value: 75.0,
                    },
                ],
            },
            action: Action::Buy { size_pct: 100.0 },
        },
        exit: Some(Rule {
            condition: Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Gt,
                value: 50.0,
            },
            action: Action::Close,
        }),
    };

    let candles = create_mock_candles(100, 42000.0, 10.0);
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_simulate_different_size_pcts() {
    let candles = create_mock_candles(100, 42000.0, 10.0);
    let config = default_sim_config();

    // Test with 50% position size
    let strategy = Strategy {
        name: "Half Size".to_string(),
        instrument: Instrument {
            symbol: "BTCUSD".to_string(),
            coin: "BTC".to_string(),
            venue: "HL".to_string(),
            timeframe: "1h".to_string(),
        },
        indicators: vec![IndicatorSpec {
            id: "rsi".to_string(),
            indicator_type: "RSI".to_string(),
            params: [("period".to_string(), 14.0)].into_iter().collect(),
            outputs: vec!["value".to_string()],
        }],
        entry: Rule {
            condition: Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Lt,
                value: 50.0, // More likely to trigger
            },
            action: Action::Buy { size_pct: 50.0 }, // Half position
        },
        exit: Some(Rule {
            condition: Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Gt,
                value: 60.0,
            },
            action: Action::Close,
        }),
    };

    let result = simulate(&candles, &strategy, &config).await.unwrap();

    // With smaller position sizes, we should have some cash remaining when in position
    assert!(result.final_equity > 0.0);
}

#[tokio::test]
async fn test_simulate_trade_cooldown() {
    let candles = create_mock_candles(100, 42000.0, 10.0);
    let config_no_cooldown = SimConfig {
        initial_capital: 10000.0,
        maker_fee_bps: -1,
        taker_fee_bps: 10,
        slippage_bps: 5,
        trade_cooldown_ms: None,
    };

    let config_with_cooldown = SimConfig {
        initial_capital: 10000.0,
        maker_fee_bps: -1,
        taker_fee_bps: 10,
        slippage_bps: 5,
        trade_cooldown_ms: Some(3600000), // 1 hour cooldown
    };

    let strategy = create_rsi_strategy();

    let result_no_cooldown = simulate(&candles, &strategy, &config_no_cooldown)
        .await
        .unwrap();
    let result_with_cooldown = simulate(&candles, &strategy, &config_with_cooldown)
        .await
        .unwrap();

    // With cooldown, there should be fewer or equal trades
    assert!(result_with_cooldown.num_trades <= result_no_cooldown.num_trades);
}

#[tokio::test]
async fn test_simulate_preserves_initial_capital_no_trades() {
    // Create a strategy that never triggers
    let strategy = Strategy {
        name: "Never Trade".to_string(),
        instrument: Instrument {
            symbol: "BTCUSD".to_string(),
            coin: "BTC".to_string(),
            venue: "HL".to_string(),
            timeframe: "1h".to_string(),
        },
        indicators: vec![IndicatorSpec {
            id: "rsi".to_string(),
            indicator_type: "RSI".to_string(),
            params: [("period".to_string(), 14.0)].into_iter().collect(),
            outputs: vec!["value".to_string()],
        }],
        entry: Rule {
            condition: Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Lt,
                value: -100.0, // RSI can never be negative
            },
            action: Action::Buy { size_pct: 100.0 },
        },
        exit: None,
    };

    let candles = create_mock_candles(100, 42000.0, 10.0);
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await.unwrap();

    // No trades should occur
    assert_eq!(result.num_trades, 0);
    // Final equity should equal initial capital
    assert!((result.final_equity - config.initial_capital).abs() < 0.001);
}

#[tokio::test]
async fn test_simulate_json_strategy() {
    let json = r#"{
        "name": "JSON RSI Strategy",
        "instrument": {
            "symbol": "BTCUSD",
            "coin": "BTC",
            "venue": "HL",
            "timeframe": "1h"
        },
        "indicators": [
            {
                "id": "rsi",
                "type": "RSI",
                "params": { "period": 14 },
                "outputs": ["value"]
            }
        ],
        "entry": {
            "condition": {
                "type": "threshold",
                "indicator": "rsi",
                "op": "lt",
                "value": 30.0
            },
            "action": {
                "type": "buy",
                "size_pct": 100.0
            }
        },
        "exit": {
            "condition": {
                "type": "threshold",
                "indicator": "rsi",
                "op": "gt",
                "value": 70.0
            },
            "action": {
                "type": "close"
            }
        }
    }"#;

    let strategy: Strategy = serde_json::from_str(json).unwrap();
    let candles = create_mock_candles(100, 42000.0, 10.0);
    let config = default_sim_config();

    let result = simulate(&candles, &strategy, &config).await;

    assert!(result.is_ok());
}
