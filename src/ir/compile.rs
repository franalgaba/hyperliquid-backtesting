use crate::indicators2::IndicatorRegistry;
use crate::ir::types::*;
use anyhow::{Context, Result};

pub struct CompiledStrategy {
    pub instrument: Instrument,
    pub indicators: Vec<CompiledIndicator>,
    pub entry_node: String,
    pub nodes: std::collections::HashMap<String, Node>,
    /// Exit graph entry node (None if no exit graph provided)
    pub exit_entry_node: Option<String>,
    /// Exit graph nodes (None if no exit graph provided)
    pub exit_nodes: Option<std::collections::HashMap<String, Node>>,
}

pub struct CompiledIndicator {
    pub id: String,
    pub indicator_type: String,
    pub params: std::collections::HashMap<String, f64>,
    pub outputs: Vec<String>,
    pub lookback: usize,
}

pub fn compile_strategy(ir: &StrategyIr) -> Result<CompiledStrategy> {
    if ir.scopes.is_empty() {
        anyhow::bail!("Strategy IR has no scopes");
    }

    // For v1, we only support single-scope strategies
    let scope = &ir.scopes[0];
    let instrument = scope.instrument.clone();

    // Compile indicators
    let mut compiled_indicators = Vec::new();
    let registry = IndicatorRegistry::new();

    for ind_spec in &scope.indicators {
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
        instrument,
        indicators: compiled_indicators,
        entry_node: scope.graph.entry.clone(),
        nodes: scope.graph.nodes.clone(),
        exit_entry_node: scope.exit_graph.as_ref().map(|g| g.entry.clone()),
        exit_nodes: scope.exit_graph.as_ref().map(|g| g.nodes.clone()),
    })
}
