use anyhow::Result;
use std::path::Path;
use crate::orders::types::SimResult;

pub fn write_results(result: &SimResult, out_path: &Path) -> Result<()> {
    // Write JSON
    let json_str = serde_json::to_string_pretty(result)?;
    std::fs::write(out_path, json_str)?;

    // Also write CSV files
    let base_path = out_path.parent().unwrap_or(Path::new("."));
    let base_name = out_path.file_stem().and_then(|s| s.to_str()).unwrap_or("results");

    // Write trades CSV
    let trades_path = base_path.join(format!("{}_trades.csv", base_name));
    let mut wtr = csv::Writer::from_path(&trades_path)?;
    wtr.write_record(&["timestamp", "symbol", "side", "size", "price", "fee", "order_id"])?;
    for trade in &result.trades {
        wtr.write_record(&[
            trade.timestamp.to_string(),
            trade.symbol.clone(),
            trade.side.clone(),
            trade.size.to_string(),
            trade.price.to_string(),
            trade.fee.to_string(),
            trade.order_id.to_string(),
        ])?;
    }
    wtr.flush()?;

    // Write equity curve CSV
    let equity_path = base_path.join(format!("{}_equity.csv", base_name));
    let mut wtr = csv::Writer::from_path(&equity_path)?;
    wtr.write_record(&["timestamp", "equity", "cash", "position_value"])?;
    for point in &result.equity_curve {
        wtr.write_record(&[
            point.timestamp.to_string(),
            point.equity.to_string(),
            point.cash.to_string(),
            point.position_value.to_string(),
        ])?;
    }
    wtr.flush()?;

    Ok(())
}

