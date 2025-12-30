#![cfg(test)]

mod tests {
    use crate::{Agent, AgentState, AgentType, Database, Message, MessageRole};
    use chrono::{Duration, Utc};

    async fn setup_test_data(db: &Database) {
        // Create test agents with different states and durations
        let now = Utc::now();

        // Successful story developer agents
        for i in 0..10 {
            let created_at = now - Duration::hours(24) + Duration::hours(i);
            let completed_at = created_at + Duration::minutes(30 + i * 10);

            let mut agent = Agent::new(AgentType::StoryDeveloper, format!("Task {}", i));
            agent.created_at = created_at;
            agent.completed_at = Some(completed_at);
            agent.updated_at = completed_at;
            agent.state = AgentState::Completed;

            db.insert_agent(&agent).await.unwrap();

            // Add messages with token counts
            let msg = Message::user(agent.id, "Test input");
            db.insert_message(&msg).await.unwrap();

            let mut msg = Message::assistant(agent.id, "Test output");
            msg.input_tokens = (1000 + i * 100) as i32;
            msg.output_tokens = (500 + i * 50) as i32;
            let msg_id = db.insert_message(&msg).await.unwrap();
            assert!(msg_id > 0);
        }

        // Failed story developer agents
        for i in 0..2 {
            let created_at = now - Duration::hours(12) + Duration::hours(i);
            let completed_at = created_at + Duration::minutes(15);

            let mut agent = Agent::new(AgentType::StoryDeveloper, format!("Failed task {}", i));
            agent.created_at = created_at;
            agent.completed_at = Some(completed_at);
            agent.updated_at = completed_at;
            agent.state = AgentState::Failed;
            agent.error_message = Some("Database connection timeout".to_string());

            db.insert_agent(&agent).await.unwrap();

            // Add messages with lower token counts
            let msg = Message::user(agent.id, "Test input");
            db.insert_message(&msg).await.unwrap();

            let mut msg = Message::assistant(agent.id, "Error");
            msg.input_tokens = 500;
            msg.output_tokens = 200;
            let msg_id = db.insert_message(&msg).await.unwrap();
            assert!(msg_id > 0);
        }

        // Code reviewer agents (all successful)
        for i in 0..5 {
            let created_at = now - Duration::hours(24) + Duration::hours(i * 2);
            let completed_at = created_at + Duration::minutes(10 + i * 5);

            let mut agent = Agent::new(AgentType::CodeReviewer, format!("Review {}", i));
            agent.created_at = created_at;
            agent.completed_at = Some(completed_at);
            agent.updated_at = completed_at;
            agent.state = AgentState::Completed;

            db.insert_agent(&agent).await.unwrap();

            let msg = Message::user(agent.id, "Review this code");
            db.insert_message(&msg).await.unwrap();

            let mut msg = Message::assistant(agent.id, "LGTM");
            msg.input_tokens = 800;
            msg.output_tokens = 400;
            let msg_id = db.insert_message(&msg).await.unwrap();
            assert!(msg_id > 0);
        }
    }

    #[tokio::test]
    async fn test_get_agent_executions() {
        let db = Database::in_memory().await.unwrap();
        setup_test_data(&db).await;

        // First, verify agents were created
        let all_agents = db.list_agents().await.unwrap();
        assert_eq!(all_agents.len(), 17, "Should have created 17 agents");

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        // Get all story developer executions
        let executions = db
            .get_agent_executions(Some("story_developer"), start, end)
            .await
            .unwrap();

        assert_eq!(executions.len(), 12, "Should find 12 executions (10 successful + 2 failed), found {}", executions.len());

        // Verify successful executions
        let successful = executions
            .iter()
            .filter(|e| e.state == AgentState::Completed)
            .count();
        assert_eq!(successful, 10);

        // Verify failed executions
        let failed = executions
            .iter()
            .filter(|e| e.state == AgentState::Failed)
            .count();
        assert_eq!(failed, 2);

        // Verify token data exists
        assert!(executions.iter().any(|e| e.input_tokens > 0));
        assert!(executions.iter().any(|e| e.output_tokens > 0));
    }

    #[tokio::test]
    async fn test_calculate_agent_performance() {
        let db = Database::in_memory().await.unwrap();
        setup_test_data(&db).await;

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        let performance = db
            .calculate_agent_performance("story_developer", start, end)
            .await
            .unwrap();

        assert_eq!(performance.agent_type, "story_developer");
        assert_eq!(performance.total_executions, 12);
        assert_eq!(performance.successful_executions, 10);
        assert_eq!(performance.failed_executions, 2);

        // Success rate should be ~83.33%
        assert!((performance.success_rate - 83.33).abs() < 1.0);

        // Should have duration stats
        assert!(performance.avg_duration_seconds > 0.0);
        assert!(performance.p50_duration_seconds > 0.0);
        assert!(performance.p95_duration_seconds >= performance.p50_duration_seconds);

        // Should have token stats
        assert!(performance.avg_tokens_per_execution > 0);
    }

    #[tokio::test]
    async fn test_compare_agent_performance() {
        let db = Database::in_memory().await.unwrap();
        setup_test_data(&db).await;

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        let comparisons = db.compare_agent_performance(start, end).await.unwrap();

        assert_eq!(comparisons.len(), 2); // story-developer and code-reviewer

        // Find code reviewer
        let code_reviewer = comparisons
            .iter()
            .find(|c| c.agent_type == "code_reviewer")
            .unwrap();

        // Code reviewer should have 100% success rate
        assert_eq!(code_reviewer.success_rate, 100.0);
        assert_eq!(code_reviewer.total_executions, 5);

        // Should be ranked first by success rate
        assert_eq!(code_reviewer.rank_by_success_rate, 1);
    }

    #[tokio::test]
    async fn test_analyze_agent_errors() {
        let db = Database::in_memory().await.unwrap();
        setup_test_data(&db).await;

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        let analysis = db
            .analyze_agent_errors("story_developer", start, end)
            .await
            .unwrap();

        assert_eq!(analysis.total_errors, 2);
        assert!((analysis.error_rate - 16.67).abs() < 1.0); // 2 out of 12

        // Should have identified the error pattern
        assert!(!analysis.top_patterns.is_empty());
        assert_eq!(analysis.top_patterns[0].pattern, "Database connection timeout");
        assert_eq!(analysis.top_patterns[0].occurrences, 2);
    }

    #[tokio::test]
    async fn test_analyze_agent_errors_no_errors() {
        let db = Database::in_memory().await.unwrap();
        setup_test_data(&db).await;

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        let analysis = db
            .analyze_agent_errors("code_reviewer", start, end)
            .await
            .unwrap();

        assert_eq!(analysis.total_errors, 0);
        assert_eq!(analysis.error_rate, 0.0);
        assert!(analysis.top_patterns.is_empty());
    }

    #[tokio::test]
    async fn test_calculate_token_efficiency() {
        let db = Database::in_memory().await.unwrap();
        setup_test_data(&db).await;

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        let efficiency = db
            .calculate_token_efficiency("story_developer", start, end)
            .await
            .unwrap();

        assert!(efficiency.total_tokens > 0);
        assert!(efficiency.input_tokens > 0);
        assert!(efficiency.output_tokens > 0);
        assert!(efficiency.tokens_per_successful_task > 0.0);
        assert!(efficiency.avg_input_per_task > 0.0);
        assert!(efficiency.avg_output_per_task > 0.0);

        // Should be reasonably efficient
        assert!(efficiency.tokens_per_successful_task > 1000.0);
    }

    #[tokio::test]
    async fn test_get_performance_trend() {
        let db = Database::in_memory().await.unwrap();
        setup_test_data(&db).await;

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        let trend = db
            .get_performance_trend("story_developer", start, end, 12)
            .await
            .unwrap();

        // Should have some data points
        assert!(!trend.data_points.is_empty());

        // Should have a trend direction
        assert!(matches!(
            trend.direction,
            crate::performance_analytics::TrendDirection::Improving
                | crate::performance_analytics::TrendDirection::Stable
                | crate::performance_analytics::TrendDirection::Degrading
        ));
    }

    #[tokio::test]
    async fn test_get_agent_executions_no_data() {
        let db = Database::in_memory().await.unwrap();

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        let executions = db
            .get_agent_executions(Some("story_developer"), start, end)
            .await
            .unwrap();

        assert_eq!(executions.len(), 0);
    }

    #[tokio::test]
    async fn test_calculate_agent_performance_no_data() {
        let db = Database::in_memory().await.unwrap();

        let start = Utc::now() - Duration::days(365);
        let end = Utc::now() + Duration::days(1);

        let performance = db
            .calculate_agent_performance("story_developer", start, end)
            .await
            .unwrap();

        assert_eq!(performance.total_executions, 0);
        assert_eq!(performance.successful_executions, 0);
        assert_eq!(performance.failed_executions, 0);
        assert_eq!(performance.success_rate, 0.0);
    }
}
