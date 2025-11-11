use anyhow::Result;

/// Map timeframe string to Hyperliquid API interval string
pub fn map_timeframe_to_interval(timeframe: &str) -> Result<String> {
    match timeframe.to_lowercase().as_str() {
        "1m" => Ok("1m".to_string()),
        "5m" => Ok("5m".to_string()),
        "15m" => Ok("15m".to_string()),
        "1h" => Ok("1h".to_string()),
        "4h" => Ok("4h".to_string()),
        "1d" => Ok("1d".to_string()),
        "1w" => Ok("1w".to_string()),
        _ => anyhow::bail!("Unsupported timeframe: {}", timeframe),
    }
}

