use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Simplified strategy definition replacing the complex IR system.
/// Strategies define indicators, entry/exit conditions, and actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    /// Strategy name
    pub name: String,
    /// Target instrument
    pub instrument: Instrument,
    /// Indicators to compute
    pub indicators: Vec<IndicatorSpec>,
    /// Entry rule (when to open a position)
    pub entry: Rule,
    /// Exit rule (when to close a position)
    pub exit: Option<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    pub symbol: String,
    pub coin: String,
    pub venue: String,
    pub timeframe: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorSpec {
    pub id: String,
    #[serde(rename = "type")]
    pub indicator_type: String,
    pub params: HashMap<String, f64>,
    pub outputs: Vec<String>,
}

/// A rule combines a condition with an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub condition: Condition,
    pub action: Action,
}

/// Condition for triggering an action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Condition {
    /// Compare an indicator value to a constant
    #[serde(rename = "threshold")]
    Threshold {
        indicator: String,
        op: ComparisonOp,
        value: f64,
    },
    /// Compare two indicators
    #[serde(rename = "crossover")]
    Crossover {
        fast: String,
        slow: String,
        direction: CrossDirection,
    },
    /// Logical AND of multiple conditions
    #[serde(rename = "and")]
    And { conditions: Vec<Condition> },
    /// Logical OR of multiple conditions
    #[serde(rename = "or")]
    Or { conditions: Vec<Condition> },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOp {
    Lt,
    Lte,
    Eq,
    Ne,
    Gte,
    Gt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossDirection {
    Above,
    Below,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// Buy with a percentage of available capital
    #[serde(rename = "buy")]
    Buy { size_pct: f64 },
    /// Sell with a percentage of position
    #[serde(rename = "sell")]
    Sell { size_pct: f64 },
    /// Close entire position
    #[serde(rename = "close")]
    Close,
}

/// Compiled strategy ready for execution
pub struct CompiledStrategy {
    pub instrument: Instrument,
    pub indicators: Vec<CompiledIndicator>,
    pub entry: Rule,
    pub exit: Option<Rule>,
}

pub struct CompiledIndicator {
    pub id: String,
    pub indicator_type: String,
    pub params: HashMap<String, f64>,
    pub outputs: Vec<String>,
    pub lookback: usize,
}
