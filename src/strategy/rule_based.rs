use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::data::types::Candle;
use crate::indicators2::{create_indicator, IndicatorEvaluator};
use crate::strategy::types::Action as StrategyAction;
use crate::portfolio::Portfolio;
use crate::strategy::{compile_strategy, EvalState, Strategy};

/// Strategy adapter for the JSON rule-based strategy format.
pub struct RuleBasedStrategy {
    compiled: crate::strategy::CompiledStrategy,
    indicators: HashMap<String, Box<dyn IndicatorEvaluator>>,
    eval_state: EvalState,
    max_lookback: usize,
}

impl RuleBasedStrategy {
    pub fn from_strategy(strategy: &Strategy) -> Result<Self> {
        let compiled = compile_strategy(strategy)?;

        let mut indicators: HashMap<String, Box<dyn IndicatorEvaluator>> = HashMap::new();
        for ind in &compiled.indicators {
            let evaluator = create_indicator(&ind.indicator_type, &ind.params)
                .with_context(|| format!("Failed to create indicator: {}", ind.indicator_type))?;
            indicators.insert(ind.id.clone(), evaluator);
        }

        let max_lookback = compiled
            .indicators
            .iter()
            .map(|i| i.lookback)
            .max()
            .unwrap_or(0);

        Ok(Self {
            compiled,
            indicators,
            eval_state: EvalState::new(),
            max_lookback,
        })
    }

    fn update_indicators(&mut self, candle: &Candle) -> Result<()> {
        for evaluator in self.indicators.values_mut() {
            evaluator.update(candle)?;
        }
        Ok(())
    }

    fn indicator_values(&self) -> Result<HashMap<String, f64>> {
        let mut values = HashMap::new();
        for (id, evaluator) in &self.indicators {
            if let Ok(val) = evaluator.value("value") {
                values.insert(id.clone(), val);
            }
            for output in &["signal", "histogram", "upper", "lower", "middle"] {
                if let Ok(val) = evaluator.value(output) {
                    values.insert(format!("{}.{}", id, output), val);
                }
            }
        }
        Ok(values)
    }
}

impl crate::strategy::BacktestStrategy for RuleBasedStrategy {
    fn warmup(&self) -> usize {
        self.max_lookback
    }

    fn on_warmup(&mut self, candle: &Candle) -> Result<()> {
        self.update_indicators(candle)
    }

    fn on_candle(
        &mut self,
        candle: &Candle,
        portfolio: &Portfolio,
    ) -> Result<Option<StrategyAction>> {
        self.update_indicators(candle)?;

        let indicator_values = self.indicator_values()?;
        let position_size = portfolio.get_position(&self.compiled.instrument.coin);
        let is_flat = position_size.abs() < 1e-10;

        let action = if is_flat {
            if self
                .eval_state
                .evaluate(&self.compiled.entry.condition, &indicator_values)
            {
                Some(self.compiled.entry.action.clone())
            } else {
                None
            }
        } else if let Some(exit_rule) = &self.compiled.exit {
            if self
                .eval_state
                .evaluate(&exit_rule.condition, &indicator_values)
            {
                Some(exit_rule.action.clone())
            } else {
                None
            }
        } else {
            None
        };

        self.eval_state.update(&indicator_values);
        Ok(action)
    }
}
