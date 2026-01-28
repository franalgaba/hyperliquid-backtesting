use crate::orders::types::{Position, Side, Trade};
use crate::fees::FeeCalculator;

#[derive(Debug, Clone)]
pub struct Portfolio {
    pub cash: f64,
    pub positions: std::collections::HashMap<String, Position>,
    pub fee_calc: FeeCalculator,
}

impl Portfolio {
    pub fn new(initial_capital: f64, fee_calc: FeeCalculator) -> Self {
        Self {
            cash: initial_capital,
            positions: std::collections::HashMap::new(),
            fee_calc,
        }
    }

    pub fn get_position(&self, symbol: &str) -> f64 {
        self.positions
            .get(symbol)
            .map(|p| p.size)
            .unwrap_or(0.0)
    }

    pub fn get_position_value(&self, symbol: &str, current_price: f64) -> f64 {
        self.positions
            .get(symbol)
            .map(|p| p.size * current_price)
            .unwrap_or(0.0)
    }

    pub fn total_equity(&self, symbol: &str, current_price: f64) -> f64 {
        self.cash + self.get_position_value(symbol, current_price)
    }

    pub fn execute_trade(&mut self, trade: &Trade, current_price: f64) {
        let position = self.positions.entry(trade.symbol.clone()).or_insert_with(|| Position {
            symbol: trade.symbol.clone(),
            size: 0.0,
            entry_price: current_price,
            tp_price: None,
            sl_price: None,
        });

        let side = if trade.side == "BUY" { Side::Buy } else { Side::Sell };
        let notional = trade.size * trade.price;
        let fee = trade.fee;

        match side {
            Side::Buy => {
                if position.size < 0.0 {
                    let short_abs = position.size.abs();
                    if trade.size < short_abs {
                        position.size += trade.size;
                    } else if (trade.size - short_abs).abs() < 1e-10 {
                        position.size = 0.0;
                        position.entry_price = trade.price;
                    } else {
                        let new_long = trade.size - short_abs;
                        position.size = new_long;
                        position.entry_price = trade.price;
                    }
                } else if position.size == 0.0 {
                    position.entry_price = trade.price;
                    position.size = trade.size;
                } else {
                    let total_cost = position.size * position.entry_price + notional;
                    position.size += trade.size;
                    position.entry_price = total_cost / position.size;
                }
                self.cash -= notional + fee;
            }
            Side::Sell => {
                if position.size > 0.0 {
                    if trade.size < position.size {
                        position.size -= trade.size;
                    } else if (trade.size - position.size).abs() < 1e-10 {
                        position.size = 0.0;
                    } else {
                        let new_short = trade.size - position.size;
                        position.size = -new_short;
                        position.entry_price = trade.price;
                    }
                } else if position.size == 0.0 {
                    position.entry_price = trade.price;
                    position.size = -trade.size;
                } else {
                    let total_proceeds = position.size.abs() * position.entry_price + notional;
                    let new_size = position.size.abs() + trade.size;
                    position.size = -new_size;
                    position.entry_price = total_proceeds / new_size;
                }

                if position.size.abs() < 1e-10 {
                    position.size = 0.0;
                }
                self.cash += notional - fee;
            }
        }
    }

    pub fn update_position_price(&mut self, _symbol: &str, _current_price: f64) {
        // Entry price doesn't change, but we track it for PnL calculation
        // Position size remains the same
        // This method is a placeholder for future position price tracking logic
    }
}

