use crate::orderbook::OrderBook;
use crate::orders::types::{Action, Order, OrderStatus, Side};

/// Execution engine for perpetual futures orders.
///
/// Handles order execution logic against the order book, including:
/// - Market order execution (immediate fills)
/// - Limit order fill detection
/// - Partial fill tracking
/// - Order status management
pub struct PerpsExecution;

impl PerpsExecution {
    /// Executes a market order against the current order book.
    ///
    /// Market orders execute immediately by sweeping the book. They fill as much as possible
    /// at the best available prices, potentially across multiple levels.
    ///
    /// # Parameters
    ///
    /// - `order`: Market order to execute (mutated in place - `filled_sz` and `status` updated)
    /// - `book`: Current order book state
    ///
    /// # Returns
    ///
    /// - `Some(FillResult)` if order executed (fully or partially)
    /// - `None` if no liquidity available
    ///
    /// # Behavior
    ///
    /// - Sweeps the book to fill as much as possible
    /// - Updates `order.filled_sz` and `order.status` in place
    /// - Handles partial fills correctly (updates remaining size)
    /// - Market orders are always taker (not maker)
    ///
    /// # Edge Cases
    ///
    /// - Returns `None` if order already fully filled
    /// - Returns `None` if no liquidity available
    /// - Handles partial fills when book depth is insufficient
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::perps::execution::PerpsExecution;
    /// use crate::orders::types::{Order, Action, Side, OrderStatus};
    /// use crate::orderbook::OrderBook;
    ///
    /// # fn example(mut order: Order, book: OrderBook) {
    /// let mut order = Order {
    ///     id: 1,
    ///     action: Action::Market { side: Side::Buy, sz: 1.0 },
    ///     created_at: 1000,
    ///     filled_sz: 0.0,
    ///     status: OrderStatus::Pending,
    /// };
    ///
    /// if let Some(fill) = PerpsExecution::execute_market(&mut order, &book) {
    ///     println!("Filled: {} @ ${:.2}", fill.filled_sz, fill.fill_price);
    ///     if fill.order_status == OrderStatus::PartiallyFilled {
    ///         println!("Partial fill - remaining: {:.6}",
    ///                  order.action.size() - fill.filled_sz);
    ///     }
    /// } else {
    ///     println!("No liquidity available");
    /// }
    /// # }
    /// ```
    pub fn execute_market(order: &mut Order, book: &OrderBook) -> Option<FillResult> {
        match &order.action {
            Action::Market { side, sz } => {
                // Calculate remaining size to fill (accounting for any previous partial fills)
                let remaining_sz = *sz - order.filled_sz;
                if remaining_sz <= 1e-10 {
                    return None; // Already fully filled
                }

                let (filled_sz, fill_price, is_maker) = match side {
                    Side::Buy => book.sweep_market_buy(remaining_sz)?,
                    Side::Sell => book.sweep_market_sell(remaining_sz)?,
                };

                // Update order filled size
                order.filled_sz += filled_sz;

                // Determine order status based on fill
                let order_status = if order.filled_sz >= *sz - 1e-10 {
                    OrderStatus::Filled
                } else {
                    OrderStatus::PartiallyFilled
                };
                order.status = order_status.clone();

                Some(FillResult {
                    filled_sz,
                    fill_price,
                    is_maker,
                    order_status,
                })
            }
            _ => None,
        }
    }

    /// Check if a limit order can be placed (not crossing)
    pub fn can_place_limit(order: &Order, book: &OrderBook, post_only: bool) -> bool {
        match &order.action {
            Action::Limit {
                side,
                px,
                sz: _,
                post_only: order_post_only,
                ..
            } => {
                let would_cross = match side {
                    Side::Buy => book.would_cross_buy(*px),
                    Side::Sell => book.would_cross_sell(*px),
                };

                if (post_only || *order_post_only) && would_cross {
                    return false; // Reject post-only if crossing
                }

                true
            }
            _ => false,
        }
    }

    /// Checks if a limit order should fill based on current book state.
    ///
    /// Limit orders fill when the limit price crosses the best bid/ask and there's
    /// sufficient liquidity. This function verifies both conditions and handles partial fills.
    ///
    /// # Parameters
    ///
    /// - `order`: Limit order to check (mutated in place - `filled_sz`, `status`, and `action.sz` updated)
    /// - `book`: Current order book state
    ///
    /// # Returns
    ///
    /// - `Some(FillResult)` if order can fill (fully or partially)
    /// - `None` if order cannot fill (price not crossed or no liquidity)
    ///
    /// # Behavior
    ///
    /// - Checks if limit price crosses best bid/ask
    /// - Verifies sufficient liquidity at fill price
    /// - Updates order filled size and remaining size in place
    /// - Handles partial fills correctly
    /// - Limit orders that fill are maker orders
    ///
    /// # Edge Cases
    ///
    /// - Returns `None` if order already fully filled
    /// - Returns `None` if limit price doesn't cross book
    /// - Returns `None` if insufficient liquidity
    /// - Handles partial fills when liquidity < order size
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::perps::execution::PerpsExecution;
    /// use crate::orders::types::{Order, Action, Side, OrderStatus, Tif};
    /// use crate::orderbook::OrderBook;
    ///
    /// # fn example(mut order: Order, book: OrderBook) {
    /// let mut order = Order {
    ///     id: 2,
    ///     action: Action::Limit {
    ///         side: Side::Buy,
    ///         px: 50000.0,
    ///         sz: 0.5,
    ///         tif: Tif::Gtc,
    ///         post_only: false,
    ///         reduce_only: false,
    ///     },
    ///     created_at: 1000,
    ///     filled_sz: 0.0,
    ///     status: OrderStatus::Pending,
    /// };
    ///
    /// // Check on each event
    /// if let Some(fill) = PerpsExecution::check_limit_fill(&mut order, &book) {
    ///     println!("Limit order filled: {} @ ${:.2}", fill.filled_sz, fill.fill_price);
    /// }
    /// # }
    /// ```
    pub fn check_limit_fill(order: &mut Order, book: &OrderBook) -> Option<FillResult> {
        match &order.action {
            Action::Limit { side, px, sz, .. } => {
                // Calculate remaining size to fill (accounting for any previous partial fills)
                let remaining_sz = *sz - order.filled_sz;
                if remaining_sz <= 1e-10 {
                    return None; // Already fully filled
                }

                // Get best price and verify it crosses our limit price
                let (best_price, best_size) = match side {
                    Side::Buy => {
                        let ask = book.best_ask()?;
                        if ask.0 <= *px {
                            Some((ask.0, ask.1))
                        } else {
                            None
                        }
                    }
                    Side::Sell => {
                        let bid = book.best_bid()?;
                        if bid.0 >= *px {
                            Some((bid.0, bid.1))
                        } else {
                            None
                        }
                    }
                }?;

                // Verify there's sufficient liquidity at the best price
                // Fill the minimum of remaining order size and available liquidity
                let filled_sz = remaining_sz.min(best_size);
                if filled_sz <= 1e-10 {
                    return None; // No liquidity available
                }

                let fill_price = best_price;
                let is_maker = true; // Limit orders that fill are maker

                // Update order filled size and reduce remaining order size
                order.filled_sz += filled_sz;

                // Reduce remaining order size in the action (need to match again to get mutable access)
                match &mut order.action {
                    Action::Limit { ref mut sz, .. } => {
                        *sz -= filled_sz;
                        if *sz < 1e-10 {
                            *sz = 0.0;
                        }
                    }
                    _ => {}
                }

                // Determine order status based on fill
                // Get original size for comparison
                let original_sz = match &order.action {
                    Action::Limit { sz, .. } => *sz + order.filled_sz, // Reconstruct original size
                    _ => 0.0,
                };
                let order_status = if order.filled_sz >= original_sz - 1e-10 {
                    OrderStatus::Filled
                } else {
                    OrderStatus::PartiallyFilled
                };
                order.status = order_status.clone();

                Some(FillResult {
                    filled_sz,
                    fill_price,
                    is_maker,
                    order_status,
                })
            }
            _ => None,
        }
    }
}

/// Result of an order execution attempt.
///
/// Contains information about how much of an order was filled, at what price,
/// and the resulting order status.
///
/// # Fields
///
/// - `filled_sz`: Size that was filled in this execution
/// - `fill_price`: Average fill price (for market orders, this is weighted average across levels)
/// - `is_maker`: Whether this fill was a maker order (true for limit orders, false for market orders)
/// - `order_status`: Resulting order status (`Filled` or `PartiallyFilled`)
#[derive(Debug, Clone)]
pub struct FillResult {
    pub filled_sz: f64,
    pub fill_price: f64,
    pub is_maker: bool,
    pub order_status: OrderStatus,
}
