use crate::data::types::Candle;
use crate::orders::types::{EquityPoint, Trade};
use anyhow::{Context, Result};
use arrow::array::{Float64Array, Int64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Funding payment record for Parquet export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingPayment {
    pub timestamp: u64,
    pub coin: String,
    pub rate: f64,
    pub payment: f64,
    pub position_size: f64,
}

/// Export candles to Parquet format
pub fn export_candles_to_parquet(candles: &[Candle], path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Define schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("time_open", DataType::UInt64, false),
        Field::new("time_close", DataType::UInt64, false),
        Field::new("coin", DataType::Utf8, false),
        Field::new("interval", DataType::Utf8, false),
        Field::new("open", DataType::Float64, false),
        Field::new("high", DataType::Float64, false),
        Field::new("low", DataType::Float64, false),
        Field::new("close", DataType::Float64, false),
        Field::new("volume", DataType::Float64, false),
        Field::new("num_trades", DataType::Int64, false),
    ]));

    // Build arrays
    let time_open: UInt64Array = candles.iter().map(|c| c.time_open).collect();
    let time_close: UInt64Array = candles.iter().map(|c| c.time_close).collect();
    let coin: StringArray = candles.iter().map(|c| Some(c.coin.as_str())).collect();
    let interval: StringArray = candles.iter().map(|c| Some(c.interval.as_str())).collect();
    let open: Float64Array = candles.iter().map(|c| c.open).collect();
    let high: Float64Array = candles.iter().map(|c| c.high).collect();
    let low: Float64Array = candles.iter().map(|c| c.low).collect();
    let close: Float64Array = candles.iter().map(|c| c.close).collect();
    let volume: Float64Array = candles.iter().map(|c| c.volume).collect();
    let num_trades: Int64Array = candles.iter().map(|c| c.num_trades).collect();

    // Create record batch
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(time_open),
            Arc::new(time_close),
            Arc::new(coin),
            Arc::new(interval),
            Arc::new(open),
            Arc::new(high),
            Arc::new(low),
            Arc::new(close),
            Arc::new(volume),
            Arc::new(num_trades),
        ],
    )?;

    // Configure writer properties
    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();

    // Write to file
    let file = File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

/// Read candles from Parquet format
pub fn read_candles_from_parquet(path: impl AsRef<Path>) -> Result<Vec<Candle>> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    let path = path.as_ref();
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let reader = builder.build()?;

    let mut candles = Vec::new();

    for batch_result in reader {
        let batch = batch_result?;

        let time_open = batch
            .column(0)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .context("Failed to read time_open column")?;
        let time_close = batch
            .column(1)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .context("Failed to read time_close column")?;
        let coin = batch
            .column(2)
            .as_any()
            .downcast_ref::<StringArray>()
            .context("Failed to read coin column")?;
        let interval = batch
            .column(3)
            .as_any()
            .downcast_ref::<StringArray>()
            .context("Failed to read interval column")?;
        let open = batch
            .column(4)
            .as_any()
            .downcast_ref::<Float64Array>()
            .context("Failed to read open column")?;
        let high = batch
            .column(5)
            .as_any()
            .downcast_ref::<Float64Array>()
            .context("Failed to read high column")?;
        let low = batch
            .column(6)
            .as_any()
            .downcast_ref::<Float64Array>()
            .context("Failed to read low column")?;
        let close = batch
            .column(7)
            .as_any()
            .downcast_ref::<Float64Array>()
            .context("Failed to read close column")?;
        let volume = batch
            .column(8)
            .as_any()
            .downcast_ref::<Float64Array>()
            .context("Failed to read volume column")?;
        let num_trades = batch
            .column(9)
            .as_any()
            .downcast_ref::<Int64Array>()
            .context("Failed to read num_trades column")?;

        for i in 0..batch.num_rows() {
            candles.push(Candle {
                time_open: time_open.value(i),
                time_close: time_close.value(i),
                coin: coin.value(i).to_string(),
                interval: interval.value(i).to_string(),
                open: open.value(i),
                high: high.value(i),
                low: low.value(i),
                close: close.value(i),
                volume: volume.value(i),
                num_trades: num_trades.value(i),
            });
        }
    }

    Ok(candles)
}

/// Export trades to Parquet format
pub fn export_trades_to_parquet(trades: &[Trade], path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp", DataType::UInt64, false),
        Field::new("symbol", DataType::Utf8, false),
        Field::new("side", DataType::Utf8, false),
        Field::new("size", DataType::Float64, false),
        Field::new("price", DataType::Float64, false),
        Field::new("fee", DataType::Float64, false),
        Field::new("order_id", DataType::UInt64, false),
    ]));

    let timestamp: UInt64Array = trades.iter().map(|t| t.timestamp).collect();
    let symbol: StringArray = trades.iter().map(|t| Some(t.symbol.as_str())).collect();
    let side: StringArray = trades.iter().map(|t| Some(t.side.as_str())).collect();
    let size: Float64Array = trades.iter().map(|t| t.size).collect();
    let price: Float64Array = trades.iter().map(|t| t.price).collect();
    let fee: Float64Array = trades.iter().map(|t| t.fee).collect();
    let order_id: UInt64Array = trades.iter().map(|t| t.order_id).collect();

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(timestamp),
            Arc::new(symbol),
            Arc::new(side),
            Arc::new(size),
            Arc::new(price),
            Arc::new(fee),
            Arc::new(order_id),
        ],
    )?;

    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();

    let file = File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

/// Export equity curve to Parquet format
pub fn export_equity_to_parquet(equity_curve: &[EquityPoint], path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp", DataType::UInt64, false),
        Field::new("equity", DataType::Float64, false),
        Field::new("cash", DataType::Float64, false),
        Field::new("position_value", DataType::Float64, false),
    ]));

    let timestamp: UInt64Array = equity_curve.iter().map(|e| e.timestamp).collect();
    let equity: Float64Array = equity_curve.iter().map(|e| e.equity).collect();
    let cash: Float64Array = equity_curve.iter().map(|e| e.cash).collect();
    let position_value: Float64Array = equity_curve.iter().map(|e| e.position_value).collect();

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(timestamp),
            Arc::new(equity),
            Arc::new(cash),
            Arc::new(position_value),
        ],
    )?;

    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();

    let file = File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

/// Export funding payments to Parquet format
pub fn export_funding_to_parquet(payments: &[FundingPayment], path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp", DataType::UInt64, false),
        Field::new("coin", DataType::Utf8, false),
        Field::new("rate", DataType::Float64, false),
        Field::new("payment", DataType::Float64, false),
        Field::new("position_size", DataType::Float64, false),
    ]));

    let timestamp: UInt64Array = payments.iter().map(|p| p.timestamp).collect();
    let coin: StringArray = payments.iter().map(|p| Some(p.coin.as_str())).collect();
    let rate: Float64Array = payments.iter().map(|p| p.rate).collect();
    let payment: Float64Array = payments.iter().map(|p| p.payment).collect();
    let position_size: Float64Array = payments.iter().map(|p| p.position_size).collect();

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(timestamp),
            Arc::new(coin),
            Arc::new(rate),
            Arc::new(payment),
            Arc::new(position_size),
        ],
    )?;

    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();

    let file = File::create(path)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parquet_roundtrip() {
        let candles = vec![
            Candle {
                time_open: 1609459200000,
                time_close: 1609462800000,
                coin: "BTC".to_string(),
                interval: "1h".to_string(),
                open: 29000.0,
                high: 29500.0,
                low: 28800.0,
                close: 29300.0,
                volume: 1000.0,
                num_trades: 500,
            },
            Candle {
                time_open: 1609462800000,
                time_close: 1609466400000,
                coin: "BTC".to_string(),
                interval: "1h".to_string(),
                open: 29300.0,
                high: 29800.0,
                low: 29100.0,
                close: 29600.0,
                volume: 1200.0,
                num_trades: 600,
            },
        ];

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.parquet");

        export_candles_to_parquet(&candles, &path).unwrap();
        let loaded = read_candles_from_parquet(&path).unwrap();

        assert_eq!(loaded.len(), candles.len());
        assert_eq!(loaded[0].time_open, candles[0].time_open);
        assert_eq!(loaded[0].coin, candles[0].coin);
        assert!((loaded[0].close - candles[0].close).abs() < 0.001);
    }
}
