use crate::data::types::Candle;
use crate::fees::FeeCalculator;
use crate::orders::types::{Action, Order, OrderStatus, Side, Tif};
use crate::portfolio::Portfolio;

pub struct FillResult {
    pub filled_sz: f64,
    pub fill_price: f64,
    pub is_maker: bool,
    pub order_status: OrderStatus,
}

pub fn process_order_fill(
    order: &mut Order,
    candle: &Candle,
    _portfolio: &Portfolio,
    fee_calc: &FeeCalculator,
) -> Option<FillResult> {
    match &order.action {
        Action::Market { side, sz } => {
            let fill_price = if *side == Side::Buy {
                fee_calc.apply_slippage(candle.open, true)
            } else {
                fee_calc.apply_slippage(candle.open, false)
            };
            let filled_sz = *sz;
            let notional = filled_sz * fill_price;
            let _fee = fee_calc.calculate_fee(notional, false); // Market orders are always taker

            Some(FillResult {
                filled_sz,
                fill_price,
                is_maker: false,
                order_status: OrderStatus::Filled,
            })
        }
        Action::Limit {
            side,
            px,
            sz,
            tif,
            post_only,
            reduce_only: _,
        } => {
            // Check if price is touched
            let price_touched = match side {
                Side::Buy => candle.low <= *px,
                Side::Sell => candle.high >= *px,
            };

            if !price_touched {
                if *tif == Tif::Ioc {
                    return Some(FillResult {
                        filled_sz: 0.0,
                        fill_price: *px,
                        is_maker: false,
                        order_status: OrderStatus::Canceled,
                    });
                }
                return None; // GTC order, wait for next bar
            }

            // Check for post-only violation (would cross spread)
            if *post_only {
                let would_cross = match side {
                    Side::Buy => candle.open < *px || candle.low < *px,
                    Side::Sell => candle.open > *px || candle.high > *px,
                };
                if would_cross {
                    return Some(FillResult {
                        filled_sz: 0.0,
                        fill_price: *px,
                        is_maker: false,
                        order_status: OrderStatus::Canceled,
                    });
                }
            }

            // Determine fill price
            let fill_price = match side {
                Side::Buy => {
                    // Fill at limit price or better (lower)
                    if candle.open < *px {
                        candle.open // Gap down, fill at open
                    } else {
                        *px // Fill at limit
                    }
                }
                Side::Sell => {
                    // Fill at limit price or better (higher)
                    if candle.open > *px {
                        candle.open // Gap up, fill at open
                    } else {
                        *px // Fill at limit
                    }
                }
            };

            // Determine if maker (resting order) or taker (crossing)
            let is_maker = match side {
                Side::Buy => fill_price == *px && candle.open >= *px,
                Side::Sell => fill_price == *px && candle.open <= *px,
            };

            Some(FillResult {
                filled_sz: *sz,
                fill_price,
                is_maker,
                order_status: OrderStatus::Filled,
            })
        }
        Action::StopMarket { side, trigger, sz } => {
            // Check if trigger is crossed
            let triggered = match side {
                Side::Buy => candle.high >= *trigger, // Buy stop: price goes above trigger
                Side::Sell => candle.low <= *trigger, // Sell stop: price goes below trigger
            };

            if !triggered {
                return None;
            }

            // Fill at trigger or worse (slippage)
            let fill_price = match side {
                Side::Buy => {
                    let trigger_fill = candle.open.max(*trigger);
                    fee_calc.apply_slippage(trigger_fill, true)
                }
                Side::Sell => {
                    let trigger_fill = candle.open.min(*trigger);
                    fee_calc.apply_slippage(trigger_fill, false)
                }
            };

            Some(FillResult {
                filled_sz: *sz,
                fill_price,
                is_maker: false,
                order_status: OrderStatus::Filled,
            })
        }
        Action::StopLimit {
            side,
            trigger,
            px,
            sz,
            tif: _,
        } => {
            // First check if trigger is crossed
            let triggered = match side {
                Side::Buy => candle.high >= *trigger,
                Side::Sell => candle.low <= *trigger,
            };

            if !triggered {
                return None;
            }

            // Once triggered, becomes a limit order
            let price_touched = match side {
                Side::Buy => candle.low <= *px,
                Side::Sell => candle.high >= *px,
            };

            if !price_touched {
                return None; // Triggered but limit not touched
            }

            let fill_price = match side {
                Side::Buy => {
                    if candle.open < *px {
                        candle.open
                    } else {
                        *px
                    }
                }
                Side::Sell => {
                    if candle.open > *px {
                        candle.open
                    } else {
                        *px
                    }
                }
            };

            let is_maker = match side {
                Side::Buy => fill_price == *px && candle.open >= *px,
                Side::Sell => fill_price == *px && candle.open <= *px,
            };

            Some(FillResult {
                filled_sz: *sz,
                fill_price,
                is_maker,
                order_status: OrderStatus::Filled,
            })
        }
        Action::TakeMarket { side, trigger, sz } => {
            // Take profit: opposite direction from stop
            let triggered = match side {
                Side::Buy => candle.low <= *trigger, // Buy take: price goes below trigger (profit)
                Side::Sell => candle.high >= *trigger, // Sell take: price goes above trigger (profit)
            };

            if !triggered {
                return None;
            }

            let fill_price = match side {
                Side::Buy => {
                    let trigger_fill = candle.open.min(*trigger);
                    fee_calc.apply_slippage(trigger_fill, false) // Selling
                }
                Side::Sell => {
                    let trigger_fill = candle.open.max(*trigger);
                    fee_calc.apply_slippage(trigger_fill, true) // Buying
                }
            };

            Some(FillResult {
                filled_sz: *sz,
                fill_price,
                is_maker: false,
                order_status: OrderStatus::Filled,
            })
        }
        Action::TakeLimit {
            side,
            trigger,
            px,
            sz,
            tif: _,
        } => {
            let triggered = match side {
                Side::Buy => candle.low <= *trigger,
                Side::Sell => candle.high >= *trigger,
            };

            if !triggered {
                return None;
            }

            let price_touched = match side {
                Side::Buy => candle.high >= *px, // Selling at limit
                Side::Sell => candle.low <= *px, // Buying at limit
            };

            if !price_touched {
                return None;
            }

            let fill_price = match side {
                Side::Buy => {
                    if candle.open > *px {
                        candle.open
                    } else {
                        *px
                    }
                }
                Side::Sell => {
                    if candle.open < *px {
                        candle.open
                    } else {
                        *px
                    }
                }
            };

            let is_maker = match side {
                Side::Buy => fill_price == *px && candle.open <= *px,
                Side::Sell => fill_price == *px && candle.open >= *px,
            };

            Some(FillResult {
                filled_sz: *sz,
                fill_price,
                is_maker,
                order_status: OrderStatus::Filled,
            })
        }
        Action::Scale { .. } => {
            // Scale orders are expanded into multiple limit orders
            // For now, simplified: treat as single limit at midpoint
            // TODO: Implement full scale order logic
            None
        }
        Action::Twap { .. } => {
            // TWAP orders are complex and require sub-order simulation
            // For now, simplified: treat as market order
            // TODO: Implement full TWAP logic with 30s suborders
            None
        }
    }
}
