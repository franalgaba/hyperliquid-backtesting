use crate::data::types::Candle;
use crate::fees::FeeCalculator;
use crate::indicators2::{create_indicator, IndicatorEvaluator};
use crate::ingest::{parse_l2_jsonl_file, L2Event};
use crate::strategy::{compile_strategy, Action as StrategyAction, EvalState, Strategy};
use crate::orderbook::OrderBook;
use crate::orders::types::{
    Action, EquityPoint, Order, OrderStatus, Side, SimConfig, SimResult, Trade,
};
use crate::perps::execution::PerpsExecution;
use crate::perps::funding::FundingSchedule;
use crate::perps::trade_utils::{extract_side_from_action, side_to_string};
use crate::portfolio::Portfolio;
use anyhow::{Context, Result};
use futures::StreamExt;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Constants for configuration and thresholds
const DEFAULT_EVENTS_CAPACITY: usize = 100_000;
const DEFAULT_ORDERS_CAPACITY: usize = 100;
const DEFAULT_TRADES_CAPACITY: usize = 1000;
const DEFAULT_EQUITY_CURVE_CAPACITY: usize = 10000;
const FUNDING_INTERVAL_MS: u64 = 8 * 60 * 60 * 1000;
const PRICE_CHANGE_THRESHOLD: f64 = 0.0001;
const MIN_FILL_SIZE: f64 = 1e-10;
const EQUITY_RECORDING_INTERVAL_MS: u64 = 60 * 1000;

/// Perpetual futures backtesting engine using L2 order book events.
pub struct PerpsEngine {
    book: OrderBook,
    funding: FundingSchedule,
    fee_calc: FeeCalculator,
    portfolio: Portfolio,
}

impl PerpsEngine {
    pub fn new(funding: FundingSchedule, config: &SimConfig) -> Self {
        let fee_calc = FeeCalculator::new(
            config.maker_fee_bps,
            config.taker_fee_bps,
            config.slippage_bps,
        );
        let portfolio = Portfolio::new(config.initial_capital, fee_calc.clone());

        Self {
            book: OrderBook::new(),
            funding,
            fee_calc,
            portfolio,
        }
    }

    pub async fn run(
        events_dir: impl AsRef<Path>,
        strategy: &Strategy,
        config: &SimConfig,
        coin: &str,
        start_ts: u64,
        end_ts: u64,
        io_concurrency: Option<usize>,
        indicators_parallel: bool,
    ) -> Result<SimResult> {
        let compiled = compile_strategy(strategy)?;

        // Initialize indicators
        let num_indicators = compiled.indicators.len();
        let mut indicators: HashMap<String, Box<dyn IndicatorEvaluator>> =
            HashMap::with_capacity(num_indicators.max(8));
        for ind in &compiled.indicators {
            let evaluator = create_indicator(&ind.indicator_type, &ind.params)
                .with_context(|| format!("Failed to create indicator: {}", ind.indicator_type))?;
            indicators.insert(ind.id.clone(), evaluator);
        }

        // Load all events from directory
        let events_dir = events_dir.as_ref();
        let mut all_events: Vec<(u64, L2Event)> = Vec::with_capacity(DEFAULT_EVENTS_CAPACITY);

        let entries = fs::read_dir(events_dir).with_context(|| {
            format!("Failed to read events directory: {}", events_dir.display())
        })?;

        let mut jsonl_files = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                jsonl_files.push(path);
            }
        }

        // Process files in parallel
        let concurrency = io_concurrency.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get().min(8))
                .unwrap_or(4)
        });

        let mut stream = futures::stream::iter(jsonl_files)
            .map(|path| async move {
                let events = parse_l2_jsonl_file(&path).await?;
                Ok::<Vec<L2Event>, anyhow::Error>(events)
            })
            .buffer_unordered(concurrency);

        while let Some(result) = stream.next().await {
            let events = result?;
            for event in events {
                if event.ts_ms >= start_ts && event.ts_ms <= end_ts {
                    all_events.push((event.ts_ms, event));
                }
            }
        }

        all_events.sort_by_key(|(ts, _)| *ts);

        if all_events.is_empty() {
            anyhow::bail!("No events found in range");
        }

        println!("Loaded {} events for backtest", all_events.len());

        // Fetch funding schedule
        let funding = FundingSchedule::from_api(coin, start_ts, end_ts)
            .await
            .context("Failed to fetch funding history")?;

        // Initialize engine
        let mut engine = Self::new(funding, config);

        let mut active_orders: Vec<Order> = Vec::with_capacity(DEFAULT_ORDERS_CAPACITY);
        let mut next_order_id = 1u64;
        let mut trades = Vec::with_capacity(DEFAULT_TRADES_CAPACITY);
        let mut equity_curve = Vec::with_capacity(DEFAULT_EQUITY_CURVE_CAPACITY);
        let mut eval_state = EvalState::new();

        let mut last_funding_ts = 0u64;

        let max_lookback = compiled
            .indicators
            .iter()
            .map(|i| i.lookback)
            .max()
            .unwrap_or(0);

        let mut last_evaluated_price = 0.0;

        let trade_cooldown_ms = config.trade_cooldown_ms.unwrap_or(15 * 60 * 1000);
        let mut last_trade_ts: Option<u64> = None;

        #[allow(unused_variables)]
        let total_events = all_events.len();
        let coin_str = coin.to_string();

        let mut synthetic_candle = Candle {
            time_open: 0,
            time_close: 0,
            coin: coin_str.clone(),
            interval: "tick".to_string(),
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume: 0.0,
            num_trades: 0,
        };

        for (event_idx, (ts_ms, event)) in all_events.iter().enumerate() {
            // Log progress
            #[cfg(not(debug_assertions))]
            {
                if event_idx % (total_events / 10).max(1) == 0 {
                    let progress_pct = (event_idx as f64 / total_events as f64 * 100.0) as u32;
                    println!("Progress: {}%", progress_pct);
                }
            }

            engine.book.apply_snapshot(&event.levels);

            let price = match engine.book.mid_price() {
                Some(p) => p,
                None => continue,
            };

            // Update synthetic candle
            synthetic_candle.time_close = *ts_ms;
            synthetic_candle.high = synthetic_candle.high.max(price);
            synthetic_candle.low = if synthetic_candle.low == 0.0 {
                price
            } else {
                synthetic_candle.low.min(price)
            };
            synthetic_candle.close = price;
            if synthetic_candle.open == 0.0 {
                synthetic_candle.open = price;
                synthetic_candle.time_open = *ts_ms;
            }

            // Update indicators
            if indicators_parallel && indicators.len() > 1 {
                let mut evaluators: Vec<&mut Box<dyn IndicatorEvaluator>> =
                    indicators.values_mut().collect();
                let update_result: Result<()> = evaluators
                    .par_iter_mut()
                    .try_for_each(|evaluator| evaluator.update(&synthetic_candle));
                update_result?;
            } else {
                for evaluator in indicators.values_mut() {
                    evaluator.update(&synthetic_candle)?;
                }
            }

            let mut orders_to_remove = Vec::new();

            // Evaluate strategy
            let price_changed = (price - last_evaluated_price).abs()
                / last_evaluated_price.max(1.0)
                > PRICE_CHANGE_THRESHOLD;
            let should_evaluate =
                event_idx >= max_lookback && (price_changed || last_evaluated_price == 0.0);

            if should_evaluate {
                let indicator_values = get_indicator_values(&indicators)?;
                let position_size = engine.portfolio.get_position(&coin_str);
                let is_flat = position_size.abs() < 1e-10;

                if is_flat {
                    // Check entry condition (with cooldown)
                    let can_trade = last_trade_ts
                        .map(|last_ts| *ts_ms >= last_ts + trade_cooldown_ms)
                        .unwrap_or(true);

                    if can_trade && eval_state.evaluate(&compiled.entry.condition, &indicator_values) {
                        if let Some(order) = create_order_from_strategy_action(
                            &compiled.entry.action,
                            &synthetic_candle,
                            next_order_id,
                            &engine.portfolio,
                        )? {
                            active_orders.push(order);
                            next_order_id += 1;
                        }
                    }
                } else if let Some(exit_rule) = &compiled.exit {
                    // Check exit condition (no cooldown for exits)
                    if eval_state.evaluate(&exit_rule.condition, &indicator_values) {
                        if let Some(order) = create_order_from_strategy_action(
                            &exit_rule.action,
                            &synthetic_candle,
                            next_order_id,
                            &engine.portfolio,
                        )? {
                            active_orders.push(order);
                            next_order_id += 1;
                        }
                    }
                }

                eval_state.update(&indicator_values);
                last_evaluated_price = price;
            }

            // Execute market orders
            let mut market_orders_to_execute = Vec::new();
            let mut market_order_indices = Vec::new();

            for (idx, order) in active_orders.iter().enumerate() {
                if matches!(order.action, Action::Market { .. }) {
                    market_orders_to_execute.push(order.clone());
                    market_order_indices.push(idx);
                }
            }

            for (order_idx, mut order) in market_orders_to_execute.into_iter().enumerate() {
                let original_idx = market_order_indices[order_idx];

                if let Some(fill_result) = PerpsExecution::execute_market(&mut order, &engine.book) {
                    if fill_result.filled_sz > MIN_FILL_SIZE {
                        let side = match extract_side_from_action(&order.action) {
                            Some(s) => s,
                            None => continue,
                        };

                        process_trade_fill(
                            &fill_result,
                            &order,
                            *ts_ms,
                            coin,
                            &coin_str,
                            side,
                            &mut engine.portfolio,
                            &engine.fee_calc,
                            &mut trades,
                        );

                        last_trade_ts = Some(*ts_ms);
                    }

                    orders_to_remove.push(original_idx);
                }
            }

            // Remove executed orders
            for &idx in orders_to_remove.iter().rev() {
                let last_idx = active_orders.len() - 1;
                if idx != last_idx {
                    active_orders.swap(idx, last_idx);
                }
                active_orders.pop();
            }
            orders_to_remove.clear();

            // Check limit orders
            for (idx, order) in active_orders.iter_mut().enumerate() {
                if let Some(fill_result) = PerpsExecution::check_limit_fill(order, &engine.book) {
                    let side = match extract_side_from_action(&order.action) {
                        Some(s) => s,
                        None => continue,
                    };

                    process_trade_fill(
                        &fill_result,
                        order,
                        *ts_ms,
                        coin,
                        &coin_str,
                        side,
                        &mut engine.portfolio,
                        &engine.fee_calc,
                        &mut trades,
                    );

                    last_trade_ts = Some(*ts_ms);

                    if fill_result.order_status == OrderStatus::Filled {
                        orders_to_remove.push(idx);
                    }
                }
            }

            for &idx in orders_to_remove.iter().rev() {
                let last_idx = active_orders.len() - 1;
                if idx != last_idx {
                    active_orders.swap(idx, last_idx);
                }
                active_orders.pop();
            }

            // Apply funding
            if *ts_ms - last_funding_ts >= FUNDING_INTERVAL_MS {
                apply_funding_payment(&mut engine, coin, price, *ts_ms);
                last_funding_ts = *ts_ms;
            }

            // Record equity
            if *ts_ms % EQUITY_RECORDING_INTERVAL_MS == 0 {
                record_equity_point(&mut equity_curve, &engine.portfolio, coin, price, *ts_ms);
            }
        }

        // Calculate final metrics
        let final_price = engine.book.mid_price().unwrap_or(0.0);
        let final_equity = engine.portfolio.total_equity(coin, final_price);
        let total_return = final_equity - config.initial_capital;
        let total_return_pct = (total_return / config.initial_capital) * 100.0;
        let num_trades = trades.len();

        Ok(SimResult {
            trades,
            equity_curve,
            final_equity,
            total_return,
            total_return_pct,
            num_trades,
            win_rate: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            max_drawdown: 0.0,
            max_drawdown_pct: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
        })
    }
}

fn get_indicator_values(
    indicators: &HashMap<String, Box<dyn IndicatorEvaluator>>,
) -> Result<HashMap<String, f64>> {
    let mut values = HashMap::new();
    for (id, evaluator) in indicators {
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

fn process_trade_fill(
    fill_result: &crate::perps::execution::FillResult,
    order: &Order,
    timestamp: u64,
    _coin: &str,
    coin_str: &str,
    side: Side,
    portfolio: &mut Portfolio,
    fee_calc: &FeeCalculator,
    trades: &mut Vec<Trade>,
) {
    let notional = fill_result.filled_sz * fill_result.fill_price;
    let fee = fee_calc.calculate_fee(notional, fill_result.is_maker);

    let trade = Trade {
        timestamp,
        symbol: coin_str.to_string(),
        side: side_to_string(side).to_string(),
        size: fill_result.filled_sz,
        price: fill_result.fill_price,
        fee,
        order_id: order.id,
    };

    portfolio.execute_trade(&trade, fill_result.fill_price);
    trades.push(trade);
}

fn apply_funding_payment(engine: &mut PerpsEngine, coin: &str, price: f64, ts_ms: u64) {
    if let Some(position) = engine.portfolio.positions.get(coin) {
        if position.size.abs() > MIN_FILL_SIZE {
            let notional = position.size.abs() * price;
            let funding_payment = engine.funding.calculate_payment(notional, ts_ms);

            if position.size > 0.0 {
                engine.portfolio.cash -= funding_payment;
            } else {
                engine.portfolio.cash += funding_payment;
            }
        }
    }
}

fn record_equity_point(
    equity_curve: &mut Vec<EquityPoint>,
    portfolio: &Portfolio,
    coin: &str,
    price: f64,
    timestamp: u64,
) {
    let equity = portfolio.total_equity(coin, price);
    equity_curve.push(EquityPoint {
        timestamp,
        equity,
        cash: portfolio.cash,
        position_value: portfolio.get_position_value(coin, price),
    });
}
