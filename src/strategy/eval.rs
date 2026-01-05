use crate::strategy::types::*;
use std::collections::HashMap;

/// State for tracking crossover conditions
pub struct EvalState {
    /// Previous indicator values for crossover detection
    prev_values: HashMap<String, f64>,
}

impl EvalState {
    pub fn new() -> Self {
        Self {
            prev_values: HashMap::new(),
        }
    }

    /// Update state with current indicator values
    pub fn update(&mut self, current_values: &HashMap<String, f64>) {
        self.prev_values = current_values.clone();
    }

    /// Evaluate a condition against current indicator values
    pub fn evaluate(&self, condition: &Condition, values: &HashMap<String, f64>) -> bool {
        match condition {
            Condition::Threshold {
                indicator,
                op,
                value,
            } => {
                if let Some(&ind_value) = values.get(indicator) {
                    compare(ind_value, *op, *value)
                } else {
                    false
                }
            }
            Condition::Crossover {
                fast,
                slow,
                direction,
            } => {
                let curr_fast = values.get(fast).copied();
                let curr_slow = values.get(slow).copied();
                let prev_fast = self.prev_values.get(fast).copied();
                let prev_slow = self.prev_values.get(slow).copied();

                match (curr_fast, curr_slow, prev_fast, prev_slow) {
                    (Some(cf), Some(cs), Some(pf), Some(ps)) => match direction {
                        CrossDirection::Above => pf <= ps && cf > cs,
                        CrossDirection::Below => pf >= ps && cf < cs,
                    },
                    _ => false,
                }
            }
            Condition::And { conditions } => {
                conditions.iter().all(|c| self.evaluate(c, values))
            }
            Condition::Or { conditions } => {
                conditions.iter().any(|c| self.evaluate(c, values))
            }
        }
    }
}

impl Default for EvalState {
    fn default() -> Self {
        Self::new()
    }
}

fn compare(lhs: f64, op: ComparisonOp, rhs: f64) -> bool {
    match op {
        ComparisonOp::Lt => lhs < rhs,
        ComparisonOp::Lte => lhs <= rhs,
        ComparisonOp::Eq => (lhs - rhs).abs() < f64::EPSILON,
        ComparisonOp::Ne => (lhs - rhs).abs() >= f64::EPSILON,
        ComparisonOp::Gte => lhs >= rhs,
        ComparisonOp::Gt => lhs > rhs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_condition() {
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
    fn test_crossover_above() {
        let mut state = EvalState::new();

        // Initial state: fast below slow
        let prev_values: HashMap<String, f64> = [
            ("fast_ma".to_string(), 99.0),
            ("slow_ma".to_string(), 100.0),
        ]
        .into_iter()
        .collect();
        state.update(&prev_values);

        // Current state: fast above slow
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
    fn test_and_condition() {
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
}
