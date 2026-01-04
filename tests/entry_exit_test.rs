#[cfg(test)]
mod tests {
    use hl_backtest::ir::compile::compile_strategy;
    use hl_backtest::ir::types::*;
    use std::collections::HashMap;

    fn create_test_ir_with_exit() -> StrategyIr {
        // Create a simple IR with both entry and exit graphs
        let entry_graph = Graph {
            entry: "entry1".to_string(),
            nodes: {
                let mut nodes = HashMap::new();
                nodes.insert(
                    "entry1".to_string(),
                    Node::Condition {
                        expr: Expr {
                            op: ComparisonOp::Lt,
                            lhs: ExprValue::Series {
                                series: "close".to_string(),
                            },
                            rhs: ExprValue::Const { r#const: 50000.0 },
                        },
                        true_branch: "buy1".to_string(),
                        false_branch: "end".to_string(),
                    },
                );
                nodes.insert(
                    "buy1".to_string(),
                    Node::Action {
                        action: ActionSpec {
                            kind: ActionType::Buy,
                            symbol: "BTCUSDC".to_string(),
                            sizing: {
                                let mut m = HashMap::new();
                                m.insert("mode".to_string(), serde_json::Value::String("cash".to_string()));
                                m.insert("value".to_string(), serde_json::Value::Number(1000.0.into()));
                                m
                            },
                            order: {
                                let mut m = HashMap::new();
                                m.insert("type".to_string(), serde_json::Value::String("MARKET".to_string()));
                                m
                            },
                        },
                        guards: vec![],
                        next: "end".to_string(),
                    },
                );
                nodes.insert("end".to_string(), Node::Terminal);
                nodes
            },
        };

        let exit_graph = Graph {
            entry: "exit1".to_string(),
            nodes: {
                let mut nodes = HashMap::new();
                nodes.insert(
                    "exit1".to_string(),
                    Node::Condition {
                        expr: Expr {
                            op: ComparisonOp::Gt,
                            lhs: ExprValue::Series {
                                series: "close".to_string(),
                            },
                            rhs: ExprValue::Const { r#const: 51000.0 },
                        },
                        true_branch: "close1".to_string(),
                        false_branch: "end".to_string(),
                    },
                );
                nodes.insert(
                    "close1".to_string(),
                    Node::Action {
                        action: ActionSpec {
                            kind: ActionType::Close,
                            symbol: "BTCUSDC".to_string(),
                            sizing: {
                                let mut m = HashMap::new();
                                m.insert("mode".to_string(), serde_json::Value::String("cash".to_string()));
                                m.insert("value".to_string(), serde_json::Value::Number(1000.0.into()));
                                m
                            },
                            order: {
                                let mut m = HashMap::new();
                                m.insert("type".to_string(), serde_json::Value::String("MARKET".to_string()));
                                m
                            },
                        },
                        guards: vec![],
                        next: "end".to_string(),
                    },
                );
                nodes.insert("end".to_string(), Node::Terminal);
                nodes
            },
        };

        StrategyIr {
            version: "1.0".to_string(),
            compiler_version: "1.0.0".to_string(),
            registry_versions: HashMap::new(),
            defaults_version: "2025-01-01".to_string(),
            meta: HashMap::new(),
            settings: HashMap::new(),
            scopes: vec![Scope {
                instrument: Instrument {
                    symbol: "BTCUSDC".to_string(),
                    coin: "BTC".to_string(),
                    venue: "HYPERLIQUID_SPOT".to_string(),
                    timeframe: "1h".to_string(),
                },
                indicators: vec![],
                graph: entry_graph,
                exit_graph: Some(exit_graph),
            }],
            ir_hash: "test".to_string(),
        }
    }

    #[test]
    fn test_compile_strategy_with_exit_graph() {
        let ir = create_test_ir_with_exit();
        let compiled = compile_strategy(&ir).unwrap();

        // Verify entry graph is compiled
        assert_eq!(compiled.entry_node, "entry1");
        assert!(compiled.nodes.contains_key("entry1"));
        assert!(compiled.nodes.contains_key("buy1"));

        // Verify exit graph is compiled
        assert!(compiled.exit_entry_node.is_some());
        assert_eq!(compiled.exit_entry_node.as_ref().unwrap(), "exit1");
        assert!(compiled.exit_nodes.is_some());
        let exit_nodes = compiled.exit_nodes.as_ref().unwrap();
        assert!(exit_nodes.contains_key("exit1"));
        assert!(exit_nodes.contains_key("close1"));
    }

    #[test]
    fn test_compile_strategy_without_exit_graph() {
        let mut ir = create_test_ir_with_exit();
        ir.scopes[0].exit_graph = None;

        let compiled = compile_strategy(&ir).unwrap();

        // Entry graph should still be compiled
        assert_eq!(compiled.entry_node, "entry1");

        // Exit graph should be None
        assert!(compiled.exit_entry_node.is_none());
        assert!(compiled.exit_nodes.is_none());
    }
}

