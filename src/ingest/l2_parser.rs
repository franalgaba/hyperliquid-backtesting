use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader as TokioBufReader};
use lz4_flex::frame::FrameDecoder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLevel {
    pub px: f64,
    pub sz: f64,
    pub n: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Event {
    pub ts_ms: u64,
    pub levels: Vec<Vec<OrderLevel>>, // [bids, asks] or full snapshot
}

#[derive(Debug, Deserialize)]
struct RawL2Entry {
    time: String,
    raw: RawData,
}

#[derive(Debug, Deserialize)]
struct RawData {
    data: L2Data,
}

#[derive(Debug, Deserialize)]
struct L2Data {
    time: u64,
    levels: Vec<Vec<RawOrderLevel>>,
}

#[derive(Debug, Deserialize)]
struct RawOrderLevel {
    px: String,
    sz: String,
    n: u64,
}

/// Decompress LZ4 file and parse JSONL into L2Event stream
pub async fn parse_l2_file(file_path: impl AsRef<Path>) -> Result<Vec<L2Event>> {
    let file = File::open(file_path.as_ref())
        .await
        .with_context(|| format!("Failed to open file: {:?}", file_path.as_ref()))?;
    
    let mut reader = TokioBufReader::new(file);
    let mut compressed_data = Vec::new();
    reader.read_to_end(&mut compressed_data).await?;

    // Decompress LZ4 frame format
    let mut decoder = FrameDecoder::new(compressed_data.as_slice());
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)
        .context("Failed to decompress LZ4 frame data")?;

    // Parse JSONL
    let mut events = Vec::new();
    let reader = BufReader::new(decompressed.as_slice());
    
    for line in reader.lines() {
        let line = line.context("Failed to read line")?;
        if line.trim().is_empty() {
            continue;
        }

        let entry: RawL2Entry = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse JSON line: {}", line))?;

        // Convert raw format to our L2Event
        let levels: Result<Vec<Vec<OrderLevel>>, anyhow::Error> = entry.raw.data.levels
            .iter()
            .map(|level| {
                level.iter().map(|o| {
                    Ok(OrderLevel {
                        px: o.px.parse()
                            .with_context(|| format!("Invalid price: {}", o.px))?,
                        sz: o.sz.parse()
                            .with_context(|| format!("Invalid size: {}", o.sz))?,
                        n: o.n,
                    })
                }).collect::<Result<Vec<_>, _>>()
            })
            .collect();

        events.push(L2Event {
            ts_ms: entry.raw.data.time,
            levels: levels?,
        });
    }

    Ok(events)
}

/// Parse JSONL file directly (for already-decompressed events)
/// Optimized: stream parsing to reduce memory usage
pub async fn parse_l2_jsonl_file(file_path: impl AsRef<Path>) -> Result<Vec<L2Event>> {
    let file = File::open(file_path.as_ref())
        .await
        .with_context(|| format!("Failed to open JSONL file: {:?}", file_path.as_ref()))?;
    
    // Use buffered line-by-line reading instead of loading entire file
    let reader = TokioBufReader::new(file);
    let mut lines = reader.lines();
    let mut events = Vec::new();
    
    // Pre-allocate with estimated capacity (most files have similar event counts)
    events.reserve(10000); // Estimate ~10k events per hour
    
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        // Try parsing as simplified L2Event format first (from build-events)
        if let Ok(event) = serde_json::from_str::<L2Event>(&line) {
            events.push(event);
            continue;
        }

        // Fall back to raw S3 format
        let entry: RawL2Entry = serde_json::from_str(&line)
            .with_context(|| format!("Failed to parse JSON line"))?;

        let levels: Result<Vec<Vec<OrderLevel>>, anyhow::Error> = entry.raw.data.levels
            .iter()
            .map(|level| {
                level.iter().map(|o| {
                    Ok(OrderLevel {
                        px: o.px.parse()
                            .with_context(|| format!("Invalid price: {}", o.px))?,
                        sz: o.sz.parse()
                            .with_context(|| format!("Invalid size: {}", o.sz))?,
                        n: o.n,
                    })
                }).collect::<Result<Vec<_>, _>>()
            })
            .collect();

        events.push(L2Event {
            ts_ms: entry.raw.data.time,
            levels: levels?,
        });
    }
    
    Ok(events)
}

/// Parse JSONL string directly
/// Handles both formats:
/// 1. Raw S3 format: {"time":"...","raw":{"data":{"time":...,"levels":...}}}
/// 2. Simplified format: {"ts_ms":...,"levels":...} (from build-events)
pub fn parse_l2_jsonl(jsonl: &str) -> Result<Vec<L2Event>> {
    let mut events = Vec::new();
    
    for line in jsonl.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Try parsing as simplified L2Event format first (from build-events)
        if let Ok(event) = serde_json::from_str::<L2Event>(line) {
            events.push(event);
            continue;
        }

        // Fall back to raw S3 format
        let entry: RawL2Entry = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSON line: {}", line))?;

        let levels: Result<Vec<Vec<OrderLevel>>, anyhow::Error> = entry.raw.data.levels
            .iter()
            .map(|level| {
                level.iter().map(|o| {
                    Ok(OrderLevel {
                        px: o.px.parse()
                            .with_context(|| format!("Invalid price: {}", o.px))?,
                        sz: o.sz.parse()
                            .with_context(|| format!("Invalid size: {}", o.sz))?,
                        n: o.n,
                    })
                }).collect::<Result<Vec<_>, _>>()
            })
            .collect();

        events.push(L2Event {
            ts_ms: entry.raw.data.time,
            levels: levels?,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_l2_jsonl() {
        let jsonl = r#"{"time":"2023-09-16T09:00:00Z","raw":{"data":{"time":1694858400000,"levels":[[{"px":"25000","sz":"1.5","n":1},{"px":"24999","sz":"2.0","n":2}],[{"px":"25001","sz":"1.0","n":3},{"px":"25002","sz":"1.5","n":4}]]}}}
{"time":"2023-09-16T09:00:01Z","raw":{"data":{"time":1694858401000,"levels":[[{"px":"25000.5","sz":"1.2","n":1}],[{"px":"25001.5","sz":"0.8","n":3}]]}}}"#;

        let events = parse_l2_jsonl(jsonl).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].ts_ms, 1694858400000);
        assert_eq!(events[0].levels.len(), 2); // bids and asks
        assert_eq!(events[0].levels[0].len(), 2); // 2 bid levels
        assert_eq!(events[0].levels[1].len(), 2); // 2 ask levels
    }
}

