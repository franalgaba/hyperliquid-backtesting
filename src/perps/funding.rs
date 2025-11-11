use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Security: Validate coin parameter to prevent injection attacks
fn validate_coin_for_api(coin: &str) -> Result<()> {
    // Prevent path traversal and injection
    if coin.contains("..") || coin.contains('/') || coin.contains('\\') || coin.contains('\0') {
        anyhow::bail!("Invalid coin name: contains invalid characters");
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingPoint {
    pub ts_ms: u64,
    pub rate: f64, // Funding rate (e.g., 0.0001 = 0.01%)
}

#[derive(Debug, Clone, Deserialize)]
struct FundingHistoryResponse {
    coin: String,
    #[serde(rename = "fundingRate")]
    funding_rate: String,
    premium: String,
    time: u64,
}

#[derive(Debug, Clone, Serialize)]
struct FundingHistoryRequest {
    #[serde(rename = "type")]
    request_type: String,
    coin: String,
    #[serde(rename = "startTime")]
    start_time: u64,
    #[serde(rename = "endTime")]
    end_time: u64,
}

#[derive(Debug, Clone)]
pub struct FundingSchedule {
    points: Vec<FundingPoint>,
}

impl FundingSchedule {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Fetches funding history from Hyperliquid API.
    ///
    /// Retrieves historical funding rates for a perpetual futures contract over a specified
    /// time range. Funding rates are used to calculate funding payments every 8 hours.
    ///
    /// # Parameters
    ///
    /// - `coin`: Coin symbol (e.g., "BTC", "ETH")
    /// - `start_ts`: Start timestamp in milliseconds (Unix epoch)
    /// - `end_ts`: End timestamp in milliseconds (Unix epoch)
    ///
    /// # Returns
    ///
    /// `FundingSchedule` containing historical funding rates sorted by timestamp.
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - API request fails (network error, timeout)
    /// - Coin parameter is invalid (security validation)
    /// - Timestamp range is invalid (start > end or range > 1 year)
    /// - Funding rate parsing fails
    ///
    /// # Security
    ///
    /// - Validates coin parameter (prevents injection attacks)
    /// - Validates timestamp range (prevents DoS with large ranges)
    /// - Uses HTTPS with certificate validation
    /// - 30-second timeout on requests
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use crate::perps::FundingSchedule;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let funding = FundingSchedule::from_api(
    ///     "BTC",
    ///     1694858400000,  // Start: 2023-09-16 09:00:00 UTC
    ///     1694865600000,  // End:   2023-09-16 11:00:00 UTC
    /// ).await?;
    ///
    /// // Get funding rate at a specific time
    /// if let Some(rate) = funding.rate_at(1694858400000) {
    ///     println!("Funding rate: {:.6}", rate);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # API Reference
    ///
    /// See: <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/perpetuals#retrieve-historical-funding-rates>
    pub async fn from_api(
        coin: &str,
        start_ts: u64,
        end_ts: u64,
    ) -> Result<Self> {
        // Security: Validate coin parameter
        validate_coin_for_api(coin)?;
        
        // Security: Validate timestamp range (prevent unreasonable requests)
        if start_ts > end_ts {
            anyhow::bail!("Invalid timestamp range: start_ts must be <= end_ts");
        }
        
        // Prevent requests for extremely large ranges (potential DoS)
        const MAX_RANGE_MS: u64 = 365 * 24 * 60 * 60 * 1000; // 1 year
        if end_ts.saturating_sub(start_ts) > MAX_RANGE_MS {
            anyhow::bail!("Timestamp range too large: maximum 1 year");
        }
        
        // Security: Create HTTP client with explicit certificate validation
        // Note: reqwest validates certificates by default, but being explicit is better
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(false) // Explicitly require valid certificates
            .timeout(std::time::Duration::from_secs(30)) // Add timeout to prevent hanging
            .build()
            .context("Failed to create HTTP client")?;
        
        let request = FundingHistoryRequest {
            request_type: "fundingHistory".to_string(),
            coin: coin.to_string(),
            start_time: start_ts,
            end_time: end_ts,
        };

        let response = client
            .post("https://api.hyperliquid.xyz/info")
            .json(&request)
            .send()
            .await
            .context("Failed to send funding history request")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Funding history API returned error: {}",
                response.status()
            );
        }

        let funding_data: Vec<FundingHistoryResponse> = response
            .json()
            .await
            .context("Failed to parse funding history response")?;

        let mut schedule = Self::new();
        
        for entry in funding_data {
            let rate = entry.funding_rate
                .parse::<f64>()
                .with_context(|| format!("Invalid funding rate: {}", entry.funding_rate))?;
            
            schedule.add_point(entry.time, rate);
        }

        Ok(schedule)
    }

    /// Adds a funding rate point to the schedule.
    ///
    /// Points are automatically sorted by timestamp after insertion.
    ///
    /// # Parameters
    ///
    /// - `ts_ms`: Timestamp in milliseconds
    /// - `rate`: Funding rate (e.g., 0.0001 = 0.01%)
    pub fn add_point(&mut self, ts_ms: u64, rate: f64) {
        self.points.push(FundingPoint { ts_ms, rate });
        self.points.sort_by_key(|p| p.ts_ms);
    }

    /// Gets the funding rate at a specific timestamp.
    ///
    /// Returns the most recent funding rate at or before the specified timestamp.
    ///
    /// # Parameters
    ///
    /// - `ts_ms`: Timestamp in milliseconds
    ///
    /// # Returns
    ///
    /// - `Some(rate)` if a rate is found at or before the timestamp
    /// - `None` if no rate is available
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::perps::FundingSchedule;
    ///
    /// let mut schedule = FundingSchedule::new();
    /// schedule.add_point(1000, 0.0001);
    /// schedule.add_point(2000, 0.0002);
    ///
    /// assert_eq!(schedule.rate_at(1500), Some(0.0001));  // Uses rate at 1000
    /// assert_eq!(schedule.rate_at(2500), Some(0.0002));  // Uses rate at 2000
    /// assert_eq!(schedule.rate_at(500), None);           // No rate available
    /// ```
    pub fn rate_at(&self, ts_ms: u64) -> Option<f64> {
        // Find the funding point at or before this timestamp
        self.points
            .iter()
            .rev()
            .find(|p| p.ts_ms <= ts_ms)
            .map(|p| p.rate)
    }

    /// Calculates funding payment for a position.
    ///
    /// Funding payment = notional * rate
    ///
    /// # Parameters
    ///
    /// - `notional`: Position notional value (size * price)
    /// - `ts_ms`: Timestamp for rate lookup
    ///
    /// # Returns
    ///
    /// Funding payment amount:
    /// - Positive value = received (short positions)
    /// - Negative value = paid (long positions)
    ///
    /// # Example
    ///
    /// ```rust
    /// use crate::perps::FundingSchedule;
    ///
    /// let mut schedule = FundingSchedule::new();
    /// schedule.add_point(1000, 0.0001);  // 0.01% rate
    ///
    /// let notional = 10000.0;  // $10,000 position
    /// let payment = schedule.calculate_payment(notional, 1000);
    /// assert_eq!(payment, 1.0);  // $1 payment
    /// ```
    pub fn calculate_payment(&self, notional: f64, ts_ms: u64) -> f64 {
        if let Some(rate) = self.rate_at(ts_ms) {
            notional * rate
        } else {
            0.0
        }
    }

    /// Get all funding timestamps in a range
    pub fn timestamps_in_range(&self, start_ts: u64, end_ts: u64) -> Vec<u64> {
        self.points
            .iter()
            .filter(|p| p.ts_ms >= start_ts && p.ts_ms <= end_ts)
            .map(|p| p.ts_ms)
            .collect()
    }
}

impl Default for FundingSchedule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_funding_schedule() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);
        schedule.add_point(2000, 0.0002);
        schedule.add_point(3000, 0.00015);

        assert_eq!(schedule.rate_at(1500), Some(0.0001));
        assert_eq!(schedule.rate_at(2500), Some(0.0002));
        assert_eq!(schedule.rate_at(500), None);

        let payment = schedule.calculate_payment(10000.0, 1500);
        assert_eq!(payment, 1.0); // 10000 * 0.0001
    }
}

