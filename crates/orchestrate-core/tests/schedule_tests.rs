//! Tests for schedule database operations

use chrono::Utc;
use orchestrate_core::{Database, Schedule, ScheduleRun, ScheduleRunStatus};

#[tokio::test]
async fn test_insert_and_get_schedule() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "daily-backup".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Run daily backup".to_string(),
    );

    let id = db.insert_schedule(&schedule).await.unwrap();
    assert!(id > 0);

    let retrieved = db.get_schedule(id).await.unwrap().unwrap();
    assert_eq!(retrieved.name, "daily-backup");
    assert_eq!(retrieved.cron_expression, "0 0 * * *");
    assert_eq!(retrieved.agent_type, "BackgroundController");
    assert_eq!(retrieved.task, "Run daily backup");
    assert!(retrieved.enabled);
    assert!(retrieved.last_run.is_none());
    assert!(retrieved.next_run.is_none());
}

#[tokio::test]
async fn test_get_schedule_by_name() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "hourly-sync".to_string(),
        "0 * * * *".to_string(),
        "BackgroundController".to_string(),
        "Sync data hourly".to_string(),
    );

    db.insert_schedule(&schedule).await.unwrap();

    let retrieved = db.get_schedule_by_name("hourly-sync").await.unwrap().unwrap();
    assert_eq!(retrieved.name, "hourly-sync");
    assert_eq!(retrieved.cron_expression, "0 * * * *");

    // Non-existent schedule should return None
    let not_found = db.get_schedule_by_name("non-existent").await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_list_schedules() {
    let db = Database::in_memory().await.unwrap();

    let schedule1 = Schedule::new(
        "schedule-1".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Task 1".to_string(),
    );

    let mut schedule2 = Schedule::new(
        "schedule-2".to_string(),
        "0 12 * * *".to_string(),
        "BackgroundController".to_string(),
        "Task 2".to_string(),
    );
    schedule2.enabled = false;

    db.insert_schedule(&schedule1).await.unwrap();
    db.insert_schedule(&schedule2).await.unwrap();

    // List all schedules
    let all = db.list_schedules(false).await.unwrap();
    assert_eq!(all.len(), 2);

    // List only enabled schedules
    let enabled = db.list_schedules(true).await.unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].name, "schedule-1");
}

#[tokio::test]
async fn test_update_schedule() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "test-schedule".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Original task".to_string(),
    );

    let id = db.insert_schedule(&schedule).await.unwrap();
    let mut retrieved = db.get_schedule(id).await.unwrap().unwrap();

    // Update the schedule
    retrieved.task = "Updated task".to_string();
    retrieved.enabled = false;
    retrieved.last_run = Some(Utc::now());
    retrieved.next_run = Some(Utc::now());

    db.update_schedule(&retrieved).await.unwrap();

    // Verify update
    let updated = db.get_schedule(id).await.unwrap().unwrap();
    assert_eq!(updated.task, "Updated task");
    assert!(!updated.enabled);
    assert!(updated.last_run.is_some());
    assert!(updated.next_run.is_some());
}

#[tokio::test]
async fn test_delete_schedule() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "to-delete".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Delete me".to_string(),
    );

    let id = db.insert_schedule(&schedule).await.unwrap();

    // Delete should return true
    let deleted = db.delete_schedule(id).await.unwrap();
    assert!(deleted);

    // Schedule should not exist anymore
    let not_found = db.get_schedule(id).await.unwrap();
    assert!(not_found.is_none());

    // Deleting non-existent schedule should return false
    let not_deleted = db.delete_schedule(id).await.unwrap();
    assert!(!not_deleted);
}

#[tokio::test]
async fn test_insert_and_get_schedule_runs() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "test-schedule".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Test task".to_string(),
    );

    let schedule_id = db.insert_schedule(&schedule).await.unwrap();

    let run = ScheduleRun {
        id: 0,
        schedule_id,
        agent_id: None,
        started_at: Utc::now(),
        completed_at: None,
        status: ScheduleRunStatus::Running,
        error_message: None,
    };

    let run_id = db.insert_schedule_run(&run).await.unwrap();
    assert!(run_id > 0);

    // Get runs for schedule
    let runs = db.get_schedule_runs(schedule_id, 10).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].schedule_id, schedule_id);
    assert_eq!(runs[0].agent_id, None);
    assert_eq!(runs[0].status, ScheduleRunStatus::Running);
}

#[tokio::test]
async fn test_update_schedule_run() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "test-schedule".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Test task".to_string(),
    );

    let schedule_id = db.insert_schedule(&schedule).await.unwrap();

    let run = ScheduleRun {
        id: 0,
        schedule_id,
        agent_id: None,
        started_at: Utc::now(),
        completed_at: None,
        status: ScheduleRunStatus::Running,
        error_message: None,
    };

    let run_id = db.insert_schedule_run(&run).await.unwrap();

    // Update the run to completed
    let updated_run = ScheduleRun {
        id: run_id,
        schedule_id,
        agent_id: None,
        started_at: run.started_at,
        completed_at: Some(Utc::now()),
        status: ScheduleRunStatus::Completed,
        error_message: None,
    };

    db.update_schedule_run(&updated_run).await.unwrap();

    // Verify update
    let runs = db.get_schedule_runs(schedule_id, 10).await.unwrap();
    assert_eq!(runs[0].status, ScheduleRunStatus::Completed);
    assert!(runs[0].completed_at.is_some());
}

#[tokio::test]
async fn test_schedule_run_failed_status() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "test-schedule".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Test task".to_string(),
    );

    let schedule_id = db.insert_schedule(&schedule).await.unwrap();

    let run = ScheduleRun {
        id: 0,
        schedule_id,
        agent_id: None,
        started_at: Utc::now(),
        completed_at: Some(Utc::now()),
        status: ScheduleRunStatus::Failed,
        error_message: Some("Test error".to_string()),
    };

    db.insert_schedule_run(&run).await.unwrap();

    let runs = db.get_schedule_runs(schedule_id, 10).await.unwrap();
    assert_eq!(runs[0].status, ScheduleRunStatus::Failed);
    assert_eq!(runs[0].error_message, Some("Test error".to_string()));
}

#[tokio::test]
async fn test_cascade_delete_schedule_runs() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "test-schedule".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Test task".to_string(),
    );

    let schedule_id = db.insert_schedule(&schedule).await.unwrap();

    // Create multiple runs
    for _i in 0..3 {
        let run = ScheduleRun {
            id: 0,
            schedule_id,
            agent_id: None,
            started_at: Utc::now(),
            completed_at: None,
            status: ScheduleRunStatus::Running,
            error_message: None,
        };
        db.insert_schedule_run(&run).await.unwrap();
    }

    let runs = db.get_schedule_runs(schedule_id, 10).await.unwrap();
    assert_eq!(runs.len(), 3);

    // Delete the schedule
    db.delete_schedule(schedule_id).await.unwrap();

    // Runs should be deleted due to CASCADE
    let runs_after = db.get_schedule_runs(schedule_id, 10).await.unwrap();
    assert_eq!(runs_after.len(), 0);
}

#[tokio::test]
async fn test_schedule_locking() {
    let db = Database::in_memory().await.unwrap();

    let schedule = Schedule::new(
        "test-lock".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Test locking".to_string(),
    );

    let schedule_id = db.insert_schedule(&schedule).await.unwrap();

    // First lock should succeed
    let locked1 = db.try_lock_schedule(schedule_id).await.unwrap();
    assert!(locked1);

    // Second lock should fail (already locked)
    let locked2 = db.try_lock_schedule(schedule_id).await.unwrap();
    assert!(!locked2);

    // Unlock
    db.unlock_schedule(schedule_id).await.unwrap();

    // Third lock should succeed (unlocked)
    let locked3 = db.try_lock_schedule(schedule_id).await.unwrap();
    assert!(locked3);
}

#[tokio::test]
async fn test_get_due_schedules() {
    use chrono::TimeZone;
    let db = Database::in_memory().await.unwrap();

    // Schedule 1: due (past)
    let mut schedule1 = Schedule::new(
        "due-schedule".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Due task".to_string(),
    );
    schedule1.next_run = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
    db.insert_schedule(&schedule1).await.unwrap();

    // Schedule 2: not due (future)
    let mut schedule2 = Schedule::new(
        "future-schedule".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Future task".to_string(),
    );
    schedule2.next_run = Some(Utc::now() + chrono::Duration::hours(24));
    db.insert_schedule(&schedule2).await.unwrap();

    // Schedule 3: disabled but due
    let mut schedule3 = Schedule::new(
        "disabled-schedule".to_string(),
        "0 0 * * *".to_string(),
        "BackgroundController".to_string(),
        "Disabled task".to_string(),
    );
    schedule3.enabled = false;
    schedule3.next_run = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
    db.insert_schedule(&schedule3).await.unwrap();

    // Get due schedules - should only return schedule1
    let due = db.get_due_schedules().await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].name, "due-schedule");
}
