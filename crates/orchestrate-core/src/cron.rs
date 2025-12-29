//! Cron expression parsing and scheduling
//!
//! This module provides functionality to parse cron expressions and calculate
//! next run times for scheduled tasks.

use chrono::{DateTime, Utc};
use cron::Schedule as CronLib;
use crate::Error;
use std::str::FromStr;

/// Parses and manages cron expressions for scheduling
#[derive(Debug, Clone)]
pub struct CronSchedule {
    expression: String,
    schedule: CronLib,
}

impl CronSchedule {
    /// Create a new cron schedule from an expression
    ///
    /// Supports:
    /// - Standard 5-field cron expressions (min hour day month weekday)
    /// - Extended syntax: @daily, @weekly, @hourly, @monthly, @yearly
    ///
    /// # Examples
    /// ```no_run
    /// use orchestrate_core::CronSchedule;
    ///
    /// let schedule = CronSchedule::new("0 2 * * *").unwrap(); // Daily at 2 AM
    /// let schedule = CronSchedule::new("@daily").unwrap();     // Same as above
    /// ```
    pub fn new(expression: &str) -> Result<Self, Error> {
        // Expand extended syntax to standard cron
        let expanded = expand_extended_syntax(expression);

        // Parse the cron expression
        let schedule = CronLib::from_str(&expanded)
            .map_err(|e| Error::Other(format!("Invalid cron expression '{}': {}", expression, e)))?;

        Ok(Self {
            expression: expression.to_string(),
            schedule,
        })
    }

    /// Calculate the next run time from a given time
    ///
    /// # Arguments
    /// * `from` - The time to calculate from (typically current time)
    ///
    /// # Returns
    /// The next scheduled execution time
    pub fn next_after(&self, from: &DateTime<Utc>) -> Result<DateTime<Utc>, Error> {
        self.schedule
            .after(from)
            .next()
            .ok_or_else(|| Error::Other("Failed to calculate next run time".to_string()))
    }

    /// Get the raw cron expression
    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// Validate a cron expression without creating a schedule
    pub fn validate(expression: &str) -> Result<(), Error> {
        Self::new(expression)?;
        Ok(())
    }
}

/// Expand extended cron syntax (@daily, @weekly, etc.) to standard cron
///
/// The cron library expects 6 fields: sec min hour day month weekday
/// We convert 5-field expressions (min hour day month weekday) to 6-field
/// Note: This library uses 1-7 for weekdays where 1=Sunday, 2=Monday, ... 7=Saturday
fn expand_extended_syntax(expression: &str) -> String {
    match expression {
        "@yearly" | "@annually" => "0 0 0 1 1 *".to_string(),  // Yearly on Jan 1 at midnight
        "@monthly" => "0 0 0 1 * *".to_string(),                // Monthly on the 1st at midnight
        "@weekly" => "0 0 0 * * 1".to_string(),                 // Weekly on Sunday (1) at midnight
        "@daily" | "@midnight" => "0 0 0 * * *".to_string(),    // Daily at midnight
        "@hourly" => "0 0 * * * *".to_string(),                 // Every hour at :00
        _ => {
            // Check if it's a 5-field cron expression and convert to 6-field
            let fields: Vec<&str> = expression.split_whitespace().collect();
            if fields.len() == 5 {
                // Convert weekday from standard cron format (0-6, Sun-Sat) to
                // this library's format (1-7, Sun-Sat)
                let mut converted_fields = vec!["0".to_string()]; // Add seconds
                for (i, field) in fields.iter().enumerate() {
                    if i == 4 {
                        // This is the weekday field
                        converted_fields.push(convert_weekday_field(field));
                    } else {
                        converted_fields.push(field.to_string());
                    }
                }
                converted_fields.join(" ")
            } else {
                expression.to_string()
            }
        }
    }
}

/// Convert weekday field from standard cron format (0-6, Sun-Sat) to
/// the format expected by the cron library (1-7, Sun-Sat)
/// Mapping: 0->1 (Sun), 1->2 (Mon), 2->3 (Tue), 3->4 (Wed), 4->5 (Thu), 5->6 (Fri), 6->7 (Sat)
fn convert_weekday_field(field: &str) -> String {
    // Handle wildcards - pass through as-is
    if field == "*" {
        return field.to_string();
    }

    // Handle step values like */2 or 0-6/2
    if field.contains('/') {
        // For now, just increment the base numbers
        let parts: Vec<&str> = field.split('/').collect();
        if parts.len() == 2 {
            let base = convert_weekday_simple(parts[0]);
            return format!("{}/{}", base, parts[1]);
        }
        return field.to_string();
    }

    // Handle comma-separated values
    if field.contains(',') {
        let parts: Vec<String> = field.split(',')
            .map(|s| convert_weekday_simple(s))
            .collect();
        return parts.join(",");
    }

    // Handle ranges
    if field.contains('-') {
        let parts: Vec<&str> = field.split('-').collect();
        if parts.len() == 2 {
            let start = convert_weekday_simple(parts[0]);
            let end = convert_weekday_simple(parts[1]);
            return format!("{}-{}", start, end);
        }
    }

    // Simple single value
    convert_weekday_simple(field)
}

/// Convert a simple weekday value (0-6) to (1-7)
fn convert_weekday_simple(value: &str) -> String {
    match value {
        "0" => "1".to_string(),  // Sunday
        "1" => "2".to_string(),  // Monday
        "2" => "3".to_string(),  // Tuesday
        "3" => "4".to_string(),  // Wednesday
        "4" => "5".to_string(),  // Thursday
        "5" => "6".to_string(),  // Friday
        "6" => "7".to_string(),  // Saturday
        "*" => "*".to_string(),  // Wildcard
        _ => value.to_string(),  // Pass through anything else (might be already converted or an error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_parse_standard_cron_daily_at_2am() {
        let schedule = CronSchedule::new("0 2 * * *");
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_parse_standard_cron_weekly_sunday() {
        let schedule = CronSchedule::new("0 0 * * 0");
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_parse_standard_cron_every_15_minutes() {
        let schedule = CronSchedule::new("*/15 * * * *");
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_parse_extended_syntax_daily() {
        let schedule = CronSchedule::new("@daily");
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_parse_extended_syntax_weekly() {
        let schedule = CronSchedule::new("@weekly");
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_parse_extended_syntax_hourly() {
        let schedule = CronSchedule::new("@hourly");
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_parse_extended_syntax_monthly() {
        let schedule = CronSchedule::new("@monthly");
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_parse_extended_syntax_yearly() {
        let schedule = CronSchedule::new("@yearly");
        assert!(schedule.is_ok());
    }

    #[test]
    fn test_parse_invalid_expression() {
        let schedule = CronSchedule::new("invalid cron");
        assert!(schedule.is_err());
    }

    #[test]
    fn test_parse_invalid_field_count() {
        let schedule = CronSchedule::new("0 0 0");
        assert!(schedule.is_err());
    }

    #[test]
    fn test_parse_invalid_field_value() {
        let schedule = CronSchedule::new("60 0 * * *"); // Minutes can't be 60
        assert!(schedule.is_err());
    }

    #[test]
    fn test_next_run_daily_at_2am() {
        let schedule = CronSchedule::new("0 2 * * *").unwrap();

        // Current time is 2025-01-15 01:00:00 UTC
        let now = Utc.with_ymd_and_hms(2025, 1, 15, 1, 0, 0).unwrap();
        let next = schedule.next_after(&now).unwrap();

        // Next run should be 2025-01-15 02:00:00 UTC (same day)
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 15, 2, 0, 0).unwrap());
    }

    #[test]
    fn test_next_run_daily_at_2am_after_2am() {
        let schedule = CronSchedule::new("0 2 * * *").unwrap();

        // Current time is 2025-01-15 03:00:00 UTC (after 2 AM)
        let now = Utc.with_ymd_and_hms(2025, 1, 15, 3, 0, 0).unwrap();
        let next = schedule.next_after(&now).unwrap();

        // Next run should be 2025-01-16 02:00:00 UTC (next day)
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 16, 2, 0, 0).unwrap());
    }

    #[test]
    fn test_next_run_every_15_minutes() {
        let schedule = CronSchedule::new("*/15 * * * *").unwrap();

        // Current time is 2025-01-15 10:07:30 UTC
        let now = Utc.with_ymd_and_hms(2025, 1, 15, 10, 7, 30).unwrap();
        let next = schedule.next_after(&now).unwrap();

        // Next run should be 2025-01-15 10:15:00 UTC
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 15, 10, 15, 0).unwrap());
    }

    #[test]
    fn test_next_run_weekly_sunday() {
        let schedule = CronSchedule::new("0 0 * * 0").unwrap();

        // Current time is Wednesday 2025-01-15 10:00:00 UTC
        let now = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let next = schedule.next_after(&now).unwrap();

        // Next run should be Sunday 2025-01-19 00:00:00 UTC
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 19, 0, 0, 0).unwrap());
    }

    #[test]
    fn test_next_run_extended_daily() {
        let schedule = CronSchedule::new("@daily").unwrap();

        // Current time is 2025-01-15 10:00:00 UTC
        let now = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let next = schedule.next_after(&now).unwrap();

        // Next run should be 2025-01-16 00:00:00 UTC (midnight next day)
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 16, 0, 0, 0).unwrap());
    }

    #[test]
    fn test_next_run_extended_hourly() {
        let schedule = CronSchedule::new("@hourly").unwrap();

        // Current time is 2025-01-15 10:30:00 UTC
        let now = Utc.with_ymd_and_hms(2025, 1, 15, 10, 30, 0).unwrap();
        let next = schedule.next_after(&now).unwrap();

        // Next run should be 2025-01-15 11:00:00 UTC
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 15, 11, 0, 0).unwrap());
    }

    #[test]
    fn test_next_run_extended_weekly() {
        let schedule = CronSchedule::new("@weekly").unwrap();

        // Current time is Wednesday 2025-01-15 10:00:00 UTC
        let now = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let next = schedule.next_after(&now).unwrap();

        // Next run should be Sunday 2025-01-19 00:00:00 UTC
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 1, 19, 0, 0, 0).unwrap());
    }

    #[test]
    fn test_validate_valid_expression() {
        assert!(CronSchedule::validate("0 2 * * *").is_ok());
        assert!(CronSchedule::validate("@daily").is_ok());
    }

    #[test]
    fn test_validate_invalid_expression() {
        assert!(CronSchedule::validate("invalid").is_err());
        assert!(CronSchedule::validate("60 0 * * *").is_err());
    }

    #[test]
    fn test_expression_getter() {
        let schedule = CronSchedule::new("0 2 * * *").unwrap();
        assert_eq!(schedule.expression(), "0 2 * * *");
    }
}
