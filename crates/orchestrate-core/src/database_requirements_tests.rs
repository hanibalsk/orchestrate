//! Tests for requirements database operations

use crate::{
    ArtifactType, ClarifyingQuestion, Database, EffortEstimate, GeneratedStory, ImpactAnalysis,
    LinkType, Requirement, RequirementPriority, RequirementStatus, RequirementType, RiskLevel,
    StoryComplexity, TraceabilityLink, TraceabilityMatrix,
};
use chrono::Utc;

#[tokio::test]
async fn test_create_and_get_requirement() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-001",
        "User Authentication",
        "System must authenticate users with email and password",
        RequirementType::Functional,
    );

    db.create_requirement(&req).await.unwrap();

    let retrieved = db.get_requirement("REQ-001").await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "REQ-001");
    assert_eq!(retrieved.title, "User Authentication");
    assert_eq!(retrieved.requirement_type, RequirementType::Functional);
    assert_eq!(retrieved.status, RequirementStatus::Draft);
}

#[tokio::test]
async fn test_update_requirement() {
    let db = Database::in_memory().await.unwrap();

    let mut req = Requirement::new(
        "REQ-002",
        "Data Encryption",
        "All data must be encrypted at rest",
        RequirementType::Security,
    );

    db.create_requirement(&req).await.unwrap();

    // Update requirement
    req.status = RequirementStatus::Approved;
    req.priority = RequirementPriority::High;
    req.stakeholders = vec!["Security Team".to_string(), "CTO".to_string()];
    req.version = 2;

    db.update_requirement(&req).await.unwrap();

    let retrieved = db.get_requirement("REQ-002").await.unwrap().unwrap();
    assert_eq!(retrieved.status, RequirementStatus::Approved);
    assert_eq!(retrieved.priority, RequirementPriority::High);
    assert_eq!(retrieved.version, 2);
    assert_eq!(retrieved.stakeholders.len(), 2);
}

#[tokio::test]
async fn test_list_requirements() {
    let db = Database::in_memory().await.unwrap();

    let req1 = Requirement::new(
        "REQ-010",
        "Requirement 1",
        "Description 1",
        RequirementType::Functional,
    );
    let req2 = Requirement::new(
        "REQ-011",
        "Requirement 2",
        "Description 2",
        RequirementType::Performance,
    );

    db.create_requirement(&req1).await.unwrap();
    db.create_requirement(&req2).await.unwrap();

    let all = db.list_requirements(None, None).await.unwrap();
    assert!(all.len() >= 2);

    let functional = db
        .list_requirements(Some(RequirementType::Functional), None)
        .await
        .unwrap();
    assert!(functional.iter().any(|r| r.id == "REQ-010"));

    let draft = db
        .list_requirements(None, Some(RequirementStatus::Draft))
        .await
        .unwrap();
    assert!(draft.iter().any(|r| r.id == "REQ-010"));
}

#[tokio::test]
async fn test_create_and_get_clarifying_question() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-020",
        "Test Requirement",
        "Test",
        RequirementType::Functional,
    );
    db.create_requirement(&req).await.unwrap();

    let question = ClarifyingQuestion {
        id: "Q-001".to_string(),
        requirement_id: "REQ-020".to_string(),
        question: "Should we support OAuth?".to_string(),
        context: "Authentication methods".to_string(),
        options: vec![
            "Yes, OAuth 2.0".to_string(),
            "No, just email/password".to_string(),
        ],
        answer: None,
        answered_at: None,
    };

    db.create_clarifying_question(&question).await.unwrap();

    let retrieved = db.get_clarifying_question("Q-001").await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.question, "Should we support OAuth?");
    assert_eq!(retrieved.options.len(), 2);
}

#[tokio::test]
async fn test_answer_clarifying_question() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-021",
        "Test",
        "Test",
        RequirementType::Functional,
    );
    db.create_requirement(&req).await.unwrap();

    let question = ClarifyingQuestion {
        id: "Q-002".to_string(),
        requirement_id: "REQ-021".to_string(),
        question: "Test question?".to_string(),
        context: "Test".to_string(),
        options: vec!["Option A".to_string(), "Option B".to_string()],
        answer: None,
        answered_at: None,
    };

    db.create_clarifying_question(&question).await.unwrap();

    // Answer the question
    db.answer_clarifying_question("Q-002", "Option A")
        .await
        .unwrap();

    let retrieved = db.get_clarifying_question("Q-002").await.unwrap().unwrap();
    assert_eq!(retrieved.answer, Some("Option A".to_string()));
    assert!(retrieved.answered_at.is_some());
}

#[tokio::test]
async fn test_get_unanswered_questions() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-030",
        "Test",
        "Test",
        RequirementType::Functional,
    );
    db.create_requirement(&req).await.unwrap();

    let q1 = ClarifyingQuestion {
        id: "Q-010".to_string(),
        requirement_id: "REQ-030".to_string(),
        question: "Question 1?".to_string(),
        context: "Context".to_string(),
        options: vec![],
        answer: None,
        answered_at: None,
    };

    let q2 = ClarifyingQuestion {
        id: "Q-011".to_string(),
        requirement_id: "REQ-030".to_string(),
        question: "Question 2?".to_string(),
        context: "Context".to_string(),
        options: vec![],
        answer: Some("Answer".to_string()),
        answered_at: Some(Utc::now()),
    };

    db.create_clarifying_question(&q1).await.unwrap();
    db.create_clarifying_question(&q2).await.unwrap();

    let unanswered = db.get_unanswered_questions("REQ-030").await.unwrap();
    assert_eq!(unanswered.len(), 1);
    assert_eq!(unanswered[0].id, "Q-010");
}

#[tokio::test]
async fn test_create_and_get_generated_story() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-040",
        "Test",
        "Test",
        RequirementType::Functional,
    );
    db.create_requirement(&req).await.unwrap();

    let story = GeneratedStory {
        title: "User Login".to_string(),
        user_type: "registered user".to_string(),
        goal: "log in to the system".to_string(),
        benefit: "I can access my account".to_string(),
        acceptance_criteria: vec![
            "User can enter credentials".to_string(),
            "System validates credentials".to_string(),
        ],
        complexity: StoryComplexity::Medium,
        related_requirements: vec!["REQ-040".to_string()],
        suggested_epic: Some("Authentication".to_string()),
    };

    let story_id = db.create_generated_story(&story, "REQ-040").await.unwrap();

    let retrieved = db.get_generated_story(&story_id).await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.title, "User Login");
    assert_eq!(retrieved.complexity, StoryComplexity::Medium);
    assert_eq!(retrieved.acceptance_criteria.len(), 2);
}

#[tokio::test]
async fn test_get_stories_for_requirement() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-050",
        "Test",
        "Test",
        RequirementType::Functional,
    );
    db.create_requirement(&req).await.unwrap();

    let story1 = GeneratedStory {
        title: "Story 1".to_string(),
        user_type: "user".to_string(),
        goal: "goal 1".to_string(),
        benefit: "benefit 1".to_string(),
        acceptance_criteria: vec![],
        complexity: StoryComplexity::Simple,
        related_requirements: vec!["REQ-050".to_string()],
        suggested_epic: None,
    };

    let story2 = GeneratedStory {
        title: "Story 2".to_string(),
        user_type: "user".to_string(),
        goal: "goal 2".to_string(),
        benefit: "benefit 2".to_string(),
        acceptance_criteria: vec![],
        complexity: StoryComplexity::Complex,
        related_requirements: vec!["REQ-050".to_string()],
        suggested_epic: None,
    };

    db.create_generated_story(&story1, "REQ-050")
        .await
        .unwrap();
    db.create_generated_story(&story2, "REQ-050")
        .await
        .unwrap();

    let stories = db.get_stories_for_requirement("REQ-050").await.unwrap();
    assert_eq!(stories.len(), 2);
}

#[tokio::test]
async fn test_create_and_get_traceability_link() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-060",
        "Test",
        "Test",
        RequirementType::Functional,
    );
    db.create_requirement(&req).await.unwrap();

    let link = TraceabilityLink {
        source_type: ArtifactType::Requirement,
        source_id: "REQ-060".to_string(),
        target_type: ArtifactType::Story,
        target_id: "STORY-001".to_string(),
        link_type: LinkType::ImplementedBy,
        created_at: Utc::now(),
    };

    db.create_traceability_link(&link).await.unwrap();

    let links = db
        .get_traceability_links_for_requirement("REQ-060")
        .await
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target_id, "STORY-001");
}

#[tokio::test]
async fn test_build_traceability_matrix() {
    let db = Database::in_memory().await.unwrap();

    let req1 = Requirement::new(
        "REQ-070",
        "Req 1",
        "Test",
        RequirementType::Functional,
    );
    let req2 = Requirement::new(
        "REQ-071",
        "Req 2",
        "Test",
        RequirementType::Functional,
    );

    db.create_requirement(&req1).await.unwrap();
    db.create_requirement(&req2).await.unwrap();

    let link1 = TraceabilityLink {
        source_type: ArtifactType::Requirement,
        source_id: "REQ-070".to_string(),
        target_type: ArtifactType::Story,
        target_id: "STORY-010".to_string(),
        link_type: LinkType::ImplementedBy,
        created_at: Utc::now(),
    };

    let link2 = TraceabilityLink {
        source_type: ArtifactType::Requirement,
        source_id: "REQ-070".to_string(),
        target_type: ArtifactType::Test,
        target_id: "TEST-010".to_string(),
        link_type: LinkType::TestedBy,
        created_at: Utc::now(),
    };

    db.create_traceability_link(&link1).await.unwrap();
    db.create_traceability_link(&link2).await.unwrap();

    let matrix = db
        .build_traceability_matrix(vec!["REQ-070".to_string(), "REQ-071".to_string()])
        .await
        .unwrap();

    assert_eq!(matrix.requirements.len(), 2);
    assert!(matrix.requirements.contains(&"REQ-070".to_string()));
    assert!(matrix.requirements.contains(&"REQ-071".to_string()));
    assert_eq!(matrix.links.len(), 2);

    // Check coverage
    let cov = matrix.coverage.get("REQ-070").unwrap();
    assert_eq!(cov.stories_count, 1);
    assert_eq!(cov.tests_count, 1);
    assert!(cov.is_fully_covered);
}

#[tokio::test]
async fn test_create_and_get_impact_analysis() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-080",
        "Test",
        "Test",
        RequirementType::Functional,
    );
    db.create_requirement(&req).await.unwrap();

    let analysis = ImpactAnalysis {
        requirement_id: "REQ-080".to_string(),
        affected_stories: vec!["STORY-020".to_string(), "STORY-021".to_string()],
        affected_code_files: vec!["auth.rs".to_string()],
        affected_tests: vec!["auth_test.rs".to_string()],
        estimated_effort: EffortEstimate::Medium,
        risk_level: RiskLevel::High,
        recommendations: vec![
            "Update integration tests".to_string(),
            "Review security implications".to_string(),
        ],
        generated_at: Utc::now(),
    };

    db.create_impact_analysis(&analysis).await.unwrap();

    let retrieved = db.get_impact_analysis("REQ-080").await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.affected_stories.len(), 2);
    assert_eq!(retrieved.risk_level, RiskLevel::High);
    assert_eq!(retrieved.recommendations.len(), 2);
}

#[tokio::test]
async fn test_delete_requirement() {
    let db = Database::in_memory().await.unwrap();

    let req = Requirement::new(
        "REQ-090",
        "Test",
        "Test",
        RequirementType::Functional,
    );
    db.create_requirement(&req).await.unwrap();

    let deleted = db.delete_requirement("REQ-090").await.unwrap();
    assert!(deleted);

    let retrieved = db.get_requirement("REQ-090").await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_requirement_with_relationships() {
    let db = Database::in_memory().await.unwrap();

    let mut req = Requirement::new(
        "REQ-100",
        "User Profile",
        "Users must have editable profiles",
        RequirementType::Functional,
    );

    req.actors = vec!["Registered User".to_string(), "Admin".to_string()];
    req.acceptance_criteria = vec![
        "User can edit their profile".to_string(),
        "Changes are saved immediately".to_string(),
    ];
    req.dependencies = vec!["REQ-001".to_string()];
    req.related_requirements = vec!["REQ-101".to_string()];
    req.tags = vec!["user".to_string(), "profile".to_string()];

    db.create_requirement(&req).await.unwrap();

    let retrieved = db.get_requirement("REQ-100").await.unwrap().unwrap();
    assert_eq!(retrieved.actors.len(), 2);
    assert_eq!(retrieved.acceptance_criteria.len(), 2);
    assert_eq!(retrieved.dependencies.len(), 1);
    assert_eq!(retrieved.related_requirements.len(), 1);
    assert_eq!(retrieved.tags.len(), 2);
}
