//! Integration tests for orchestrate
//!
//! These tests verify end-to-end behavior across multiple crates

use orchestrate_core::{
    Agent, AgentContext, AgentState, AgentType, Database, Epic,
    Message, PrStatus, PullRequest, MergeStrategy,
    network::{AgentId, StepOutput, StepOutputType},
};
use orchestrate_web::api::{AppState, create_api_router};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::util::ServiceExt;
use uuid::Uuid;

// ==================== Test Helpers ====================

async fn setup_db() -> Database {
    Database::in_memory().await.unwrap()
}

async fn body_to_string(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// ==================== Database Integration Tests ====================

mod database {
    use super::*;

    #[tokio::test]
    async fn test_agent_lifecycle() {
        let db = setup_db().await;

        // Create agent
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Implement login feature");
        let agent_id = agent.id;

        // Insert
        db.insert_agent(&agent).await.unwrap();

        // Retrieve
        let fetched = db.get_agent(agent_id).await.unwrap().unwrap();
        assert_eq!(fetched.id, agent_id);
        assert_eq!(fetched.task, "Implement login feature");
        assert_eq!(fetched.state, AgentState::Created);

        // Update state
        agent.transition_to(AgentState::Initializing).unwrap();
        agent.transition_to(AgentState::Running).unwrap();
        db.update_agent(&agent).await.unwrap();

        let fetched = db.get_agent(agent_id).await.unwrap().unwrap();
        assert_eq!(fetched.state, AgentState::Running);

        // Complete
        agent.transition_to(AgentState::Completed).unwrap();
        db.update_agent(&agent).await.unwrap();

        let fetched = db.get_agent(agent_id).await.unwrap().unwrap();
        assert_eq!(fetched.state, AgentState::Completed);
        assert!(fetched.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_agent_with_context() {
        let db = setup_db().await;

        let context = AgentContext {
            pr_number: Some(42),
            branch_name: Some("feature/auth".to_string()),
            custom: serde_json::json!({"repo": "hanibalsk/orchestrate"}),
            ..Default::default()
        };

        let agent = Agent::new(AgentType::CodeReviewer, "Review PR #42")
            .with_context(context);

        db.insert_agent(&agent).await.unwrap();

        let fetched = db.get_agent(agent.id).await.unwrap().unwrap();
        assert_eq!(fetched.context.pr_number, Some(42));
        assert_eq!(fetched.context.branch_name, Some("feature/auth".to_string()));
    }

    #[tokio::test]
    async fn test_agent_parent_child_relationship() {
        let db = setup_db().await;

        // Create parent agent
        let parent = Agent::new(AgentType::BmadOrchestrator, "Orchestrate epic");
        db.insert_agent(&parent).await.unwrap();

        // Create child agent
        let child = Agent::new(AgentType::StoryDeveloper, "Develop story 1")
            .with_parent(parent.id);
        db.insert_agent(&child).await.unwrap();

        let fetched_child = db.get_agent(child.id).await.unwrap().unwrap();
        assert_eq!(fetched_child.parent_agent_id, Some(parent.id));
    }

    #[tokio::test]
    async fn test_list_agents_by_state() {
        let db = setup_db().await;

        // Create agents in different states
        let mut running1 = Agent::new(AgentType::StoryDeveloper, "Task 1");
        running1.transition_to(AgentState::Initializing).unwrap();
        running1.transition_to(AgentState::Running).unwrap();
        db.insert_agent(&running1).await.unwrap();

        let mut running2 = Agent::new(AgentType::CodeReviewer, "Task 2");
        running2.transition_to(AgentState::Initializing).unwrap();
        running2.transition_to(AgentState::Running).unwrap();
        db.insert_agent(&running2).await.unwrap();

        let created = Agent::new(AgentType::Explorer, "Task 3");
        db.insert_agent(&created).await.unwrap();

        // List running agents
        let running = db.list_agents_by_state(AgentState::Running).await.unwrap();
        assert_eq!(running.len(), 2);

        // List created agents
        let created_list = db.list_agents_by_state(AgentState::Created).await.unwrap();
        assert_eq!(created_list.len(), 1);
    }

    #[tokio::test]
    async fn test_optimistic_locking() {
        let db = setup_db().await;

        let mut agent = Agent::new(AgentType::StoryDeveloper, "Task");
        db.insert_agent(&agent).await.unwrap();

        let original_updated_at = agent.updated_at.to_rfc3339();

        // First update should succeed
        agent.transition_to(AgentState::Initializing).unwrap();
        let updated = db.update_agent_with_version(&agent, &original_updated_at).await.unwrap();
        assert!(updated);

        // Second update with same version should fail
        agent.transition_to(AgentState::Running).unwrap();
        let updated = db.update_agent_with_version(&agent, &original_updated_at).await.unwrap();
        assert!(!updated);
    }

    #[tokio::test]
    async fn test_message_persistence() {
        let db = setup_db().await;

        let agent = Agent::new(AgentType::StoryDeveloper, "Task");
        db.insert_agent(&agent).await.unwrap();

        // Insert messages
        let user_msg = Message::user(agent.id, "Please implement the login form");
        let assistant_msg = Message::assistant(agent.id, "I'll implement the login form now.")
            .with_tokens(100, 50);

        db.insert_message(&user_msg).await.unwrap();
        db.insert_message(&assistant_msg).await.unwrap();

        // Retrieve messages
        let messages = db.get_messages(agent.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Please implement the login form");
        assert_eq!(messages[1].content, "I'll implement the login form now.");
        assert_eq!(messages[1].input_tokens, 100);
        assert_eq!(messages[1].output_tokens, 50);
    }

    #[tokio::test]
    async fn test_pr_lifecycle() {
        let db = setup_db().await;

        // Create PR
        let pr = PullRequest::new("feature/login")
            .with_epic("epic-1")
            .with_title("Add login feature")
            .with_strategy(MergeStrategy::Squash);

        let pr_id = db.insert_pr(&pr).await.unwrap();
        assert!(pr_id > 0);

        // Get pending PRs
        let pending = db.get_pending_prs().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].branch_name, "feature/login");

        // Update status to merged
        db.update_pr_status(pr_id, PrStatus::Merged).await.unwrap();

        // Should no longer be in pending
        let pending = db.get_pending_prs().await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_epic_upsert() {
        let db = setup_db().await;

        // Create agent first (foreign key constraint for epic.start)
        let agent = Agent::new(AgentType::BmadOrchestrator, "Orchestrate epic");
        db.insert_agent(&agent).await.unwrap();

        // Create epic
        let mut epic = Epic::new("7A", "Implement authentication");
        db.upsert_epic(&epic).await.unwrap();

        // Get pending epics
        let pending = db.get_pending_epics().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Implement authentication");

        // Update epic - use the agent we created
        epic.start(agent.id);
        db.upsert_epic(&epic).await.unwrap();

        // Should no longer be pending
        let pending = db.get_pending_epics().await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_step_output_persistence() {
        let db = setup_db().await;

        // Create an agent first (foreign key constraint)
        let agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        db.insert_agent(&agent).await.unwrap();

        let agent_id = AgentId::from_uuid(agent.id);

        // Create step output
        let output = StepOutput::new(
            agent_id,
            "code_generator",
            StepOutputType::Artifact,
            serde_json::json!({
                "file": "src/login.rs",
                "content": "pub fn login() {}"
            }),
        ).unwrap();

        let output_id = db.insert_step_output(&output).await.unwrap();
        assert!(output_id > 0);

        // Retrieve outputs
        let outputs = db.get_step_outputs(agent_id).await.unwrap();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].skill_name, "code_generator");
        assert_eq!(outputs[0].output_type, StepOutputType::Artifact);
    }

    #[tokio::test]
    async fn test_unconsumed_step_outputs() {
        let db = setup_db().await;

        // Create agents first (foreign key constraint)
        let producer = Agent::new(AgentType::StoryDeveloper, "Producer task");
        let consumer = Agent::new(AgentType::CodeReviewer, "Consumer task");
        db.insert_agent(&producer).await.unwrap();
        db.insert_agent(&consumer).await.unwrap();

        let producer_id = AgentId::from_uuid(producer.id);
        let consumer_id = AgentId::from_uuid(consumer.id);

        // Create outputs
        let output1 = StepOutput::new(
            producer_id,
            "analyzer",
            StepOutputType::SkillResult,
            serde_json::json!({"analysis": "done"}),
        ).unwrap();
        let output2 = StepOutput::new(
            producer_id,
            "analyzer",
            StepOutputType::SkillResult,
            serde_json::json!({"analysis": "pending"}),
        ).unwrap();

        db.insert_step_output(&output1).await.unwrap();
        let output2_id = db.insert_step_output(&output2).await.unwrap();

        // Both should be unconsumed
        let unconsumed = db.get_dependency_outputs(&[producer_id]).await.unwrap();
        assert_eq!(unconsumed.len(), 2);

        // Mark one as consumed
        db.mark_outputs_consumed(&[output2_id], consumer_id).await.unwrap();

        // Only one should be unconsumed
        let unconsumed = db.get_dependency_outputs(&[producer_id]).await.unwrap();
        assert_eq!(unconsumed.len(), 1);
    }
}

// ==================== API Integration Tests ====================

mod api {
    use super::*;

    async fn setup_api() -> (axum::Router, Arc<AppState>) {
        let db = Database::in_memory().await.unwrap();
        let state = Arc::new(AppState::new(db, None));
        let router = create_api_router(state.clone());
        (router, state)
    }

    #[tokio::test]
    async fn test_create_and_manage_agent_workflow() {
        let (router, state) = setup_api().await;

        // Step 1: Create agent via API
        let create_response = router.clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"agent_type":"story_developer","task":"Build feature"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create_response.status(), StatusCode::OK);
        let body = body_to_string(create_response.into_body()).await;
        let created: serde_json::Value = serde_json::from_str(&body).unwrap();
        let agent_id = created["id"].as_str().unwrap();

        // Verify in database
        let db_agent = state.db.get_agent(Uuid::parse_str(agent_id).unwrap()).await.unwrap();
        assert!(db_agent.is_some());

        // Step 2: Transition agent to running in database (simulating agent execution)
        let mut agent = db_agent.unwrap();
        agent.transition_to(AgentState::Initializing).unwrap();
        agent.transition_to(AgentState::Running).unwrap();
        state.db.update_agent(&agent).await.unwrap();

        // Step 3: Pause agent via API
        let pause_response = router.clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/pause", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(pause_response.status(), StatusCode::OK);

        // Verify paused in database
        let paused_agent = state.db.get_agent(Uuid::parse_str(agent_id).unwrap()).await.unwrap().unwrap();
        assert_eq!(paused_agent.state, AgentState::Paused);

        // Step 4: Resume agent via API
        let resume_response = router.clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/resume", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resume_response.status(), StatusCode::OK);

        // Verify running again
        let running_agent = state.db.get_agent(Uuid::parse_str(agent_id).unwrap()).await.unwrap().unwrap();
        assert_eq!(running_agent.state, AgentState::Running);

        // Step 5: Terminate agent
        let terminate_response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/terminate", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(terminate_response.status(), StatusCode::OK);

        // Verify terminated
        let terminated_agent = state.db.get_agent(Uuid::parse_str(agent_id).unwrap()).await.unwrap().unwrap();
        assert_eq!(terminated_agent.state, AgentState::Terminated);
    }

    #[tokio::test]
    async fn test_system_status_reflects_database_state() {
        let (router, state) = setup_api().await;

        // Insert agents with various states directly to DB
        let mut running = Agent::new(AgentType::StoryDeveloper, "Running task");
        running.transition_to(AgentState::Initializing).unwrap();
        running.transition_to(AgentState::Running).unwrap();
        state.db.insert_agent(&running).await.unwrap();

        let mut paused = Agent::new(AgentType::CodeReviewer, "Paused task");
        paused.transition_to(AgentState::Initializing).unwrap();
        paused.transition_to(AgentState::Running).unwrap();
        paused.transition_to(AgentState::Paused).unwrap();
        state.db.insert_agent(&paused).await.unwrap();

        let mut completed = Agent::new(AgentType::Explorer, "Completed task");
        completed.transition_to(AgentState::Initializing).unwrap();
        completed.transition_to(AgentState::Running).unwrap();
        completed.transition_to(AgentState::Completed).unwrap();
        state.db.insert_agent(&completed).await.unwrap();

        let mut failed = Agent::new(AgentType::IssueFixer, "Failed task");
        failed.transition_to(AgentState::Initializing).unwrap();
        failed.transition_to(AgentState::Failed).unwrap();
        state.db.insert_agent(&failed).await.unwrap();

        // Query status via API
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_to_string(response.into_body()).await;
        let status: serde_json::Value = serde_json::from_str(&body).unwrap();

        assert_eq!(status["total_agents"], 4);
        assert_eq!(status["running_agents"], 1);
        assert_eq!(status["paused_agents"], 1);
        assert_eq!(status["completed_agents"], 1);
    }

    #[tokio::test]
    async fn test_messages_endpoint_with_conversation() {
        let (router, state) = setup_api().await;

        // Create agent
        let agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        state.db.insert_agent(&agent).await.unwrap();

        // Add messages
        let msg1 = Message::user(agent.id, "Hello");
        let msg2 = Message::assistant(agent.id, "Hi there!");
        let msg3 = Message::user(agent.id, "Can you help?");
        let msg4 = Message::assistant(agent.id, "Of course!");

        state.db.insert_message(&msg1).await.unwrap();
        state.db.insert_message(&msg2).await.unwrap();
        state.db.insert_message(&msg3).await.unwrap();
        state.db.insert_message(&msg4).await.unwrap();

        // Retrieve via API
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/agents/{}/messages", agent.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_to_string(response.into_body()).await;
        let messages: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();

        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "Hello");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"], "Hi there!");
    }

    #[tokio::test]
    async fn test_concurrent_agent_operations() {
        let (_, state) = setup_api().await;

        // Create multiple agents concurrently
        let db = state.db.clone();
        let handles: Vec<_> = (0..10).map(|i| {
            let db = db.clone();
            tokio::spawn(async move {
                let agent = Agent::new(AgentType::StoryDeveloper, format!("Task {}", i));
                db.insert_agent(&agent).await.unwrap();
                agent.id
            })
        }).collect();

        // Wait for all to complete
        let ids: Vec<Uuid> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // Verify all were created
        let all_agents = state.db.list_agents().await.unwrap();
        assert_eq!(all_agents.len(), 10);

        // Verify each agent exists
        for id in ids {
            let agent = state.db.get_agent(id).await.unwrap();
            assert!(agent.is_some());
        }
    }
}

// ==================== Workflow Integration Tests ====================

mod workflow {
    use super::*;

    #[tokio::test]
    async fn test_epic_to_pr_workflow() {
        let db = setup_db().await;

        // Create orchestrator agent first (foreign key constraint)
        let orchestrator = Agent::new(AgentType::BmadOrchestrator, "Orchestrate OAuth2 epic");
        db.insert_agent(&orchestrator).await.unwrap();

        // Create an epic
        let mut epic = Epic::new("AUTH-1", "Implement OAuth2 authentication");
        db.upsert_epic(&epic).await.unwrap();

        // Simulate orchestrator starting the epic
        epic.start(orchestrator.id);
        db.upsert_epic(&epic).await.unwrap();

        // Create story developer agent
        let context = AgentContext {
            epic_id: Some("AUTH-1".to_string()),
            ..Default::default()
        };
        let mut story_dev = Agent::new(AgentType::StoryDeveloper, "Implement OAuth2 flow")
            .with_context(context);
        story_dev.transition_to(AgentState::Initializing).unwrap();
        story_dev.transition_to(AgentState::Running).unwrap();
        db.insert_agent(&story_dev).await.unwrap();

        // Story developer produces output
        let code_output = StepOutput::new(
            AgentId::from_uuid(story_dev.id),
            "code_generator",
            StepOutputType::Artifact,
            serde_json::json!({
                "files": ["src/oauth.rs", "src/auth/mod.rs"],
                "tests_passing": true
            }),
        ).unwrap();
        db.insert_step_output(&code_output).await.unwrap();

        // Story developer completes
        story_dev.transition_to(AgentState::Completed).unwrap();
        db.update_agent(&story_dev).await.unwrap();

        // Create PR
        let pr = PullRequest::new("feature/oauth2")
            .with_epic("AUTH-1")
            .with_title("Implement OAuth2 authentication")
            .with_strategy(MergeStrategy::Squash);
        let pr_id = db.insert_pr(&pr).await.unwrap();

        // Simulate PR being merged
        db.update_pr_status(pr_id, PrStatus::Merged).await.unwrap();

        // Complete the epic
        epic.complete();
        db.upsert_epic(&epic).await.unwrap();

        // Verify final states
        let final_epic = db.get_pending_epics().await.unwrap();
        assert!(final_epic.is_empty()); // Epic is completed, not pending

        let pending_prs = db.get_pending_prs().await.unwrap();
        assert!(pending_prs.is_empty()); // PR is merged, not pending

        let completed_agent = db.get_agent(story_dev.id).await.unwrap().unwrap();
        assert_eq!(completed_agent.state, AgentState::Completed);
    }

    #[tokio::test]
    async fn test_agent_hierarchy() {
        let db = setup_db().await;

        // Create orchestrator (parent)
        let mut orchestrator = Agent::new(AgentType::BmadOrchestrator, "Manage epic implementation");
        orchestrator.transition_to(AgentState::Initializing).unwrap();
        orchestrator.transition_to(AgentState::Running).unwrap();
        db.insert_agent(&orchestrator).await.unwrap();

        // Create child agents
        let child1 = Agent::new(AgentType::StoryDeveloper, "Story 1")
            .with_parent(orchestrator.id);
        let child2 = Agent::new(AgentType::StoryDeveloper, "Story 2")
            .with_parent(orchestrator.id);
        let child3 = Agent::new(AgentType::CodeReviewer, "Review all stories")
            .with_parent(orchestrator.id);

        db.insert_agent(&child1).await.unwrap();
        db.insert_agent(&child2).await.unwrap();
        db.insert_agent(&child3).await.unwrap();

        // List all agents
        let all_agents = db.list_agents().await.unwrap();
        assert_eq!(all_agents.len(), 4);

        // Verify parent-child relationships
        for agent in &all_agents {
            if agent.id != orchestrator.id {
                assert_eq!(agent.parent_agent_id, Some(orchestrator.id));
            }
        }
    }

    #[tokio::test]
    async fn test_step_output_consumption_chain() {
        let db = setup_db().await;

        // Create agents first (foreign key constraint)
        let agent_a_record = Agent::new(AgentType::StoryDeveloper, "Producer");
        let agent_b_record = Agent::new(AgentType::CodeReviewer, "Consumer");
        db.insert_agent(&agent_a_record).await.unwrap();
        db.insert_agent(&agent_b_record).await.unwrap();

        // Agent A produces output
        let agent_a = AgentId::from_uuid(agent_a_record.id);
        let agent_b = AgentId::from_uuid(agent_b_record.id);

        let output = StepOutput::new(
            agent_a,
            "analyzer",
            StepOutputType::SkillResult,
            serde_json::json!({"findings": ["issue1", "issue2"]}),
        ).unwrap();
        let output_id = db.insert_step_output(&output).await.unwrap();

        // Verify output is unconsumed
        let unconsumed = db.get_dependency_outputs(&[agent_a]).await.unwrap();
        assert_eq!(unconsumed.len(), 1);

        // Agent B consumes the output
        db.mark_outputs_consumed(&[output_id], agent_b).await.unwrap();

        // Verify output is now consumed
        let unconsumed = db.get_dependency_outputs(&[agent_a]).await.unwrap();
        assert!(unconsumed.is_empty());

        // Verify consumer is recorded
        let all_outputs = db.get_step_outputs(agent_a).await.unwrap();
        assert_eq!(all_outputs.len(), 1);
        assert!(all_outputs[0].consumed);
        assert_eq!(all_outputs[0].consumed_by, Some(agent_b));
    }
}
