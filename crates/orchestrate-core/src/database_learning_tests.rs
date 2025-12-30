//! Tests for database learning operations (success patterns, feedback, experiments)

#[cfg(test)]
mod tests {
    use crate::{
        database::Database, AgentType, Feedback, FeedbackRating, FeedbackSource, SuccessPattern,
        SuccessPatternType,
    };
    use uuid::Uuid;

    async fn setup_test_db() -> Database {
        Database::in_memory().await.expect("Failed to create test database")
    }

    // Story 1: Success Pattern Detection Tests

    #[tokio::test]
    async fn test_upsert_success_pattern_creates_new() {
        let db = setup_test_db().await;

        let pattern = SuccessPattern::new(
            SuccessPatternType::ToolSequence,
            "test_sig_001",
            serde_json::json!({"tools": ["Read", "Write"]}),
        )
        .with_agent_type(AgentType::StoryDeveloper)
        .with_task_type(Some("feature"));

        let result = db.upsert_success_pattern(&pattern).await;
        assert!(result.is_ok());

        // Verify it was created
        let fetched = db
            .get_success_pattern_by_signature("test_sig_001")
            .await
            .unwrap();
        assert!(fetched.is_some());
        let p = fetched.unwrap();
        assert_eq!(p.pattern_type, SuccessPatternType::ToolSequence);
        assert_eq!(p.agent_type, Some(AgentType::StoryDeveloper));
        assert_eq!(p.occurrence_count, 1);
    }

    #[tokio::test]
    async fn test_upsert_success_pattern_increments_count() {
        let db = setup_test_db().await;

        let pattern = SuccessPattern::new(
            SuccessPatternType::ContextSize,
            "test_sig_002",
            serde_json::json!({"messages": 10}),
        );

        // First insert
        db.upsert_success_pattern(&pattern).await.unwrap();

        // Second insert (should increment)
        db.upsert_success_pattern(&pattern).await.unwrap();

        let fetched = db
            .get_success_pattern_by_signature("test_sig_002")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.occurrence_count, 2);
    }

    #[tokio::test]
    async fn test_get_success_patterns_for_agent() {
        let db = setup_test_db().await;

        // Insert patterns for different agent types
        let pattern1 = SuccessPattern::new(
            SuccessPatternType::ToolSequence,
            "sig_dev_1",
            serde_json::json!({}),
        )
        .with_agent_type(AgentType::StoryDeveloper);

        let pattern2 = SuccessPattern::new(
            SuccessPatternType::PromptStructure,
            "sig_dev_2",
            serde_json::json!({}),
        )
        .with_agent_type(AgentType::StoryDeveloper);

        let pattern3 = SuccessPattern::new(
            SuccessPatternType::Timing,
            "sig_reviewer_1",
            serde_json::json!({}),
        )
        .with_agent_type(AgentType::CodeReviewer);

        db.upsert_success_pattern(&pattern1).await.unwrap();
        db.upsert_success_pattern(&pattern2).await.unwrap();
        db.upsert_success_pattern(&pattern3).await.unwrap();

        // Fetch patterns for StoryDeveloper
        let patterns = db
            .get_success_patterns_for_agent(AgentType::StoryDeveloper, 10)
            .await
            .unwrap();

        assert_eq!(patterns.len(), 2);
        assert!(patterns
            .iter()
            .all(|p| p.agent_type == Some(AgentType::StoryDeveloper)));
    }

    #[tokio::test]
    async fn test_get_success_patterns_by_type() {
        let db = setup_test_db().await;

        let pattern1 = SuccessPattern::new(
            SuccessPatternType::ToolSequence,
            "sig_tool_1",
            serde_json::json!({}),
        );

        let pattern2 = SuccessPattern::new(
            SuccessPatternType::ToolSequence,
            "sig_tool_2",
            serde_json::json!({}),
        );

        let pattern3 = SuccessPattern::new(
            SuccessPatternType::PromptStructure,
            "sig_prompt_1",
            serde_json::json!({}),
        );

        db.upsert_success_pattern(&pattern1).await.unwrap();
        db.upsert_success_pattern(&pattern2).await.unwrap();
        db.upsert_success_pattern(&pattern3).await.unwrap();

        let patterns = db
            .get_success_patterns_by_type(SuccessPatternType::ToolSequence, 10)
            .await
            .unwrap();

        assert_eq!(patterns.len(), 2);
        assert!(patterns
            .iter()
            .all(|p| p.pattern_type == SuccessPatternType::ToolSequence));
    }

    // Story 2: User Feedback Collection Tests

    #[tokio::test]
    async fn test_insert_feedback() {
        let db = setup_test_db().await;
        let agent_id = Uuid::new_v4();

        let feedback = Feedback::new(agent_id, FeedbackRating::Positive, "test@example.com")
            .with_comment("Great work!")
            .with_source(FeedbackSource::Web);

        let result = db.insert_feedback(&feedback).await;
        assert!(result.is_ok());
        let inserted = result.unwrap();
        assert!(inserted.id > 0);
        assert_eq!(inserted.agent_id, agent_id);
        assert_eq!(inserted.rating, FeedbackRating::Positive);
    }

    #[tokio::test]
    async fn test_insert_feedback_with_message_id() {
        let db = setup_test_db().await;
        let agent_id = Uuid::new_v4();

        let feedback = Feedback::new(agent_id, FeedbackRating::Negative, "user@example.com")
            .with_message_id(123)
            .with_comment("This was incorrect");

        let inserted = db.insert_feedback(&feedback).await.unwrap();
        assert_eq!(inserted.message_id, Some(123));
    }

    #[tokio::test]
    async fn test_get_feedback_for_agent() {
        let db = setup_test_db().await;
        let agent_id = Uuid::new_v4();
        let other_agent_id = Uuid::new_v4();

        // Insert feedback for different agents
        let f1 = Feedback::new(agent_id, FeedbackRating::Positive, "user1");
        let f2 = Feedback::new(agent_id, FeedbackRating::Negative, "user2");
        let f3 = Feedback::new(other_agent_id, FeedbackRating::Positive, "user3");

        db.insert_feedback(&f1).await.unwrap();
        db.insert_feedback(&f2).await.unwrap();
        db.insert_feedback(&f3).await.unwrap();

        let feedback = db.get_feedback_for_agent(agent_id).await.unwrap();
        assert_eq!(feedback.len(), 2);
        assert!(feedback.iter().all(|f| f.agent_id == agent_id));
    }

    #[tokio::test]
    async fn test_get_feedback_stats() {
        let db = setup_test_db().await;
        let agent_id = Uuid::new_v4();

        // Insert varied feedback
        for _ in 0..7 {
            db.insert_feedback(&Feedback::new(
                agent_id,
                FeedbackRating::Positive,
                "user",
            ))
            .await
            .unwrap();
        }
        for _ in 0..2 {
            db.insert_feedback(&Feedback::new(
                agent_id,
                FeedbackRating::Negative,
                "user",
            ))
            .await
            .unwrap();
        }
        db.insert_feedback(&Feedback::new(agent_id, FeedbackRating::Neutral, "user"))
            .await
            .unwrap();

        let stats = db.get_feedback_stats_for_agent(agent_id).await.unwrap();
        assert_eq!(stats.total, 10);
        assert_eq!(stats.positive, 7);
        assert_eq!(stats.negative, 2);
        assert_eq!(stats.neutral, 1);
        assert!((stats.score - 0.5).abs() < 0.01); // (7-2)/10 = 0.5
        assert!((stats.positive_percentage - 70.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_get_recent_feedback() {
        let db = setup_test_db().await;
        let agent_id = Uuid::new_v4();

        // Insert some feedback
        for i in 0..5 {
            let rating = if i % 2 == 0 {
                FeedbackRating::Positive
            } else {
                FeedbackRating::Negative
            };
            db.insert_feedback(&Feedback::new(agent_id, rating, "user"))
                .await
                .unwrap();
        }

        let recent = db.get_recent_feedback(3).await.unwrap();
        assert_eq!(recent.len(), 3);
        // Should be in descending order by created_at
        assert!(recent[0].created_at >= recent[1].created_at);
        assert!(recent[1].created_at >= recent[2].created_at);
    }

    // Story 3: Effectiveness Scoring Tests
    // These are mostly tested through the instruction_effectiveness methods
    // which are already implemented in the database

    #[tokio::test]
    async fn test_effectiveness_with_feedback() {
        // This is an integration test showing how feedback links to effectiveness
        // The actual effectiveness calculation is done in the instruction module
        let db = setup_test_db().await;
        let agent_id = Uuid::new_v4();

        // Add positive feedback
        let feedback = Feedback::new(agent_id, FeedbackRating::Positive, "user");
        let inserted = db.insert_feedback(&feedback).await.unwrap();

        // Verify it's retrievable and has correct score
        assert_eq!(inserted.rating.score(), 1.0);

        // Add negative feedback
        let feedback2 = Feedback::new(agent_id, FeedbackRating::Negative, "user");
        let inserted2 = db.insert_feedback(&feedback2).await.unwrap();
        assert_eq!(inserted2.rating.score(), -1.0);
    }

    // Story 4: A/B Testing Framework Tests

    #[tokio::test]
    async fn test_create_experiment() {
        use crate::{Experiment, ExperimentMetric, ExperimentType};

        let db = setup_test_db().await;

        let experiment = Experiment::new(
            "Test Prompt Variation".to_string(),
            ExperimentType::Prompt,
            ExperimentMetric::SuccessRate,
        )
        .with_hypothesis("Adding examples will improve success rate".to_string())
        .with_min_samples(50);

        let result = db.create_experiment(experiment).await;
        assert!(result.is_ok());
        let created = result.unwrap();
        assert!(created.id > 0);
        assert_eq!(created.name, "Test Prompt Variation");
        assert_eq!(created.min_samples, 50);
    }

    #[tokio::test]
    async fn test_create_experiment_variant() {
        use crate::{Experiment, ExperimentMetric, ExperimentType, ExperimentVariant};

        let db = setup_test_db().await;

        let experiment = Experiment::new(
            "Model Test".to_string(),
            ExperimentType::Model,
            ExperimentMetric::CompletionTime,
        );
        let created_exp = db.create_experiment(experiment).await.unwrap();

        // Create control variant
        let control = ExperimentVariant::new(created_exp.id, "Control".to_string(), true)
            .with_config(serde_json::json!({"model": "gpt-4"}));

        let result = db.create_experiment_variant(control).await;
        assert!(result.is_ok());
        let created_variant = result.unwrap();
        assert_eq!(created_variant.experiment_id, created_exp.id);
        assert!(created_variant.is_control);
    }

    #[tokio::test]
    async fn test_assign_variant() {
        use crate::{Experiment, ExperimentMetric, ExperimentType, ExperimentVariant};

        let db = setup_test_db().await;

        let experiment = Experiment::new(
            "Assign Test".to_string(),
            ExperimentType::Prompt,
            ExperimentMetric::SuccessRate,
        );
        let exp = db.create_experiment(experiment).await.unwrap();

        let variant = ExperimentVariant::new(exp.id, "VariantA".to_string(), false);
        let var = db.create_experiment_variant(variant).await.unwrap();

        let agent_id = Uuid::new_v4();
        let result = db.assign_variant(exp.id, var.id, agent_id).await;
        assert!(result.is_ok());

        // Verify assignment
        let assignment = db.get_variant_assignment(exp.id, agent_id).await.unwrap();
        assert!(assignment.is_some());
        assert_eq!(assignment.unwrap().variant_id, var.id);
    }

    #[tokio::test]
    async fn test_record_observation() {
        use crate::{Experiment, ExperimentMetric, ExperimentType, ExperimentVariant};

        let db = setup_test_db().await;

        let experiment = Experiment::new(
            "Observation Test".to_string(),
            ExperimentType::Instruction,
            ExperimentMetric::TokenUsage,
        );
        let exp = db.create_experiment(experiment).await.unwrap();

        let variant = ExperimentVariant::new(exp.id, "V1".to_string(), true);
        let var = db.create_experiment_variant(variant).await.unwrap();

        let agent_id = Uuid::new_v4();
        let assignment = db.assign_variant(exp.id, var.id, agent_id).await.unwrap();

        // Record observation
        let result = db
            .record_experiment_observation(assignment.id, "token_usage", 1500.0)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_experiment_results() {
        use crate::{Experiment, ExperimentMetric, ExperimentType, ExperimentVariant};

        let db = setup_test_db().await;

        let experiment = Experiment::new(
            "Results Test".to_string(),
            ExperimentType::Model,
            ExperimentMetric::SuccessRate,
        );
        let exp = db.create_experiment(experiment).await.unwrap();

        // Create two variants
        let control = ExperimentVariant::new(exp.id, "Control".to_string(), true);
        let treatment = ExperimentVariant::new(exp.id, "Treatment".to_string(), false);

        let control_var = db.create_experiment_variant(control).await.unwrap();
        let treatment_var = db.create_experiment_variant(treatment).await.unwrap();

        // Add some observations
        for i in 0..10 {
            let agent_id = Uuid::new_v4();
            let assignment = db
                .assign_variant(exp.id, control_var.id, agent_id)
                .await
                .unwrap();
            db.record_experiment_observation(assignment.id, "success_rate", if i < 7 { 1.0 } else { 0.0 })
                .await
                .unwrap();
        }

        for i in 0..10 {
            let agent_id = Uuid::new_v4();
            let assignment = db
                .assign_variant(exp.id, treatment_var.id, agent_id)
                .await
                .unwrap();
            db.record_experiment_observation(assignment.id, "success_rate", if i < 9 { 1.0 } else { 0.0 })
                .await
                .unwrap();
        }

        let results = db.get_experiment_results(exp.id).await.unwrap();
        assert_eq!(results.variants.len(), 2);
        assert_eq!(results.total_samples, 20);
    }

    #[tokio::test]
    async fn test_update_experiment_status() {
        use crate::{Experiment, ExperimentMetric, ExperimentStatus, ExperimentType};

        let db = setup_test_db().await;

        let mut experiment = Experiment::new(
            "Status Test".to_string(),
            ExperimentType::Prompt,
            ExperimentMetric::SuccessRate,
        );
        experiment = db.create_experiment(experiment).await.unwrap();

        // Update to running
        let result = db
            .update_experiment_status(experiment.id, ExperimentStatus::Running)
            .await;
        assert!(result.is_ok());

        let updated = db.get_experiment(experiment.id).await.unwrap().unwrap();
        assert_eq!(updated.status, ExperimentStatus::Running);
        assert!(updated.started_at.is_some());
    }
}
