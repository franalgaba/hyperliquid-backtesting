use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

use crate::data::types::Candle;
use crate::ingest::l2_parser::{parse_l2_file, parse_l2_jsonl_file, L2Event};

fn interval_ms(interval: &str) -> Result<u64> {
    match interval.to_lowercase().as_str() {
        "1m" => Ok(60 * 1000),
        "5m" => Ok(5 * 60 * 1000),
        "15m" => Ok(15 * 60 * 1000),
        "1h" => Ok(60 * 60 * 1000),
        "4h" => Ok(4 * 60 * 60 * 1000),
        "1d" => Ok(24 * 60 * 60 * 1000),
        "1w" => Ok(7 * 24 * 60 * 60 * 1000),
        _ => anyhow::bail!("Unsupported interval: {}", interval),
    }
}

fn best_bid_ask(event: &L2Event) -> Option<(f64, f64)> {
    if event.levels.len() < 2 {
        return None;
    }
    let bids = &event.levels[0];
    let asks = &event.levels[1];
    if bids.is_empty() || asks.is_empty() {
        return None;
    }

    let best_bid = bids.iter().map(|o| o.px).fold(f64::MIN, f64::max);
    let best_ask = asks.iter().map(|o| o.px).fold(f64::MAX, f64::min);

    if best_bid <= 0.0 || best_ask <= 0.0 {
        return None;
    }

    Some((best_bid, best_ask))
}

pub async fn build_ohlc_from_events(
    events_dir: impl AsRef<Path>,
    coin: &str,
    interval: &str,
    fill_gaps: bool,
) -> Result<(Vec<Candle>, usize)> {
    let interval_ms = interval_ms(interval)?;
    let events_dir = events_dir.as_ref();

    let mut files: Vec<_> = std::fs::read_dir(events_dir)
        .with_context(|| format!("Failed to read events dir: {}", events_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            matches!(
                e.path().extension().and_then(|s| s.to_str()),
                Some("jsonl") | Some("lz4")
            )
        })
        .collect();

    files.sort_by_key(|e| e.path());

    println!(
        "Building OHLC from {} files in {}",
        files.len(),
        events_dir.display()
    );

    let mut candles: Vec<Candle> = Vec::new();
    let mut current_start: Option<u64> = None;
    let mut current: Option<Candle> = None;
    let mut file_idx = 0usize;
    let mut total_events = 0usize;

    let total_files = files.len();
    for entry in &files {
        file_idx += 1;
        if file_idx == 1 || file_idx % 50 == 0 {
            println!("Processing file {}/{}", file_idx, total_files);
        }
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let events = if ext == "lz4" {
            parse_l2_file(&path).await?
        } else {
            parse_l2_jsonl_file(&path).await?
        };
        total_events += events.len();
        for event in events {
            let (best_bid, best_ask) = match best_bid_ask(&event) {
                Some(v) => v,
                None => continue,
            };
            let mid = (best_bid + best_ask) / 2.0;
            let bucket_start = (event.ts_ms / interval_ms) * interval_ms;

            if current_start.is_none() || current_start != Some(bucket_start) {
                if let Some(candle) = current.take() {
                    candles.push(candle);
                }
                current_start = Some(bucket_start);
                current = Some(Candle {
                    time_open: bucket_start,
                    time_close: bucket_start + interval_ms - 1,
                    coin: coin.to_string(),
                    interval: interval.to_string(),
                    open: mid,
                    close: mid,
                    high: mid,
                    low: mid,
                    volume: 0.0,
                    num_trades: 0,
                });
            } else if let Some(ref mut candle) = current {
                candle.close = mid;
                if mid > candle.high {
                    candle.high = mid;
                }
                if mid < candle.low {
                    candle.low = mid;
                }
            }
        }
    }

    if let Some(candle) = current.take() {
        candles.push(candle);
    }

    println!("Parsed {} L2 events", total_events);

    if candles.is_empty() || !fill_gaps {
        return Ok((candles, 0));
    }

    candles.sort_by_key(|c| c.time_open);
    let mut map: BTreeMap<u64, Candle> = BTreeMap::new();
    for candle in candles {
        map.insert(candle.time_open, candle);
    }

    let first = *map.keys().next().unwrap();
    let last = *map.keys().last().unwrap();

    let mut filled = Vec::new();
    let mut filled_count = 0usize;
    let mut prev_close = map.get(&first).unwrap().close;

    let mut t = first;
    while t <= last {
        if let Some(candle) = map.get(&t) {
            prev_close = candle.close;
            filled.push(candle.clone());
        } else {
            filled.push(Candle {
                time_open: t,
                time_close: t + interval_ms - 1,
                coin: coin.to_string(),
                interval: interval.to_string(),
                open: prev_close,
                close: prev_close,
                high: prev_close,
                low: prev_close,
                volume: 0.0,
                num_trades: 0,
            });
            filled_count += 1;
        }
        t += interval_ms;
    }

    Ok((filled, filled_count))
}
