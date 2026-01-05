use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub time_open: u64,
    pub time_close: u64,
    pub coin: String,
    pub interval: String,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
    pub num_trades: i64,
}

impl Candle {
    pub fn from_sdk_candle(sdk: &hyperliquid_rust_sdk::CandlesSnapshotResponse) -> Self {
        Self {
            time_open: sdk.time_open,
            time_close: sdk.time_close,
            coin: sdk.coin.clone(),
            interval: sdk.candle_interval.clone(),
            open: sdk.open.parse().unwrap_or(0.0),
            close: sdk.close.parse().unwrap_or(0.0),
            high: sdk.high.parse().unwrap_or(0.0),
            low: sdk.low.parse().unwrap_or(0.0),
            volume: sdk.vlm.parse().unwrap_or(0.0),
            num_trades: sdk.num_trades as i64,
        }
    }
}

