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

// ==================== Instruction System Integration Tests ====================

mod instructions {
    use super::*;
    use orchestrate_core::{
        CustomInstruction, InstructionSource, LearningEngine, LearningPattern,
        PatternStatus, PatternType,
    };

    #[tokio::test]
    async fn test_instruction_crud() {
        let db = setup_db().await;

        // Create a global instruction
        let instruction = CustomInstruction::global(
            "no-force-push",
            "Never use git push --force without explicit user approval",
        ).with_priority(150);

        let id = db.insert_instruction(&instruction).await.unwrap();
        assert!(id > 0);

        // Retrieve by ID
        let fetched = db.get_instruction(id).await.unwrap().unwrap();
        assert_eq!(fetched.name, "no-force-push");
        assert_eq!(fetched.priority, 150);
        assert!(fetched.enabled);

        // Retrieve by name
        let fetched_by_name = db.get_instruction_by_name("no-force-push").await.unwrap().unwrap();
        assert_eq!(fetched_by_name.id, id);

        // Update
        let mut updated = fetched;
        updated.content = "Never use force push".to_string();
        updated.priority = 200;
        db.update_instruction(&updated).await.unwrap();

        let fetched = db.get_instruction(id).await.unwrap().unwrap();
        assert_eq!(fetched.content, "Never use force push");
        assert_eq!(fetched.priority, 200);

        // Disable
        db.set_instruction_enabled(id, false).await.unwrap();
        let fetched = db.get_instruction(id).await.unwrap().unwrap();
        assert!(!fetched.enabled);

        // Delete
        db.delete_instruction(id).await.unwrap();
        let fetched = db.get_instruction(id).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_instructions_for_agent_type() {
        let db = setup_db().await;

        // Create global instruction
        let global = CustomInstruction::global(
            "global-rule",
            "This applies to all agents",
        );
        db.insert_instruction(&global).await.unwrap();

        // Create instruction for StoryDeveloper
        let story_dev = CustomInstruction::for_agent_type(
            "story-dev-rule",
            "This applies only to story developers",
            AgentType::StoryDeveloper,
        );
        db.insert_instruction(&story_dev).await.unwrap();

        // Create instruction for CodeReviewer
        let code_rev = CustomInstruction::for_agent_type(
            "code-rev-rule",
            "This applies only to code reviewers",
            AgentType::CodeReviewer,
        );
        db.insert_instruction(&code_rev).await.unwrap();

        // Story developer should get global + story-dev-rule
        let story_insts = db.get_instructions_for_agent(AgentType::StoryDeveloper).await.unwrap();
        assert_eq!(story_insts.len(), 2);
        assert!(story_insts.iter().any(|i| i.name == "global-rule"));
        assert!(story_insts.iter().any(|i| i.name == "story-dev-rule"));
        assert!(!story_insts.iter().any(|i| i.name == "code-rev-rule"));

        // Code reviewer should get global + code-rev-rule
        let code_insts = db.get_instructions_for_agent(AgentType::CodeReviewer).await.unwrap();
        assert_eq!(code_insts.len(), 2);
        assert!(code_insts.iter().any(|i| i.name == "global-rule"));
        assert!(code_insts.iter().any(|i| i.name == "code-rev-rule"));
        assert!(!code_insts.iter().any(|i| i.name == "story-dev-rule"));
    }

    #[tokio::test]
    async fn test_instruction_usage_and_effectiveness() {
        let db = setup_db().await;

        // Create instruction
        let instruction = CustomInstruction::global("test-rule", "Test content");
        let id = db.insert_instruction(&instruction).await.unwrap();

        // Create an agent
        let agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        db.insert_agent(&agent).await.unwrap();

        // Record usage
        db.record_instruction_usage(id, agent.id, None).await.unwrap();
        db.record_instruction_usage(id, agent.id, None).await.unwrap();

        // Record outcomes
        db.record_instruction_outcome(id, true, Some(10.5)).await.unwrap();
        db.record_instruction_outcome(id, false, Some(5.0)).await.unwrap();

        // Check effectiveness
        let eff = db.get_instruction_effectiveness(id).await.unwrap().unwrap();
        assert_eq!(eff.usage_count, 2);
        assert_eq!(eff.success_count, 1);
        assert_eq!(eff.failure_count, 1);
        assert_eq!(eff.success_rate, 0.5);
    }

    #[tokio::test]
    async fn test_penalty_system() {
        let db = setup_db().await;

        // Create instruction
        let instruction = CustomInstruction::global("penalty-test", "Test content");
        let id = db.insert_instruction(&instruction).await.unwrap();

        // Apply penalties
        db.apply_penalty(id, 0.2, "test_failure").await.unwrap();
        db.apply_penalty(id, 0.3, "another_failure").await.unwrap();

        let eff = db.get_instruction_effectiveness(id).await.unwrap().unwrap();
        assert!((eff.penalty_score - 0.5).abs() < 0.001);

        // Decay penalty
        db.decay_penalty(id, 0.1).await.unwrap();

        let eff = db.get_instruction_effectiveness(id).await.unwrap().unwrap();
        assert!((eff.penalty_score - 0.4).abs() < 0.001);

        // Reset penalty
        db.reset_penalty(id).await.unwrap();

        let eff = db.get_instruction_effectiveness(id).await.unwrap().unwrap();
        assert!((eff.penalty_score).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_auto_disable_penalized() {
        let db = setup_db().await;

        // Create instructions with different penalty scores
        let low_penalty = CustomInstruction::global("low-penalty", "Content");
        let id1 = db.insert_instruction(&low_penalty).await.unwrap();
        db.apply_penalty(id1, 0.3, "minor").await.unwrap();

        let high_penalty = CustomInstruction::global("high-penalty", "Content");
        let id2 = db.insert_instruction(&high_penalty).await.unwrap();
        db.apply_penalty(id2, 0.8, "major").await.unwrap();

        // Auto-disable with threshold 0.7
        let disabled = db.auto_disable_penalized(0.7).await.unwrap();
        assert_eq!(disabled.len(), 1);
        assert!(disabled.contains(&id2));

        // Verify states
        let inst1 = db.get_instruction(id1).await.unwrap().unwrap();
        assert!(inst1.enabled);

        let inst2 = db.get_instruction(id2).await.unwrap().unwrap();
        assert!(!inst2.enabled);
    }

    #[tokio::test]
    async fn test_learning_pattern_workflow() {
        let db = setup_db().await;

        // Create a pattern
        let pattern = LearningPattern::new(
            PatternType::ErrorPattern,
            "error_12345678",
            serde_json::json!({
                "error_text": "Permission denied",
                "category": "permission_error",
            }),
        ).with_agent_type(AgentType::StoryDeveloper);

        db.upsert_learning_pattern(&pattern).await.unwrap();

        // Check patterns
        let patterns = db.list_patterns(None).await.unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].occurrence_count, 1);

        // Upsert same pattern - should increment count
        db.upsert_learning_pattern(&pattern).await.unwrap();

        let patterns = db.list_patterns(None).await.unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].occurrence_count, 2);

        // Get pattern by ID
        let fetched = db.get_pattern(patterns[0].id).await.unwrap().unwrap();
        assert_eq!(fetched.pattern_signature, "error_12345678");
    }

    #[tokio::test]
    async fn test_pattern_approval_creates_instruction() {
        let db = setup_db().await;

        // Create a pattern
        let pattern = LearningPattern::new(
            PatternType::ErrorPattern,
            "approve_test_123",
            serde_json::json!({
                "error_text": "Connection timeout",
                "category": "timeout_error",
            }),
        );

        db.upsert_learning_pattern(&pattern).await.unwrap();

        let patterns = db.list_patterns(Some(PatternStatus::Observed)).await.unwrap();
        let pattern_id = patterns[0].id;

        // Generate instruction from pattern
        let engine = LearningEngine::new();
        let instruction = engine.generate_instruction_from_pattern(&patterns[0]).unwrap();

        // Insert instruction and update pattern status
        let instruction_id = db.insert_instruction(&instruction).await.unwrap();
        db.update_pattern_status(pattern_id, PatternStatus::Approved, Some(instruction_id)).await.unwrap();

        // Verify pattern is now approved
        let pattern = db.get_pattern(pattern_id).await.unwrap().unwrap();
        assert_eq!(pattern.status, PatternStatus::Approved);
        assert_eq!(pattern.instruction_id, Some(instruction_id));

        // Verify instruction was created
        let inst = db.get_instruction(instruction_id).await.unwrap().unwrap();
        assert!(inst.name.starts_with("learned_"));
        assert_eq!(inst.source, InstructionSource::Learned);
    }

    #[tokio::test]
    async fn test_process_patterns() {
        let db = setup_db().await;

        // Create multiple occurrences of same pattern (to meet threshold)
        let pattern = LearningPattern::new(
            PatternType::ErrorPattern,
            "process_test_456",
            serde_json::json!({
                "error_text": "File not found",
                "category": "not_found_error",
            }),
        );

        // Insert pattern multiple times to meet min_occurrences
        for _ in 0..5 {
            db.upsert_learning_pattern(&pattern).await.unwrap();
        }

        // Process patterns with default config (min_occurrences = 3)
        let engine = LearningEngine::new();
        let created = engine.process_patterns(&db).await.unwrap();

        assert_eq!(created.len(), 1);
        assert!(created[0].name.starts_with("learned_"));

        // Pattern should now be pending_review (confidence < auto_approve_threshold)
        // because 5 occurrences gives confidence of 0.8 < 0.9
        let patterns = db.list_patterns(Some(PatternStatus::PendingReview)).await.unwrap();
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].instruction_id.is_some());
    }

    #[tokio::test]
    async fn test_delete_ineffective_instructions() {
        let db = setup_db().await;

        // Create an agent first for usage tracking
        let agent = Agent::new(AgentType::StoryDeveloper, "Test");
        db.insert_agent(&agent).await.unwrap();

        // Create a learned instruction with high penalty and low success
        let instruction = CustomInstruction::learned(
            "ineffective-learned",
            "This instruction doesn't help",
            0.5,
        );
        let id = db.insert_instruction(&instruction).await.unwrap();

        // Simulate lots of usage with low success
        // Need to record usage (increments usage_count) and outcomes
        for _ in 0..15 {
            db.record_instruction_usage(id, agent.id, None).await.unwrap();
            db.record_instruction_outcome(id, false, None).await.unwrap();
        }
        db.record_instruction_usage(id, agent.id, None).await.unwrap();
        db.record_instruction_outcome(id, true, None).await.unwrap();

        // Apply high penalty
        db.apply_penalty(id, 1.0, "high_failure_rate").await.unwrap();

        // Verify metrics before deletion
        let eff = db.get_instruction_effectiveness(id).await.unwrap().unwrap();
        assert_eq!(eff.usage_count, 16);
        assert_eq!(eff.success_count, 1);
        assert_eq!(eff.failure_count, 15);
        assert!(eff.penalty_score >= 1.0);

        // Delete ineffective (penalty >= 1.0, usage >= 10, success < 30%, source = learned)
        let deleted = db.delete_ineffective_instructions().await.unwrap();
        assert_eq!(deleted.len(), 1);
        assert!(deleted.contains(&"ineffective-learned".to_string()));

        // Verify deleted
        let inst = db.get_instruction(id).await.unwrap();
        assert!(inst.is_none());
    }
}

// ==================== Token Tracking Tests ====================

mod token_tracking {
    use super::*;
    use orchestrate_core::Session;

    #[tokio::test]
    async fn test_record_session_tokens() {
        let db = setup_db().await;

        // Create agent and session
        let agent = Agent::new(AgentType::StoryDeveloper, "Test token tracking");
        db.insert_agent(&agent).await.unwrap();

        let session = Session::new(agent.id);
        db.create_session(&session).await.unwrap();

        // Record token usage for multiple turns
        db.record_session_tokens(
            &session.id,
            agent.id,
            1,      // turn
            1000,   // input_tokens
            500,    // output_tokens
            800,    // cache_read_tokens
            200,    // cache_write_tokens
            5000,   // context_window_used
            10,     // messages_included
            2,      // messages_summarized
        ).await.unwrap();

        db.record_session_tokens(
            &session.id,
            agent.id,
            2,
            1200,
            600,
            1000,
            100,
            6000,
            12,
            3,
        ).await.unwrap();

        // Get session stats
        let stats = db.get_session_token_stats(&session.id).await.unwrap();

        assert_eq!(stats.turn_count, 2);
        assert_eq!(stats.total_input_tokens, 2200);
        assert_eq!(stats.total_output_tokens, 1100);
        assert_eq!(stats.total_cache_read_tokens, 1800);
        assert_eq!(stats.total_cache_write_tokens, 300);
        assert_eq!(stats.total_messages_summarized, 5);

        // Cache hit rate should be (1800 / 2200) * 100 = 81.8%
        assert!(stats.cache_hit_rate > 80.0 && stats.cache_hit_rate < 83.0);
    }

    #[tokio::test]
    async fn test_get_agent_token_stats() {
        let db = setup_db().await;

        // Create agent
        let agent = Agent::new(AgentType::Explorer, "Test agent tokens");
        db.insert_agent(&agent).await.unwrap();

        // Create session and record tokens
        let session = Session::new(agent.id);
        db.create_session(&session).await.unwrap();

        db.record_session_tokens(
            &session.id,
            agent.id,
            1,
            2000,
            1000,
            1500,
            500,
            10000,
            20,
            5,
        ).await.unwrap();

        // Get stats by agent ID
        let stats = db.get_agent_token_stats(agent.id).await.unwrap();

        assert_eq!(stats.turn_count, 1);
        assert_eq!(stats.total_input_tokens, 2000);
        assert_eq!(stats.total_output_tokens, 1000);
        assert_eq!(stats.total_cache_read_tokens, 1500);
    }

    #[tokio::test]
    async fn test_daily_token_usage_aggregation() {
        let db = setup_db().await;

        // Record usage for different models
        db.update_daily_token_usage(
            "claude-sonnet-4-20250514",
            5000,
            2000,
            3000,
            1000,
        ).await.unwrap();

        db.update_daily_token_usage(
            "claude-sonnet-4-20250514",
            3000,
            1500,
            2000,
            500,
        ).await.unwrap();

        db.update_daily_token_usage(
            "claude-haiku-3-20240307",
            1000,
            500,
            800,
            200,
        ).await.unwrap();

        // Get daily usage
        let usage = db.get_daily_token_usage(1).await.unwrap();

        // Should have 2 entries (one per model for today)
        assert_eq!(usage.len(), 2);

        // Find sonnet entry
        let sonnet = usage.iter()
            .find(|u| u.model.contains("sonnet"))
            .unwrap();

        assert_eq!(sonnet.total_input_tokens, 8000);
        assert_eq!(sonnet.total_output_tokens, 3500);
        assert_eq!(sonnet.total_cache_read_tokens, 5000);
        assert_eq!(sonnet.total_cache_write_tokens, 1500);
        assert_eq!(sonnet.request_count, 2);

        // Cost should be calculated (Sonnet: $3/$15 per 1M)
        assert!(sonnet.estimated_cost_usd.is_some());
        let cost = sonnet.estimated_cost_usd.unwrap();
        assert!(cost > 0.0);
    }

    #[tokio::test]
    async fn test_empty_token_stats() {
        let db = setup_db().await;

        // Agent with no token data
        let agent = Agent::new(AgentType::CodeReviewer, "No tokens");
        db.insert_agent(&agent).await.unwrap();

        // Get stats should return zeros
        let stats = db.get_agent_token_stats(agent.id).await.unwrap();

        assert_eq!(stats.turn_count, 0);
        assert_eq!(stats.total_input_tokens, 0);
        assert_eq!(stats.total_output_tokens, 0);
        assert_eq!(stats.cache_hit_rate, 0.0);
    }

    #[tokio::test]
    async fn test_instruction_token_tracking() {
        let db = setup_db().await;

        // Create instruction
        let instruction = orchestrate_core::CustomInstruction::global("token-test", "Test instruction for token tracking");
        let id = db.insert_instruction(&instruction).await.unwrap();

        // Record token usage - this should not fail
        db.update_instruction_tokens(id, 5000, 2000, 3000, 1000).await.unwrap();
        db.update_instruction_tokens(id, 3000, 1500, 2000, 500).await.unwrap();

        // Verify instruction still exists and can be retrieved
        let inst = db.get_instruction(id).await.unwrap();
        assert!(inst.is_some());

        // Verify effectiveness record exists
        let eff = db.get_instruction_effectiveness(id).await.unwrap();
        assert!(eff.is_some());
    }
}
