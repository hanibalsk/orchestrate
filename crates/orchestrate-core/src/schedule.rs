//! Schedule types and data model
//!
//! This module defines the data structures for scheduled agent execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::{CronSchedule, Error};

/// A scheduled agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Unique schedule ID
    pub id: i64,
    /// Human-readable name
    pub name: String,
    /// Cron expression for scheduling
    pub cron_expression: String,
    /// Type of agent to execute
    pub agent_type: String,
    /// Task description for the agent
    pub task: String,
    /// Whether the schedule is enabled
    pub enabled: bool,
    /// Last execution time
    pub last_run: Option<DateTime<Utc>>,
    /// Next scheduled execution time
    pub next_run: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Schedule {
    /// Create a new schedule
    pub fn new(name: String, cron_expression: String, agent_type: String, task: String) -> Self {
        Self {
            id: 0, // Will be set by database
            name,
            cron_expression,
            agent_type,
            task,
            enabled: true,
            last_run: None,
            next_run: None,
            created_at: Utc::now(),
        }
    }

    /// Calculate the next run time based on the cron expression
    ///
    /// This will validate the cron expression and calculate the next execution time
    /// from the current time (or from last_run if provided).
    ///
    /// # Returns
    /// The next scheduled execution time, or an error if the cron expression is invalid
    pub fn calculate_next_run(&self) -> Result<DateTime<Utc>, Error> {
        let cron = CronSchedule::new(&self.cron_expression)?;
        let from = self.last_run.unwrap_or_else(Utc::now);
        cron.next_after(&from)
    }

    /// Update the next_run field based on the cron expression
    ///
    /// This is a convenience method that calculates and sets the next_run field.
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if the cron expression is invalid
    pub fn update_next_run(&mut self) -> Result<(), Error> {
        self.next_run = Some(self.calculate_next_run()?);
        Ok(())
    }

    /// Validate the cron expression without calculating next run
    ///
    /// # Returns
    /// Ok(()) if the cron expression is valid, or an error otherwise
    pub fn validate_cron(&self) -> Result<(), Error> {
        CronSchedule::validate(&self.cron_expression)
    }
}

/// A schedule execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRun {
    /// Unique run ID
    pub id: i64,
    /// Schedule ID
    pub schedule_id: i64,
    /// Agent ID that was created
    pub agent_id: Option<String>,
    /// Execution start time
    pub started_at: DateTime<Utc>,
    /// Execution end time
    pub completed_at: Option<DateTime<Utc>>,
    /// Execution status
    pub status: ScheduleRunStatus,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl ScheduleRun {
    /// Create a new schedule run record
    pub fn new(schedule_id: i64) -> Self {
        Self {
            id: 0, // Will be set by database
            schedule_id,
            agent_id: None,
            started_at: Utc::now(),
            completed_at: None,
            status: ScheduleRunStatus::Running,
            error_message: None,
        }
    }

    /// Mark the run as completed
    pub fn mark_completed(&mut self, agent_id: String) {
        self.agent_id = Some(agent_id);
        self.completed_at = Some(Utc::now());
        self.status = ScheduleRunStatus::Completed;
    }

    /// Mark the run as failed
    pub fn mark_failed(&mut self, error: String) {
        self.completed_at = Some(Utc::now());
        self.status = ScheduleRunStatus::Failed;
        self.error_message = Some(error);
    }
}

/// Status of a schedule run
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduleRunStatus {
    /// Run is in progress
    Running,
    /// Run completed successfully
    Completed,
    /// Run failed
    Failed,
}

impl ScheduleRunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ScheduleRunStatus::Running => "running",
            ScheduleRunStatus::Completed => "completed",
            ScheduleRunStatus::Failed => "failed",
        }
    }
}

impl std::str::FromStr for ScheduleRunStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(ScheduleRunStatus::Running),
            "completed" => Ok(ScheduleRunStatus::Completed),
            "failed" => Ok(ScheduleRunStatus::Failed),
            _ => Err(crate::Error::Other(format!("Invalid schedule run status: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_new() {
        let schedule = Schedule::new(
            "daily-backup".to_string(),
            "0 0 * * *".to_string(),
            "BackgroundController".to_string(),
            "Run daily backup".to_string(),
        );

        assert_eq!(schedule.name, "daily-backup");
        assert_eq!(schedule.cron_expression, "0 0 * * *");
        assert_eq!(schedule.agent_type, "BackgroundController");
        assert_eq!(schedule.task, "Run daily backup");
        assert!(schedule.enabled);
        assert!(schedule.last_run.is_none());
        assert!(schedule.next_run.is_none());
    }

    #[test]
    fn test_schedule_run_status_as_str() {
        assert_eq!(ScheduleRunStatus::Running.as_str(), "running");
        assert_eq!(ScheduleRunStatus::Completed.as_str(), "completed");
        assert_eq!(ScheduleRunStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_schedule_run_status_from_str() {
        assert_eq!(
            "running".parse::<ScheduleRunStatus>().unwrap(),
            ScheduleRunStatus::Running
        );
        assert_eq!(
            "completed".parse::<ScheduleRunStatus>().unwrap(),
            ScheduleRunStatus::Completed
        );
        assert_eq!(
            "failed".parse::<ScheduleRunStatus>().unwrap(),
            ScheduleRunStatus::Failed
        );
        assert!("invalid".parse::<ScheduleRunStatus>().is_err());
    }

    #[test]
    fn test_schedule_validate_cron() {
        let schedule = Schedule::new(
            "test".to_string(),
            "0 2 * * *".to_string(),
            "TestAgent".to_string(),
            "Test task".to_string(),
        );

        assert!(schedule.validate_cron().is_ok());
    }

    #[test]
    fn test_schedule_validate_cron_invalid() {
        let schedule = Schedule::new(
            "test".to_string(),
            "invalid cron".to_string(),
            "TestAgent".to_string(),
            "Test task".to_string(),
        );

        assert!(schedule.validate_cron().is_err());
    }

    #[test]
    fn test_schedule_calculate_next_run() {
        let schedule = Schedule::new(
            "hourly-test".to_string(),
            "@hourly".to_string(),
            "TestAgent".to_string(),
            "Test task".to_string(),
        );

        let next_run = schedule.calculate_next_run().unwrap();
        let now = Utc::now();

        // Next run should be within the next hour
        let duration = next_run.signed_duration_since(now);
        assert!(duration.num_seconds() > 0);
        assert!(duration.num_seconds() <= 3600);
    }

    #[test]
    fn test_schedule_update_next_run() {
        let mut schedule = Schedule::new(
            "daily-test".to_string(),
            "@daily".to_string(),
            "TestAgent".to_string(),
            "Test task".to_string(),
        );

        assert!(schedule.next_run.is_none());

        schedule.update_next_run().unwrap();

        assert!(schedule.next_run.is_some());
        let next_run = schedule.next_run.unwrap();
        let now = Utc::now();

        // Next run should be in the future
        assert!(next_run > now);
    }

    #[test]
    fn test_schedule_calculate_next_run_with_last_run() {
        use chrono::TimeZone;

        let mut schedule = Schedule::new(
            "test".to_string(),
            "0 0 * * *".to_string(), // Daily at midnight
            "TestAgent".to_string(),
            "Test task".to_string(),
        );

        // Set last_run to a specific time
        schedule.last_run = Some(Utc.with_ymd_and_hms(2025, 1, 15, 0, 0, 0).unwrap());

        let next_run = schedule.calculate_next_run().unwrap();

        // Next run should be Jan 16 at midnight
        assert_eq!(next_run, Utc.with_ymd_and_hms(2025, 1, 16, 0, 0, 0).unwrap());
    }
}
