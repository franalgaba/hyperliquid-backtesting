use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::data::types::Candle;
use crate::data::loader::fetch_candles_from_api;

pub struct Cache {
    base_dir: PathBuf,
}

impl Cache {
    pub fn new() -> Result<Self> {
        let base_dir = PathBuf::from("data/hyperliquid");
        fs::create_dir_all(&base_dir)
            .with_context(|| format!("Failed to create cache directory: {}", base_dir.display()))?;

        Ok(Self { base_dir })
    }

    pub fn cache_path(&self, asset: &str, interval: &str) -> PathBuf {
        self.base_dir.join(asset).join(format!("{interval}.csv"))
    }

    pub async fn fetch_and_cache(
        &self,
        asset: &str,
        interval: &str,
        start_ts: u64,
        end_ts: u64,
    ) -> Result<()> {
        let candles = fetch_candles_from_api(asset, interval, start_ts, end_ts).await?;

        let cache_path = self.cache_path(asset, interval);
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create cache directory: {}", parent.display()))?;
        }

        let mut wtr = csv::Writer::from_path(&cache_path)
            .with_context(|| format!("Failed to create CSV file: {}", cache_path.display()))?;

        wtr.write_record(&[
            "time_open",
            "time_close",
            "coin",
            "interval",
            "open",
            "close",
            "high",
            "low",
            "volume",
            "num_trades",
        ])?;

        for candle in &candles {
            wtr.write_record(&[
                candle.time_open.to_string(),
                candle.time_close.to_string(),
                candle.coin.clone(),
                candle.interval.clone(),
                candle.open.to_string(),
                candle.close.to_string(),
                candle.high.to_string(),
                candle.low.to_string(),
                candle.volume.to_string(),
                candle.num_trades.to_string(),
            ])?;
        }

        wtr.flush()?;
        Ok(())
    }

    pub fn load_cached(&self, asset: &str, interval: &str) -> Result<Vec<Candle>> {
        let cache_path = self.cache_path(asset, interval);
        if !cache_path.exists() {
            return Ok(vec![]);
        }

        let mut rdr = csv::Reader::from_path(&cache_path)
            .with_context(|| format!("Failed to read CSV file: {}", cache_path.display()))?;

        let mut candles = Vec::new();
        for result in rdr.deserialize() {
            let record: CandleRecord = result?;
            candles.push(Candle {
                time_open: record.time_open,
                time_close: record.time_close,
                coin: record.coin,
                interval: record.interval,
                open: record.open,
                close: record.close,
                high: record.high,
                low: record.low,
                volume: record.volume,
                num_trades: record.num_trades,
            });
        }

        Ok(candles)
    }
}

#[derive(serde::Deserialize)]
struct CandleRecord {
    time_open: u64,
    time_close: u64,
    coin: String,
    interval: String,
    open: f64,
    close: f64,
    high: f64,
    low: f64,
    volume: f64,
    num_trades: i64,
}

