use crate::orders::types::{Action, Side, Trade};
use crate::perps::execution::FillResult;

/// Convert `Side` enum to its string representation.
///
/// # Parameters
///
/// - `side`: The side to convert (`Side::Buy` or `Side::Sell`)
///
/// # Returns
///
/// A static string slice: `"BUY"` or `"SELL"`
///
/// # Example
///
/// ```rust
/// use crate::perps::trade_utils::side_to_string;
/// use crate::orders::types::Side;
///
/// let side_str = side_to_string(Side::Buy);  // "BUY"
/// assert_eq!(side_str, "BUY");
/// ```
pub fn side_to_string(side: Side) -> &'static str {
    match side {
        Side::Buy => "BUY",
        Side::Sell => "SELL",
    }
}

/// Extracts the side from an order action.
///
/// # Parameters
///
/// - `action`: The order action (Market, Limit, etc.)
///
/// # Returns
///
/// - `Some(Side)` if the action has a side (Market or Limit orders)
/// - `None` for action types without sides (Stop, Take, etc.)
///
/// # Example
///
/// ```rust
/// use crate::perps::trade_utils::extract_side_from_action;
/// use crate::orders::types::{Action, Side};
///
/// let action = Action::Market { side: Side::Buy, sz: 1.0 };
/// let side = extract_side_from_action(&action);
/// assert_eq!(side, Some(Side::Buy));
/// ```
pub fn extract_side_from_action(action: &Action) -> Option<Side> {
    match action {
        Action::Market { side, .. } => Some(*side),
        Action::Limit { side, .. } => Some(*side),
        _ => None,
    }
}

/// Create a Trade from a fill result
pub fn create_trade_from_fill(
    fill_result: &FillResult,
    order_id: u64,
    timestamp: u64,
    symbol: &str,
    side: Side,
) -> Trade {
    Trade {
        timestamp,
        symbol: symbol.to_string(),
        side: side_to_string(side).to_string(),
        size: fill_result.filled_sz,
        price: fill_result.fill_price,
        fee: 0.0, // Fee will be calculated separately
        order_id,
    }
}

/// Calculate trade fee from notional and maker flag
pub fn calculate_trade_fee(notional: f64, is_maker: bool, fee_calc: &crate::fees::FeeCalculator) -> f64 {
    fee_calc.calculate_fee(notional, is_maker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::types::{Action, Side, Tif};

    #[test]
    fn test_side_to_string_buy() {
        assert_eq!(side_to_string(Side::Buy), "BUY");
    }

    #[test]
    fn test_side_to_string_sell() {
        assert_eq!(side_to_string(Side::Sell), "SELL");
    }

    #[test]
    fn test_extract_side_from_market_buy() {
        let action = Action::Market { side: Side::Buy, sz: 1.0 };
        assert_eq!(extract_side_from_action(&action), Some(Side::Buy));
    }

    #[test]
    fn test_extract_side_from_market_sell() {
        let action = Action::Market { side: Side::Sell, sz: 1.0 };
        assert_eq!(extract_side_from_action(&action), Some(Side::Sell));
    }

    #[test]
    fn test_extract_side_from_limit_buy() {
        let action = Action::Limit {
            side: Side::Buy,
            px: 50000.0,
            sz: 0.5,
            tif: Tif::Gtc,
            post_only: false,
            reduce_only: false,
        };
        assert_eq!(extract_side_from_action(&action), Some(Side::Buy));
    }

    #[test]
    fn test_extract_side_from_limit_sell() {
        let action = Action::Limit {
            side: Side::Sell,
            px: 50000.0,
            sz: 0.5,
            tif: Tif::Gtc,
            post_only: false,
            reduce_only: false,
        };
        assert_eq!(extract_side_from_action(&action), Some(Side::Sell));
    }

    #[test]
    fn test_extract_side_from_stop_market_returns_none() {
        let action = Action::StopMarket {
            side: Side::Buy,
            trigger: 50000.0,
            sz: 1.0,
        };
        assert_eq!(extract_side_from_action(&action), None);
    }

    #[test]
    fn test_create_trade_from_fill() {
        let fill_result = FillResult {
            filled_sz: 0.5,
            fill_price: 50000.0,
            is_maker: true,
            order_status: crate::orders::types::OrderStatus::Filled,
        };

        let trade = create_trade_from_fill(&fill_result, 123, 1000, "BTC", Side::Buy);

        assert_eq!(trade.order_id, 123);
        assert_eq!(trade.timestamp, 1000);
        assert_eq!(trade.symbol, "BTC");
        assert_eq!(trade.side, "BUY");
        assert_eq!(trade.size, 0.5);
        assert_eq!(trade.price, 50000.0);
        assert_eq!(trade.fee, 0.0); // Fee calculated separately
    }

    #[test]
    fn test_calculate_trade_fee_maker() {
        use crate::fees::FeeCalculator;
        let fee_calc = FeeCalculator::new(-1, 10, 5); // -1 bps maker, 10 bps taker
        let notional = 10000.0;
        let fee = calculate_trade_fee(notional, true, &fee_calc);
        // Maker fee: -1 bps = -0.01% = -1.0
        assert!((fee - (-1.0)).abs() < 0.01);
    }

    #[test]
    fn test_calculate_trade_fee_taker() {
        use crate::fees::FeeCalculator;
        let fee_calc = FeeCalculator::new(-1, 10, 5);
        let notional = 10000.0;
        let fee = calculate_trade_fee(notional, false, &fee_calc);
        // Taker fee: 10 bps = 0.1% = 10.0
        assert!((fee - 10.0).abs() < 0.01);
    }
}
