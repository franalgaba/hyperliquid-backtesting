//! Tests for the strategy module

use hl_backtest::strategy::{
    compile_strategy, Action, ComparisonOp, Condition, CrossDirection, EvalState, Instrument,
    IndicatorSpec, Rule, Strategy,
};
use std::collections::HashMap;

fn create_test_strategy() -> Strategy {
    Strategy {
        name: "Test RSI Strategy".to_string(),
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

#[test]
fn test_strategy_creation() {
    let strategy = create_test_strategy();
    assert_eq!(strategy.name, "Test RSI Strategy");
    assert_eq!(strategy.instrument.coin, "BTC");
    assert_eq!(strategy.indicators.len(), 1);
    assert!(strategy.exit.is_some());
}

#[test]
fn test_strategy_json_serialization() {
    let strategy = create_test_strategy();
    let json = serde_json::to_string(&strategy).unwrap();
    let parsed: Strategy = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, strategy.name);
    assert_eq!(parsed.instrument.coin, strategy.instrument.coin);
    assert_eq!(parsed.indicators.len(), strategy.indicators.len());
}

#[test]
fn test_strategy_json_parsing() {
    let json = r#"{
        "name": "RSI Strategy",
        "instrument": {
            "symbol": "ETHUSD",
            "coin": "ETH",
            "venue": "HL",
            "timeframe": "4h"
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
    assert_eq!(strategy.name, "RSI Strategy");
    assert_eq!(strategy.instrument.coin, "ETH");
    assert_eq!(strategy.instrument.timeframe, "4h");
}

#[test]
fn test_crossover_condition_parsing() {
    let json = r#"{
        "name": "MA Crossover",
        "instrument": { "symbol": "BTCUSD", "coin": "BTC", "venue": "HL", "timeframe": "1h" },
        "indicators": [
            { "id": "sma_10", "type": "SMA", "params": { "period": 10 }, "outputs": ["value"] },
            { "id": "sma_50", "type": "SMA", "params": { "period": 50 }, "outputs": ["value"] }
        ],
        "entry": {
            "condition": {
                "type": "crossover",
                "fast": "sma_10",
                "slow": "sma_50",
                "direction": "above"
            },
            "action": { "type": "buy", "size_pct": 100.0 }
        }
    }"#;

    let strategy: Strategy = serde_json::from_str(json).unwrap();
    match &strategy.entry.condition {
        Condition::Crossover {
            fast,
            slow,
            direction,
        } => {
            assert_eq!(fast, "sma_10");
            assert_eq!(slow, "sma_50");
            assert!(matches!(direction, CrossDirection::Above));
        }
        _ => panic!("Expected crossover condition"),
    }
}

#[test]
fn test_and_condition_parsing() {
    let json = r#"{
        "name": "Multi-Indicator",
        "instrument": { "symbol": "BTCUSD", "coin": "BTC", "venue": "HL", "timeframe": "1h" },
        "indicators": [
            { "id": "rsi", "type": "RSI", "params": { "period": 14 }, "outputs": ["value"] },
            { "id": "macd", "type": "MACD", "params": { "fast": 12, "slow": 26, "signal": 9 }, "outputs": ["histogram"] }
        ],
        "entry": {
            "condition": {
                "type": "and",
                "conditions": [
                    { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 30.0 },
                    { "type": "threshold", "indicator": "macd", "op": "gt", "value": 0.0 }
                ]
            },
            "action": { "type": "buy", "size_pct": 100.0 }
        }
    }"#;

    let strategy: Strategy = serde_json::from_str(json).unwrap();
    match &strategy.entry.condition {
        Condition::And { conditions } => {
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected AND condition"),
    }
}

#[test]
fn test_or_condition_parsing() {
    let json = r#"{
        "name": "RSI Extreme",
        "instrument": { "symbol": "BTCUSD", "coin": "BTC", "venue": "HL", "timeframe": "1h" },
        "indicators": [
            { "id": "rsi", "type": "RSI", "params": { "period": 14 }, "outputs": ["value"] }
        ],
        "entry": {
            "condition": {
                "type": "or",
                "conditions": [
                    { "type": "threshold", "indicator": "rsi", "op": "lt", "value": 20.0 },
                    { "type": "threshold", "indicator": "rsi", "op": "gt", "value": 80.0 }
                ]
            },
            "action": { "type": "buy", "size_pct": 50.0 }
        }
    }"#;

    let strategy: Strategy = serde_json::from_str(json).unwrap();
    match &strategy.entry.condition {
        Condition::Or { conditions } => {
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected OR condition"),
    }
}

#[test]
fn test_compile_strategy() {
    let strategy = create_test_strategy();
    let compiled = compile_strategy(&strategy).unwrap();

    assert_eq!(compiled.instrument.coin, "BTC");
    assert_eq!(compiled.indicators.len(), 1);
    assert!(compiled.indicators[0].lookback > 0);
}

#[test]
fn test_eval_state_threshold_lt() {
    let state = EvalState::new();
    let values: HashMap<String, f64> = [("rsi".to_string(), 25.0)].into_iter().collect();

    let condition = Condition::Threshold {
        indicator: "rsi".to_string(),
        op: ComparisonOp::Lt,
        value: 30.0,
    };

    assert!(state.evaluate(&condition, &values));
}

#[test]
fn test_eval_state_threshold_gt() {
    let state = EvalState::new();
    let values: HashMap<String, f64> = [("rsi".to_string(), 75.0)].into_iter().collect();

    let condition = Condition::Threshold {
        indicator: "rsi".to_string(),
        op: ComparisonOp::Gt,
        value: 70.0,
    };

    assert!(state.evaluate(&condition, &values));
}

#[test]
fn test_eval_state_threshold_eq() {
    let state = EvalState::new();
    let values: HashMap<String, f64> = [("rsi".to_string(), 50.0)].into_iter().collect();

    let condition = Condition::Threshold {
        indicator: "rsi".to_string(),
        op: ComparisonOp::Eq,
        value: 50.0,
    };

    assert!(state.evaluate(&condition, &values));
}

#[test]
fn test_eval_state_threshold_missing_indicator() {
    let state = EvalState::new();
    let values: HashMap<String, f64> = HashMap::new();

    let condition = Condition::Threshold {
        indicator: "rsi".to_string(),
        op: ComparisonOp::Lt,
        value: 30.0,
    };

    // Missing indicator should return false
    assert!(!state.evaluate(&condition, &values));
}

#[test]
fn test_eval_state_crossover_above() {
    let mut state = EvalState::new();

    // Previous: fast below slow
    let prev_values: HashMap<String, f64> = [
        ("fast_ma".to_string(), 99.0),
        ("slow_ma".to_string(), 100.0),
    ]
    .into_iter()
    .collect();
    state.update(&prev_values);

    // Current: fast above slow
    let curr_values: HashMap<String, f64> = [
        ("fast_ma".to_string(), 101.0),
        ("slow_ma".to_string(), 100.0),
    ]
    .into_iter()
    .collect();

    let condition = Condition::Crossover {
        fast: "fast_ma".to_string(),
        slow: "slow_ma".to_string(),
        direction: CrossDirection::Above,
    };

    assert!(state.evaluate(&condition, &curr_values));
}

#[test]
fn test_eval_state_crossover_below() {
    let mut state = EvalState::new();

    // Previous: fast above slow
    let prev_values: HashMap<String, f64> = [
        ("fast_ma".to_string(), 101.0),
        ("slow_ma".to_string(), 100.0),
    ]
    .into_iter()
    .collect();
    state.update(&prev_values);

    // Current: fast below slow
    let curr_values: HashMap<String, f64> = [
        ("fast_ma".to_string(), 99.0),
        ("slow_ma".to_string(), 100.0),
    ]
    .into_iter()
    .collect();

    let condition = Condition::Crossover {
        fast: "fast_ma".to_string(),
        slow: "slow_ma".to_string(),
        direction: CrossDirection::Below,
    };

    assert!(state.evaluate(&condition, &curr_values));
}

#[test]
fn test_eval_state_crossover_no_cross() {
    let mut state = EvalState::new();

    // Previous: fast above slow
    let prev_values: HashMap<String, f64> = [
        ("fast_ma".to_string(), 101.0),
        ("slow_ma".to_string(), 100.0),
    ]
    .into_iter()
    .collect();
    state.update(&prev_values);

    // Current: still fast above slow (no cross)
    let curr_values: HashMap<String, f64> = [
        ("fast_ma".to_string(), 102.0),
        ("slow_ma".to_string(), 100.0),
    ]
    .into_iter()
    .collect();

    let condition = Condition::Crossover {
        fast: "fast_ma".to_string(),
        slow: "slow_ma".to_string(),
        direction: CrossDirection::Above,
    };

    assert!(!state.evaluate(&condition, &curr_values));
}

#[test]
fn test_eval_state_and_condition() {
    let state = EvalState::new();
    let values: HashMap<String, f64> = [
        ("rsi".to_string(), 25.0),
        ("macd".to_string(), 0.5),
    ]
    .into_iter()
    .collect();

    let condition = Condition::And {
        conditions: vec![
            Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Lt,
                value: 30.0,
            },
            Condition::Threshold {
                indicator: "macd".to_string(),
                op: ComparisonOp::Gt,
                value: 0.0,
            },
        ],
    };

    assert!(state.evaluate(&condition, &values));
}

#[test]
fn test_eval_state_and_condition_one_false() {
    let state = EvalState::new();
    let values: HashMap<String, f64> = [
        ("rsi".to_string(), 25.0),
        ("macd".to_string(), -0.5), // This makes the second condition false
    ]
    .into_iter()
    .collect();

    let condition = Condition::And {
        conditions: vec![
            Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Lt,
                value: 30.0,
            },
            Condition::Threshold {
                indicator: "macd".to_string(),
                op: ComparisonOp::Gt,
                value: 0.0,
            },
        ],
    };

    assert!(!state.evaluate(&condition, &values));
}

#[test]
fn test_eval_state_or_condition() {
    let state = EvalState::new();
    let values: HashMap<String, f64> = [("rsi".to_string(), 15.0)].into_iter().collect();

    let condition = Condition::Or {
        conditions: vec![
            Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Lt,
                value: 20.0, // True
            },
            Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Gt,
                value: 80.0, // False
            },
        ],
    };

    assert!(state.evaluate(&condition, &values));
}

#[test]
fn test_eval_state_or_condition_all_false() {
    let state = EvalState::new();
    let values: HashMap<String, f64> = [("rsi".to_string(), 50.0)].into_iter().collect();

    let condition = Condition::Or {
        conditions: vec![
            Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Lt,
                value: 20.0, // False
            },
            Condition::Threshold {
                indicator: "rsi".to_string(),
                op: ComparisonOp::Gt,
                value: 80.0, // False
            },
        ],
    };

    assert!(!state.evaluate(&condition, &values));
}

#[test]
fn test_action_buy_serialization() {
    let action = Action::Buy { size_pct: 75.0 };
    let json = serde_json::to_string(&action).unwrap();
    let parsed: Action = serde_json::from_str(&json).unwrap();

    match parsed {
        Action::Buy { size_pct } => assert_eq!(size_pct, 75.0),
        _ => panic!("Expected Buy action"),
    }
}

#[test]
fn test_action_sell_serialization() {
    let action = Action::Sell { size_pct: 50.0 };
    let json = serde_json::to_string(&action).unwrap();
    let parsed: Action = serde_json::from_str(&json).unwrap();

    match parsed {
        Action::Sell { size_pct } => assert_eq!(size_pct, 50.0),
        _ => panic!("Expected Sell action"),
    }
}

#[test]
fn test_action_close_serialization() {
    let action = Action::Close;
    let json = serde_json::to_string(&action).unwrap();
    let parsed: Action = serde_json::from_str(&json).unwrap();

    assert!(matches!(parsed, Action::Close));
}

#[test]
fn test_all_comparison_operators() {
    let state = EvalState::new();

    // Test LTE
    let values: HashMap<String, f64> = [("x".to_string(), 30.0)].into_iter().collect();
    let cond_lte = Condition::Threshold {
        indicator: "x".to_string(),
        op: ComparisonOp::Lte,
        value: 30.0,
    };
    assert!(state.evaluate(&cond_lte, &values));

    // Test GTE
    let cond_gte = Condition::Threshold {
        indicator: "x".to_string(),
        op: ComparisonOp::Gte,
        value: 30.0,
    };
    assert!(state.evaluate(&cond_gte, &values));

    // Test NE
    let values_ne: HashMap<String, f64> = [("x".to_string(), 25.0)].into_iter().collect();
    let cond_ne = Condition::Threshold {
        indicator: "x".to_string(),
        op: ComparisonOp::Ne,
        value: 30.0,
    };
    assert!(state.evaluate(&cond_ne, &values_ne));
}
