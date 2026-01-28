use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::types::RequestPayer;
use aws_types::region::Region;
use futures::stream;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const BUCKET: &str = "hyperliquid-archive";

/// Security: Validate coin parameter to prevent path traversal attacks
fn validate_coin(coin: &str) -> Result<()> {
    // Prevent path traversal
    if coin.contains("..") || coin.contains('/') || coin.contains('\\') {
        anyhow::bail!("Invalid coin name: contains path traversal characters");
    }
    
    // Validate length
    if coin.is_empty() || coin.len() > 20 {
        anyhow::bail!("Invalid coin name: must be 1-20 characters");
    }
    
    // Only allow alphanumeric and underscore
    if !coin.chars().all(|c| c.is_alphanumeric() || c == '_') {
        anyhow::bail!("Invalid coin name: only alphanumeric and underscore allowed");
    }
    
    Ok(())
}

#[derive(Debug, Clone)]
pub struct MissingEntry {
    pub date: String,
    pub hour: u8,
    pub error: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    Downloaded,
    Skipped,
}

#[derive(Debug, Clone)]
pub struct DownloadOutcome {
    pub path: PathBuf,
    pub status: DownloadStatus,
    pub bytes: u64,
}

#[derive(Debug, Default, Clone)]
pub struct DownloadReport {
    pub downloaded: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
    pub missing: Vec<MissingEntry>,
    pub bytes_downloaded: u64,
}

pub struct S3Downloader {
    client: S3Client,
    base_dir: PathBuf,
}

impl S3Downloader {
    pub async fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        // Configure AWS SDK for S3 access
        // Note: Based on the Python script pattern (using --request-payer requester),
        // the hyperliquid-archive bucket appears to have "Requester Pays" enabled.
        // This means AWS credentials are required and you pay for data transfer.
        // If downloads fail, ensure AWS credentials are configured.
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .load()
            .await;
        
        let client = S3Client::new(&config);
        
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir)
            .await
            .context("Failed to create base directory")?;

        Ok(Self { client, base_dir })
    }

    /// Find the earliest and latest available dates in the S3 archive (YYYYMMDD).
    pub async fn date_bounds(&self) -> Result<Option<(String, String)>> {
        let mut paginator = self
            .client
            .list_objects_v2()
            .bucket(BUCKET)
            .prefix("market_data/")
            .delimiter("/")
            .request_payer(RequestPayer::Requester)
            .into_paginator()
            .send();

        let mut earliest: Option<String> = None;
        let mut latest: Option<String> = None;

        while let Some(page) = paginator.next().await {
            let page = page.context("Failed to list S3 prefixes")?;
            for prefix in page.common_prefixes() {
                if let Some(raw) = prefix.prefix() {
                    if let Some(date) = raw
                        .strip_prefix("market_data/")
                        .and_then(|s| s.strip_suffix('/'))
                    {
                        if date.len() == 8 && date.chars().all(|c| c.is_ascii_digit()) {
                            let update_earliest = earliest
                                .as_ref()
                                .map(|current| date < current.as_str())
                                .unwrap_or(true);
                            if update_earliest {
                                earliest = Some(date.to_string());
                            }

                            let update_latest = latest
                                .as_ref()
                                .map(|current| date > current.as_str())
                                .unwrap_or(true);
                            if update_latest {
                                latest = Some(date.to_string());
                            }
                        }
                    }
                }
            }
        }

        match (earliest, latest) {
            (Some(min), Some(max)) => Ok(Some((min, max))),
            _ => Ok(None),
        }
    }

    /// Download L2 book data from S3
    /// Format: market_data/{YYYYMMDD}/{H}/l2Book/{COIN}.lz4
    pub async fn download_l2_book(
        &self,
        coin: &str,
        date: &str, // YYYYMMDD
        hour: u8,   // 0-23
    ) -> Result<DownloadOutcome> {
        // Security: Validate coin parameter to prevent path traversal
        validate_coin(coin)?;
        
        // Security: Validate date format (YYYYMMDD, 8 digits)
        if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
            anyhow::bail!("Invalid date format: must be YYYYMMDD (8 digits)");
        }
        
        // Security: Validate hour range
        if hour > 23 {
            anyhow::bail!("Invalid hour: must be 0-23");
        }
        
        let key = format!("market_data/{}/{}/l2Book/{}.lz4", date, hour, coin);
        let output_path = self.base_dir
            .join(coin)
            .join(format!("{}-{}.lz4", date, hour));

        // Create parent directory
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create output directory")?;
        }

        // Skip if file already exists
        if output_path.exists() {
            return Ok(DownloadOutcome {
                path: output_path,
                status: DownloadStatus::Skipped,
                bytes: 0,
            });
        }

        // Download with request-payer=requester
        let request = self
            .client
            .get_object()
            .bucket(BUCKET)
            .key(&key)
            .request_payer(RequestPayer::Requester);

        let response = request
            .send()
            .await
            .with_context(|| format!("Failed to download s3://{}/{}", BUCKET, key))?;

        // Stream to file
        let mut file = fs::File::create(&output_path)
            .await
            .context("Failed to create output file")?;

        let mut body = response.body;
        let mut bytes: u64 = 0;
        while let Some(chunk) = body.next().await {
            let chunk = chunk.context("Failed to read chunk")?;
            bytes += chunk.len() as u64;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;

        Ok(DownloadOutcome {
            path: output_path,
            status: DownloadStatus::Downloaded,
            bytes,
        })
    }

    /// Download multiple hours for a date range
    pub async fn download_range(
        &self,
        coin: &str,
        start_date: &str, // YYYYMMDD
        start_hour: u8,
        end_date: &str,   // YYYYMMDD
        end_hour: u8,
        concurrency: Option<usize>,
    ) -> Result<DownloadReport> {
        let mut downloaded = Vec::new();
        let mut skipped = Vec::new();
        let mut missing = Vec::new();
        let mut bytes_downloaded: u64 = 0;

        let start = chrono::NaiveDate::parse_from_str(start_date, "%Y%m%d")
            .context("Invalid start date format (use YYYYMMDD)")?;
        let end = chrono::NaiveDate::parse_from_str(end_date, "%Y%m%d")
            .context("Invalid end date format (use YYYYMMDD)")?;

        let mut tasks: Vec<(String, u8)> = Vec::new();
        let mut current_date = start;
        while current_date <= end {
            let date_str = current_date.format("%Y%m%d").to_string();

            let hour_start = if current_date == start { start_hour } else { 0 };
            let hour_end = if current_date == end { end_hour } else { 23 };

            for hour in hour_start..=hour_end {
                tasks.push((date_str.clone(), hour));
            }

            current_date = current_date.succ_opt().context("Date overflow")?;
        }

        let max_concurrency = concurrency.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });

        let total = tasks.len() as u64;
        let progress = ProgressBar::new(total);
        progress.set_style(
            ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {wide_bar:.cyan/blue} {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("=>-"),
        );

        let coin = coin.to_string();
        let mut stream = stream::iter(tasks)
            .map(|(date, hour)| {
                let coin = coin.clone();
                async move {
                    let res = self.download_l2_book(&coin, &date, hour).await;
                    (date, hour, res)
                }
            })
            .buffer_unordered(max_concurrency);

        while let Some((date, hour, result)) = stream.next().await {
            progress.inc(1);
            match result {
                Ok(outcome) => {
                    match outcome.status {
                        DownloadStatus::Downloaded => {
                            bytes_downloaded += outcome.bytes;
                            downloaded.push(outcome.path);
                        }
                        DownloadStatus::Skipped => {
                            skipped.push(outcome.path);
                        }
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    eprintln!("Warning: Failed to download {} {} {}: {}", coin, date, hour, err_msg);
                    missing.push(MissingEntry {
                        date,
                        hour,
                        error: err_msg,
                    });
                }
            }
        }

        progress.finish_with_message("Download complete");

        Ok(DownloadReport {
            downloaded,
            skipped,
            missing,
            bytes_downloaded,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    #[ignore] // Requires AWS credentials and network
    async fn test_download_l2_book() {
        let temp_dir = TempDir::new().unwrap();
        let downloader = S3Downloader::new(temp_dir.path()).await.unwrap();
        
        // This will fail without proper setup, but tests the structure
        let result = downloader.download_l2_book("BTC", "20230916", 9).await;
        // Just check it doesn't panic - actual download requires S3 access
        println!("Download result: {:?}", result);
    }
}

