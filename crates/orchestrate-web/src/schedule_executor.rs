//! Schedule executor service
//!
//! Background service that polls for due schedules and executes them.
//!
//! ## Overview
//!
//! The schedule executor is a background service that:
//! - Polls the database for schedules that are due for execution
//! - Spawns agents for each due schedule
//! - Records execution history in the schedule_runs table
//! - Updates schedule metadata (last_run, next_run)
//! - Prevents concurrent execution of the same schedule using database locks
//!
//! ## Concurrency
//!
//! The executor uses database-level locking to prevent the same schedule from being
//! executed concurrently by multiple executor instances. Locks expire after 5 minutes
//! to prevent deadlocks in case of crashes.
//!
//! ## Configuration
//!
//! The executor can be configured with:
//! - `poll_interval_secs`: How often to check for due schedules (default: 60s)
//!
//! ## Example
//!
//! ```rust,no_run
//! use orchestrate_web::{ScheduleExecutor, ScheduleExecutorConfig};
//! use orchestrate_core::Database;
//! use std::sync::Arc;
//!
//! # async fn example() {
//! let database = Arc::new(Database::new("orchestrate.db").await.unwrap());
//! let config = ScheduleExecutorConfig::default();
//! let executor = ScheduleExecutor::new(database, config);
//!
//! // Run the executor (this will block)
//! executor.run().await;
//! # }
//! ```

use orchestrate_core::{Agent, AgentType, Database, Schedule, ScheduleRun};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

/// Policy for handling missed schedules
#[derive(Clone, Debug, PartialEq)]
pub enum MissedSchedulePolicy {
    /// Run the schedule immediately once
    RunImmediately,
    /// Skip missed runs and wait for next scheduled time
    Skip,
    /// Catch up missed runs (up to a limit)
    CatchUp,
}

impl Default for MissedSchedulePolicy {
    fn default() -> Self {
        Self::RunImmediately
    }
}

/// Schedule executor configuration
#[derive(Clone, Debug)]
pub struct ScheduleExecutorConfig {
    /// Polling interval in seconds
    pub poll_interval_secs: u64,
    /// Policy for handling missed schedules
    pub missed_policy: MissedSchedulePolicy,
    /// Maximum number of catch-up runs
    pub catch_up_limit: usize,
}

impl Default for ScheduleExecutorConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
            missed_policy: MissedSchedulePolicy::RunImmediately,
            catch_up_limit: 3,
        }
    }
}

/// Schedule executor service
pub struct ScheduleExecutor {
    database: Arc<Database>,
    config: ScheduleExecutorConfig,
}

impl ScheduleExecutor {
    /// Create a new schedule executor
    pub fn new(database: Arc<Database>, config: ScheduleExecutorConfig) -> Self {
        Self { database, config }
    }

    /// Run the executor loop (blocking)
    pub async fn run(&self) {
        info!(
            poll_interval_secs = self.config.poll_interval_secs,
            "Starting schedule executor"
        );

        loop {
            if let Err(e) = self.check_and_execute().await {
                error!(error = %e, "Error checking and executing schedules");
            }

            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }

    /// Check for due schedules and execute them
    pub async fn check_and_execute(&self) -> orchestrate_core::Result<()> {
        let due_schedules = self.database.get_due_schedules().await?;

        if due_schedules.is_empty() {
            debug!("No schedules are due for execution");
            return Ok(());
        }

        info!(count = due_schedules.len(), "Found due schedules");

        for schedule in due_schedules {
            if let Err(e) = self.execute_schedule(schedule).await {
                error!(
                    error = %e,
                    "Failed to execute schedule"
                );
            }
        }

        Ok(())
    }

    /// Execute a single schedule
    async fn execute_schedule(&self, mut schedule: Schedule) -> orchestrate_core::Result<()> {
        let schedule_id = schedule.id;
        let schedule_name = schedule.name.clone();

        // Try to acquire lock for this schedule
        let locked = self.database.try_lock_schedule(schedule_id).await?;
        if !locked {
            debug!(
                schedule_id = schedule_id,
                schedule_name = %schedule_name,
                "Schedule is already being executed, skipping"
            );
            return Ok(());
        }

        // Check if this is a missed schedule
        let now = chrono::Utc::now();
        let next_run = schedule.next_run.unwrap_or(now);
        let is_missed = next_run < now;

        if is_missed {
            let missed_duration = now.signed_duration_since(next_run);
            warn!(
                schedule_id = schedule_id,
                schedule_name = %schedule_name,
                missed_duration_secs = missed_duration.num_seconds(),
                policy = ?self.config.missed_policy,
                "Schedule missed its run time"
            );

            // Handle based on policy
            match self.config.missed_policy {
                MissedSchedulePolicy::Skip => {
                    info!(
                        schedule_id = schedule_id,
                        schedule_name = %schedule_name,
                        "Skipping missed schedule per policy"
                    );

                    // Just update next_run without executing
                    schedule.last_run = Some(now);
                    schedule.update_next_run()?;
                    self.database.update_schedule(&schedule).await?;
                    self.database.unlock_schedule(schedule_id).await?;

                    return Ok(());
                }
                MissedSchedulePolicy::RunImmediately => {
                    info!(
                        schedule_id = schedule_id,
                        schedule_name = %schedule_name,
                        "Running missed schedule immediately"
                    );

                    // Run once and update
                    self.execute_schedule_once(&mut schedule).await?;
                }
                MissedSchedulePolicy::CatchUp => {
                    info!(
                        schedule_id = schedule_id,
                        schedule_name = %schedule_name,
                        "Catching up missed schedule runs"
                    );

                    // Calculate how many runs were missed
                    let missed_count = self.calculate_missed_runs(&schedule, now).await?;
                    let runs_to_execute = missed_count.min(self.config.catch_up_limit);

                    info!(
                        schedule_id = schedule_id,
                        missed_count = missed_count,
                        runs_to_execute = runs_to_execute,
                        catch_up_limit = self.config.catch_up_limit,
                        "Executing catch-up runs"
                    );

                    // Execute multiple times
                    for i in 0..runs_to_execute {
                        debug!(
                            schedule_id = schedule_id,
                            run_number = i + 1,
                            total = runs_to_execute,
                            "Executing catch-up run"
                        );
                        self.execute_schedule_once(&mut schedule).await?;
                    }
                }
            }
        } else {
            // Normal execution for schedules that are due
            info!(
                schedule_id = schedule_id,
                schedule_name = %schedule_name,
                "Executing schedule"
            );

            self.execute_schedule_once(&mut schedule).await?;
        }

        // Release the lock
        self.database.unlock_schedule(schedule_id).await?;

        info!(
            schedule_id = schedule_id,
            next_run = ?schedule.next_run,
            "Updated schedule for next run"
        );

        Ok(())
    }

    /// Execute a schedule once
    async fn execute_schedule_once(&self, schedule: &mut Schedule) -> orchestrate_core::Result<()> {
        let schedule_id = schedule.id;

        // Create a schedule run record
        let mut run = ScheduleRun::new(schedule_id);
        let run_id = self.database.insert_schedule_run(&run).await?;
        run.id = run_id;

        // Try to execute the schedule
        match self.spawn_agent(schedule).await {
            Ok(agent_id) => {
                info!(
                    schedule_id = schedule_id,
                    agent_id = %agent_id,
                    "Successfully spawned agent for schedule"
                );

                // Mark run as completed
                run.mark_completed(agent_id.to_string());
                self.database.update_schedule_run(&run).await?;
            }
            Err(e) => {
                warn!(
                    schedule_id = schedule_id,
                    error = %e,
                    "Failed to spawn agent for schedule"
                );

                // Mark run as failed
                run.mark_failed(e.to_string());
                self.database.update_schedule_run(&run).await?;
            }
        }

        // Update schedule: set last_run and calculate next_run
        schedule.last_run = Some(chrono::Utc::now());
        schedule.update_next_run()?;

        self.database.update_schedule(schedule).await?;

        Ok(())
    }

    /// Calculate how many runs were missed
    async fn calculate_missed_runs(
        &self,
        schedule: &Schedule,
        now: chrono::DateTime<chrono::Utc>,
    ) -> orchestrate_core::Result<usize> {
        use orchestrate_core::CronSchedule;

        let next_run = schedule.next_run.unwrap_or(now);
        if next_run >= now {
            return Ok(0);
        }

        let cron = CronSchedule::new(&schedule.cron_expression)?;
        let mut count = 0;
        let mut current = next_run;

        // Count how many runs were missed
        // We iterate from the last known next_run and count all occurrences
        // that should have happened but are now in the past
        loop {
            let next_occurrence = cron.next_after(&current)?;

            if next_occurrence >= now {
                // We've caught up to the present
                break;
            }

            count += 1;

            if count >= 100 {
                // Cap at 100 to prevent infinite loops
                break;
            }

            current = next_occurrence;
        }

        Ok(count)
    }

    /// Spawn an agent for the given schedule
    async fn spawn_agent(&self, schedule: &Schedule) -> orchestrate_core::Result<uuid::Uuid> {
        // Parse agent type from string
        let agent_type = AgentType::from_str(&schedule.agent_type)?;

        // Create the agent
        let agent = Agent::new(agent_type, schedule.task.clone());
        let agent_id = agent.id;

        // Insert into database
        self.database.insert_agent(&agent).await?;

        Ok(agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use orchestrate_core::ScheduleRunStatus;

    #[tokio::test]
    async fn test_executor_executes_due_schedule() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule that is due
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@hourly".to_string(),
            "background_controller".to_string(),
            "Test scheduled task".to_string(),
        );

        // Set next_run to past time so it's due
        schedule.next_run = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let schedule_id = database.insert_schedule(&schedule).await.unwrap();
        schedule.id = schedule_id;

        // Execute
        let executor = ScheduleExecutor::new(database.clone(), ScheduleExecutorConfig::default());
        executor.check_and_execute().await.unwrap();

        // Verify schedule was executed
        let updated_schedule = database.get_schedule(schedule_id).await.unwrap().unwrap();

        // last_run should be set
        assert!(updated_schedule.last_run.is_some());

        // next_run should be updated to future
        assert!(updated_schedule.next_run.is_some());
        assert!(updated_schedule.next_run.unwrap() > Utc::now());

        // Verify agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_type, AgentType::BackgroundController);
        assert_eq!(agents[0].task, "Test scheduled task");

        // Verify schedule run was recorded
        let runs = database.get_schedule_runs(schedule_id, 10).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].schedule_id, schedule_id);
        assert!(runs[0].agent_id.is_some());
        assert_eq!(runs[0].status, ScheduleRunStatus::Completed);
    }

    #[tokio::test]
    async fn test_executor_skips_future_schedule() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule that is not due yet
        let mut schedule = Schedule::new(
            "future-schedule".to_string(),
            "@daily".to_string(),
            "background_controller".to_string(),
            "Future task".to_string(),
        );

        // Set next_run to future time
        schedule.next_run = Some(Utc::now() + chrono::Duration::hours(24));
        database.insert_schedule(&schedule).await.unwrap();

        // Execute
        let executor = ScheduleExecutor::new(database.clone(), ScheduleExecutorConfig::default());
        executor.check_and_execute().await.unwrap();

        // No agent should be created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_executor_skips_disabled_schedule() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a disabled schedule that is due
        let mut schedule = Schedule::new(
            "disabled-schedule".to_string(),
            "@hourly".to_string(),
            "background_controller".to_string(),
            "Disabled task".to_string(),
        );

        schedule.enabled = false;
        schedule.next_run = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        database.insert_schedule(&schedule).await.unwrap();

        // Execute
        let executor = ScheduleExecutor::new(database.clone(), ScheduleExecutorConfig::default());
        executor.check_and_execute().await.unwrap();

        // No agent should be created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_executor_handles_multiple_due_schedules() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create multiple schedules that are due
        for i in 1..=3 {
            let mut schedule = Schedule::new(
                format!("schedule-{}", i),
                "@hourly".to_string(),
                "background_controller".to_string(),
                format!("Task {}", i),
            );
            schedule.next_run = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
            database.insert_schedule(&schedule).await.unwrap();
        }

        // Execute
        let executor = ScheduleExecutor::new(database.clone(), ScheduleExecutorConfig::default());
        executor.check_and_execute().await.unwrap();

        // All three agents should be created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 3);
    }

    #[tokio::test]
    async fn test_executor_handles_execution_error() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule with invalid agent type
        let mut schedule = Schedule::new(
            "error-schedule".to_string(),
            "@hourly".to_string(),
            "InvalidAgentType".to_string(),
            "This should fail".to_string(),
        );
        schedule.next_run = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let schedule_id = database.insert_schedule(&schedule).await.unwrap();

        // Execute
        let executor = ScheduleExecutor::new(database.clone(), ScheduleExecutorConfig::default());
        executor.check_and_execute().await.unwrap();

        // Schedule should still be updated (next_run moved forward)
        let updated_schedule = database.get_schedule(schedule_id).await.unwrap().unwrap();
        assert!(updated_schedule.last_run.is_some());

        // Verify schedule run was recorded as failed
        let runs = database.get_schedule_runs(schedule_id, 10).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, ScheduleRunStatus::Failed);
        assert!(runs[0].error_message.is_some());
    }

    #[tokio::test]
    async fn test_executor_handles_empty_schedules() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Execute with no schedules
        let executor = ScheduleExecutor::new(database.clone(), ScheduleExecutorConfig::default());
        executor.check_and_execute().await.unwrap();

        // Should not error
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_executor_prevents_concurrent_execution() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule that is due
        let mut schedule = Schedule::new(
            "concurrent-test".to_string(),
            "@hourly".to_string(),
            "background_controller".to_string(),
            "Concurrent test task".to_string(),
        );
        schedule.next_run = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let schedule_id = database.insert_schedule(&schedule).await.unwrap();

        // Create two executors running concurrently
        let executor1 = ScheduleExecutor::new(database.clone(), ScheduleExecutorConfig::default());
        let executor2 = ScheduleExecutor::new(database.clone(), ScheduleExecutorConfig::default());

        // Run both concurrently
        let (result1, result2) = tokio::join!(
            executor1.check_and_execute(),
            executor2.check_and_execute()
        );

        // Both should succeed (one acquires lock, other skips)
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Only one agent should be created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        // Only one schedule run should be recorded
        let runs = database.get_schedule_runs(schedule_id, 10).await.unwrap();
        assert_eq!(runs.len(), 1);
    }

    #[tokio::test]
    async fn test_missed_schedule_run_immediately_policy() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule that missed its run time (2 hours ago)
        let mut schedule = Schedule::new(
            "missed-schedule".to_string(),
            "@hourly".to_string(),
            "background_controller".to_string(),
            "Missed task".to_string(),
        );
        schedule.next_run = Some(Utc::now() - chrono::Duration::hours(2));
        let schedule_id = database.insert_schedule(&schedule).await.unwrap();

        // Execute with RunImmediately policy (default)
        let config = ScheduleExecutorConfig {
            missed_policy: MissedSchedulePolicy::RunImmediately,
            ..Default::default()
        };
        let executor = ScheduleExecutor::new(database.clone(), config);
        executor.check_and_execute().await.unwrap();

        // Schedule should be executed immediately
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        // Verify a schedule run was recorded
        let runs = database.get_schedule_runs(schedule_id, 10).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, ScheduleRunStatus::Completed);
    }

    #[tokio::test]
    async fn test_missed_schedule_skip_policy() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule that missed its run time (2 hours ago)
        let mut schedule = Schedule::new(
            "missed-schedule".to_string(),
            "@hourly".to_string(),
            "background_controller".to_string(),
            "Missed task".to_string(),
        );
        schedule.next_run = Some(Utc::now() - chrono::Duration::hours(2));
        let schedule_id = database.insert_schedule(&schedule).await.unwrap();

        // Execute with Skip policy
        let config = ScheduleExecutorConfig {
            missed_policy: MissedSchedulePolicy::Skip,
            ..Default::default()
        };
        let executor = ScheduleExecutor::new(database.clone(), config);
        executor.check_and_execute().await.unwrap();

        // No agent should be created (skipped)
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);

        // No schedule run should be recorded
        let runs = database.get_schedule_runs(schedule_id, 10).await.unwrap();
        assert_eq!(runs.len(), 0);

        // Next run should be updated to next scheduled time
        let updated_schedule = database.get_schedule(schedule_id).await.unwrap().unwrap();
        assert!(updated_schedule.next_run.is_some());
        assert!(updated_schedule.next_run.unwrap() > Utc::now());
    }

    #[tokio::test]
    async fn test_missed_schedule_catch_up_policy() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule that missed multiple runs (every 15 mins, missed 5 times)
        let mut schedule = Schedule::new(
            "catch-up-schedule".to_string(),
            "*/15 * * * *".to_string(), // Every 15 minutes
            "background_controller".to_string(),
            "Catch-up task".to_string(),
        );
        // Set next_run to 75 minutes ago (5 missed runs)
        schedule.next_run = Some(Utc::now() - chrono::Duration::minutes(75));
        let schedule_id = database.insert_schedule(&schedule).await.unwrap();

        // Execute with CatchUp policy (limit 3)
        let config = ScheduleExecutorConfig {
            missed_policy: MissedSchedulePolicy::CatchUp,
            catch_up_limit: 3,
            ..Default::default()
        };
        let executor = ScheduleExecutor::new(database.clone(), config);
        executor.check_and_execute().await.unwrap();

        // Should execute 3 times (catch_up_limit)
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 3);

        // Verify 3 schedule runs were recorded
        let runs = database.get_schedule_runs(schedule_id, 10).await.unwrap();
        assert_eq!(runs.len(), 3);
    }

    #[tokio::test]
    async fn test_missed_schedule_catch_up_less_than_limit() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule that missed 2 runs
        let mut schedule = Schedule::new(
            "catch-up-schedule-2".to_string(),
            "@hourly".to_string(),
            "background_controller".to_string(),
            "Catch-up task 2".to_string(),
        );
        // Set next_run to 2 hours ago (2 missed runs)
        schedule.next_run = Some(Utc::now() - chrono::Duration::hours(2));
        let schedule_id = database.insert_schedule(&schedule).await.unwrap();

        // Execute with CatchUp policy (limit 3)
        let config = ScheduleExecutorConfig {
            missed_policy: MissedSchedulePolicy::CatchUp,
            catch_up_limit: 3,
            ..Default::default()
        };
        let executor = ScheduleExecutor::new(database.clone(), config);
        executor.check_and_execute().await.unwrap();

        // Should execute 2 times (actual missed runs < limit)
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 2);

        // Verify 2 schedule runs were recorded
        let runs = database.get_schedule_runs(schedule_id, 10).await.unwrap();
        assert_eq!(runs.len(), 2);
    }

    #[tokio::test]
    async fn test_missed_schedule_logging() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create a schedule that missed its run time
        let mut schedule = Schedule::new(
            "logged-missed-schedule".to_string(),
            "@hourly".to_string(),
            "background_controller".to_string(),
            "Logged missed task".to_string(),
        );
        schedule.next_run = Some(Utc::now() - chrono::Duration::hours(3));
        database.insert_schedule(&schedule).await.unwrap();

        // Execute with RunImmediately policy
        let config = ScheduleExecutorConfig {
            missed_policy: MissedSchedulePolicy::RunImmediately,
            ..Default::default()
        };
        let executor = ScheduleExecutor::new(database.clone(), config);

        // This should log the missed schedule event
        // We can't easily test the log output, but we ensure it doesn't panic
        executor.check_and_execute().await.unwrap();

        // Verify execution happened
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
    }
}
