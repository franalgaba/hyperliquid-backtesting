use anyhow::{Context, Result};
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};

use crate::data::types::Candle;
use crate::data::Cache;
use crate::util::map_timeframe_to_interval;

pub async fn load_candles(
    cache: &Cache,
    asset: &str,
    interval: &str,
    start_ts: u64,
    end_ts: u64,
) -> Result<Vec<Candle>> {
    // Try cache first
    let mut candles = cache.load_cached(asset, interval)?;

    // Filter by time range
    candles.retain(|c| c.time_open >= start_ts && c.time_open <= end_ts);

    // If cache is empty or incomplete, fetch from API
    if candles.is_empty() {
        candles = fetch_candles_from_api(asset, interval, start_ts, end_ts).await?;
        // Cache the fetched data
        cache
            .fetch_and_cache(asset, interval, start_ts, end_ts)
            .await?;
    }

    // Sort by time
    candles.sort_by_key(|c| c.time_open);

    Ok(candles)
}

pub async fn fetch_candles_from_api(
    asset: &str,
    interval: &str,
    start_ts: u64,
    end_ts: u64,
) -> Result<Vec<Candle>> {
    let client = InfoClient::new(None, Some(BaseUrl::Mainnet))
        .await
        .context("Failed to create InfoClient")?;

    let hl_interval = map_timeframe_to_interval(interval)?;
    let hl_interval_clone = hl_interval.clone();

    let sdk_candles = client
        .candles_snapshot(asset.to_string(), hl_interval, start_ts, end_ts)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch candles from API: asset={}, interval={}, start={}, end={}",
                asset, hl_interval_clone, start_ts, end_ts
            )
        })?;

    if sdk_candles.is_empty() {
        anyhow::bail!(
            "API returned no candles for {} {} in range {} to {}.\n\
            Note: Hyperliquid does not provide historical candle data for spot markets.\n\
            See https://hyperliquid.gitbook.io/hyperliquid-docs/historical-data\n\
            You may need to use cached data or generate test data.",
            asset,
            hl_interval_clone,
            start_ts,
            end_ts
        );
    }

    Ok(sdk_candles.iter().map(Candle::from_sdk_candle).collect())
}
