//! Schedule types and data model
//!
//! This module defines the data structures for scheduled agent execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}
