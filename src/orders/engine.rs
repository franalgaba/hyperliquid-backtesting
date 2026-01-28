use crate::data::types::Candle;
use crate::fees::FeeCalculator;
use crate::strategy::{Action as StrategyAction, BacktestStrategy, RuleBasedStrategy, Strategy};
use crate::orders::fills::process_order_fill;
use crate::orders::types::{
    Action, Order, OrderStatus, Side, SimConfig, SimResult, Trade, EquityPoint,
};
use crate::portfolio::Portfolio;
use anyhow::Result;

pub async fn simulate(
    candles: &[Candle],
    strategy: &Strategy,
    config: &SimConfig,
) -> Result<SimResult> {
    let mut rule_strategy = RuleBasedStrategy::from_strategy(strategy)?;
    simulate_with_strategy(candles, &mut rule_strategy, config).await
}

pub async fn simulate_with_strategy(
    candles: &[Candle],
    strategy: &mut dyn BacktestStrategy,
    config: &SimConfig,
) -> Result<SimResult> {
    let fee_calc = FeeCalculator::new(
        config.maker_fee_bps,
        config.taker_fee_bps,
        config.slippage_bps,
    );
    let mut portfolio = Portfolio::new(config.initial_capital, fee_calc.clone());

    let warmup = strategy.warmup();
    if candles.len() < warmup {
        anyhow::bail!(
            "Not enough candles: need at least {}, got {}",
            warmup,
            candles.len()
        );
    }

    for candle in candles.iter().take(warmup) {
        strategy.on_warmup(candle)?;
    }

    let mut active_orders: Vec<Order> = Vec::new();
    let mut next_order_id = 1u64;
    let mut trades = Vec::new();
    let mut equity_curve = Vec::new();

    for candle in candles.iter().skip(warmup) {
        if let Some(action) = strategy.on_candle(candle, &portfolio)? {
            if let Some(order) = create_order_from_strategy_action(
                &action,
                candle,
                next_order_id,
                &portfolio,
            )? {
                active_orders.push(order);
                next_order_id += 1;
            }
        }

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

        for idx in orders_to_remove.iter().rev() {
            active_orders.remove(*idx);
        }

        let current_price = candle.close;
        let equity = portfolio.total_equity(&candle.coin, current_price);
        equity_curve.push(EquityPoint {
            timestamp: candle.time_open,
            equity,
            cash: portfolio.cash,
            position_value: portfolio.get_position_value(&candle.coin, current_price),
        });
    }

    let final_equity = equity_curve
        .last()
        .map(|e| e.equity)
        .unwrap_or(config.initial_capital);
    let total_return = final_equity - config.initial_capital;
    let total_return_pct = (total_return / config.initial_capital) * 100.0;

    let (win_rate, avg_win, avg_loss) = calculate_trade_stats(&trades, &equity_curve);
    let (max_drawdown, max_drawdown_pct) = calculate_drawdown(&equity_curve, config.initial_capital);
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

fn create_order_from_strategy_action(
    action: &StrategyAction,
    candle: &Candle,
    order_id: u64,
    portfolio: &Portfolio,
) -> Result<Option<Order>> {
    let (side, sz) = match action {
        StrategyAction::Buy { size_pct } => {
            let pos_size = portfolio.get_position(&candle.coin);
            let sz = if pos_size < -1e-10 {
                pos_size.abs() * size_pct / 100.0
            } else {
                let equity = portfolio.total_equity(&candle.coin, candle.close);
                (equity * size_pct / 100.0) / candle.close
            };
            (Side::Buy, sz)
        }
        StrategyAction::Sell { size_pct } => {
            let pos_size = portfolio.get_position(&candle.coin);
            let sz = if pos_size > 1e-10 {
                pos_size.abs() * size_pct / 100.0
            } else {
                let equity = portfolio.total_equity(&candle.coin, candle.close);
                (equity * size_pct / 100.0) / candle.close
            };
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

    if sz <= 0.0 || sz.is_nan() || sz.is_infinite() {
        return Ok(None);
    }

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
