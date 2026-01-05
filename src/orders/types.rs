use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tif {
    Gtc,  // Good Till Cancel
    Ioc,  // Immediate or Cancel
    Alo,  // Add Liquidity Only (Post Only)
}

#[derive(Debug, Clone)]
pub enum Action {
    Market {
        side: Side,
        sz: f64,
    },
    Limit {
        side: Side,
        px: f64,
        sz: f64,
        tif: Tif,
        post_only: bool,
        reduce_only: bool,
    },
    StopMarket {
        side: Side,
        trigger: f64,
        sz: f64,
    },
    StopLimit {
        side: Side,
        trigger: f64,
        px: f64,
        sz: f64,
        tif: Tif,
    },
    TakeMarket {
        side: Side,
        trigger: f64,
        sz: f64,
    },
    TakeLimit {
        side: Side,
        trigger: f64,
        px: f64,
        sz: f64,
        tif: Tif,
    },
    Scale {
        side: Side,
        from_px: f64,
        to_px: f64,
        steps: u32,
        total_sz: f64,
    },
    Twap {
        side: Side,
        total_sz: f64,
        duration_s: u64,
    },
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub action: Action,
    pub created_at: u64,
    pub filled_sz: f64,
    pub status: OrderStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Canceled,
    Triggered, // For stop/take orders that have been triggered but not filled
}

#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub size: f64, // Positive for long, negative for short
    pub entry_price: f64,
    pub tp_price: Option<f64>,
    pub sl_price: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct SimConfig {
    pub initial_capital: f64,
    pub maker_fee_bps: i16,
    pub taker_fee_bps: i16,
    pub slippage_bps: u16,
    /// Minimum time between trades in milliseconds (cooldown period)
    /// Prevents excessive trading when strategy triggers frequently
    /// Default: 15 minutes (900,000 ms)
    pub trade_cooldown_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub timestamp: u64,
    pub symbol: String,
    pub side: String,
    pub size: f64,
    pub price: f64,
    pub fee: f64,
    pub order_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp: u64,
    pub equity: f64,
    pub cash: f64,
    pub position_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimResult {
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<EquityPoint>,
    pub final_equity: f64,
    pub total_return: f64,
    pub total_return_pct: f64,
    pub num_trades: usize,
    pub win_rate: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub max_drawdown: f64,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
}

