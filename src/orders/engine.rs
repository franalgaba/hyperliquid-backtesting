use crate::data::types::Candle;
use crate::fees::FeeCalculator;
use crate::indicators2::{create_indicator, IndicatorEvaluator};
use crate::ir::compile::compile_strategy;
use crate::ir::types::{ActionSpec, ActionType, ComparisonOp, ExprValue, Node, StrategyIr};
use crate::orders::fills::process_order_fill;
use crate::orders::types::{
    Action, Order, OrderStatus, Side, SimConfig, SimResult, Trade, EquityPoint,
};
use crate::portfolio::Portfolio;
use anyhow::{Context, Result};
use std::collections::HashMap;

pub async fn simulate(
    candles: &[Candle],
    ir: &StrategyIr,
    config: &SimConfig,
) -> Result<SimResult> {
    let compiled = compile_strategy(ir)?;
    let fee_calc = FeeCalculator::new(
        config.maker_fee_bps,
        config.taker_fee_bps,
        config.slippage_bps,
    );
    let mut portfolio = Portfolio::new(config.initial_capital, fee_calc.clone());

    // Initialize indicators
    let mut indicators: HashMap<String, Box<dyn IndicatorEvaluator>> = HashMap::new();
    for ind in &compiled.indicators {
        let evaluator = create_indicator(&ind.indicator_type, &ind.params)
            .with_context(|| format!("Failed to create indicator: {}", ind.indicator_type))?;
        indicators.insert(ind.id.clone(), evaluator);
    }

    // Warm up indicators
    let max_lookback = compiled
        .indicators
        .iter()
        .map(|i| i.lookback)
        .max()
        .unwrap_or(0);

    if candles.len() < max_lookback {
        anyhow::bail!(
            "Not enough candles: need at least {}, got {}",
            max_lookback,
            candles.len()
        );
    }

    // Warm up phase
    for i in 0..max_lookback.min(candles.len()) {
        let candle = &candles[i];
        for evaluator in indicators.values_mut() {
            evaluator.update(candle)?;
        }
    }

    // Active orders and positions
    let mut active_orders: Vec<Order> = Vec::new();
    let mut next_order_id = 1u64;
    let mut trades = Vec::new();
    let mut equity_curve = Vec::new();

    // Main simulation loop
    for (_idx, candle) in candles.iter().enumerate().skip(max_lookback) {
        // Update indicators
        for evaluator in indicators.values_mut() {
            evaluator.update(candle)?;
        }

        // Determine which graph to evaluate based on position
        let position_size = portfolio.get_position(&candle.coin);
        let is_flat = position_size.abs() < 1e-10;
        
        if is_flat {
            // Flat position: evaluate entry graph
            evaluate_graph(
                &compiled.entry_node,
                &compiled.nodes,
                &compiled,
                &indicators,
                candle,
                &mut active_orders,
                &mut next_order_id,
                &portfolio,
            )?;
        } else {
            // In position: evaluate exit graph
            if let (Some(exit_entry), Some(exit_nodes)) = 
                (&compiled.exit_entry_node, &compiled.exit_nodes) {
                evaluate_graph(
                    exit_entry,
                    exit_nodes,
                    &compiled,
                    &indicators,
                    candle,
                    &mut active_orders,
                    &mut next_order_id,
                    &portfolio,
                )?;
            }
        }

        // Process active orders
        let mut orders_to_remove = Vec::new();
        for (order_idx, order) in active_orders.iter_mut().enumerate() {
            if let Some(fill_result) = process_order_fill(order, candle, &portfolio, &fee_calc) {
                if fill_result.order_status == OrderStatus::Filled {
                    let notional = fill_result.filled_sz * fill_result.fill_price;
                    let fee = fee_calc.calculate_fee(notional, fill_result.is_maker);

                    let trade = Trade {
                        timestamp: candle.time_open,
                        symbol: candle.coin.clone(),
                        side: match order.action {
                            Action::Market { side, .. }
                            | Action::Limit { side, .. }
                            | Action::StopMarket { side, .. }
                            | Action::StopLimit { side, .. }
                            | Action::TakeMarket { side, .. }
                            | Action::TakeLimit { side, .. } => {
                                if side == Side::Buy {
                                    "BUY"
                                } else {
                                    "SELL"
                                }
                            }
                            _ => "UNKNOWN",
                        }
                        .to_string(),
                        size: fill_result.filled_sz,
                        price: fill_result.fill_price,
                        fee,
                        order_id: order.id,
                    };

                    portfolio.execute_trade(&trade, fill_result.fill_price);
                    trades.push(trade);
                    orders_to_remove.push(order_idx);
                } else if fill_result.order_status == OrderStatus::Canceled {
                    orders_to_remove.push(order_idx);
                }
            }
        }

        // Remove filled/canceled orders (in reverse to maintain indices)
        for idx in orders_to_remove.iter().rev() {
            active_orders.remove(*idx);
        }

        // Record equity
        let current_price = candle.close;
        let equity = portfolio.total_equity(&candle.coin, current_price);
        equity_curve.push(EquityPoint {
            timestamp: candle.time_open,
            equity,
            cash: portfolio.cash,
            position_value: portfolio.get_position_value(&candle.coin, current_price),
        });
    }

    // Calculate metrics
    let final_equity = equity_curve.last().map(|e| e.equity).unwrap_or(config.initial_capital);
    let total_return = final_equity - config.initial_capital;
    let total_return_pct = (total_return / config.initial_capital) * 100.0;

    // Calculate win rate and PnL stats
    let (win_rate, avg_win, avg_loss) = calculate_trade_stats(&trades, &equity_curve);

    // Calculate drawdown
    let (max_drawdown, max_drawdown_pct) = calculate_drawdown(&equity_curve, config.initial_capital);

    // Calculate Sharpe and Sortino ratios
    let sharpe_ratio = calculate_sharpe_ratio(&equity_curve);
    let sortino_ratio = calculate_sortino_ratio(&equity_curve);

    let num_trades = trades.len();
    Ok(SimResult {
        trades,
        equity_curve,
        final_equity,
        total_return,
        total_return_pct,
        num_trades,
        win_rate,
        avg_win,
        avg_loss,
        max_drawdown,
        max_drawdown_pct,
        sharpe_ratio,
        sortino_ratio,
    })
}

/// Evaluate a graph (entry or exit) for candle-based backtesting
fn evaluate_graph(
    entry_node: &str,
    nodes: &std::collections::HashMap<String, crate::ir::types::Node>,
    compiled: &crate::ir::compile::CompiledStrategy,
    indicators: &HashMap<String, Box<dyn IndicatorEvaluator>>,
    candle: &Candle,
    active_orders: &mut Vec<Order>,
    next_order_id: &mut u64,
    portfolio: &Portfolio,
) -> Result<()> {
    use crate::ir::types::Node;
    
    let mut current_node = entry_node;

    loop {
        let node = nodes
            .get(current_node)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", current_node))?;

        match node {
            Node::Condition {
                expr,
                true_branch,
                false_branch,
            } => {
                let result = evaluate_expr(expr, indicators, candle)?;
                current_node = if result { true_branch } else { false_branch };
            }
            Node::Action { action, guards: _, next } => {
                // Check guards (simplified for v1)
                // TODO: Implement guard logic

                // Create order from action
                if let Some(order) = create_order_from_action(action, candle, *next_order_id, portfolio)? {
                    active_orders.push(order);
                    *next_order_id += 1;
                }

                current_node = next;
            }
            Node::Terminal => {
                break;
            }
        }
    }

    Ok(())
}

fn evaluate_expr(
    expr: &crate::ir::types::Expr,
    indicators: &HashMap<String, Box<dyn IndicatorEvaluator>>,
    candle: &Candle,
) -> Result<bool> {
    let lhs_val = get_expr_value(&expr.lhs, indicators, candle)?;
    let rhs_val = get_expr_value(&expr.rhs, indicators, candle)?;

    let result = match expr.op {
        ComparisonOp::Lt => lhs_val < rhs_val,
        ComparisonOp::Lte => lhs_val <= rhs_val,
        ComparisonOp::Eq => (lhs_val - rhs_val).abs() < 1e-10,
        ComparisonOp::Ne => (lhs_val - rhs_val).abs() >= 1e-10,
        ComparisonOp::Gte => lhs_val >= rhs_val,
        ComparisonOp::Gt => lhs_val > rhs_val,
        ComparisonOp::CrossesAbove => {
            // Simplified: check if current value is above and previous was below
            // TODO: Track previous values for proper cross detection
            lhs_val > rhs_val
        }
        ComparisonOp::CrossesBelow => {
            // Simplified: check if current value is below and previous was above
            lhs_val < rhs_val
        }
    };

    Ok(result)
}

fn get_expr_value(
    expr_val: &ExprValue,
    indicators: &HashMap<String, Box<dyn IndicatorEvaluator>>,
    candle: &Candle,
) -> Result<f64> {
    match expr_val {
        ExprValue::Ref { r#ref: ref_path } => {
            let parts: Vec<&str> = ref_path.split('.').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid indicator reference: {}", ref_path);
            }
            let ind_id = parts[0];
            let output = parts[1];

            let evaluator = indicators
                .get(ind_id)
                .ok_or_else(|| anyhow::anyhow!("Indicator not found: {}", ind_id))?;
            evaluator.value(output)
        }
        ExprValue::Const { r#const: val } => Ok(*val),
        ExprValue::Series { series } => match series.as_str() {
            "open" => Ok(candle.open),
            "high" => Ok(candle.high),
            "low" => Ok(candle.low),
            "close" => Ok(candle.close),
            "volume" => Ok(candle.volume),
            _ => anyhow::bail!("Unknown series: {}", series),
        },
    }
}

fn create_order_from_action(
    action: &ActionSpec,
    candle: &Candle,
    order_id: u64,
    portfolio: &Portfolio,
) -> Result<Option<Order>> {
    let side = match action.kind {
        ActionType::Buy => Side::Buy,
        ActionType::Sell => Side::Sell,
        ActionType::Close => {
            // Close existing position
            let pos_size = portfolio.get_position(&action.symbol);
            if pos_size.abs() < 1e-10 {
                return Ok(None); // No position to close
            }
            if pos_size > 0.0 {
                Side::Sell
            } else {
                Side::Buy
            }
        }
        ActionType::Alert => return Ok(None), // Alerts don't create orders
    };

    // Calculate size
    let sizing_mode = action
        .sizing
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("cash");
    let sizing_value = action
        .sizing
        .get("value")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let sz = match sizing_mode {
        "cash" => sizing_value / candle.close,
        "qty" => sizing_value,
        "pct" => {
            let equity = portfolio.total_equity(&action.symbol, candle.close);
            (equity * sizing_value / 100.0) / candle.close
        }
        _ => sizing_value / candle.close, // Default to cash
    };

    // Get order type
    let order_type_str = action
        .order
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("MARKET");

    let order_action = match order_type_str {
        "MARKET" => Action::Market { side, sz },
        "LIMIT" => {
            let limit_px_raw = action
                .order
                .get("limit")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            // If limit is 0.0 or not provided, use current price
            let limit_px = if limit_px_raw == 0.0 {
                candle.close
            } else {
                limit_px_raw
            };
            let tif_str = action
                .order
                .get("tif")
                .and_then(|v| v.as_str())
                .unwrap_or("GTC");
            let tif = match tif_str {
                "GTC" => crate::orders::types::Tif::Gtc,
                "IOC" => crate::orders::types::Tif::Ioc,
                "ALO" => crate::orders::types::Tif::Alo,
                _ => crate::orders::types::Tif::Gtc,
            };
            let post_only = tif == crate::orders::types::Tif::Alo;
            let reduce_only = action.kind == ActionType::Close;

            Action::Limit {
                side,
                px: limit_px,
                sz,
                tif,
                post_only,
                reduce_only,
            }
        }
        "STOP_MARKET" => {
            let trigger = action
                .order
                .get("trigger")
                .and_then(|v| v.as_f64())
                .unwrap_or(candle.close);
            Action::StopMarket { side, trigger, sz }
        }
        "STOP_LIMIT" => {
            let trigger = action
                .order
                .get("trigger")
                .and_then(|v| v.as_f64())
                .unwrap_or(candle.close);
            let limit_px = action
                .order
                .get("limit")
                .and_then(|v| v.as_f64())
                .unwrap_or(candle.close);
            let tif_str = action
                .order
                .get("tif")
                .and_then(|v| v.as_str())
                .unwrap_or("GTC");
            let tif = match tif_str {
                "GTC" => crate::orders::types::Tif::Gtc,
                "IOC" => crate::orders::types::Tif::Ioc,
                "ALO" => crate::orders::types::Tif::Alo,
                _ => crate::orders::types::Tif::Gtc,
            };
            Action::StopLimit {
                side,
                trigger,
                px: limit_px,
                sz,
                tif,
            }
        }
        "TAKE_MARKET" => {
            let trigger = action
                .order
                .get("trigger")
                .and_then(|v| v.as_f64())
                .unwrap_or(candle.close);
            Action::TakeMarket { side, trigger, sz }
        }
        "TAKE_LIMIT" => {
            let trigger = action
                .order
                .get("trigger")
                .and_then(|v| v.as_f64())
                .unwrap_or(candle.close);
            let limit_px = action
                .order
                .get("limit")
                .and_then(|v| v.as_f64())
                .unwrap_or(candle.close);
            let tif_str = action
                .order
                .get("tif")
                .and_then(|v| v.as_str())
                .unwrap_or("GTC");
            let tif = match tif_str {
                "GTC" => crate::orders::types::Tif::Gtc,
                "IOC" => crate::orders::types::Tif::Ioc,
                "ALO" => crate::orders::types::Tif::Alo,
                _ => crate::orders::types::Tif::Gtc,
            };
            Action::TakeLimit {
                side,
                trigger,
                px: limit_px,
                sz,
                tif,
            }
        }
        _ => return Ok(None), // Unknown order type
    };

    Ok(Some(Order {
        id: order_id,
        action: order_action,
        created_at: candle.time_open,
        filled_sz: 0.0,
        status: OrderStatus::Pending,
    }))
}

fn calculate_trade_stats(
    trades: &[Trade],
    equity_curve: &[EquityPoint],
) -> (f64, f64, f64) {
    if trades.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    // Simplified: calculate based on equity curve changes
    let mut wins = 0;
    let mut losses = 0;
    let mut total_win = 0.0;
    let mut total_loss = 0.0;

    for i in 1..equity_curve.len() {
        let change = equity_curve[i].equity - equity_curve[i - 1].equity;
        if change > 0.0 {
            wins += 1;
            total_win += change;
        } else if change < 0.0 {
            losses += 1;
            total_loss += change.abs();
        }
    }

    let total_trades = wins + losses;
    let win_rate = if total_trades > 0 {
        wins as f64 / total_trades as f64
    } else {
        0.0
    };
    let avg_win = if wins > 0 { total_win / wins as f64 } else { 0.0 };
    let avg_loss = if losses > 0 { total_loss / losses as f64 } else { 0.0 };

    (win_rate, avg_win, avg_loss)
}

fn calculate_drawdown(equity_curve: &[EquityPoint], initial_capital: f64) -> (f64, f64) {
    if equity_curve.is_empty() {
        return (0.0, 0.0);
    }

    let mut max_equity = initial_capital;
    let mut max_drawdown = 0.0;
    let mut max_drawdown_pct = 0.0;

    for point in equity_curve {
        if point.equity > max_equity {
            max_equity = point.equity;
        }
        let drawdown = max_equity - point.equity;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
            max_drawdown_pct = (drawdown / max_equity) * 100.0;
        }
    }

    (max_drawdown, max_drawdown_pct)
}

fn calculate_sharpe_ratio(equity_curve: &[EquityPoint]) -> f64 {
    if equity_curve.len() < 2 {
        return 0.0;
    }

    let returns: Vec<f64> = equity_curve
        .windows(2)
        .map(|w| (w[1].equity - w[0].equity) / w[0].equity)
        .collect();

    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|r| (r - mean_return).powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    let std_dev = variance.sqrt();

    if std_dev == 0.0 {
        return 0.0;
    }

    // Annualized Sharpe (assuming daily returns)
    mean_return / std_dev * (252.0_f64).sqrt()
}

fn calculate_sortino_ratio(equity_curve: &[EquityPoint]) -> f64 {
    if equity_curve.len() < 2 {
        return 0.0;
    }

    let returns: Vec<f64> = equity_curve
        .windows(2)
        .map(|w| (w[1].equity - w[0].equity) / w[0].equity)
        .collect();

    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
    let downside_variance = returns
        .iter()
        .filter(|r| **r < 0.0)
        .map(|r| r.powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    let downside_std = downside_variance.sqrt();

    if downside_std == 0.0 {
        return 0.0;
    }

    // Annualized Sortino
    mean_return / downside_std * (252.0_f64).sqrt()
}

