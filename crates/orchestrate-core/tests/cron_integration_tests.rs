//! Integration tests for cron expression parsing and schedule integration

use chrono::{TimeZone, Utc};
use orchestrate_core::{CronSchedule, Schedule};

#[test]
fn test_cron_all_extended_syntax() {
    // Test all extended syntax forms
    let expressions = vec![
        "@hourly",
        "@daily",
        "@weekly",
        "@monthly",
        "@yearly",
    ];

    for expr in expressions {
        let cron = CronSchedule::new(expr);
        assert!(cron.is_ok(), "Failed to parse: {}", expr);

        let cron = cron.unwrap();
        let now = Utc::now();
        let next = cron.next_after(&now);
        assert!(next.is_ok(), "Failed to calculate next run for: {}", expr);
        assert!(next.unwrap() > now, "Next run should be in the future for: {}", expr);
    }
}

#[test]
fn test_cron_common_patterns() {
    // Test common real-world cron patterns
    let patterns = vec![
        ("0 2 * * *", "Daily at 2 AM"),
        ("0 */6 * * *", "Every 6 hours"),
        ("*/15 * * * *", "Every 15 minutes"),
        ("0 0 * * 1", "Weekly on Monday"),
        ("0 0 1 * *", "Monthly on the 1st"),
        ("0 9 * * 1-5", "Weekdays at 9 AM"),
    ];

    for (expr, description) in patterns {
        let cron = CronSchedule::new(expr);
        assert!(cron.is_ok(), "Failed to parse {}: {}", description, expr);

        let cron = cron.unwrap();
        let now = Utc::now();
        let next = cron.next_after(&now);
        assert!(next.is_ok(), "Failed to calculate next run for {}: {}", description, expr);
        assert!(next.unwrap() > now, "Next run should be in the future for {}: {}", description, expr);
    }
}

#[test]
fn test_schedule_with_cron_integration() {
    let mut schedule = Schedule::new(
        "security-scan".to_string(),
        "0 2 * * *".to_string(), // Daily at 2 AM
        "SecurityScanner".to_string(),
        "Run daily security scan".to_string(),
    );

    // Validate the cron expression
    assert!(schedule.validate_cron().is_ok());

    // Calculate next run
    let next_run = schedule.calculate_next_run();
    assert!(next_run.is_ok());

    // Update next_run field
    assert!(schedule.update_next_run().is_ok());
    assert!(schedule.next_run.is_some());
    assert!(schedule.next_run.unwrap() > Utc::now());
}

#[test]
fn test_schedule_with_last_run() {
    let mut schedule = Schedule::new(
        "backup".to_string(),
        "0 2 * * *".to_string(), // Daily at 2 AM
        "BackupAgent".to_string(),
        "Daily backup".to_string(),
    );

    // Set last run to yesterday at 2 AM
    let yesterday = Utc.with_ymd_and_hms(2025, 1, 14, 2, 0, 0).unwrap();
    schedule.last_run = Some(yesterday);

    // Calculate next run should be based on yesterday
    let next_run = schedule.calculate_next_run().unwrap();

    // Next run should be today at 2 AM (Jan 15, 2025 at 2 AM)
    let expected_next = Utc.with_ymd_and_hms(2025, 1, 15, 2, 0, 0).unwrap();
    assert_eq!(next_run, expected_next);
}

#[test]
fn test_schedule_invalid_cron() {
    let schedule = Schedule::new(
        "invalid".to_string(),
        "not a cron expression".to_string(),
        "TestAgent".to_string(),
        "Test".to_string(),
    );

    assert!(schedule.validate_cron().is_err());
    assert!(schedule.calculate_next_run().is_err());
}

#[test]
fn test_weekday_conversion() {
    // Sunday in standard cron is 0, but we convert to 1 for the library
    let schedule = CronSchedule::new("0 0 * * 0").unwrap(); // Sunday

    let wednesday = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
    let next = schedule.next_after(&wednesday).unwrap();

    // Next Sunday is Jan 19
    let expected = Utc.with_ymd_and_hms(2025, 1, 19, 0, 0, 0).unwrap();
    assert_eq!(next, expected);
}

#[test]
fn test_multiple_schedules_different_frequencies() {
    let schedules = vec![
        ("hourly-task", "@hourly"),
        ("daily-task", "@daily"),
        ("weekly-task", "@weekly"),
    ];

    let now = Utc::now();
    let mut next_runs = Vec::new();

    for (name, expr) in schedules {
        let mut schedule = Schedule::new(
            name.to_string(),
            expr.to_string(),
            "TestAgent".to_string(),
            "Test".to_string(),
        );

        schedule.update_next_run().unwrap();
        next_runs.push((name, schedule.next_run.unwrap()));
    }

    // Verify they're all in the future
    for (name, next_run) in &next_runs {
        assert!(next_run > &now, "{} next run should be in the future", name);
    }

    // Verify hourly is sooner than daily, which is sooner than weekly
    assert!(next_runs[0].1 < next_runs[1].1, "Hourly should run before daily");
    assert!(next_runs[1].1 < next_runs[2].1, "Daily should run before weekly");
}
