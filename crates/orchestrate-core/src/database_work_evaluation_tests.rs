//! Database tests for work evaluation operations
//! Epic 016: Autonomous Epic Processing - Story 8

use crate::database::Database;
use crate::work_evaluation::{
    CiCheckResult, CiStatus, CriterionCheck, ReviewIssue, ReviewIssueSeverity, ReviewResult,
    ReviewVerdict, StoryEvaluationRecord, WorkCompletionStatus, WorkEvaluationResult, WorkEvaluator,
};

async fn setup_test_db() -> Database {
    Database::new(":memory:").await.expect("Failed to create test database")
}

// ==================== Story Evaluation Tests ====================

#[tokio::test]
async fn test_create_story_evaluation() {
    let db = setup_test_db().await;

    let evaluator = WorkEvaluator::new();
    let result = evaluator.evaluate(
        Some(crate::decision_engine::AgentStatus::Complete),
        vec![CriterionCheck::met("Implement feature")],
        vec![CiCheckResult::new("build", CiStatus::Passed)],
        Some(ReviewResult::new(ReviewVerdict::Approved)),
        None,
    );

    let record = StoryEvaluationRecord::from_result("story-1", "agent-1", &result)
        .with_session("session-1");

    let id = db.create_story_evaluation(&record).await.unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn test_get_story_evaluation() {
    let db = setup_test_db().await;

    let evaluator = WorkEvaluator::new();
    let result = evaluator.evaluate(
        Some(crate::decision_engine::AgentStatus::Complete),
        vec![
            CriterionCheck::met("Criterion 1"),
            CriterionCheck::unmet("Criterion 2"),
        ],
        vec![CiCheckResult::new("test", CiStatus::Passed)],
        None,
        None,
    );

    let record = StoryEvaluationRecord::from_result("story-2", "agent-2", &result);
    let id = db.create_story_evaluation(&record).await.unwrap();

    let fetched = db.get_story_evaluation(id).await.unwrap().unwrap();
    assert_eq!(fetched.story_id, "story-2");
    assert_eq!(fetched.agent_id, "agent-2");
    assert_eq!(fetched.criteria_met_count, 1);
    assert_eq!(fetched.criteria_total_count, 2);
}

#[tokio::test]
async fn test_get_story_evaluations_for_story() {
    let db = setup_test_db().await;

    let evaluator = WorkEvaluator::new();

    // Create multiple evaluations for same story
    for i in 0..3 {
        let result = evaluator.evaluate(
            None,
            vec![CriterionCheck::met(&format!("Criterion {}", i))],
            vec![],
            None,
            None,
        );
        let record = StoryEvaluationRecord::from_result("story-multi", &format!("agent-{}", i), &result);
        db.create_story_evaluation(&record).await.unwrap();
    }

    let evaluations = db.get_story_evaluations("story-multi").await.unwrap();
    assert_eq!(evaluations.len(), 3);
}

#[tokio::test]
async fn test_get_latest_story_evaluation() {
    let db = setup_test_db().await;

    let evaluator = WorkEvaluator::new();

    // Create first evaluation
    let result1 = evaluator.evaluate(
        None,
        vec![CriterionCheck::unmet("Criterion")],
        vec![],
        None,
        None,
    );
    let record1 = StoryEvaluationRecord::from_result("story-latest", "agent-1", &result1);
    db.create_story_evaluation(&record1).await.unwrap();

    // Wait a bit to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Create second evaluation
    let result2 = evaluator.evaluate(
        Some(crate::decision_engine::AgentStatus::Complete),
        vec![CriterionCheck::met("Criterion")],
        vec![CiCheckResult::new("build", CiStatus::Passed)],
        Some(ReviewResult::new(ReviewVerdict::Approved)),
        None,
    );
    let record2 = StoryEvaluationRecord::from_result("story-latest", "agent-2", &result2);
    db.create_story_evaluation(&record2).await.unwrap();

    let latest = db.get_latest_story_evaluation("story-latest").await.unwrap().unwrap();
    assert_eq!(latest.agent_id, "agent-2");
    assert!(latest.ci_passed);
}

#[tokio::test]
async fn test_get_story_evaluations_for_session() {
    let db = setup_test_db().await;

    let evaluator = WorkEvaluator::new();

    // Create evaluations for a session
    for story_num in 1..=3 {
        let result = evaluator.evaluate(
            None,
            vec![CriterionCheck::met("Criterion")],
            vec![],
            None,
            None,
        );
        let record = StoryEvaluationRecord::from_result(
            &format!("story-{}", story_num),
            "agent-1",
            &result,
        )
        .with_session("session-abc");
        db.create_story_evaluation(&record).await.unwrap();
    }

    let session_evals = db.get_story_evaluations_for_session("session-abc").await.unwrap();
    assert_eq!(session_evals.len(), 3);
}

// ==================== Code Review Result Tests ====================

#[tokio::test]
async fn test_create_code_review_result() {
    let db = setup_test_db().await;

    let review = ReviewResult::new(ReviewVerdict::Approved)
        .with_reviewer("reviewer-1")
        .with_iteration(1);

    let id = db
        .create_code_review_result("story-rev-1", "agent-1", Some("session-1"), &review)
        .await
        .unwrap();

    assert!(id > 0);
}

#[tokio::test]
async fn test_code_review_with_issues() {
    let db = setup_test_db().await;

    let review = ReviewResult::new(ReviewVerdict::ChangesRequested)
        .with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::High, "Security issue"),
            ReviewIssue::new(ReviewIssueSeverity::Medium, "Consider refactoring"),
            ReviewIssue::new(ReviewIssueSeverity::Nitpick, "Typo in comment"),
        ])
        .with_iteration(1);

    db.create_code_review_result("story-rev-issues", "agent-1", None, &review)
        .await
        .unwrap();

    let reviews = db.get_code_review_results("story-rev-issues").await.unwrap();
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0].issues.len(), 3);
    assert_eq!(reviews[0].verdict, ReviewVerdict::ChangesRequested);
}

#[tokio::test]
async fn test_get_latest_code_review_result() {
    let db = setup_test_db().await;

    // Create first review
    let review1 = ReviewResult::new(ReviewVerdict::ChangesRequested).with_iteration(1);
    db.create_code_review_result("story-rev-latest", "agent-1", None, &review1)
        .await
        .unwrap();

    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Create second review
    let review2 = ReviewResult::new(ReviewVerdict::Approved).with_iteration(2);
    db.create_code_review_result("story-rev-latest", "agent-1", None, &review2)
        .await
        .unwrap();

    let latest = db.get_latest_code_review_result("story-rev-latest").await.unwrap().unwrap();
    assert_eq!(latest.verdict, ReviewVerdict::Approved);
    assert_eq!(latest.iteration, 2);
}

#[tokio::test]
async fn test_get_review_iteration_count() {
    let db = setup_test_db().await;

    // Create 3 review iterations
    for i in 1..=3 {
        let review = ReviewResult::new(if i == 3 {
            ReviewVerdict::Approved
        } else {
            ReviewVerdict::ChangesRequested
        })
        .with_iteration(i);
        db.create_code_review_result("story-iter-count", "agent-1", None, &review)
            .await
            .unwrap();
    }

    let count = db.get_review_iteration_count("story-iter-count").await.unwrap();
    assert_eq!(count, 3);
}

// ==================== CI Check Result Tests ====================

#[tokio::test]
async fn test_create_ci_check_result() {
    let db = setup_test_db().await;

    let check = CiCheckResult::new("build", CiStatus::Passed)
        .with_url("https://ci.example.com/build/123");

    let id = db
        .create_ci_check_result(Some("story-ci-1"), "agent-1", None, &check)
        .await
        .unwrap();

    assert!(id > 0);
}

#[tokio::test]
async fn test_ci_check_result_with_failure() {
    let db = setup_test_db().await;

    let check = CiCheckResult::new("test", CiStatus::Failed)
        .with_failure("3 tests failed: test_a, test_b, test_c");

    db.create_ci_check_result(Some("story-ci-fail"), "agent-1", None, &check)
        .await
        .unwrap();

    let checks = db.get_ci_check_results("story-ci-fail").await.unwrap();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].status, CiStatus::Failed);
    assert!(checks[0].failure_details.is_some());
}

#[tokio::test]
async fn test_get_ci_check_results() {
    let db = setup_test_db().await;

    // Create multiple checks
    let checks_to_create = vec![
        CiCheckResult::new("build", CiStatus::Passed),
        CiCheckResult::new("test", CiStatus::Passed),
        CiCheckResult::new("lint", CiStatus::Failed).with_failure("Lint errors"),
    ];

    for check in &checks_to_create {
        db.create_ci_check_result(Some("story-ci-multi"), "agent-1", None, check)
            .await
            .unwrap();
    }

    let results = db.get_ci_check_results("story-ci-multi").await.unwrap();
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_get_latest_ci_check_results() {
    let db = setup_test_db().await;

    // Create first round of checks
    let checks1 = vec![
        CiCheckResult::new("build", CiStatus::Passed),
        CiCheckResult::new("test", CiStatus::Failed),
    ];
    for check in &checks1 {
        db.create_ci_check_result(Some("story-ci-latest"), "agent-1", None, check)
            .await
            .unwrap();
    }

    // No need to wait - using MAX(id) for ordering which is reliable

    // Create second round - test now passes
    let check2 = CiCheckResult::new("test", CiStatus::Passed);
    db.create_ci_check_result(Some("story-ci-latest"), "agent-1", None, &check2)
        .await
        .unwrap();

    let latest = db.get_latest_ci_check_results("story-ci-latest").await.unwrap();

    // Should have 2 unique check names (build and test - latest of each)
    assert_eq!(latest.len(), 2);

    // Find the test check - should be the latest (passed)
    let test_check = latest.iter().find(|c| c.name == "test").unwrap();
    assert_eq!(test_check.status, CiStatus::Passed);
}

// ==================== Story Evaluation Stats Tests ====================

#[tokio::test]
async fn test_get_story_evaluation_stats() {
    let db = setup_test_db().await;

    let evaluator = WorkEvaluator::new();

    // Create evaluations with different statuses
    let statuses = vec![
        (Some(crate::decision_engine::AgentStatus::Complete), vec![CriterionCheck::met("C")], CiStatus::Passed, Some(ReviewResult::new(ReviewVerdict::Approved))), // complete
        (None, vec![CriterionCheck::unmet("C")], CiStatus::Pending, None), // in_progress
        (Some(crate::decision_engine::AgentStatus::Blocked), vec![], CiStatus::Pending, None), // blocked
        (Some(crate::decision_engine::AgentStatus::Complete), vec![CriterionCheck::met("C")], CiStatus::Failed, None), // needs_ci_fixes
    ];

    for (i, (status, criteria, ci, review)) in statuses.into_iter().enumerate() {
        let result = evaluator.evaluate(
            status,
            criteria,
            vec![CiCheckResult::new("build", ci)],
            review,
            None,
        );
        let record = StoryEvaluationRecord::from_result(
            &format!("story-stats-{}", i),
            "agent-1",
            &result,
        )
        .with_session("session-stats");
        db.create_story_evaluation(&record).await.unwrap();
    }

    let stats = db.get_story_evaluation_stats(Some("session-stats")).await.unwrap();
    assert_eq!(stats.total, 4);
}

#[tokio::test]
async fn test_get_story_evaluation_stats_all() {
    let db = setup_test_db().await;

    let evaluator = WorkEvaluator::new();

    // Create some evaluations
    for i in 0..5 {
        let result = evaluator.evaluate(None, vec![], vec![], None, None);
        let record = StoryEvaluationRecord::from_result(
            &format!("story-all-{}", i),
            "agent-1",
            &result,
        );
        db.create_story_evaluation(&record).await.unwrap();
    }

    let stats = db.get_story_evaluation_stats(None).await.unwrap();
    assert_eq!(stats.total, 5);
}

// ==================== Integration Tests ====================

#[tokio::test]
async fn test_full_evaluation_workflow() {
    let db = setup_test_db().await;
    let evaluator = WorkEvaluator::new();

    // Step 1: Initial evaluation - in progress
    let result1 = evaluator.evaluate(
        None,
        vec![
            CriterionCheck::unmet("Add tests"),
            CriterionCheck::unmet("Implement feature"),
        ],
        vec![],
        None,
        None,
    );
    let record1 = StoryEvaluationRecord::from_result("story-workflow", "agent-1", &result1)
        .with_session("session-wf");
    db.create_story_evaluation(&record1).await.unwrap();

    // Step 2: Implementation complete, CI running
    let result2 = evaluator.evaluate(
        Some(crate::decision_engine::AgentStatus::Complete),
        vec![
            CriterionCheck::met("Add tests"),
            CriterionCheck::met("Implement feature"),
        ],
        vec![
            CiCheckResult::new("build", CiStatus::Passed),
            CiCheckResult::new("test", CiStatus::Running),
        ],
        None,
        None,
    );
    let record2 = StoryEvaluationRecord::from_result("story-workflow", "agent-1", &result2)
        .with_session("session-wf");
    db.create_story_evaluation(&record2).await.unwrap();

    // Record CI results
    db.create_ci_check_result(
        Some("story-workflow"),
        "agent-1",
        Some("session-wf"),
        &CiCheckResult::new("build", CiStatus::Passed),
    )
    .await
    .unwrap();

    db.create_ci_check_result(
        Some("story-workflow"),
        "agent-1",
        Some("session-wf"),
        &CiCheckResult::new("test", CiStatus::Passed),
    )
    .await
    .unwrap();

    db.create_ci_check_result(
        Some("story-workflow"),
        "agent-1",
        Some("session-wf"),
        &CiCheckResult::new("lint", CiStatus::Passed),
    )
    .await
    .unwrap();

    // Step 3: Review submitted with changes requested
    let review1 = ReviewResult::new(ReviewVerdict::ChangesRequested)
        .with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::High, "Add error handling"),
        ])
        .with_iteration(1);
    db.create_code_review_result("story-workflow", "agent-1", Some("session-wf"), &review1)
        .await
        .unwrap();

    // Step 4: Changes made, review approved
    let review2 = ReviewResult::new(ReviewVerdict::Approved).with_iteration(2);
    db.create_code_review_result("story-workflow", "agent-1", Some("session-wf"), &review2)
        .await
        .unwrap();

    // Final evaluation
    let result3 = evaluator.evaluate(
        Some(crate::decision_engine::AgentStatus::Complete),
        vec![
            CriterionCheck::met("Add tests"),
            CriterionCheck::met("Implement feature"),
        ],
        vec![
            CiCheckResult::new("build", CiStatus::Passed),
            CiCheckResult::new("test", CiStatus::Passed),
            CiCheckResult::new("lint", CiStatus::Passed),
        ],
        Some(ReviewResult::new(ReviewVerdict::Approved)),
        Some(crate::work_evaluation::PrMergeStatus::Mergeable),
    );
    let record3 = StoryEvaluationRecord::from_result("story-workflow", "agent-1", &result3)
        .with_session("session-wf");
    db.create_story_evaluation(&record3).await.unwrap();

    // Verify final state
    let latest = db.get_latest_story_evaluation("story-workflow").await.unwrap().unwrap();
    assert!(latest.ci_passed);
    assert!(latest.review_passed);
    assert!(latest.pr_mergeable);

    let iterations = db.get_review_iteration_count("story-workflow").await.unwrap();
    assert_eq!(iterations, 2);

    let ci_results = db.get_latest_ci_check_results("story-workflow").await.unwrap();
    assert_eq!(ci_results.len(), 3);
    assert!(ci_results.iter().all(|c| c.status.is_passing()));
}
