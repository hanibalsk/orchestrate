//! GitHub webhook receiver
//!
//! Handles incoming GitHub webhook events with signature verification.

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use orchestrate_core::{Database, WebhookEvent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Webhook configuration
#[derive(Clone)]
pub struct WebhookConfig {
    /// GitHub webhook secret for HMAC verification
    pub secret: Option<String>,
}

impl WebhookConfig {
    pub fn new(secret: Option<String>) -> Self {
        Self { secret }
    }
}

/// Webhook handler state
#[derive(Clone)]
pub struct WebhookState {
    pub config: WebhookConfig,
    pub database: Database,
}

impl WebhookState {
    pub fn new(config: WebhookConfig, database: Database) -> Self {
        Self { config, database }
    }
}

/// Webhook response
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub status: String,
    pub message: String,
}

/// GitHub webhook handler
///
/// Receives GitHub webhook events, verifies signatures, and processes them asynchronously.
pub async fn github_webhook_handler(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Extract event type
    let event_type = match headers.get("x-github-event") {
        Some(value) => match value.to_str() {
            Ok(v) => v.to_string(),
            Err(_) => {
                warn!("Invalid X-GitHub-Event header");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(WebhookResponse {
                        status: "error".to_string(),
                        message: "Invalid X-GitHub-Event header".to_string(),
                    }),
                );
            }
        },
        None => {
            warn!("Missing X-GitHub-Event header");
            return (
                StatusCode::BAD_REQUEST,
                Json(WebhookResponse {
                    status: "error".to_string(),
                    message: "Missing X-GitHub-Event header".to_string(),
                }),
            );
        }
    };

    // Extract delivery ID
    let delivery_id = headers
        .get("x-github-delivery")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    debug!(
        event_type = %event_type,
        delivery_id = ?delivery_id,
        "Received GitHub webhook"
    );

    // Verify signature if secret is configured
    if let Some(ref secret) = state.config.secret {
        let signature = match headers.get("x-hub-signature-256") {
            Some(value) => match value.to_str() {
                Ok(v) => v,
                Err(_) => {
                    warn!("Invalid X-Hub-Signature-256 header");
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(WebhookResponse {
                            status: "error".to_string(),
                            message: "Invalid X-Hub-Signature-256 header".to_string(),
                        }),
                    );
                }
            },
            None => {
                warn!("Missing X-Hub-Signature-256 header");
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(WebhookResponse {
                        status: "error".to_string(),
                        message: "Missing signature".to_string(),
                    }),
                );
            }
        };

        if !verify_signature(secret, &body, signature) {
            error!("Invalid webhook signature");
            return (
                StatusCode::UNAUTHORIZED,
                Json(WebhookResponse {
                    status: "error".to_string(),
                    message: "Invalid signature".to_string(),
                }),
            );
        }
    }

    // Parse payload (basic validation)
    let payload_str = match std::str::from_utf8(&body) {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "Invalid UTF-8 in payload");
            return (
                StatusCode::BAD_REQUEST,
                Json(WebhookResponse {
                    status: "error".to_string(),
                    message: "Invalid UTF-8 in payload".to_string(),
                }),
            );
        }
    };

    match serde_json::from_str::<serde_json::Value>(payload_str) {
        Ok(payload) => {
            info!(
                event_type = %event_type,
                delivery_id = ?delivery_id,
                "Webhook payload received and verified"
            );
            debug!(payload = ?payload, "Webhook payload");
        }
        Err(e) => {
            warn!(error = %e, "Failed to parse webhook payload");
            return (
                StatusCode::BAD_REQUEST,
                Json(WebhookResponse {
                    status: "error".to_string(),
                    message: format!("Invalid JSON payload: {}", e),
                }),
            );
        }
    }

    // Queue event for async processing
    let delivery_id_str = delivery_id.unwrap_or_else(|| {
        // Generate a delivery ID if not provided (shouldn't happen with GitHub)
        uuid::Uuid::new_v4().to_string()
    });

    let webhook_event = WebhookEvent::new(
        delivery_id_str.clone(),
        event_type.clone(),
        payload_str.to_string(),
    );

    match state.database.insert_webhook_event(&webhook_event).await {
        Ok(id) => {
            info!(
                event_id = id,
                delivery_id = %delivery_id_str,
                event_type = %event_type,
                "Webhook event queued"
            );
        }
        Err(e) => {
            error!(error = %e, "Failed to queue webhook event");
            // Don't return error to GitHub - we've received the webhook
            // Log the error and continue
        }
    }

    // Return 200 OK quickly
    (
        StatusCode::OK,
        Json(WebhookResponse {
            status: "ok".to_string(),
            message: "Webhook received".to_string(),
        }),
    )
}

/// Verify GitHub webhook signature using HMAC-SHA256
///
/// GitHub sends the signature in the format: "sha256=<hex-encoded-hmac>"
fn verify_signature(secret: &str, payload: &[u8], signature: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Parse signature (should be "sha256=<hex>")
    let signature = match signature.strip_prefix("sha256=") {
        Some(sig) => sig,
        None => {
            warn!("Signature doesn't start with 'sha256='");
            return false;
        }
    };

    // Decode hex signature
    let expected_signature = match hex::decode(signature) {
        Ok(sig) => sig,
        Err(e) => {
            warn!(error = %e, "Failed to decode signature hex");
            return false;
        }
    };

    // Compute HMAC
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(e) => {
            error!(error = %e, "Failed to create HMAC");
            return false;
        }
    };
    mac.update(payload);

    // Verify signature
    match mac.verify_slice(&expected_signature) {
        Ok(()) => true,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, Method},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use axum::Router;
    use axum::routing::post;

    /// Helper to read response body as string
    async fn body_to_string(body: Body) -> String {
        let bytes = body.collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    /// Helper to create test router
    async fn create_test_router(secret: Option<String>) -> Router {
        let config = WebhookConfig::new(secret);
        let database = orchestrate_core::Database::in_memory().await.unwrap();
        let state = Arc::new(WebhookState::new(config, database));
        Router::new()
            .route("/webhooks/github", post(github_webhook_handler))
            .with_state(state)
    }

    /// Helper to compute GitHub signature
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
    async fn test_webhook_missing_event_header() {
        let router = create_test_router(None).await;

        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/webhooks/github")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"action":"opened"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_to_string(response.into_body()).await;
        let resp: WebhookResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "error");
        assert!(resp.message.contains("Missing X-GitHub-Event"));
    }

    #[tokio::test]
    async fn test_webhook_malformed_json() {
        let router = create_test_router(None).await;

        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/webhooks/github")
                    .header("content-type", "application/json")
                    .header("x-github-event", "pull_request")
                    .header("x-github-delivery", "12345")
                    .body(Body::from("not json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_to_string(response.into_body()).await;
        let resp: WebhookResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "error");
        assert!(resp.message.contains("Invalid JSON payload"));
    }

    #[tokio::test]
    async fn test_webhook_without_signature_when_no_secret() {
        let router = create_test_router(None).await;

        let payload = r#"{"action":"opened","number":1}"#;
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/webhooks/github")
                    .header("content-type", "application/json")
                    .header("x-github-event", "pull_request")
                    .header("x-github-delivery", "12345-67890")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_to_string(response.into_body()).await;
        let resp: WebhookResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "ok");
        assert_eq!(resp.message, "Webhook received");
    }

    #[tokio::test]
    async fn test_webhook_missing_signature_when_secret_configured() {
        let router = create_test_router(Some("my-secret".to_string())).await;

        let payload = r#"{"action":"opened","number":1}"#;
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/webhooks/github")
                    .header("content-type", "application/json")
                    .header("x-github-event", "pull_request")
                    .header("x-github-delivery", "12345")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = body_to_string(response.into_body()).await;
        let resp: WebhookResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "error");
        assert!(resp.message.contains("Missing signature"));
    }

    #[tokio::test]
    async fn test_webhook_invalid_signature() {
        let router = create_test_router(Some("my-secret".to_string())).await;

        let payload = r#"{"action":"opened","number":1}"#;
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/webhooks/github")
                    .header("content-type", "application/json")
                    .header("x-github-event", "pull_request")
                    .header("x-github-delivery", "12345")
                    .header("x-hub-signature-256", "sha256=invalid")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = body_to_string(response.into_body()).await;
        let resp: WebhookResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "error");
        assert!(resp.message.contains("Invalid signature"));
    }

    #[tokio::test]
    async fn test_webhook_valid_signature() {
        let secret = "my-secret";
        let router = create_test_router(Some(secret.to_string())).await;

        let payload = r#"{"action":"opened","number":1}"#;
        let signature = compute_github_signature(secret, payload);

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

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_to_string(response.into_body()).await;
        let resp: WebhookResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "ok");
        assert_eq!(resp.message, "Webhook received");
    }

    #[tokio::test]
    async fn test_webhook_logs_event_details() {
        // This test verifies that the webhook handler logs the event type and delivery ID
        // We're mainly testing that the handler doesn't panic and returns success
        let router = create_test_router(None).await;

        let payload = r#"{"action":"synchronize","pull_request":{"number":42}}"#;
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/webhooks/github")
                    .header("content-type", "application/json")
                    .header("x-github-event", "pull_request")
                    .header("x-github-delivery", "test-delivery-id")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_verify_signature_valid() {
        let secret = "test-secret";
        let payload = b"test payload";

        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let result = mac.finalize();
        let signature = format!("sha256={}", hex::encode(result.into_bytes()));

        assert!(verify_signature(secret, payload, &signature));
    }

    #[test]
    fn test_verify_signature_invalid() {
        let secret = "test-secret";
        let payload = b"test payload";
        let wrong_signature = "sha256=0000000000000000000000000000000000000000000000000000000000000000";

        assert!(!verify_signature(secret, payload, wrong_signature));
    }

    #[test]
    fn test_verify_signature_missing_prefix() {
        let secret = "test-secret";
        let payload = b"test payload";
        let signature_without_prefix = "0000000000000000000000000000000000000000000000000000000000000000";

        assert!(!verify_signature(secret, payload, signature_without_prefix));
    }

    #[test]
    fn test_verify_signature_invalid_hex() {
        let secret = "test-secret";
        let payload = b"test payload";
        let invalid_hex = "sha256=not-hex-string";

        assert!(!verify_signature(secret, payload, invalid_hex));
    }

    #[tokio::test]
    async fn test_webhook_queues_event() {
        let database = orchestrate_core::Database::in_memory().await.unwrap();
        let config = WebhookConfig::new(None);
        let state = Arc::new(WebhookState::new(config, database.clone()));
        let router = Router::new()
            .route("/webhooks/github", post(github_webhook_handler))
            .with_state(state);

        let payload = r#"{"action":"opened","number":123}"#;
        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/webhooks/github")
                    .header("content-type", "application/json")
                    .header("x-github-event", "pull_request")
                    .header("x-github-delivery", "test-queue-delivery")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify event was queued
        let event = database
            .get_webhook_event_by_delivery_id("test-queue-delivery")
            .await
            .unwrap();
        assert!(event.is_some());
        let event = event.unwrap();
        assert_eq!(event.event_type, "pull_request");
        assert_eq!(event.payload, payload);
        assert_eq!(event.status, orchestrate_core::WebhookEventStatus::Pending);
    }
}
