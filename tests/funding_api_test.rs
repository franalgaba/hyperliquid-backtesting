#[cfg(test)]
mod tests {
    use hl_backtest::perps::funding::FundingSchedule;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_fetch_funding_history() {
        // Test fetching funding history for ETH
        // Using a recent date range (last 7 days)
        let end_ts = chrono::Utc::now().timestamp() as u64 * 1000;
        let start_ts = end_ts - (7 * 24 * 60 * 60 * 1000); // 7 days ago

        let schedule = FundingSchedule::from_api("ETH", start_ts, end_ts)
            .await
            .expect("Failed to fetch funding history");

        // Should have at least some funding points (funding occurs every 8 hours)
        // Over 7 days, we should have at least 7 * 3 = 21 funding points
        let points = schedule.timestamps_in_range(start_ts, end_ts);
        assert!(
            points.len() > 0,
            "Expected at least some funding points, got {}",
            points.len()
        );

        // Verify rates are reasonable (typically between -0.001 and 0.001)
        for ts in &points {
            if let Some(rate) = schedule.rate_at(*ts) {
                assert!(
                    rate.abs() < 0.01,
                    "Funding rate {} seems unreasonable",
                    rate
                );
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_funding_history_empty_range() {
        // Test with a very old date range (should return empty or minimal data)
        let start_ts = 1000000000000; // Very old timestamp
        let end_ts = 1000000100000; // 1 hour later

        let schedule = FundingSchedule::from_api("BTC", start_ts, end_ts)
            .await
            .expect("Failed to fetch funding history");

        // Should handle empty results gracefully
        let points = schedule.timestamps_in_range(start_ts, end_ts);
        // Empty is fine - just verify it doesn't panic
        println!("Found {} funding points in old range", points.len());
    }
}

