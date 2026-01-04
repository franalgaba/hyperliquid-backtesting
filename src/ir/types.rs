use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyIr {
    pub version: String,
    #[serde(rename = "compiler_version")]
    pub compiler_version: String,
    #[serde(rename = "registry_versions")]
    pub registry_versions: std::collections::HashMap<String, String>,
    #[serde(rename = "defaults_version")]
    pub defaults_version: String,
    pub meta: std::collections::HashMap<String, String>,
    pub settings: std::collections::HashMap<String, serde_json::Value>,
    pub scopes: Vec<Scope>,
    #[serde(rename = "ir_hash")]
    pub ir_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub instrument: Instrument,
    pub indicators: Vec<IndicatorSpec>,
    pub graph: Graph,
    /// Exit graph for closing positions. Required for complete strategies.
    /// When in a position, the engine evaluates this graph instead of the entry graph.
    #[serde(rename = "exit_graph")]
    pub exit_graph: Option<Graph>,
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
    pub params: std::collections::HashMap<String, f64>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub entry: String,
    pub nodes: std::collections::HashMap<String, Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Node {
    #[serde(rename = "condition")]
    Condition {
        expr: Expr,
        #[serde(rename = "true")]
        true_branch: String,
        #[serde(rename = "false")]
        false_branch: String,
    },
    #[serde(rename = "action")]
    Action {
        action: ActionSpec,
        guards: Vec<String>,
        next: String,
    },
    #[serde(rename = "terminal")]
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expr {
    pub op: ComparisonOp,
    pub lhs: ExprValue,
    pub rhs: ExprValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExprValue {
    #[serde(rename = "ref")]
    Ref { #[serde(rename = "ref")] r#ref: String },
    #[serde(rename = "const")]
    Const { #[serde(rename = "const")] r#const: f64 },
    #[serde(rename = "series")]
    Series { #[serde(rename = "series")] series: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComparisonOp {
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "<=")]
    Lte,
    #[serde(rename = "==")]
    Eq,
    #[serde(rename = "!=")]
    Ne,
    #[serde(rename = ">=")]
    Gte,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = "crosses_above")]
    CrossesAbove,
    #[serde(rename = "crosses_below")]
    CrossesBelow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSpec {
    pub kind: ActionType,
    pub symbol: String,
    pub sizing: std::collections::HashMap<String, serde_json::Value>,
    pub order: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ActionType {
    Buy,
    Sell,
    Close,
    Alert,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Market,
    Limit,
    StopLimit,
    StopMarket,
    TakeLimit,
    TakeMarket,
}

