use crate::data::types::Candle;
use crate::fees::FeeCalculator;
use crate::indicators2::{create_indicator, IndicatorEvaluator};
use crate::strategy::{compile_strategy, Action as StrategyAction, EvalState, Strategy};
use crate::orders::fills::process_order_fill;
use crate::orders::types::{
    Action, Order, OrderStatus, Side, SimConfig, SimResult, Trade, EquityPoint,
};
use crate::portfolio::Portfolio;
use anyhow::{Context, Result};
use std::collections::HashMap;

pub async fn simulate(
    candles: &[Candle],
    strategy: &Strategy,
    config: &SimConfig,
) -> Result<SimResult> {
    let compiled = compile_strategy(strategy)?;
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
    let mut eval_state = EvalState::new();

    // Main simulation loop
    for (_idx, candle) in candles.iter().enumerate().skip(max_lookback) {
        // Update indicators
        for evaluator in indicators.values_mut() {
            evaluator.update(candle)?;
        }

        // Get current indicator values
        let indicator_values = get_indicator_values(&indicators)?;

        // Determine which rule to evaluate based on position
        let position_size = portfolio.get_position(&candle.coin);
        let is_flat = position_size.abs() < 1e-10;

        if is_flat {
            // Flat position: evaluate entry rule
            if eval_state.evaluate(&compiled.entry.condition, &indicator_values) {
                if let Some(order) = create_order_from_strategy_action(
                    &compiled.entry.action,
                    candle,
                    next_order_id,
                    &portfolio,
                )? {
                    active_orders.push(order);
                    next_order_id += 1;
                }
            }
        } else if let Some(exit_rule) = &compiled.exit {
            // In position: evaluate exit rule
            if eval_state.evaluate(&exit_rule.condition, &indicator_values) {
                if let Some(order) = create_order_from_strategy_action(
                    &exit_rule.action,
                    candle,
                    next_order_id,
                    &portfolio,
                )? {
                    active_orders.push(order);
                    next_order_id += 1;
                }
            }
        }

        // Update eval state with current values for crossover detection
        eval_state.update(&indicator_values);

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

fn get_indicator_values(
    indicators: &HashMap<String, Box<dyn IndicatorEvaluator>>,
) -> Result<HashMap<String, f64>> {
    let mut values = HashMap::new();
    for (id, evaluator) in indicators {
        // Get the primary output value
        if let Ok(val) = evaluator.value("value") {
            values.insert(id.clone(), val);
        }
        // Also try common output names
        for output in &["signal", "histogram", "upper", "lower", "middle"] {
            if let Ok(val) = evaluator.value(output) {
                values.insert(format!("{}.{}", id, output), val);
            }
        }
    }
    Ok(values)
}

fn create_order_from_strategy_action(
    action: &StrategyAction,
    candle: &Candle,
    order_id: u64,
    portfolio: &Portfolio,
) -> Result<Option<Order>> {
    let (side, sz) = match action {
        StrategyAction::Buy { size_pct } => {
            let equity = portfolio.total_equity(&candle.coin, candle.close);
            let sz = (equity * size_pct / 100.0) / candle.close;
            (Side::Buy, sz)
        }
        StrategyAction::Sell { size_pct } => {
            let pos_size = portfolio.get_position(&candle.coin);
            let sz = pos_size.abs() * size_pct / 100.0;
            (Side::Sell, sz)
        }
        StrategyAction::Close => {
            let pos_size = portfolio.get_position(&candle.coin);
            if pos_size.abs() < 1e-10 {
                return Ok(None);
            }
            let side = if pos_size > 0.0 { Side::Sell } else { Side::Buy };
            (side, pos_size.abs())
        }
    };

    Ok(Some(Order {
        id: order_id,
        action: Action::Market { side, sz },
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

    mean_return / downside_std * (252.0_f64).sqrt()
}
