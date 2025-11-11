#[cfg(test)]
mod tests {
    use hl_backtest::perps::funding::FundingSchedule;

    #[test]
    fn test_funding_schedule_new() {
        let schedule = FundingSchedule::new();
        // Should be empty initially
        assert_eq!(schedule.rate_at(1000), None);
    }

    #[test]
    fn test_funding_schedule_add_point() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);
        schedule.add_point(2000, 0.0002);
        schedule.add_point(3000, 0.00015);

        // Points should be sorted
        assert_eq!(schedule.rate_at(500), None);
        assert_eq!(schedule.rate_at(1500), Some(0.0001));
        assert_eq!(schedule.rate_at(2500), Some(0.0002));
        assert_eq!(schedule.rate_at(3500), Some(0.00015));
    }

    #[test]
    fn test_funding_schedule_rate_at_exact_timestamp() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);
        schedule.add_point(2000, 0.0002);

        assert_eq!(schedule.rate_at(1000), Some(0.0001));
        assert_eq!(schedule.rate_at(2000), Some(0.0002));
    }

    #[test]
    fn test_funding_schedule_rate_at_between_timestamps() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);
        schedule.add_point(2000, 0.0002);

        // Should return rate from previous timestamp
        assert_eq!(schedule.rate_at(1500), Some(0.0001));
    }

    #[test]
    fn test_funding_schedule_rate_at_before_first() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);

        assert_eq!(schedule.rate_at(500), None);
    }

    #[test]
    fn test_funding_schedule_calculate_payment() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001); // 0.01%

        let notional = 10000.0;
        let payment = schedule.calculate_payment(notional, 1000);
        assert_eq!(payment, 1.0); // 10000 * 0.0001 = 1.0
    }

    #[test]
    fn test_funding_schedule_calculate_payment_no_rate() {
        let schedule = FundingSchedule::new();

        let notional = 10000.0;
        let payment = schedule.calculate_payment(notional, 1000);
        assert_eq!(payment, 0.0); // No rate available
    }

    #[test]
    fn test_funding_schedule_calculate_payment_large_notional() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);

        let notional = 1_000_000.0; // $1M
        let payment = schedule.calculate_payment(notional, 1000);
        assert_eq!(payment, 100.0); // 1M * 0.0001 = 100
    }

    #[test]
    fn test_funding_schedule_timestamps_in_range() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);
        schedule.add_point(2000, 0.0002);
        schedule.add_point(3000, 0.00015);
        schedule.add_point(4000, 0.0003);

        let timestamps = schedule.timestamps_in_range(1500, 3500);
        assert_eq!(timestamps.len(), 2);
        assert_eq!(timestamps[0], 2000);
        assert_eq!(timestamps[1], 3000);
    }

    #[test]
    fn test_funding_schedule_timestamps_in_range_empty() {
        let schedule = FundingSchedule::new();

        let timestamps = schedule.timestamps_in_range(1000, 2000);
        assert_eq!(timestamps.len(), 0);
    }

    #[test]
    fn test_funding_schedule_timestamps_in_range_no_overlap() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);
        schedule.add_point(2000, 0.0002);

        let timestamps = schedule.timestamps_in_range(5000, 6000);
        assert_eq!(timestamps.len(), 0);
    }

    #[test]
    fn test_funding_schedule_unsorted_points() {
        let mut schedule = FundingSchedule::new();
        // Add points out of order
        schedule.add_point(3000, 0.00015);
        schedule.add_point(1000, 0.0001);
        schedule.add_point(2000, 0.0002);

        // Should still work correctly (points are sorted internally)
        assert_eq!(schedule.rate_at(1500), Some(0.0001));
        assert_eq!(schedule.rate_at(2500), Some(0.0002));
        assert_eq!(schedule.rate_at(3500), Some(0.00015));
    }

    #[test]
    fn test_funding_schedule_negative_rate() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, -0.0001); // Negative funding rate

        let notional = 10000.0;
        let payment = schedule.calculate_payment(notional, 1000);
        assert_eq!(payment, -1.0); // Negative payment
    }

    #[test]
    fn test_funding_schedule_zero_rate() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0); // Zero funding rate

        let notional = 10000.0;
        let payment = schedule.calculate_payment(notional, 1000);
        assert_eq!(payment, 0.0);
    }

    #[test]
    fn test_funding_schedule_duplicate_timestamps() {
        let mut schedule = FundingSchedule::new();
        schedule.add_point(1000, 0.0001);
        schedule.add_point(1000, 0.0002); // Duplicate timestamp

        // Should use the last added rate for that timestamp
        // (implementation-dependent, but should not panic)
        let rate = schedule.rate_at(1000);
        assert!(rate.is_some());
    }
}
