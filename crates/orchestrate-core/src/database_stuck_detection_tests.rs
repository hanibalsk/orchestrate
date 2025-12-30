//! Database tests for stuck detection and work evaluation operations

use crate::stuck_detection::{
    EvaluationStatus, EvaluationType, StuckDetection, StuckSeverity, StuckType, WorkEvaluation,
};
use crate::Database;

#[tokio::test]
async fn test_create_and_get_work_evaluation() {
    let db = Database::in_memory().await.unwrap();

    let eval = WorkEvaluation::new("agent-123", EvaluationType::Progress, EvaluationStatus::Healthy)
        .with_progress(50, 100)
        .with_tokens(50000, 100000)
        .with_duration(300);

    let id = db.create_work_evaluation(&eval).await.unwrap();
    assert!(id > 0);

    let retrieved = db.get_work_evaluation(id).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.agent_id, "agent-123");
    assert_eq!(retrieved.evaluation_type, EvaluationType::Progress);
    assert_eq!(retrieved.status, EvaluationStatus::Healthy);
    assert_eq!(retrieved.turn_count, Some(50));
    assert_eq!(retrieved.max_turns, Some(100));
    assert_eq!(retrieved.token_count, Some(50000));
    assert_eq!(retrieved.max_tokens, Some(100000));
    assert_eq!(retrieved.duration_secs, Some(300));
}

#[tokio::test]
async fn test_work_evaluation_with_session_and_story() {
    let db = Database::in_memory().await.unwrap();

    let eval = WorkEvaluation::new("agent-456", EvaluationType::Completion, EvaluationStatus::Complete)
        .with_session("session-abc")
        .with_story("story-001");

    let id = db.create_work_evaluation(&eval).await.unwrap();

    let retrieved = db.get_work_evaluation(id).await.unwrap().unwrap();
    assert_eq!(retrieved.session_id, Some("session-abc".to_string()));
    assert_eq!(retrieved.story_id, Some("story-001".to_string()));
}

#[tokio::test]
async fn test_work_evaluation_with_details() {
    let db = Database::in_memory().await.unwrap();

    let details = serde_json::json!({
        "files_changed": 5,
        "tests_passed": true,
        "review_status": "pending"
    });

    let eval = WorkEvaluation::new("agent-789", EvaluationType::StuckCheck, EvaluationStatus::Warning)
        .with_details(details.clone());

    let id = db.create_work_evaluation(&eval).await.unwrap();

    let retrieved = db.get_work_evaluation(id).await.unwrap().unwrap();
    assert_eq!(retrieved.details, details);
}

#[tokio::test]
async fn test_get_work_evaluations_for_agent() {
    let db = Database::in_memory().await.unwrap();

    // Create multiple evaluations for same agent
    let eval1 = WorkEvaluation::new("agent-multi", EvaluationType::Progress, EvaluationStatus::Healthy);
    db.create_work_evaluation(&eval1).await.unwrap();

    let eval2 = WorkEvaluation::new("agent-multi", EvaluationType::StuckCheck, EvaluationStatus::Warning);
    db.create_work_evaluation(&eval2).await.unwrap();

    // Create one for different agent
    let eval3 = WorkEvaluation::new("agent-other", EvaluationType::Progress, EvaluationStatus::Healthy);
    db.create_work_evaluation(&eval3).await.unwrap();

    let evals = db.get_work_evaluations_for_agent("agent-multi").await.unwrap();
    assert_eq!(evals.len(), 2);
}

#[tokio::test]
async fn test_get_latest_evaluation() {
    let db = Database::in_memory().await.unwrap();

    // Create evaluations with slight delay
    let eval1 = WorkEvaluation::new("agent-latest", EvaluationType::Progress, EvaluationStatus::Healthy);
    db.create_work_evaluation(&eval1).await.unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let eval2 = WorkEvaluation::new("agent-latest", EvaluationType::StuckCheck, EvaluationStatus::Stuck);
    db.create_work_evaluation(&eval2).await.unwrap();

    // Should get the most recent one
    let latest = db.get_latest_evaluation("agent-latest").await.unwrap();
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().status, EvaluationStatus::Stuck);
}

#[tokio::test]
async fn test_work_evaluation_not_found() {
    let db = Database::in_memory().await.unwrap();

    let result = db.get_work_evaluation(99999).await.unwrap();
    assert!(result.is_none());
}

// ==================== Stuck Detection Tests ====================

#[tokio::test]
async fn test_create_and_get_stuck_detection() {
    let db = Database::in_memory().await.unwrap();

    let detection = StuckDetection::new("agent-stuck", StuckType::TurnLimit, StuckSeverity::High)
        .with_details(serde_json::json!({
            "turn_count": 95,
            "max_turns": 100
        }));

    let id = db.create_stuck_detection(&detection).await.unwrap();
    assert!(id > 0);

    let retrieved = db.get_stuck_detection(id).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.agent_id, "agent-stuck");
    assert_eq!(retrieved.detection_type, StuckType::TurnLimit);
    assert_eq!(retrieved.severity, StuckSeverity::High);
    assert!(!retrieved.resolved);
}

#[tokio::test]
async fn test_stuck_detection_with_session() {
    let db = Database::in_memory().await.unwrap();

    let detection = StuckDetection::new("agent-session", StuckType::CiTimeout, StuckSeverity::Medium)
        .with_session("session-xyz");

    let id = db.create_stuck_detection(&detection).await.unwrap();

    let retrieved = db.get_stuck_detection(id).await.unwrap().unwrap();
    assert_eq!(retrieved.session_id, Some("session-xyz".to_string()));
}

#[tokio::test]
async fn test_update_stuck_detection_resolve() {
    let db = Database::in_memory().await.unwrap();

    let mut detection = StuckDetection::new("agent-resolve", StuckType::MergeConflict, StuckSeverity::High);

    let id = db.create_stuck_detection(&detection).await.unwrap();
    detection.id = id;

    // Resolve the detection
    detection.resolve("rebased branch to fix conflicts");
    db.update_stuck_detection(&detection).await.unwrap();

    let retrieved = db.get_stuck_detection(id).await.unwrap().unwrap();
    assert!(retrieved.resolved);
    assert_eq!(retrieved.resolution_action, Some("rebased branch to fix conflicts".to_string()));
    assert!(retrieved.resolved_at.is_some());
}

#[tokio::test]
async fn test_get_unresolved_stuck_detections() {
    let db = Database::in_memory().await.unwrap();

    // Create unresolved detections
    let det1 = StuckDetection::new("agent-unresolved", StuckType::RateLimit, StuckSeverity::Medium);
    db.create_stuck_detection(&det1).await.unwrap();

    let det2 = StuckDetection::new("agent-unresolved", StuckType::ErrorLoop, StuckSeverity::High);
    db.create_stuck_detection(&det2).await.unwrap();

    // Create a resolved detection
    let mut det3 = StuckDetection::new("agent-unresolved", StuckType::NoProgress, StuckSeverity::Low);
    let id3 = db.create_stuck_detection(&det3).await.unwrap();
    det3.id = id3;
    det3.resolve("agent made progress");
    db.update_stuck_detection(&det3).await.unwrap();

    // Should only get 2 unresolved
    let unresolved = db.get_unresolved_stuck_detections("agent-unresolved").await.unwrap();
    assert_eq!(unresolved.len(), 2);
}

#[tokio::test]
async fn test_get_stuck_detections_for_agent() {
    let db = Database::in_memory().await.unwrap();

    // Create detections for agent
    let det1 = StuckDetection::new("agent-all", StuckType::ContextLimit, StuckSeverity::Critical);
    db.create_stuck_detection(&det1).await.unwrap();

    let det2 = StuckDetection::new("agent-all", StuckType::ReviewDelay, StuckSeverity::Medium);
    db.create_stuck_detection(&det2).await.unwrap();

    // Create one for different agent
    let det3 = StuckDetection::new("agent-different", StuckType::TurnLimit, StuckSeverity::High);
    db.create_stuck_detection(&det3).await.unwrap();

    let all = db.get_stuck_detections_for_agent("agent-all").await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_get_all_unresolved_stuck_detections() {
    let db = Database::in_memory().await.unwrap();

    // Create unresolved from different agents
    let det1 = StuckDetection::new("agent-a", StuckType::TurnLimit, StuckSeverity::High);
    db.create_stuck_detection(&det1).await.unwrap();

    let det2 = StuckDetection::new("agent-b", StuckType::MergeConflict, StuckSeverity::Critical);
    db.create_stuck_detection(&det2).await.unwrap();

    let det3 = StuckDetection::new("agent-c", StuckType::RateLimit, StuckSeverity::Low);
    db.create_stuck_detection(&det3).await.unwrap();

    let all = db.get_all_unresolved_stuck_detections().await.unwrap();
    assert_eq!(all.len(), 3);

    // Verify all severities are present
    let severities: Vec<_> = all.iter().map(|d| d.severity).collect();
    assert!(severities.contains(&StuckSeverity::Critical));
    assert!(severities.contains(&StuckSeverity::High));
    assert!(severities.contains(&StuckSeverity::Low));
}

#[tokio::test]
async fn test_count_stuck_detections_by_type() {
    let db = Database::in_memory().await.unwrap();

    // Create detections of various types
    let det1 = StuckDetection::new("agent-1", StuckType::TurnLimit, StuckSeverity::High);
    db.create_stuck_detection(&det1).await.unwrap();

    let det2 = StuckDetection::new("agent-2", StuckType::TurnLimit, StuckSeverity::Medium);
    db.create_stuck_detection(&det2).await.unwrap();

    let det3 = StuckDetection::new("agent-3", StuckType::ErrorLoop, StuckSeverity::Critical);
    db.create_stuck_detection(&det3).await.unwrap();

    let counts = db.count_stuck_detections_by_type().await.unwrap();
    assert_eq!(counts.get("turn_limit"), Some(&2));
    assert_eq!(counts.get("error_loop"), Some(&1));
}

#[tokio::test]
async fn test_stuck_detection_all_types() {
    let db = Database::in_memory().await.unwrap();

    let types = vec![
        StuckType::TurnLimit,
        StuckType::NoProgress,
        StuckType::CiTimeout,
        StuckType::ReviewDelay,
        StuckType::MergeConflict,
        StuckType::RateLimit,
        StuckType::ContextLimit,
        StuckType::ErrorLoop,
    ];

    for stuck_type in types {
        let detection = StuckDetection::new("agent-types", stuck_type, StuckSeverity::Medium);
        let id = db.create_stuck_detection(&detection).await.unwrap();
        let retrieved = db.get_stuck_detection(id).await.unwrap().unwrap();
        assert_eq!(retrieved.detection_type, stuck_type);
    }
}

#[tokio::test]
async fn test_stuck_detection_all_severities() {
    let db = Database::in_memory().await.unwrap();

    let severities = vec![
        StuckSeverity::Low,
        StuckSeverity::Medium,
        StuckSeverity::High,
        StuckSeverity::Critical,
    ];

    for severity in severities {
        let detection = StuckDetection::new("agent-sev", StuckType::TurnLimit, severity);
        let id = db.create_stuck_detection(&detection).await.unwrap();
        let retrieved = db.get_stuck_detection(id).await.unwrap().unwrap();
        assert_eq!(retrieved.severity, severity);
    }
}

#[tokio::test]
async fn test_stuck_detection_not_found() {
    let db = Database::in_memory().await.unwrap();

    let result = db.get_stuck_detection(99999).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_work_evaluation_all_types() {
    let db = Database::in_memory().await.unwrap();

    let types = vec![
        EvaluationType::Progress,
        EvaluationType::Completion,
        EvaluationType::StuckCheck,
        EvaluationType::ReviewOutcome,
        EvaluationType::CiStatus,
    ];

    for eval_type in types {
        let eval = WorkEvaluation::new("agent-eval", eval_type, EvaluationStatus::Healthy);
        let id = db.create_work_evaluation(&eval).await.unwrap();
        let retrieved = db.get_work_evaluation(id).await.unwrap().unwrap();
        assert_eq!(retrieved.evaluation_type, eval_type);
    }
}

#[tokio::test]
async fn test_work_evaluation_all_statuses() {
    let db = Database::in_memory().await.unwrap();

    let statuses = vec![
        EvaluationStatus::Healthy,
        EvaluationStatus::Warning,
        EvaluationStatus::Stuck,
        EvaluationStatus::Failed,
        EvaluationStatus::Complete,
    ];

    for status in statuses {
        let eval = WorkEvaluation::new("agent-status", EvaluationType::Progress, status);
        let id = db.create_work_evaluation(&eval).await.unwrap();
        let retrieved = db.get_work_evaluation(id).await.unwrap().unwrap();
        assert_eq!(retrieved.status, status);
    }
}
