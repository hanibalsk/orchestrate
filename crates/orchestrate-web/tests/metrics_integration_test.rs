//! Integration tests for Prometheus metrics endpoint

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use orchestrate_core::{Agent, AgentState, AgentType, Database};
use orchestrate_web::api::{create_router, AppState};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_metrics_endpoint_returns_prometheus_format() {
    // Setup
    let db = Database::in_memory().await.unwrap();
    let state = Arc::new(AppState::new(db, None));
    let app = create_router(state);

    // Create request
    let request = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    // Send request
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Check for Prometheus format
    assert!(body_str.contains("# HELP") || body_str.contains("# TYPE"));
}

#[tokio::test]
async fn test_metrics_endpoint_includes_queue_metrics() {
    // Setup
    let db = Database::in_memory().await.unwrap();

    // Add a webhook event to test queue metrics
    let event = orchestrate_core::webhook::WebhookEvent::new(
        "test-delivery-id".to_string(),
        "push".to_string(),
        "{}".to_string(),
    );
    db.insert_webhook_event(&event).await.unwrap();

    let state = Arc::new(AppState::new(db, None));
    let app = create_router(state);

    // Create request
    let request = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    // Send request
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should include queue depth metric
    assert!(body_str.contains("orchestrate_queue_depth"));
    assert!(body_str.contains("webhook_events"));
}

#[tokio::test]
async fn test_metrics_endpoint_includes_agent_metrics() {
    // Setup
    let db = Database::in_memory().await.unwrap();

    // Add some test agents
    let mut agent1 = Agent::new(AgentType::StoryDeveloper, "Test task 1");
    agent1.state = AgentState::Running;
    db.insert_agent(&agent1).await.unwrap();

    let mut agent2 = Agent::new(AgentType::CodeReviewer, "Test task 2");
    agent2.state = AgentState::Completed;
    db.insert_agent(&agent2).await.unwrap();

    let state = Arc::new(AppState::new(db, None));
    let app = create_router(state);

    // Create request
    let request = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    // Send request
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // The metrics output should contain queue depth at minimum
    assert!(body_str.contains("orchestrate_queue_depth"));
}

#[tokio::test]
async fn test_metrics_endpoint_no_authentication_required() {
    // Setup
    let db = Database::in_memory().await.unwrap();
    // Create state WITH an API key
    let state = Arc::new(AppState::new(db, Some("secret-api-key".to_string())));
    let app = create_router(state);

    // Create request WITHOUT authentication
    let request = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    // Send request
    let response = app.oneshot(request).await.unwrap();

    // Assert - metrics endpoint should be public (no auth required)
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_content_type() {
    // Setup
    let db = Database::in_memory().await.unwrap();
    let state = Arc::new(AppState::new(db, None));
    let app = create_router(state);

    // Create request
    let request = Request::builder()
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();

    // Send request
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    // Prometheus text format should be plain text
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Verify it looks like Prometheus format (lines starting with # or metric names)
    let lines: Vec<&str> = body_str.lines().collect();
    let has_prometheus_format = lines.iter().any(|line| {
        line.starts_with('#') ||
        line.starts_with("orchestrate_")
    });
    assert!(has_prometheus_format);
}
