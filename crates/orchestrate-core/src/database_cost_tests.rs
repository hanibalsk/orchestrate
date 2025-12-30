//! Database tests for cost analytics operations

use crate::cost_analytics::{
    BudgetPeriod, CostBudget, CostRecommendation, EntityType, RecommendationType,
};
use crate::database::Database;

#[tokio::test]
async fn test_create_and_get_budget() {
    let db = Database::in_memory().await.unwrap();

    let budget = CostBudget::new(BudgetPeriod::Monthly, 100.0)
        .with_alert_threshold(85)
        .with_start_date("2025-01-01".to_string());

    let created = db.create_cost_budget(budget).await.unwrap();
    assert!(created.id.is_some());
    assert_eq!(created.period_type, BudgetPeriod::Monthly);
    assert_eq!(created.amount_usd, 100.0);
    assert_eq!(created.alert_threshold_percent, 85);

    let fetched = db.get_cost_budget(created.id.unwrap()).await.unwrap().unwrap();
    assert_eq!(fetched.amount_usd, 100.0);
    assert_eq!(fetched.alert_threshold_percent, 85);
}

#[tokio::test]
async fn test_get_active_budget() {
    let db = Database::in_memory().await.unwrap();

    let budget1 = CostBudget::new(BudgetPeriod::Monthly, 100.0)
        .with_start_date("2024-01-01".to_string());
    let budget2 = CostBudget::new(BudgetPeriod::Monthly, 200.0)
        .with_start_date("2025-01-01".to_string());

    db.create_cost_budget(budget1).await.unwrap();
    db.create_cost_budget(budget2).await.unwrap();

    let active = db
        .get_active_budget(BudgetPeriod::Monthly)
        .await
        .unwrap()
        .unwrap();
    // Should get the most recent budget
    assert_eq!(active.amount_usd, 200.0);
}

#[tokio::test]
async fn test_update_cost_by_agent() {
    let db = Database::in_memory().await.unwrap();

    // First create an agent
    let agent = crate::Agent::new(crate::AgentType::StoryDeveloper, "test task");
    db.insert_agent(&agent).await.unwrap();

    let agent_id = agent.id.to_string();

    // Update cost
    db.update_cost_by_agent(
        &agent_id,
        "claude-sonnet-3.5",
        100_000,
        50_000,
        20_000,
        10_000,
    )
    .await
    .unwrap();

    // Get costs for the agent
    let costs = db.get_costs_by_agent(&agent_id, 7).await.unwrap();
    assert_eq!(costs.len(), 1);
    assert_eq!(costs[0].entity_id, agent_id);
    assert_eq!(costs[0].model, "claude-sonnet-3.5");
    assert_eq!(costs[0].total_input_tokens, 100_000);
    assert_eq!(costs[0].total_output_tokens, 50_000);
}

#[tokio::test]
async fn test_update_cost_by_epic() {
    let db = Database::in_memory().await.unwrap();

    // Create an epic
    let epic = crate::Epic::new("Epic 001", "Test Epic");
    db.upsert_epic(&epic).await.unwrap();

    // Update cost
    db.update_cost_by_epic("Epic 001", "claude-sonnet-3.5", 100_000, 50_000, 20_000, 10_000)
        .await
        .unwrap();

    // Get costs for the epic
    let costs = db.get_costs_by_epic("Epic 001", 7).await.unwrap();
    assert_eq!(costs.len(), 1);
    assert_eq!(costs[0].entity_id, "Epic 001");
    assert!(costs[0].estimated_cost_usd > 0.0);
}

#[tokio::test]
async fn test_update_cost_by_story() {
    let db = Database::in_memory().await.unwrap();

    // Create epic first
    let epic = crate::Epic::new("Epic 001", "Test Epic");
    db.upsert_epic(&epic).await.unwrap();

    // Create a story
    let story = crate::Story::new("Story 1", "Epic 001", "Test Story");
    db.upsert_story(&story).await.unwrap();

    // Update cost
    db.update_cost_by_story("Story 1", "claude-sonnet-3.5", 100_000, 50_000, 20_000, 10_000)
        .await
        .unwrap();

    // Get costs for the story
    let costs = db.get_costs_by_story("Story 1", 7).await.unwrap();
    assert_eq!(costs.len(), 1);
    assert_eq!(costs[0].entity_id, "Story 1");
}

#[tokio::test]
async fn test_aggregate_costs_across_entities() {
    let db = Database::in_memory().await.unwrap();

    // Create multiple agents
    let agent1 = crate::Agent::new(crate::AgentType::StoryDeveloper, "task 1");
    let agent2 = crate::Agent::new(crate::AgentType::CodeReviewer, "task 2");
    db.insert_agent(&agent1).await.unwrap();
    db.insert_agent(&agent2).await.unwrap();

    let agent1_id = agent1.id.to_string();
    let agent2_id = agent2.id.to_string();

    // Add costs
    db.update_cost_by_agent(&agent1_id, "claude-sonnet-3.5", 100_000, 50_000, 20_000, 10_000)
        .await
        .unwrap();
    db.update_cost_by_agent(&agent2_id, "claude-opus-4", 50_000, 25_000, 10_000, 5_000)
        .await
        .unwrap();

    let costs = db.get_costs_by_agent(&agent1_id, 7).await.unwrap();
    assert_eq!(costs.len(), 1);

    let costs = db.get_costs_by_agent(&agent2_id, 7).await.unwrap();
    assert_eq!(costs.len(), 1);
}

#[tokio::test]
async fn test_create_cost_recommendation() {
    let db = Database::in_memory().await.unwrap();

    let recommendation = CostRecommendation::new(
        RecommendationType::ModelDowngrade,
        EntityType::Agent,
        Some("agent-1".to_string()),
        "Switch from Opus to Sonnet for routine tasks".to_string(),
        10.5,
        0.8,
    );

    let created = db.create_cost_recommendation(recommendation).await.unwrap();
    assert!(created.id.is_some());
    assert_eq!(created.recommendation_type, RecommendationType::ModelDowngrade);
    assert_eq!(created.potential_savings_usd, 10.5);
    assert_eq!(created.confidence_score, 0.8);
}

#[tokio::test]
async fn test_list_cost_recommendations() {
    let db = Database::in_memory().await.unwrap();

    let rec1 = CostRecommendation::new(
        RecommendationType::ModelDowngrade,
        EntityType::Agent,
        Some("agent-1".to_string()),
        "Recommendation 1".to_string(),
        10.0,
        0.8,
    );

    let rec2 = CostRecommendation::new(
        RecommendationType::CacheOptimization,
        EntityType::Epic,
        Some("epic-1".to_string()),
        "Recommendation 2".to_string(),
        5.0,
        0.7,
    );

    db.create_cost_recommendation(rec1).await.unwrap();
    db.create_cost_recommendation(rec2).await.unwrap();

    let recommendations = db.list_cost_recommendations(false).await.unwrap();
    assert_eq!(recommendations.len(), 2);

    // Mark one as applied
    let id = recommendations[0].id.unwrap();
    db.mark_recommendation_applied(id).await.unwrap();

    let unapplied = db.list_cost_recommendations(false).await.unwrap();
    assert_eq!(unapplied.len(), 1);

    let all = db.list_cost_recommendations(true).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_generate_cost_report() {
    let db = Database::in_memory().await.unwrap();

    // Create test data
    let agent = crate::Agent::new(crate::AgentType::StoryDeveloper, "test task");
    db.insert_agent(&agent).await.unwrap();

    let agent_id = agent.id.to_string();

    // Add some token usage - need to update daily token usage too for report
    db.update_daily_token_usage("claude-sonnet-3.5", 100_000, 50_000, 20_000, 10_000)
        .await
        .unwrap();

    db.update_cost_by_agent(&agent_id, "claude-sonnet-3.5", 100_000, 50_000, 20_000, 10_000)
        .await
        .unwrap();

    // Generate report
    let report = db.generate_cost_report(7).await.unwrap();
    assert!(report.total_cost_usd >= 0.0);
    assert!(!report.daily_costs.is_empty());
}

#[tokio::test]
async fn test_cost_report_with_budget() {
    let db = Database::in_memory().await.unwrap();

    // Create a budget
    let budget = CostBudget::new(BudgetPeriod::Monthly, 100.0);
    db.create_cost_budget(budget).await.unwrap();

    // Create test data
    let agent = crate::Agent::new(crate::AgentType::StoryDeveloper, "test task");
    db.insert_agent(&agent).await.unwrap();

    let agent_id = agent.id.to_string();

    // Add costs - update both tables
    db.update_daily_token_usage("claude-opus-4", 1_000_000, 500_000, 200_000, 100_000)
        .await
        .unwrap();

    db.update_cost_by_agent(&agent_id, "claude-opus-4", 1_000_000, 500_000, 200_000, 100_000)
        .await
        .unwrap();

    // Generate report (30 days for monthly)
    let report = db.generate_cost_report(30).await.unwrap();

    if let Some(budget_status) = &report.budget_status {
        assert!(budget_status.percentage_used >= 0.0);
    }
}

#[tokio::test]
async fn test_cost_per_model() {
    let db = Database::in_memory().await.unwrap();

    // Create agents
    let agent1 = crate::Agent::new(crate::AgentType::StoryDeveloper, "task 1");
    let agent2 = crate::Agent::new(crate::AgentType::CodeReviewer, "task 2");
    db.insert_agent(&agent1).await.unwrap();
    db.insert_agent(&agent2).await.unwrap();

    let agent1_id = agent1.id.to_string();
    let agent2_id = agent2.id.to_string();

    // Add costs for different models
    db.update_cost_by_agent(&agent1_id, "claude-sonnet-3.5", 100_000, 50_000, 20_000, 10_000)
        .await
        .unwrap();
    db.update_cost_by_agent(&agent2_id, "claude-opus-4", 50_000, 25_000, 10_000, 5_000)
        .await
        .unwrap();

    let by_model = db.get_cost_by_model(7).await.unwrap();
    assert!(by_model.len() >= 1);
}
