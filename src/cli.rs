use anyhow::{Context, Result};
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::data::{load_candles, Cache};
use crate::ingest::{parse_l2_file, S3Downloader};
use crate::ir::StrategyIr;
use crate::orders::simulate;
use crate::perps::funding::FundingSchedule;
use crate::report::write_results;
use std::fs;
use tokio::fs as tokio_fs;
use tokio::io::AsyncWriteExt;

/// Supported assets for backtesting
const SUPPORTED_ASSETS: &[&str] = &["BTC", "ETH", "SOL", "HYPE"];

/// Validate that an asset/coin is supported and secure
/// Prevents path traversal attacks and ensures valid input
fn validate_asset(asset: &str) -> Result<()> {
    // Security: Prevent path traversal attacks
    if asset.contains("..") || asset.contains('/') || asset.contains('\\') {
        anyhow::bail!("Invalid asset name: contains path traversal characters");
    }
    
    // Security: Validate length
    if asset.is_empty() || asset.len() > 20 {
        anyhow::bail!("Invalid asset name: must be 1-20 characters");
    }
    
    // Security: Only allow alphanumeric characters (no special chars that could be used in paths)
    if !asset.chars().all(|c| c.is_alphanumeric()) {
        anyhow::bail!("Invalid asset name: only alphanumeric characters allowed");
    }
    
    // Check if asset is supported
    let asset_upper = asset.to_uppercase();
    if SUPPORTED_ASSETS.contains(&asset_upper.as_str()) {
        Ok(())
    } else {
        anyhow::bail!(
            "Unsupported asset: {}. Supported assets: {}",
            asset,
            SUPPORTED_ASSETS.join(", ")
        )
    }
}

#[derive(Parser)]
#[command(name = "hl-backtest")]
#[command(about = "Hyperliquid strategy backtester")]
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
    },
    /// Ingest L2 data from S3
    Ingest {
        #[command(subcommand)]
        subcommand: IngestSubcommand,
    },
    /// Run a backtest on a strategy IR
    Run {
        /// Path to strategy IR JSON file
        #[arg(long)]
        ir: PathBuf,
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
    },
        /// Run a perps backtest from L2 events
        RunPerps {
            /// Path to strategy IR JSON file
            #[arg(long)]
            ir: PathBuf,
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
            /// I/O concurrency for parallel file parsing (default: min(8, CPU cores))
            #[arg(long)]
            io_concurrency: Option<usize>,
            /// Enable parallel indicator updates (default: true in release, false in debug)
            #[arg(long)]
            indicators_par: Option<bool>,
            /// Output file path (JSON)
            #[arg(long, default_value = "results.json")]
            out: PathBuf,
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
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            Commands::Fetch {
                asset,
                interval,
                start,
                end,
            } => {
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
                Ok(())
            }
            Commands::Run {
                ir,
                asset,
                interval,
                start,
                end,
                initial_capital,
                maker_fee_bps,
                taker_fee_bps,
                slippage_bps,
                out,
            } => {
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

                // Load IR
                let ir_str = std::fs::read_to_string(&ir)
                    .with_context(|| format!("Failed to read IR file: {}", ir.display()))?;
                let strategy_ir: StrategyIr =
                    serde_json::from_str(&ir_str).context("Failed to parse IR JSON")?;

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
                };

                let result = simulate(&candles, &strategy_ir, &config).await?;

                // Write results
                write_results(&result, &out)?;
                println!("Backtest complete. Results written to {}", out.display());

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
                        out,
                    } => {
                        validate_asset(&coin)?;
                        let downloader = S3Downloader::new(&out).await?;
                        let downloaded = downloader
                            .download_range(&coin, &start, start_hour, &end, end_hour)
                            .await?;
                        println!("Downloaded {} files for {}", downloaded.len(), coin);
                        Ok(())
                    }
                    IngestSubcommand::BuildEvents { coin, input, out } => {
                        validate_asset(&coin)?;
                        // Create output directory
                        let coin_dir = out.join(&coin);
                        fs::create_dir_all(&coin_dir)
                            .context("Failed to create events directory")?;

                        // Find all .lz4 files in input directory
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

                            // Check if output file already exists
                            let output_path = coin_dir.join(format!("{}.jsonl", file_name));
                            if output_path.exists() {
                                println!("Skipping {} (already exists)", file_name);
                                continue;
                            }

                            println!("Processing {}", file_name);

                            // Parse L2 file
                            let events = parse_l2_file(&file_path).await?;

                            // Write events as JSONL
                            let mut output_file = tokio_fs::File::create(&output_path).await?;

                            for event in events {
                                let json = serde_json::to_string(&event)?;
                                output_file.write_all(json.as_bytes()).await?;
                                output_file.write_all(b"\n").await?;
                            }
                            output_file.flush().await?;
                        }

                        println!("Built events in {}", coin_dir.display());
                        Ok(())
                    }
                }
            }
                Commands::RunPerps {
                    ir,
                    coin,
                    events,
                    start,
                    end,
                    initial_capital,
                    maker_fee_bps,
                    taker_fee_bps,
                    io_concurrency,
                    indicators_par,
                    out,
                } => {
                validate_asset(&coin)?;
                // Parse date-hour strings
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

                // Convert to timestamps
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

                // Load IR
                let ir_str = std::fs::read_to_string(&ir)
                    .with_context(|| format!("Failed to read IR file: {}", ir.display()))?;
                let strategy_ir: StrategyIr =
                    serde_json::from_str(&ir_str).context("Failed to parse IR JSON")?;

                // Fetch funding schedule
                let _funding = FundingSchedule::from_api(&coin, start_ts, end_ts).await?;

                // Load events from directory
                let events_dir = events.join(&coin);
                if !events_dir.exists() {
                    anyhow::bail!("Events directory does not exist: {}", events_dir.display());
                }

                // Run perps backtest
                let config = crate::orders::SimConfig {
                    initial_capital,
                    maker_fee_bps,
                    taker_fee_bps,
                    slippage_bps: 0, // Slippage handled by order book depth
                };

                // Default indicators_par: true in release, false in debug
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
                    &strategy_ir,
                    &config,
                    &coin,
                    start_ts,
                    end_ts,
                    io_concurrency,
                    indicators_parallel,
                )
                .await?;

                // Write results
                write_results(&result, &out)?;
                println!(
                    "Perps backtest complete. Results written to {}",
                    out.display()
                );

                Ok(())
            }
        }
    }
}
