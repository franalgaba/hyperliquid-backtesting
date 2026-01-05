use crate::indicators2::IndicatorRegistry;
use crate::strategy::types::*;
use anyhow::{Context, Result};

/// Compile a strategy, resolving indicator lookbacks
pub fn compile_strategy(strategy: &Strategy) -> Result<CompiledStrategy> {
    let registry = IndicatorRegistry::new();
    let mut compiled_indicators = Vec::new();

    for ind_spec in &strategy.indicators {
        let lookback = registry
            .get_lookback(&ind_spec.indicator_type, &ind_spec.params)
            .with_context(|| format!("Unknown indicator: {}", ind_spec.indicator_type))?;

        compiled_indicators.push(CompiledIndicator {
            id: ind_spec.id.clone(),
            indicator_type: ind_spec.indicator_type.clone(),
            params: ind_spec.params.clone(),
            outputs: ind_spec.outputs.clone(),
            lookback,
        });
    }

    Ok(CompiledStrategy {
        instrument: strategy.instrument.clone(),
        indicators: compiled_indicators,
        entry: strategy.entry.clone(),
        exit: strategy.exit.clone(),
    })
}
