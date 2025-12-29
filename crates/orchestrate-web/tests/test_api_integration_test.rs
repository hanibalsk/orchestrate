//! Integration tests for test REST API endpoints

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use orchestrate_core::{
    ChangeAnalysisResult, ChangedFunction, ChangeType, CoverageReport, Database, FileCoverage,
    Language, ModuleCoverage, Priority, TestGenerationResult, TestSuggestion, TestType,
};
use orchestrate_web::{api::AppState, create_api_router};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

async fn body_to_string(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn setup_app() -> (axum::Router, Arc<AppState>) {
    let db = Database::in_memory().await.unwrap();
    let state = Arc::new(AppState::new(db, None));
    let router = create_api_router(state.clone());
    (router, state)
}

// ==================== POST /api/tests/generate Tests ====================

#[tokio::test]
async fn test_generate_unit_tests_success() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "test_type": "unit",
        "target": "src/example.rs",
        "language": "rust"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/generate")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let result: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(result.get("test_cases").is_some());
    assert!(result.get("generated_count").is_some());
}

#[tokio::test]
async fn test_generate_integration_tests_success() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "test_type": "integration",
        "target": "src/module",
        "language": "rust"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/generate")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_generate_e2e_tests_from_story() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "test_type": "e2e",
        "story_id": "STORY-123",
        "platform": "playwright"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/generate")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_generate_property_tests_success() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "test_type": "property",
        "target": "src/parser.rs",
        "language": "rust"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/generate")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_generate_tests_missing_test_type() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "target": "src/example.rs"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/generate")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Serde returns 422 UNPROCESSABLE_ENTITY for missing required fields
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_generate_tests_invalid_language() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "test_type": "unit",
        "target": "src/example.java",
        "language": "java"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/generate")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Serde returns 422 UNPROCESSABLE_ENTITY for invalid enum values
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ==================== GET /api/tests/coverage Tests ====================

#[tokio::test]
async fn test_get_coverage_report_success() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/coverage")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let report: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(report.get("timestamp").is_some());
    assert!(report.get("modules").is_some());
    assert!(report.get("overall_percent").is_some());
}

#[tokio::test]
async fn test_get_coverage_report_with_module_filter() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/coverage?module=orchestrate-core")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_coverage_report_diff_mode() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/coverage?diff=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ==================== GET /api/tests/coverage/history Tests ====================

#[tokio::test]
async fn test_get_coverage_history_success() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/coverage/history")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let history: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(history.is_array());
}

#[tokio::test]
async fn test_get_coverage_history_with_limit() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/coverage/history?limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_coverage_history_with_module_filter() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/coverage/history?module=orchestrate-web&limit=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ==================== POST /api/tests/run Tests ====================

#[tokio::test]
async fn test_trigger_test_run_all() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "scope": "all"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/run")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let result: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(result.get("run_id").is_some());
    assert!(result.get("status").is_some());
}

#[tokio::test]
async fn test_trigger_test_run_changed_only() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "scope": "changed"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/run")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_trigger_test_run_with_coverage() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "scope": "all",
        "with_coverage": true
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/run")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_trigger_test_run_specific_module() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "scope": "module",
        "target": "orchestrate-core"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/run")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ==================== GET /api/tests/runs/:id Tests ====================

#[tokio::test]
async fn test_get_test_run_results_success() {
    let (router, state) = setup_app().await;

    // First create a test run
    let payload = json!({
        "scope": "all"
    });

    let create_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/run")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_to_string(create_response.into_body()).await;
    let create_result: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run_id = create_result["run_id"].as_str().unwrap();

    // Now get the test run results
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/tests/runs/{}", run_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let result: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(result["run_id"], run_id);
    assert!(result.get("status").is_some());
    assert!(result.get("started_at").is_some());
}

#[tokio::test]
async fn test_get_test_run_results_not_found() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/runs/nonexistent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_test_run_results_with_details() {
    let (router, state) = setup_app().await;

    // Create a test run
    let payload = json!({
        "scope": "all",
        "with_coverage": true
    });

    let create_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/run")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_to_string(create_response.into_body()).await;
    let create_result: serde_json::Value = serde_json::from_str(&body).unwrap();
    let run_id = create_result["run_id"].as_str().unwrap();

    // Get with details
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/tests/runs/{}?include_details=true", run_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let result: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(result.get("test_results").is_some());
}

// ==================== GET /api/tests/suggestions Tests ====================

#[tokio::test]
async fn test_get_test_suggestions_for_pr() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/suggestions?pr_number=123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let suggestions: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(suggestions.is_array());
}

#[tokio::test]
async fn test_get_test_suggestions_missing_pr_number() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/suggestions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_test_suggestions_with_priority_filter() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/suggestions?pr_number=123&priority=high")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let suggestions: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(suggestions.is_array());
}

#[tokio::test]
async fn test_get_test_suggestions_for_branch() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/tests/suggestions?branch=feature-branch")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ==================== Error Handling Tests ====================

#[tokio::test]
async fn test_generate_tests_invalid_json() {
    let (router, _state) = setup_app().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/generate")
                .header("content-type", "application/json")
                .body(Body::from("{invalid json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_trigger_test_run_invalid_scope() {
    let (router, _state) = setup_app().await;

    let payload = json!({
        "scope": "invalid_scope"
    });

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tests/run")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
