use crate::data::types::Candle;
use crate::fees::FeeCalculator;
use crate::indicators2::{create_indicator, IndicatorEvaluator};
use crate::ingest::{parse_l2_jsonl_file, L2Event};
use crate::ir::compile::compile_strategy;
use crate::ir::types::StrategyIr;
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
/// Default capacity for events vector (pre-allocated to avoid reallocations)
const DEFAULT_EVENTS_CAPACITY: usize = 100_000;
/// Default capacity for active orders vector
const DEFAULT_ORDERS_CAPACITY: usize = 100;
/// Default capacity for trades vector
const DEFAULT_TRADES_CAPACITY: usize = 1000;
/// Default capacity for equity curve vector (1 point per minute)
const DEFAULT_EQUITY_CURVE_CAPACITY: usize = 10000;
/// Funding payment interval: 8 hours in milliseconds
const FUNDING_INTERVAL_MS: u64 = 8 * 60 * 60 * 1000;
/// Price change threshold for strategy evaluation (0.01% = 0.0001)
const PRICE_CHANGE_THRESHOLD: f64 = 0.0001;
/// Minimum fill size threshold (used to avoid floating point precision issues)
const MIN_FILL_SIZE: f64 = 1e-10;
/// Equity recording interval: 1 minute in milliseconds
const EQUITY_RECORDING_INTERVAL_MS: u64 = 60 * 1000;

/// Perpetual futures backtesting engine using L2 order book events.
///
/// The `PerpsEngine` simulates realistic trading execution by:
/// - Reconstructing order books from historical L2 snapshots
/// - Executing orders against the order book with realistic fills
/// - Tracking positions, funding payments, and fees
/// - Recording equity curves and trade history
///
/// # Architecture
///
/// PerpsEngine contains:
/// - OrderBook: Current order book state
/// - FundingSchedule: Historical funding rates
/// - FeeCalculator: Fee calculation logic
/// - Portfolio: Position and cash tracking
///
/// # Key Concepts
///
/// - **L2 Events**: Order book snapshots containing bid/ask levels
/// - **Synthetic Candles**: Price data derived from order book mid-price
/// - **Strategy Evaluation**: Graph-based strategy execution on price changes
/// - **Order Execution**: Market orders execute immediately, limit orders wait for fills
///
/// # Example
///
/// ```rust,no_run
/// use crate::perps::PerpsEngine;
/// use crate::ir::types::StrategyIr;
/// use crate::orders::types::SimConfig;
///
/// # async fn example() -> anyhow::Result<()> {
/// let funding = FundingSchedule::from_api("BTC", start_ts, end_ts).await?;
/// let engine = PerpsEngine::new(funding, &config);
///
/// let result = PerpsEngine::run(
///     "data/events/BTC",
///     &strategy_ir,
///     &config,
///     "BTC",
///     1694858400000,
///     1694865600000,
///     Some(8),
///     true,
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct PerpsEngine {
    book: OrderBook,
    funding: FundingSchedule,
    fee_calc: FeeCalculator,
    portfolio: Portfolio,
}

impl PerpsEngine {
    /// Creates a new `PerpsEngine` instance.
    ///
    /// # Parameters
    ///
    /// - `funding`: Historical funding rate schedule for the backtest period
    /// - `config`: Simulation configuration (fees, initial capital, etc.)
    ///
    /// # Returns
    ///
    /// Initialized `PerpsEngine` with empty order book and portfolio.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::perps::{PerpsEngine, FundingSchedule};
    /// use crate::orders::types::SimConfig;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let funding = FundingSchedule::from_api("BTC", start_ts, end_ts).await?;
    /// let config = SimConfig {
    ///     initial_capital: 10000.0,
    ///     maker_fee_bps: -1,
    ///     taker_fee_bps: 10,
    ///     slippage_bps: 5,
    /// };
    /// let engine = PerpsEngine::new(funding, &config);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Runs a complete backtest from L2 order book events.
    ///
    /// This is the main entry point for perps backtesting. It:
    /// 1. Loads and parses L2 event files from the specified directory
    /// 2. Reconstructs order books from snapshots
    /// 3. Evaluates the strategy on price changes
    /// 4. Executes orders against the order book
    /// 5. Tracks positions, funding payments, and fees
    /// 6. Records equity curve and trade history
    ///
    /// # Parameters
    ///
    /// - `events_dir`: Directory containing JSONL event files (one per hour)
    /// - `ir`: Compiled strategy IR (Intermediate Representation)
    /// - `config`: Simulation configuration (fees, initial capital, etc.)
    /// - `coin`: Coin symbol (e.g., "BTC", "ETH")
    /// - `start_ts`: Start timestamp in milliseconds (Unix epoch)
    /// - `end_ts`: End timestamp in milliseconds (Unix epoch)
    /// - `io_concurrency`: Optional concurrency limit for file I/O (defaults to min(CPU cores, 8))
    /// - `indicators_parallel`: Whether to update indicators in parallel (faster for multiple indicators)
    ///
    /// # Returns
    ///
    /// `SimResult` containing:
    /// - Trade history
    /// - Equity curve
    /// - Final equity and return metrics
    /// - Performance metrics (Sharpe, Sortino, drawdown, etc.)
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - Events directory cannot be read
    /// - Strategy compilation fails
    /// - Funding history cannot be fetched
    /// - No events found in the specified time range
    ///
    /// # Performance Considerations
    ///
    /// - Events are processed sequentially (order matters for accurate backtesting)
    /// - File loading is parallelized for better I/O performance
    /// - Indicator updates can be parallelized if multiple indicators exist
    /// - Strategy evaluation is throttled to price changes (0.01% threshold) to reduce unnecessary computation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::perps::PerpsEngine;
    /// use crate::ir::types::StrategyIr;
    /// use crate::orders::types::SimConfig;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let ir_str = std::fs::read_to_string("strategy.json")?;
    /// let strategy_ir: StrategyIr = serde_json::from_str(&ir_str)?;
    ///
    /// let config = SimConfig {
    ///     initial_capital: 10000.0,
    ///     maker_fee_bps: -1,
    ///     taker_fee_bps: 10,
    ///     slippage_bps: 5,
    /// };
    ///
    /// let result = PerpsEngine::run(
    ///     "data/events/BTC",
    ///     &strategy_ir,
    ///     &config,
    ///     "BTC",
    ///     1694858400000,  // 2023-09-16 09:00:00 UTC
    ///     1694865600000,  // 2023-09-16 11:00:00 UTC
    ///     Some(8),        // Use 8 concurrent file readers
    ///     true,           // Parallel indicator updates
    /// ).await?;
    ///
    /// println!("Final equity: ${:.2}", result.final_equity);
    /// println!("Total return: {:.2}%", result.total_return_pct);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(
        events_dir: impl AsRef<Path>,
        ir: &StrategyIr,
        config: &SimConfig,
        coin: &str,
        start_ts: u64,
        end_ts: u64,
        io_concurrency: Option<usize>,
        indicators_parallel: bool,
    ) -> Result<SimResult> {
        let compiled = compile_strategy(ir)?;

        // Initialize indicators with pre-sized HashMap
        let num_indicators = compiled.indicators.len();
        let mut indicators: HashMap<String, Box<dyn IndicatorEvaluator>> =
            HashMap::with_capacity(num_indicators.max(8)); // Pre-size to avoid reallocations
        for ind in &compiled.indicators {
            let evaluator = create_indicator(&ind.indicator_type, &ind.params)
                .with_context(|| format!("Failed to create indicator: {}", ind.indicator_type))?;
            indicators.insert(ind.id.clone(), evaluator);
        }

        // Load all events from directory
        let events_dir = events_dir.as_ref();

        // Pre-allocate events Vec with estimated capacity
        // Estimate: ~10k events per hour file, ~168 files for 7 days = ~1.68M events
        // But we filter by timestamp, so use conservative estimate
        let mut all_events: Vec<(u64, L2Event)> = Vec::with_capacity(DEFAULT_EVENTS_CAPACITY);

        let entries = fs::read_dir(events_dir).with_context(|| {
            format!("Failed to read events directory: {}", events_dir.display())
        })?;

        // Collect file paths first, then process in parallel
        let mut jsonl_files = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                jsonl_files.push(path);
            }
        }

        // Process files in parallel with bounded concurrency
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

        // Sort by timestamp
        all_events.sort_by_key(|(ts, _)| *ts);

        if all_events.is_empty() {
            anyhow::bail!("No events found in range");
        }

        println!("Loaded {} events for backtest", all_events.len());
        if let Some((first_ts, _)) = all_events.first() {
            if let Some((last_ts, _)) = all_events.last() {
                let first_dt = chrono::DateTime::from_timestamp_millis(*first_ts as i64)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| first_ts.to_string());
                let last_dt = chrono::DateTime::from_timestamp_millis(*last_ts as i64)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| last_ts.to_string());
                println!("Event range: {} to {}", first_dt, last_dt);
            }
        }

        // Fetch funding schedule
        let funding = FundingSchedule::from_api(coin, start_ts, end_ts)
            .await
            .context("Failed to fetch funding history")?;

        // Initialize engine
        let mut engine = Self::new(funding, config);

        // Active orders with pre-allocated capacity
        let mut active_orders: Vec<Order> = Vec::with_capacity(DEFAULT_ORDERS_CAPACITY);
        let mut next_order_id = 1u64;
        let mut trades = Vec::with_capacity(DEFAULT_TRADES_CAPACITY);
        let mut equity_curve = Vec::with_capacity(DEFAULT_EQUITY_CURVE_CAPACITY);

        // Track last funding payment timestamp
        let mut last_funding_ts = 0u64;

        // Evaluate strategy (only if we have enough data)
        let max_lookback = compiled
            .indicators
            .iter()
            .map(|i| i.lookback)
            .max()
            .unwrap_or(0);

        // Performance optimization: Track last price to skip strategy evaluation when price hasn't changed
        let mut last_evaluated_price = 0.0;
        
        // Parse timeframe from strategy to track candle boundaries for percentage drop detection
        // This ensures we compare against the previous candle's close, not the previous tick
        let timeframe_ms = parse_timeframe_to_ms(&compiled.instrument.timeframe)
            .unwrap_or(60 * 1000); // Default to 1 minute if parsing fails
        let mut prev_candle_close: Option<f64> = None;
        let mut current_candle_start_ts: Option<u64> = None;
        
        // Cooldown mechanism: Prevent excessive trades by requiring minimum time between trades
        // Default: 15 minutes cooldown (900 seconds = 900,000 ms) if not specified
        let trade_cooldown_ms = config.trade_cooldown_ms.unwrap_or(15 * 60 * 1000);
        let mut last_trade_ts: Option<u64> = None;

        // Process events
        let total_events = all_events.len();
        let log_interval = (total_events / 100).max(1); // Log every 1% or every event if < 100
        let mut last_log_ts = 0u64;
        let log_time_interval_ms = EQUITY_RECORDING_INTERVAL_MS; // Log at least every minute

        // Pre-allocate synthetic candle (reused for each event)
        let coin_str = coin.to_string(); // Allocate once
        let interval_str = "tick".to_string(); // Allocate once
        let mut synthetic_candle = Candle {
            time_open: 0,
            time_close: 0,
            coin: coin_str.clone(),
            interval: interval_str.clone(),
            open: 0.0,
            high: 0.0,
            low: 0.0,
            close: 0.0,
            volume: 0.0,
            num_trades: 0,
        };

        for (event_idx, (ts_ms, event)) in all_events.iter().enumerate() {
            // Log progress periodically (only in debug builds or with verbose flag)
            #[cfg(debug_assertions)]
            {
                if event_idx % log_interval == 0 || *ts_ms - last_log_ts >= log_time_interval_ms {
                    let progress_pct = (event_idx as f64 / total_events as f64 * 100.0) as u32;
                    let ts_dt = chrono::DateTime::from_timestamp_millis(*ts_ms as i64)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| ts_ms.to_string());
                    println!(
                        "Progress: {}% ({}/{}) | Time: {} | Trades: {} | Equity: ${:.2}",
                        progress_pct,
                        event_idx,
                        total_events,
                        ts_dt,
                        trades.len(),
                        engine
                            .portfolio
                            .total_equity(coin, engine.book.mid_price().unwrap_or(0.0))
                    );
                    last_log_ts = *ts_ms;
                }
            }

            // In release builds, only log at major milestones (every 10%)
            #[cfg(not(debug_assertions))]
            {
                if event_idx % (total_events / 10).max(1) == 0 {
                    let progress_pct = (event_idx as f64 / total_events as f64 * 100.0) as u32;
                    println!(
                        "Progress: {}% ({}/{})",
                        progress_pct, event_idx, total_events
                    );
                }
            }
            // Update order book
            engine.book.apply_snapshot(&event.levels);

            // Get current mid price for strategy evaluation
            // Cache mid_price calculation to avoid repeated calls
            let price = match engine.book.mid_price() {
                Some(p) => p,
                None => continue, // Skip if no book
            };

            // Update synthetic candle in-place (avoid allocation)
            // Track candle boundaries based on strategy timeframe for percentage drop detection
            let candle_start_ts = (*ts_ms / timeframe_ms) * timeframe_ms;
            let is_new_candle = current_candle_start_ts.map(|ts| ts != candle_start_ts).unwrap_or(true);
            
            if is_new_candle {
                // New candle started: store previous candle's close BEFORE updating
                // This ensures we capture the final close price of the previous candle
                if current_candle_start_ts.is_some() && synthetic_candle.close > 0.0 {
                    // Only store if we had a previous candle (not the first candle ever)
                    prev_candle_close = Some(synthetic_candle.close);
                }
                current_candle_start_ts = Some(candle_start_ts);
                // Reset candle OHLC for new candle
                synthetic_candle.time_open = candle_start_ts;
                synthetic_candle.open = price;
                synthetic_candle.high = price;
                synthetic_candle.low = price;
            }
            
            synthetic_candle.time_close = *ts_ms;
            synthetic_candle.high = synthetic_candle.high.max(price);
            synthetic_candle.low = synthetic_candle.low.min(price);
            synthetic_candle.close = price;

            // Update indicators (always needed to maintain rolling windows)
            // Performance: Use Rayon for parallel updates if enabled, since indicators are independent
            // Note: Indicators need continuous updates to maintain their state correctly for accurate backtesting
            if indicators_parallel && indicators.len() > 1 {
                // For parallel updates, collect evaluators into Vec for parallel processing
                // Only use parallel processing if we have multiple indicators (overhead not worth it for single indicator)
                let mut evaluators: Vec<&mut Box<dyn IndicatorEvaluator>> =
                    indicators.values_mut().collect();
                let update_result: Result<()> = evaluators
                    .par_iter_mut()
                    .try_for_each(|evaluator| evaluator.update(&synthetic_candle));
                update_result?;
            } else {
                // Sequential updates (faster for single indicator or when parallel disabled)
                for evaluator in indicators.values_mut() {
                    evaluator.update(&synthetic_candle)?;
                }
            }

            // Check limit orders for fills and execute market orders
            // Use swap_remove pattern for better performance when removing
            let mut orders_to_remove = Vec::new();

            // Evaluate strategy if we have enough data
            // Performance optimization: Only evaluate on significant price changes to reduce unnecessary order creation
            let price_changed = (price - last_evaluated_price).abs()
                / last_evaluated_price.max(1.0)
                > PRICE_CHANGE_THRESHOLD;
            let should_evaluate =
                event_idx >= max_lookback && (price_changed || last_evaluated_price == 0.0);

            if should_evaluate {
                // Determine which graph to evaluate based on position
                let coin_str = coin.to_string();
                let position_size = engine.portfolio.get_position(&coin_str);
                let is_flat = position_size.abs() < 1e-10;
                
                if is_flat {
                    // Flat position: evaluate entry graph (subject to cooldown)
                    let can_trade = last_trade_ts
                        .map(|last_ts| *ts_ms >= last_ts + trade_cooldown_ms)
                        .unwrap_or(true); // Allow first trade
                    
                    if can_trade {
                        evaluate_graph_perps(
                            &compiled.entry_node,
                            &compiled.nodes,
                            &compiled,
                            &indicators,
                            &synthetic_candle,
                            prev_candle_close,
                            &mut active_orders,
                            &mut next_order_id,
                            &engine.portfolio,
                        )?;
                    }
                } else {
                    // In position: evaluate exit graph (bypass cooldown for exits)
                    if let (Some(exit_entry), Some(exit_nodes)) = 
                        (&compiled.exit_entry_node, &compiled.exit_nodes) {
                        evaluate_graph_perps(
                            exit_entry,
                            exit_nodes,
                            &compiled,
                            &indicators,
                            &synthetic_candle,
                            prev_candle_close,
                            &mut active_orders,
                            &mut next_order_id,
                            &engine.portfolio,
                        )?;
                    }
                }
                last_evaluated_price = price;
            }

            // Execute market orders immediately (check both newly created and existing market orders)
            // Market orders should execute as soon as possible, regardless of strategy evaluation
            let mut market_orders_to_execute = Vec::new();
            let mut market_order_indices = Vec::new();

            for (idx, order) in active_orders.iter().enumerate() {
                if matches!(order.action, Action::Market { .. }) {
                    market_orders_to_execute.push(order.clone());
                    market_order_indices.push(idx);
                }
            }

            // Execute market orders and remove them from active_orders
            // Note: Market orders execute immediately against the book. The book state is not
            // modified because we're simulating against fixed historical data (backtesting).
            for (order_idx, mut order) in market_orders_to_execute.into_iter().enumerate() {
                let original_idx = market_order_indices[order_idx];

                if let Some(fill_result) = PerpsExecution::execute_market(&mut order, &engine.book)
                {
                    // Only process if order actually filled (filled_sz > 0)
                    if fill_result.filled_sz > MIN_FILL_SIZE {
                        // Extract side from order action
                        let side = match extract_side_from_action(&order.action) {
                            Some(s) => s,
                            None => continue,
                        };

                        // Create and process trade
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
                        
                        // Update cooldown timestamp
                        last_trade_ts = Some(*ts_ms);

                        // Log partial fills for market orders
                        if fill_result.order_status == OrderStatus::PartiallyFilled {
                            #[cfg(debug_assertions)]
                            {
                                println!(
                                    "  Warning: Market order {} partially filled: {:.6} / {:.6}",
                                    order.id,
                                    order.filled_sz,
                                    match &order.action {
                                        Action::Market { sz, .. } => *sz,
                                        _ => 0.0,
                                    }
                                );
                            }
                        }
                    }

                    // Mark for removal only if order was fully filled
                    // Market orders that partially fill are removed (they execute what's available)
                    if fill_result.order_status == OrderStatus::Filled {
                        orders_to_remove.push(original_idx);
                    } else {
                        // Partial fill - update the order in place and remove it
                        // (market orders should execute immediately, partial fills are removed)
                        if let Some(existing_order) = active_orders.get_mut(original_idx) {
                            existing_order.filled_sz = order.filled_sz;
                            existing_order.status = order.status.clone();
                        }
                        orders_to_remove.push(original_idx);
                    }
                } else {
                    // Market order couldn't execute (no liquidity) - log but don't remove
                    // This allows retry on next tick if liquidity becomes available
                    #[cfg(debug_assertions)]
                    {
                        println!(
                            "  Warning: Market order {} could not execute (insufficient liquidity) - will retry",
                            order.id
                        );
                    }
                    // Don't remove - allow retry on next event
                }
            }

            // Remove executed market orders from active_orders
            for &idx in orders_to_remove.iter().rev() {
                let last_idx = active_orders.len() - 1;
                if idx != last_idx {
                    active_orders.swap(idx, last_idx);
                }
                active_orders.pop();
            }
            orders_to_remove.clear();

            // Check limit orders for fills
            for (idx, order) in active_orders.iter_mut().enumerate() {
                if let Some(fill_result) = PerpsExecution::check_limit_fill(order, &engine.book) {
                    // Extract side from order action
                    let side = match extract_side_from_action(&order.action) {
                        Some(s) => s,
                        None => continue, // Skip non-limit orders (shouldn't happen here)
                    };

                    // Create and process trade
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
                    
                    // Update cooldown timestamp
                    last_trade_ts = Some(*ts_ms);

                    // Note: order.filled_sz and order.action.sz are already updated by check_limit_fill
                    // The execution module handles partial fill tracking internally

                    // Mark for removal only if order is fully filled
                    if fill_result.order_status == OrderStatus::Filled {
                        orders_to_remove.push(idx);
                    }
                    // Partial fills remain in active_orders for next check
                }
            }

            // Remove filled orders using swap_remove for O(1) removal
            // Process in reverse to maintain indices
            for &idx in orders_to_remove.iter().rev() {
                let last_idx = active_orders.len() - 1;
                if idx != last_idx {
                    active_orders.swap(idx, last_idx);
                }
                active_orders.pop();
            }

            // Apply funding payments (every 8 hours)
            if *ts_ms - last_funding_ts >= FUNDING_INTERVAL_MS {
                apply_funding_payment(&mut engine, coin, price, *ts_ms);
                last_funding_ts = *ts_ms;
            }

            // Record equity point periodically (every minute)
            if *ts_ms % EQUITY_RECORDING_INTERVAL_MS == 0 {
                record_equity_point(&mut equity_curve, &engine.portfolio, coin, price, *ts_ms);
            }
        }

        // Calculate final metrics
        let final_price = engine.book.mid_price().unwrap_or(0.0);
        let final_equity = engine.portfolio.total_equity(coin, final_price);

        // Compute metrics (simplified - reuse existing metrics module)
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
            win_rate: 0.0, // TODO: Calculate from trades
            avg_win: 0.0,
            avg_loss: 0.0,
            max_drawdown: 0.0,
            max_drawdown_pct: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
        })
    }
}

/// Evaluate a graph (entry or exit) for perps
fn evaluate_graph_perps(
    entry_node: &str,
    nodes: &std::collections::HashMap<String, crate::ir::types::Node>,
    compiled: &crate::ir::compile::CompiledStrategy,
    indicators: &HashMap<String, Box<dyn IndicatorEvaluator>>,
    candle: &Candle,
    prev_candle_close: Option<f64>,
    active_orders: &mut Vec<Order>,
    next_order_id: &mut u64,
    portfolio: &Portfolio,
) -> Result<()> {
    use crate::ir::types::Node;

    let mut current_node = entry_node.to_string();

    loop {
        let node = nodes
            .get(&current_node)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", current_node))?;

        match node {
            Node::Condition {
                expr,
                true_branch,
                false_branch,
            } => {
                let result = evaluate_expr_perps(expr, indicators, candle, prev_candle_close)?;
                current_node = if result {
                    true_branch.clone()
                } else {
                    false_branch.clone()
                };
            }
            Node::Action { action, next, .. } => {
                if let Some(order) =
                    create_order_from_action_perps(action, candle, *next_order_id, portfolio)?
                {
                    // Check for duplicate orders to prevent excessive order creation
                    // Performance: Early exit optimization - check most common case first (limit orders)
                    // An order is considered duplicate if it has the same action type, side, and similar size/price
                    let is_duplicate = active_orders.iter().any(|existing_order| {
                        match (&existing_order.action, &order.action) {
                            // Limit orders are most common - check first
                            (
                                Action::Limit {
                                    side: s1,
                                    px: px1,
                                    sz: sz1,
                                    ..
                                },
                                Action::Limit {
                                    side: s2,
                                    px: px2,
                                    sz: sz2,
                                    ..
                                },
                            ) => s1 == s2 && (px1 - px2).abs() < 1e-2 && (sz1 - sz2).abs() < 1e-6,
                            (
                                Action::Market { side: s1, sz: sz1 },
                                Action::Market { side: s2, sz: sz2 },
                            ) => s1 == s2 && (sz1 - sz2).abs() < 1e-6,
                            _ => false,
                        }
                    });

                    if !is_duplicate {
                        active_orders.push(order);
                        *next_order_id += 1;
                    }
                }
                current_node = next.clone();
            }
            Node::Terminal => {
                break;
            }
        }
    }

    Ok(())
}

/// Process a trade fill: calculate fee, create trade, execute, and log
fn process_trade_fill(
    fill_result: &crate::perps::execution::FillResult,
    order: &Order,
    timestamp: u64,
    coin: &str,
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
        symbol: coin_str.to_string(), // Still need to clone for Trade struct ownership
        side: side_to_string(side).to_string(), // Allocate once for trade
        size: fill_result.filled_sz,
        price: fill_result.fill_price,
        fee,
        order_id: order.id,
    };

    // Log trade execution (only in debug builds)
    #[cfg(debug_assertions)]
    {
        let trade_num = trades.len() + 1;
        let should_log = trade_num % 10 == 0 || trade_num <= 5;
        if should_log {
            println!(
                "  Trade #{}: {} {} @ ${:.2} | Fee: ${:.4} | Equity: ${:.2}",
                trade_num,
                trade.side,
                trade.size,
                trade.price,
                trade.fee,
                portfolio.total_equity(coin, fill_result.fill_price)
            );
        }
    }

    portfolio.execute_trade(&trade, fill_result.fill_price);
    trades.push(trade);
}

/// Applies funding payment for a position.
///
/// Funding payments are applied every 8 hours. Long positions pay funding,
/// short positions receive funding.
///
/// # Parameters
///
/// - `engine`: Engine containing portfolio and funding schedule
/// - `coin`: Coin symbol
/// - `price`: Current price for notional calculation
/// - `ts_ms`: Timestamp for funding rate lookup
fn apply_funding_payment(engine: &mut PerpsEngine, coin: &str, price: f64, ts_ms: u64) {
    if let Some(position) = engine.portfolio.positions.get(coin) {
        if position.size.abs() > MIN_FILL_SIZE {
            let notional = position.size.abs() * price;
            let funding_payment = engine.funding.calculate_payment(notional, ts_ms);

            // Funding is paid by longs, received by shorts
            if position.size > 0.0 {
                engine.portfolio.cash -= funding_payment;
            } else {
                engine.portfolio.cash += funding_payment;
            }
        }
    }
}

/// Records an equity point in the equity curve.
///
/// Equity points are recorded periodically (every minute) to track portfolio
/// value over time for performance analysis.
///
/// # Parameters
///
/// - `equity_curve`: Vector to append equity point to
/// - `portfolio`: Portfolio to calculate equity from
/// - `coin`: Coin symbol
/// - `price`: Current price
/// - `timestamp`: Timestamp for the equity point
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

fn evaluate_expr_perps(
    expr: &crate::ir::types::Expr,
    indicators: &HashMap<String, Box<dyn IndicatorEvaluator>>,
    candle: &Candle,
    prev_candle_close: Option<f64>,
) -> Result<bool> {
    use crate::ir::types::{ComparisonOp, ExprValue};

    // Diagnostic: Log expression structure (first time only)
    static STRUCTURE_LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !STRUCTURE_LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        println!("  [STRATEGY DEBUG] Expression structure - LHS: {:?}, RHS: {:?}", expr.lhs, expr.rhs);
    }

    // Special handling for percentage drop detection:
    // If RHS is a small constant (like 2.0) and LHS is close price,
    // interpret as "price dropped by 2%" by comparing to previous candle's close
    // This is a workaround for the IR format limitation
    let result = match (&expr.lhs, &expr.rhs) {
        (ExprValue::Series { series: s }, ExprValue::Const { r#const: val })
            if s == "close" && *val > 0.0 && *val < 100.0 =>
        {
            // Check if this looks like a percentage drop request
            // If RHS is between 0-100, treat as percentage drop
            // Use previous candle's close as reference (more intuitive than SMA)
            let reference = prev_candle_close.unwrap_or(candle.close);
            
            // Check if price dropped by the specified percentage
            let drop_pct = *val / 100.0; // Convert 2.0 to 0.02
            let dropped = check_percentage_drop(candle.close, reference, drop_pct);
            
            // Diagnostic logging
            static EVAL_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let count = EVAL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // Log first 10, then every 1000th evaluation to see when prev_candle_close gets set
            if count < 10 || (count > 0 && count % 1000 == 0) {
                let ref_source = if prev_candle_close.is_some() { "prev_candle" } else { "current_candle" };
                println!(
                    "  [STRATEGY DEBUG] Percentage drop check #{}: close=${:.2}, ref=${:.2} ({}), drop_pct={:.2}%, dropped={}",
                    count + 1,
                    candle.close,
                    reference,
                    ref_source,
                    drop_pct * 100.0,
                    dropped
                );
            }
            
            dropped
        }
        _ => {
            // Standard comparison logic
            let lhs_val = get_expr_value_perps(&expr.lhs, indicators, candle)?;
            let rhs_val = get_expr_value_perps(&expr.rhs, indicators, candle)?;
            
            let result = match expr.op {
                ComparisonOp::Lt => lhs_val < rhs_val,
                ComparisonOp::Lte => lhs_val <= rhs_val,
                ComparisonOp::Eq => (lhs_val - rhs_val).abs() < 1e-10,
                ComparisonOp::Ne => (lhs_val - rhs_val).abs() >= 1e-10,
                ComparisonOp::Gte => lhs_val >= rhs_val,
                ComparisonOp::Gt => lhs_val > rhs_val,
                ComparisonOp::CrossesAbove => lhs_val > rhs_val, // Simplified
                ComparisonOp::CrossesBelow => lhs_val < rhs_val, // Simplified
            };
            
            // Diagnostic logging: Show condition evaluation (first 10 times)
            static EVAL_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let count = EVAL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count < 10 {
                println!(
                    "  [STRATEGY DEBUG] Standard comparison #{}: {} {:#?} {} => {}",
                    count + 1,
                    lhs_val,
                    expr.op,
                    rhs_val,
                    result
                );
            }
            
            result
        }
    };

    Ok(result)
}

fn get_expr_value_perps(
    expr_val: &crate::ir::types::ExprValue,
    indicators: &HashMap<String, Box<dyn IndicatorEvaluator>>,
    candle: &Candle,
) -> Result<f64> {
    use crate::ir::types::ExprValue;

    match expr_val {
        ExprValue::Ref { r#ref: ref_path } => {
            // Optimize: find dot position without allocating Vec
            if let Some(dot_pos) = ref_path.find('.') {
                let ind_id = &ref_path[..dot_pos];
                let output = &ref_path[dot_pos + 1..];

                let evaluator = indicators
                    .get(ind_id)
                    .ok_or_else(|| anyhow::anyhow!("Indicator not found: {}", ind_id))?;
                evaluator.value(output)
            } else {
                anyhow::bail!("Invalid indicator reference: {}", ref_path);
            }
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

/// Helper function to check if price dropped by a percentage from a reference value
/// This enables percentage-based comparisons in strategies
fn check_percentage_drop(current: f64, reference: f64, drop_pct: f64) -> bool {
    if reference <= 0.0 {
        return false;
    }
    let pct_change = (current - reference) / reference;
    // pct_change is negative when price drops
    // We want: pct_change <= -drop_pct (e.g., -0.02 for 2% drop)
    // Which means: current <= reference * (1 - drop_pct)
    let result = pct_change <= -drop_pct;
    
    // More detailed logging for debugging
    static DETAILED_LOG_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let log_count = DETAILED_LOG_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if log_count < 5 {
        let actual_drop_pct = -pct_change * 100.0; // Convert to positive percentage
        println!(
            "    [DETAILED] current=${:.2}, ref=${:.2}, pct_change={:.4}%, required_drop={:.2}%, actual_drop={:.2}%, triggered={}",
            current,
            reference,
            pct_change * 100.0,
            drop_pct * 100.0,
            actual_drop_pct,
            result
        );
    }
    
    result
}

/// Parse timeframe string (e.g., "1h", "15m", "1d") to milliseconds
fn parse_timeframe_to_ms(timeframe: &str) -> Option<u64> {
    let s = timeframe.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    
    // Extract number and unit
    let (num_str, unit) = if let Some(pos) = s.chars().position(|c| c.is_alphabetic()) {
        (&s[..pos], &s[pos..])
    } else {
        return None;
    };
    
    let num: u64 = num_str.parse().ok()?;
    
    match unit {
        "m" | "min" | "minute" | "minutes" => Some(num * 60 * 1000),
        "h" | "hour" | "hours" => Some(num * 60 * 60 * 1000),
        "d" | "day" | "days" => Some(num * 24 * 60 * 60 * 1000),
        "s" | "sec" | "second" | "seconds" => Some(num * 1000),
        _ => None,
    }
}

fn create_order_from_action_perps(
    action: &crate::ir::types::ActionSpec,
    candle: &Candle,
    order_id: u64,
    portfolio: &Portfolio,
) -> Result<Option<Order>> {
    use crate::ir::types::ActionType;
    use crate::orders::types::{Action, Order, OrderStatus, Side, Tif};

    let side = match action.kind {
        ActionType::Buy => Side::Buy,
        ActionType::Sell => Side::Sell,
        ActionType::Close => {
            let pos_size = portfolio.get_position(&action.symbol);
            if pos_size.abs() < 1e-10 {
                return Ok(None);
            }
            if pos_size > 0.0 {
                Side::Sell
            } else {
                Side::Buy
            }
        }
        ActionType::Alert => return Ok(None),
    };

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

    // Validate inputs
    if candle.close <= 0.0 {
        anyhow::bail!("Invalid candle price: {}", candle.close);
    }
    if sizing_value < 0.0 {
        anyhow::bail!("Invalid sizing value: {}", sizing_value);
    }

    let sz = match sizing_mode {
        "cash" => {
            if sizing_value == 0.0 {
                return Ok(None); // No order if cash is zero
            }
            sizing_value / candle.close
        }
        "qty" => {
            if sizing_value == 0.0 {
                return Ok(None); // No order if quantity is zero
            }
            sizing_value
        }
        "pct" => {
            if sizing_value == 0.0 {
                return Ok(None); // No order if percentage is zero
            }
            let equity = portfolio.total_equity(&action.symbol, candle.close);
            (equity * sizing_value / 100.0) / candle.close
        }
        _ => {
            if sizing_value == 0.0 {
                return Ok(None);
            }
            sizing_value / candle.close
        }
    };

    // Final validation
    if sz <= 0.0 || sz.is_nan() || sz.is_infinite() {
        return Ok(None); // Invalid order size
    }

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
            let limit_px = if limit_px_raw == 0.0 {
                candle.close
            } else {
                limit_px_raw
            };

            // Validate limit price
            if limit_px <= 0.0 || limit_px.is_nan() || limit_px.is_infinite() {
                return Ok(None); // Invalid limit price
            }

            let tif_str = action
                .order
                .get("tif")
                .and_then(|v| v.as_str())
                .unwrap_or("GTC");
            let tif = match tif_str {
                "GTC" => Tif::Gtc,
                "IOC" => Tif::Ioc,
                "ALO" => Tif::Alo,
                _ => Tif::Gtc,
            };
            let post_only = tif == Tif::Alo;
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
        _ => return Ok(None),
    };

    Ok(Some(Order {
        id: order_id,
        action: order_action,
        created_at: candle.time_open,
        filled_sz: 0.0,
        status: OrderStatus::Pending,
    }))
}
