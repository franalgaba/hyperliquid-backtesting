use anyhow::{Context, Result};
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::data::{
    export_candles_to_parquet, export_equity_to_parquet, export_trades_to_parquet, load_candles,
    Cache,
};
use crate::ingest::{build_ohlc_from_events, parse_l2_file, S3Downloader};
use crate::strategy::{SignalStrategy, Strategy};
use crate::orders::{simulate, simulate_with_strategy};
use crate::perps::funding::FundingSchedule;
use crate::report::write_results;
use std::fs;
use std::time::Instant;
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;

/// Validate that an asset/coin name is secure (no path traversal, alphanumeric only)
fn validate_asset(asset: &str) -> Result<()> {
    if asset.contains("..") || asset.contains('/') || asset.contains('\\') {
        anyhow::bail!("Invalid asset name: contains path traversal characters");
    }

    if asset.is_empty() || asset.len() > 20 {
        anyhow::bail!("Invalid asset name: must be 1-20 characters");
    }

    if !asset.chars().all(|c| c.is_alphanumeric()) {
        anyhow::bail!("Invalid asset name: only alphanumeric characters allowed");
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "hl-backtest")]
#[command(about = "Hyperliquid data ingestor and backtester")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Fetch and cache historical candle data
    Fetch {
        /// Asset symbol (e.g., ETH, BTC)
        #[arg(long)]
        asset: String,
        /// Timeframe interval (1m, 5m, 15m, 1h, 4h, 1d, 1w)
        #[arg(long)]
        interval: String,
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: String,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: String,
        /// Export to Parquet format (optional output path)
        #[arg(long)]
        parquet: Option<PathBuf>,
    },
    /// Export cached data to Parquet format
    Export {
        /// Asset symbol (e.g., ETH, BTC)
        #[arg(long)]
        asset: String,
        /// Timeframe interval (1m, 5m, 15m, 1h, 4h, 1d, 1w)
        #[arg(long)]
        interval: String,
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: String,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: String,
        /// Output Parquet file path
        #[arg(long)]
        out: PathBuf,
    },
    /// Ingest L2 data from S3
    Ingest {
        #[command(subcommand)]
        subcommand: IngestSubcommand,
    },
    /// Run a backtest on a strategy
    Run {
        /// Path to strategy JSON file
        #[arg(long)]
        strategy: PathBuf,
        /// Asset symbol (e.g., ETH, BTC)
        #[arg(long)]
        asset: String,
        /// Timeframe interval (1m, 5m, 15m, 1h, 4h, 1d, 1w)
        #[arg(long)]
        interval: String,
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: String,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: String,
        /// Initial capital in USDC
        #[arg(long, default_value = "10000.0")]
        initial_capital: f64,
        /// Maker fee in basis points (negative for rebate)
        #[arg(long, default_value = "-1")]
        maker_fee_bps: i16,
        /// Taker fee in basis points
        #[arg(long, default_value = "10")]
        taker_fee_bps: i16,
        /// Slippage in basis points
        #[arg(long, default_value = "5")]
        slippage_bps: u16,
        /// Output file path (JSON)
        #[arg(long, default_value = "results.json")]
        out: PathBuf,
        /// Export results to Parquet (trades and equity curve)
        #[arg(long)]
        parquet_results: Option<PathBuf>,
    },
    /// Run a perps backtest from L2 events
    RunPerps {
        /// Path to strategy JSON file
        #[arg(long)]
        strategy: PathBuf,
        /// Coin symbol (e.g., BTC, ETH)
        #[arg(long)]
        coin: String,
        /// Path to events directory
        #[arg(long)]
        events: PathBuf,
        /// Start date-hour (YYYYMMDD-HH)
        #[arg(long)]
        start: String,
        /// End date-hour (YYYYMMDD-HH)
        #[arg(long)]
        end: String,
        /// Initial capital in USDC
        #[arg(long, default_value = "10000.0")]
        initial_capital: f64,
        /// Maker fee in basis points (negative for rebate)
        #[arg(long, default_value = "-1")]
        maker_fee_bps: i16,
        /// Taker fee in basis points
        #[arg(long, default_value = "10")]
        taker_fee_bps: i16,
        /// I/O concurrency for parallel file parsing
        #[arg(long)]
        io_concurrency: Option<usize>,
        /// Enable parallel indicator updates
        #[arg(long)]
        indicators_par: Option<bool>,
        /// Trade cooldown period in minutes
        #[arg(long)]
        trade_cooldown_min: Option<u64>,
        /// Output file path (JSON)
        #[arg(long, default_value = "results.json")]
        out: PathBuf,
        /// Export results to Parquet (trades and equity curve)
        #[arg(long)]
        parquet_results: Option<PathBuf>,
    },
    /// Run a backtest using precomputed signals (CSV)
    RunSignals {
        /// Path to signals CSV file (time_open,position)
        #[arg(long)]
        signals: PathBuf,
        /// Asset symbol (e.g., ETH, BTC)
        #[arg(long)]
        asset: String,
        /// Timeframe interval (1m, 5m, 15m, 1h, 4h, 1d, 1w)
        #[arg(long)]
        interval: String,
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start: String,
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end: String,
        /// Initial capital in USDC
        #[arg(long, default_value = "10000.0")]
        initial_capital: f64,
        /// Maker fee in basis points (negative for rebate)
        #[arg(long, default_value = "-1")]
        maker_fee_bps: i16,
        /// Taker fee in basis points
        #[arg(long, default_value = "10")]
        taker_fee_bps: i16,
        /// Slippage in basis points
        #[arg(long, default_value = "5")]
        slippage_bps: u16,
        /// Output file path (JSON)
        #[arg(long, default_value = "results.json")]
        out: PathBuf,
        /// Export results to Parquet (trades and equity curve)
        #[arg(long)]
        parquet_results: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum IngestSubcommand {
    /// Download L2 data from S3
    S3 {
        /// Coin symbol (e.g., BTC, ETH)
        #[arg(long)]
        coin: String,
        /// Start date (YYYYMMDD)
        #[arg(long)]
        start: String,
        /// Start hour (0-23)
        #[arg(long, default_value = "0")]
        start_hour: u8,
        /// End date (YYYYMMDD)
        #[arg(long)]
        end: String,
        /// End hour (0-23)
        #[arg(long, default_value = "23")]
        end_hour: u8,
        /// Parallel download concurrency
        #[arg(long)]
        io_concurrency: Option<usize>,
        /// Optional CSV report for missing entries
        #[arg(long)]
        report_missing: Option<PathBuf>,
        /// Output directory for downloaded files
        #[arg(long, default_value = "data/s3")]
        out: PathBuf,
    },
    /// Build events from downloaded L2 files
    BuildEvents {
        /// Coin symbol (e.g., BTC, ETH)
        #[arg(long)]
        coin: String,
        /// Input directory with .lz4 files
        #[arg(long)]
        input: PathBuf,
        /// Output directory for events
        #[arg(long, default_value = "data/events")]
        out: PathBuf,
    },
    /// Build OHLC candles from L2 events
    BuildOhlc {
        /// Coin symbol (e.g., BTC, ETH)
        #[arg(long)]
        coin: String,
        /// Events base directory (expects {events}/{coin}; supports .jsonl or .lz4)
        #[arg(long, default_value = "data/events")]
        events: PathBuf,
        /// Candle interval (1m, 5m, 15m, 1h, 4h, 1d, 1w)
        #[arg(long, default_value = "1h")]
        interval: String,
        /// Output CSV path (cache format)
        #[arg(long)]
        out_csv: PathBuf,
        /// Optional Parquet output path
        #[arg(long)]
        out_parquet: Option<PathBuf>,
        /// Fill missing candles by carrying forward the previous close
        #[arg(long)]
        fill_gaps: bool,
    },
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            Commands::Fetch {
                asset,
                interval,
                start,
                end,
                parquet,
            } => {
                let start_time = Instant::now();
                validate_asset(&asset)?;
                let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
                    .context("Invalid start date format (use YYYY-MM-DD)")?;
                let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")
                    .context("Invalid end date format (use YYYY-MM-DD)")?;

                let start_ts = start_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;
                let end_ts = end_date
                    .and_hms_opt(23, 59, 59)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;

                let cache = Cache::new()?;
                cache
                    .fetch_and_cache(&asset, &interval, start_ts, end_ts)
                    .await?;
                println!("Fetched and cached candles for {asset} {interval}");

                // Export to Parquet if requested
                if let Some(parquet_path) = parquet {
                    let candles = load_candles(&cache, &asset, &interval, start_ts, end_ts).await?;
                    export_candles_to_parquet(&candles, &parquet_path)?;
                    println!("Exported {} candles to {}", candles.len(), parquet_path.display());
                }

                let elapsed = start_time.elapsed();
                println!("Completed in {:.2}s", elapsed.as_secs_f64());
                Ok(())
            }
            Commands::Export {
                asset,
                interval,
                start,
                end,
                out,
            } => {
                let start_time = Instant::now();
                validate_asset(&asset)?;
                let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
                    .context("Invalid start date format (use YYYY-MM-DD)")?;
                let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")
                    .context("Invalid end date format (use YYYY-MM-DD)")?;

                let start_ts = start_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;
                let end_ts = end_date
                    .and_hms_opt(23, 59, 59)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;

                let cache = Cache::new()?;
                let candles = load_candles(&cache, &asset, &interval, start_ts, end_ts).await?;

                if candles.is_empty() {
                    anyhow::bail!("No candles found for {asset} {interval}. Run 'fetch' first.");
                }

                export_candles_to_parquet(&candles, &out)?;
                println!("Exported {} candles to {}", candles.len(), out.display());

                let elapsed = start_time.elapsed();
                println!("Completed in {:.2}s", elapsed.as_secs_f64());
                Ok(())
            }
            Commands::Run {
                strategy,
                asset,
                interval,
                start,
                end,
                initial_capital,
                maker_fee_bps,
                taker_fee_bps,
                slippage_bps,
                out,
                parquet_results,
            } => {
                let start_time = Instant::now();
                validate_asset(&asset)?;
                let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
                    .context("Invalid start date format (use YYYY-MM-DD)")?;
                let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")
                    .context("Invalid end date format (use YYYY-MM-DD)")?;

                let start_ts = start_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;
                let end_ts = end_date
                    .and_hms_opt(23, 59, 59)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;

                // Load strategy
                let strategy_str = std::fs::read_to_string(&strategy)
                    .with_context(|| format!("Failed to read strategy file: {}", strategy.display()))?;
                let strategy_def: Strategy =
                    serde_json::from_str(&strategy_str).context("Failed to parse strategy JSON")?;

                // Load candles
                let cache = Cache::new()?;
                let candles = load_candles(&cache, &asset, &interval, start_ts, end_ts).await?;

                if candles.is_empty() {
                    anyhow::bail!("No candles found for {asset} {interval} in date range");
                }

                // Run simulation
                let config = crate::orders::SimConfig {
                    initial_capital,
                    maker_fee_bps,
                    taker_fee_bps,
                    slippage_bps,
                    trade_cooldown_ms: None,
                };

                let result = simulate(&candles, &strategy_def, &config).await?;

                // Write results
                write_results(&result, &out)?;
                println!("Backtest complete. Results written to {}", out.display());

                // Export to Parquet if requested
                if let Some(parquet_dir) = parquet_results {
                    std::fs::create_dir_all(&parquet_dir)?;

                    let trades_path = parquet_dir.join("trades.parquet");
                    export_trades_to_parquet(&result.trades, &trades_path)?;
                    println!("Exported {} trades to {}", result.trades.len(), trades_path.display());

                    let equity_path = parquet_dir.join("equity.parquet");
                    export_equity_to_parquet(&result.equity_curve, &equity_path)?;
                    println!("Exported {} equity points to {}", result.equity_curve.len(), equity_path.display());
                }

                let elapsed = start_time.elapsed();
                println!("Completed in {:.2}s", elapsed.as_secs_f64());
                Ok(())
            }
            Commands::Ingest { subcommand } => {
                match subcommand {
                    IngestSubcommand::S3 {
                        coin,
                        start,
                        start_hour,
                        end,
                        end_hour,
                        io_concurrency,
                        report_missing,
                        out,
                    } => {
                        let start_time = Instant::now();
                        validate_asset(&coin)?;
                        let downloader = S3Downloader::new(&out).await?;

                        let mut start = start;
                        let mut start_hour = start_hour;
                        let mut end = end;
                        let mut end_hour = end_hour;

                        if let Some((earliest, latest)) = downloader.date_bounds().await? {
                            if end < earliest {
                                anyhow::bail!(
                                    "End date {} is earlier than first available data {}",
                                    end,
                                    earliest
                                );
                            }

                            if start > latest {
                                anyhow::bail!(
                                    "Start date {} is after last available data {}",
                                    start,
                                    latest
                                );
                            }

                            if start < earliest {
                                println!(
                                    "Start date {} earlier than first available data {}. Using {}.",
                                    start,
                                    earliest,
                                    earliest
                                );
                                start = earliest;
                                start_hour = 0;
                            }

                            if end > latest {
                                println!(
                                    "End date {} later than last available data {}. Using {}.",
                                    end,
                                    latest,
                                    latest
                                );
                                end = latest;
                                end_hour = 23;
                            }
                        }

                        let report = downloader
                            .download_range(
                                &coin,
                                &start,
                                start_hour,
                                &end,
                                end_hour,
                                io_concurrency,
                            )
                            .await?;
                        let downloaded = report.downloaded.len();
                        let skipped = report.skipped.len();
                        let missing = report.missing.len();
                        let bytes = report.bytes_downloaded;
                        let gib = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                        let est_cost = gib * 0.09;

                        println!("Downloaded {} files for {}", downloaded, coin);
                        println!("Skipped {} existing files", skipped);
                        println!("Missing {} entries", missing);
                        println!("Downloaded {:.2} GiB", gib);
                        println!("Estimated transfer cost: ${:.2} (at $0.09/GB)", est_cost);

                        if !report.missing.is_empty() {
                            println!(
                                "Missing {} entries (see warnings above).",
                                missing
                            );
                            if let Some(report_path) = report_missing {
                                let mut writer = csv::Writer::from_path(&report_path)?;
                                writer.write_record(["date", "hour", "error"])?;
                                for entry in &report.missing {
                                    let hour = entry.hour.to_string();
                                    writer.write_record([
                                        entry.date.as_str(),
                                        hour.as_str(),
                                        entry.error.as_str(),
                                    ])?;
                                }
                                writer.flush()?;
                                println!("Wrote missing report to {}", report_path.display());
                            }
                        }

                        let elapsed = start_time.elapsed();
                        let secs = elapsed.as_secs_f64().max(0.001);
                        let mib_per_s = (bytes as f64 / (1024.0 * 1024.0)) / secs;
                        println!("Average download rate: {:.2} MiB/s", mib_per_s);
                        println!("Completed in {:.2}s", secs);
                        Ok(())
                    }
                    IngestSubcommand::BuildEvents { coin, input, out } => {
                        let start_time = Instant::now();
                        validate_asset(&coin)?;
                        let coin_dir = out.join(&coin);
                        fs::create_dir_all(&coin_dir)
                            .context("Failed to create events directory")?;

                        let input_dir = input.join(&coin);
                        if !input_dir.exists() {
                            anyhow::bail!(
                                "Input directory does not exist: {}",
                                input_dir.display()
                            );
                        }

                        let mut files: Vec<_> = fs::read_dir(&input_dir)?
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path().extension().and_then(|s| s.to_str()) == Some("lz4")
                            })
                            .collect();
                        files.sort_by_key(|e| e.path());

                        println!("Processing {} files for {}", files.len(), coin);

                        for entry in files {
                            let file_path = entry.path();
                            let file_name = file_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .context("Invalid file name")?;

                            let output_path = coin_dir.join(format!("{}.jsonl", file_name));
                            if output_path.exists() {
                                println!("Skipping {} (already exists)", file_name);
                                continue;
                            }

                            println!("Processing {}", file_name);

                            let events = parse_l2_file(&file_path).await?;

                            let mut output_file = tokio_fs::File::create(&output_path).await?;

                            for event in events {
                                let json = serde_json::to_string(&event)?;
                                output_file.write_all(json.as_bytes()).await?;
                                output_file.write_all(b"\n").await?;
                            }
                            output_file.flush().await?;
                        }

                        println!("Built events in {}", coin_dir.display());

                        let elapsed = start_time.elapsed();
                        println!("Completed in {:.2}s", elapsed.as_secs_f64());
                        Ok(())
                    }
                    IngestSubcommand::BuildOhlc {
                        coin,
                        events,
                        interval,
                        out_csv,
                        out_parquet,
                        fill_gaps,
                    } => {
                        let start_time = Instant::now();
                        validate_asset(&coin)?;
                        let events_dir = events.join(&coin);
                        if !events_dir.exists() {
                            anyhow::bail!(
                                "Events directory does not exist: {}",
                                events_dir.display()
                            );
                        }

                        let (candles, filled) =
                            build_ohlc_from_events(&events_dir, &coin, &interval, fill_gaps)
                                .await?;

                        if candles.is_empty() {
                            anyhow::bail!("No candles generated for {coin}");
                        }

                        if let Some(parent) = out_csv.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let mut writer = csv::Writer::from_path(&out_csv)?;
                        writer.write_record([
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
                            writer.write_record([
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
                        writer.flush()?;
                        println!("Wrote {} candles to {}", candles.len(), out_csv.display());
                        if fill_gaps {
                            println!("Filled {} missing candles", filled);
                        }

                        if let Some(parquet_path) = out_parquet {
                            crate::data::export_candles_to_parquet(&candles, &parquet_path)?;
                            println!("Wrote {} candles to {}", candles.len(), parquet_path.display());
                        }

                        let elapsed = start_time.elapsed();
                        println!("Completed in {:.2}s", elapsed.as_secs_f64());
                        Ok(())
                    }
                }
            }
            Commands::RunPerps {
                strategy,
                coin,
                events,
                start,
                end,
                initial_capital,
                maker_fee_bps,
                taker_fee_bps,
                io_concurrency,
                indicators_par,
                trade_cooldown_min,
                out,
                parquet_results,
            } => {
                let start_time = Instant::now();
                validate_asset(&coin)?;

                let parse_date_hour = |s: &str| -> Result<(String, u8)> {
                    let parts: Vec<&str> = s.split('-').collect();
                    if parts.len() != 2 {
                        anyhow::bail!("Invalid date-hour format (use YYYYMMDD-HH)");
                    }
                    let date = parts[0].to_string();
                    let hour = parts[1].parse::<u8>().context("Invalid hour")?;
                    Ok((date, hour))
                };

                let (start_date, start_hour) = parse_date_hour(&start)?;
                let (end_date, end_hour) = parse_date_hour(&end)?;

                let start_ts = chrono::NaiveDate::parse_from_str(&start_date, "%Y%m%d")?
                    .and_hms_opt(start_hour.into(), 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;
                let end_ts = chrono::NaiveDate::parse_from_str(&end_date, "%Y%m%d")?
                    .and_hms_opt(end_hour.into(), 59, 59)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;

                // Load strategy
                let strategy_str = std::fs::read_to_string(&strategy)
                    .with_context(|| format!("Failed to read strategy file: {}", strategy.display()))?;
                let strategy_def: Strategy =
                    serde_json::from_str(&strategy_str).context("Failed to parse strategy JSON")?;

                // Fetch funding schedule
                let _funding = FundingSchedule::from_api(&coin, start_ts, end_ts).await?;

                let events_dir = events.join(&coin);
                if !events_dir.exists() {
                    anyhow::bail!("Events directory does not exist: {}", events_dir.display());
                }

                let config = crate::orders::SimConfig {
                    initial_capital,
                    maker_fee_bps,
                    taker_fee_bps,
                    slippage_bps: 0,
                    trade_cooldown_ms: trade_cooldown_min.map(|min| min * 60 * 1000),
                };

                let indicators_parallel = indicators_par.unwrap_or_else(|| {
                    #[cfg(not(debug_assertions))]
                    {
                        true
                    }
                    #[cfg(debug_assertions)]
                    {
                        false
                    }
                });

                let result = crate::perps::engine::PerpsEngine::run(
                    &events_dir,
                    &strategy_def,
                    &config,
                    &coin,
                    start_ts,
                    end_ts,
                    io_concurrency,
                    indicators_parallel,
                )
                .await?;

                write_results(&result, &out)?;
                println!(
                    "Perps backtest complete. Results written to {}",
                    out.display()
                );

                // Export to Parquet if requested
                if let Some(parquet_dir) = parquet_results {
                    std::fs::create_dir_all(&parquet_dir)?;

                    let trades_path = parquet_dir.join("trades.parquet");
                    export_trades_to_parquet(&result.trades, &trades_path)?;
                    println!("Exported {} trades to {}", result.trades.len(), trades_path.display());

                    let equity_path = parquet_dir.join("equity.parquet");
                    export_equity_to_parquet(&result.equity_curve, &equity_path)?;
                    println!("Exported {} equity points to {}", result.equity_curve.len(), equity_path.display());
                }

                let elapsed = start_time.elapsed();
                println!("Completed in {:.2}s", elapsed.as_secs_f64());
                Ok(())
            }
            Commands::RunSignals {
                signals,
                asset,
                interval,
                start,
                end,
                initial_capital,
                maker_fee_bps,
                taker_fee_bps,
                slippage_bps,
                out,
                parquet_results,
            } => {
                let start_time = Instant::now();
                validate_asset(&asset)?;

                let start_date = NaiveDate::parse_from_str(&start, "%Y-%m-%d")
                    .context("Invalid start date format (use YYYY-MM-DD)")?;
                let end_date = NaiveDate::parse_from_str(&end, "%Y-%m-%d")
                    .context("Invalid end date format (use YYYY-MM-DD)")?;

                let start_ts = start_date
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;
                let end_ts = end_date
                    .and_hms_opt(23, 59, 59)
                    .unwrap()
                    .and_utc()
                    .timestamp() as u64
                    * 1000;

                let cache = Cache::new()?;
                let candles = load_candles(&cache, &asset, &interval, start_ts, end_ts).await?;

                if candles.is_empty() {
                    anyhow::bail!("No candles found for {asset} {interval} in date range");
                }

                let mut signal_strategy = SignalStrategy::from_csv(&signals)?;

                let config = crate::orders::SimConfig {
                    initial_capital,
                    maker_fee_bps,
                    taker_fee_bps,
                    slippage_bps,
                    trade_cooldown_ms: None,
                };

                let result = simulate_with_strategy(&candles, &mut signal_strategy, &config).await?;

                write_results(&result, &out)?;
                println!("Signal backtest complete. Results written to {}", out.display());

                if let Some(parquet_dir) = parquet_results {
                    std::fs::create_dir_all(&parquet_dir)?;

                    let trades_path = parquet_dir.join("trades.parquet");
                    export_trades_to_parquet(&result.trades, &trades_path)?;
                    println!("Exported {} trades to {}", result.trades.len(), trades_path.display());

                    let equity_path = parquet_dir.join("equity.parquet");
                    export_equity_to_parquet(&result.equity_curve, &equity_path)?;
                    println!("Exported {} equity points to {}", result.equity_curve.len(), equity_path.display());
                }

                let elapsed = start_time.elapsed();
                println!("Completed in {:.2}s", elapsed.as_secs_f64());
                Ok(())
            }
        }
    }
}
