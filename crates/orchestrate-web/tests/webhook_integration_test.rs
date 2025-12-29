//! Integration tests for webhook endpoint

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use http_body_util::BodyExt;
use orchestrate_core::Database;
use orchestrate_web::{api::AppState, create_router_with_webhook};
use std::sync::Arc;
use tower::ServiceExt;

async fn body_to_string(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn compute_github_signature(secret: &str, payload: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    format!("sha256={}", hex::encode(result.into_bytes()))
}

#[tokio::test]
async fn test_webhook_endpoint_integration() {
    // Setup
    let db = Database::in_memory().await.unwrap();
    let app_state = Arc::new(AppState::new(db, None));
    let secret = "test-webhook-secret";
    let router = create_router_with_webhook(app_state, Some(secret.to_string()));

    // Prepare a realistic GitHub webhook payload
    let payload = r#"{
        "action": "opened",
        "number": 123,
        "pull_request": {
            "id": 1,
            "number": 123,
            "title": "Test PR",
            "state": "open",
            "user": {
                "login": "testuser"
            },
            "head": {
                "ref": "feature-branch",
                "sha": "abc123"
            },
            "base": {
                "ref": "main",
                "sha": "def456"
            }
        },
        "repository": {
            "name": "test-repo",
            "owner": {
                "login": "testowner"
            }
        }
    }"#;

    let signature = compute_github_signature(secret, payload);

    // Send webhook request
    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/webhooks/github")
                .header("content-type", "application/json")
                .header("x-github-event", "pull_request")
                .header("x-github-delivery", "12345-67890-abcdef")
                .header("x-hub-signature-256", signature)
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    // Verify response
    assert_eq!(response.status(), StatusCode::OK);

    let body = body_to_string(response.into_body()).await;
    let resp: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["message"], "Webhook received");
}

#[tokio::test]
async fn test_webhook_endpoint_without_secret() {
    // Setup without secret (signature verification disabled)
    let db = Database::in_memory().await.unwrap();
    let app_state = Arc::new(AppState::new(db, None));
    let router = create_router_with_webhook(app_state, None);

    let payload = r#"{"action":"opened","number":1}"#;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/webhooks/github")
                .header("content-type", "application/json")
                .header("x-github-event", "issues")
                .header("x-github-delivery", "test-123")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_webhook_various_event_types() {
    let db = Database::in_memory().await.unwrap();
    let app_state = Arc::new(AppState::new(db, None));
    let router = create_router_with_webhook(app_state, None);

    let event_types = vec![
        "pull_request",
        "pull_request_review",
        "check_run",
        "check_suite",
        "push",
        "issues",
        "issue_comment",
    ];

    for event_type in event_types {
        let payload = format!(r#"{{"action":"test","event":"{}"}}"#, event_type);

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/webhooks/github")
                    .header("content-type", "application/json")
                    .header("x-github-event", event_type)
                    .header("x-github-delivery", format!("test-{}", event_type))
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Failed for event type: {}",
            event_type
        );
    }
}
