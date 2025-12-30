//! Monitoring REST API endpoints
//!
//! This module provides REST API endpoints for the monitoring and alerting system:
//! - GET /api/metrics - Current metrics snapshot
//! - GET /api/metrics/history - Historical metrics
//! - GET /api/alerts - List alerts
//! - POST /api/alerts/:id/acknowledge - Acknowledge alert
//! - GET /api/health - System health status
//! - POST /api/alerts/rules - Create alert rule
//! - GET /api/audit - Query audit log
//! - GET /api/performance - Agent performance stats
//! - GET /api/costs - Cost reports

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use orchestrate_core::{
    ActorType, AgentPerformance, Alert, AlertRule, AlertSeverity, AlertStatus, AuditAction,
    AuditEntry, AuditQuery, AuditStats, BudgetPeriod, ComponentHealth, HealthStatus, MetricValue,
    MetricsSummary, SystemHealth,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::api::{ApiError, AppState};

/// Query parameters for metrics history endpoint
#[derive(Debug, Deserialize)]
pub struct MetricsHistoryQuery {
    /// Start time for the query (ISO 8601)
    #[serde(default = "default_start_time")]
    pub start: DateTime<Utc>,
    /// End time for the query (ISO 8601)
    #[serde(default = "Utc::now")]
    pub end: DateTime<Utc>,
    /// Metric name filter (optional)
    pub metric: Option<String>,
    /// Time bucket interval in seconds (default: 60)
    #[serde(default = "default_interval")]
    pub interval: i64,
}

fn default_start_time() -> DateTime<Utc> {
    Utc::now() - Duration::hours(24)
}

fn default_interval() -> i64 {
    60
}

/// Query parameters for alerts list endpoint
#[derive(Debug, Deserialize)]
pub struct AlertsQuery {
    /// Filter by status (active, acknowledged, resolved)
    pub status: Option<String>,
    /// Filter by severity (info, warning, critical)
    pub severity: Option<String>,
    /// Pagination limit (default: 100)
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Pagination offset (default: 0)
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Request body for acknowledging an alert
#[derive(Debug, Deserialize)]
pub struct AcknowledgeAlertRequest {
    pub acknowledged_by: String,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Request body for creating an alert rule
#[derive(Debug, Deserialize)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub condition: String,
    pub severity: String,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default = "default_evaluation_interval")]
    pub evaluation_interval_seconds: i64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_evaluation_interval() -> i64 {
    60
}

fn default_enabled() -> bool {
    true
}

/// Query parameters for audit log endpoint
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    /// Filter by actor
    pub actor: Option<String>,
    /// Filter by actor type
    pub actor_type: Option<String>,
    /// Filter by action
    pub action: Option<String>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by resource ID
    pub resource_id: Option<String>,
    /// Filter by success/failure
    pub success: Option<bool>,
    /// Start time (ISO 8601)
    pub start: Option<DateTime<Utc>>,
    /// End time (ISO 8601)
    pub end: Option<DateTime<Utc>>,
    /// Pagination limit (default: 100)
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Pagination offset (default: 0)
    #[serde(default)]
    pub offset: i64,
}

/// Query parameters for performance stats endpoint
#[derive(Debug, Deserialize)]
pub struct PerformanceQuery {
    /// Filter by agent type
    pub agent_type: Option<String>,
    /// Start time (ISO 8601)
    #[serde(default = "default_perf_start_time")]
    pub start: DateTime<Utc>,
    /// End time (ISO 8601)
    #[serde(default = "Utc::now")]
    pub end: DateTime<Utc>,
}

fn default_perf_start_time() -> DateTime<Utc> {
    Utc::now() - Duration::days(7)
}

/// Query parameters for cost reports endpoint
#[derive(Debug, Deserialize)]
pub struct CostQuery {
    /// Filter by period (daily, weekly, monthly)
    #[serde(default = "default_cost_period")]
    pub period: String,
    /// Start time (ISO 8601)
    pub start: Option<DateTime<Utc>>,
    /// End time (ISO 8601)
    pub end: Option<DateTime<Utc>>,
    /// Filter by epic ID
    pub epic_id: Option<String>,
    /// Filter by agent type
    pub agent_type: Option<String>,
}

fn default_cost_period() -> String {
    "monthly".to_string()
}

/// Response for metrics snapshot endpoint
#[derive(Debug, Serialize)]
pub struct MetricsSnapshotResponse {
    pub timestamp: DateTime<Utc>,
    pub metrics: Vec<MetricValue>,
    pub summary: MetricsSummary,
}

/// Response for metrics history endpoint
#[derive(Debug, Serialize)]
pub struct MetricsHistoryResponse {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub interval_seconds: i64,
    pub metrics: HashMap<String, Vec<HistoricalMetricPoint>>,
}

#[derive(Debug, Serialize)]
pub struct HistoricalMetricPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

/// Response for alerts list endpoint
#[derive(Debug, Serialize)]
pub struct AlertsListResponse {
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
    pub alerts: Vec<Alert>,
}

/// Response for alert rule creation
#[derive(Debug, Serialize)]
pub struct CreateAlertRuleResponse {
    pub id: i64,
    pub rule: AlertRule,
}

/// Response for audit log endpoint
#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
    pub entries: Vec<AuditEntry>,
    pub stats: Option<AuditStats>,
}

/// Response for performance stats endpoint
#[derive(Debug, Serialize)]
pub struct PerformanceResponse {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub stats: Vec<AgentPerformance>,
}

/// Response for cost reports endpoint
#[derive(Debug, Serialize)]
pub struct CostReportResponse {
    pub period: String,
    pub report: orchestrate_core::monitoring::CostReport,
}

/// GET /api/metrics - Current metrics snapshot
async fn get_metrics_snapshot(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<MetricsSnapshotResponse>, ApiError> {
    // TODO: Integrate MetricsCollector into AppState to enable full metrics gathering
    // For now, return empty metrics
    let metrics = Vec::new();

    // Get metrics summary from database (using default for now)
    let summary = MetricsSummary::default();

    Ok(Json(MetricsSnapshotResponse {
        timestamp: Utc::now(),
        metrics,
        summary,
    }))
}

/// GET /api/metrics/history - Historical metrics
async fn get_metrics_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MetricsHistoryQuery>,
) -> Result<Json<MetricsHistoryResponse>, ApiError> {
    // Validate time range
    if query.start >= query.end {
        return Err(ApiError::validation("Start time must be before end time"));
    }

    // Get historical metrics from database (returning empty for now)
    let metrics: HashMap<String, Vec<HistoricalMetricPoint>> = HashMap::new();

    Ok(Json(MetricsHistoryResponse {
        start: query.start,
        end: query.end,
        interval_seconds: query.interval,
        metrics,
    }))
}

/// GET /api/alerts - List alerts
async fn list_alerts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AlertsQuery>,
) -> Result<Json<AlertsListResponse>, ApiError> {
    // Parse status filter
    let status_filter = if let Some(status_str) = &query.status {
        Some(parse_alert_status(status_str)?)
    } else {
        None
    };

    // Parse severity filter (for validation)
    if let Some(severity_str) = &query.severity {
        let _ = parse_alert_severity(severity_str)?;
    }

    // Get alerts from database (using existing list_alerts_by_status)
    let status_str = status_filter.map(|s| format!("{:?}", s).to_lowercase());
    let alerts = state
        .db
        .list_alerts_by_status(
            status_str.as_deref(),
            query.severity.as_deref(),
            query.limit,
            query.offset,
        )
        .await
        .map_err(|e| ApiError::internal(format!("Failed to list alerts: {}", e)))?;

    let total = alerts.len() as i64;
    let offset = query.offset as usize;
    let limit = query.limit as usize;

    // Apply pagination
    let paginated_alerts: Vec<Alert> = alerts
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    Ok(Json(AlertsListResponse {
        total,
        offset: query.offset,
        limit: query.limit,
        alerts: paginated_alerts,
    }))
}

/// POST /api/alerts/:id/acknowledge - Acknowledge alert
async fn acknowledge_alert(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<AcknowledgeAlertRequest>,
) -> Result<Json<Alert>, ApiError> {
    // Get alert
    let mut alert = state
        .db
        .get_alert(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get alert: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Alert"))?;

    // Manually update alert fields for acknowledgment
    alert.status = AlertStatus::Acknowledged;
    alert.acknowledged_at = Some(Utc::now());
    alert.acknowledged_by = Some(req.acknowledged_by.clone());

    // Update in database
    state
        .db
        .update_alert(&alert)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update alert: {}", e)))?;

    // Create audit log entry
    let mut audit_entry = AuditEntry::new(
        &req.acknowledged_by,
        AuditAction::AlertAcknowledged,
        "alert",
        id.to_string(),
    );
    if let Some(notes) = &req.notes {
        audit_entry = audit_entry.with_detail("notes", serde_json::json!(notes));
    }
    let _ = state.db.insert_audit_entry(&audit_entry).await;

    Ok(Json(alert))
}

/// GET /api/health - System health status
async fn get_system_health(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SystemHealth>, ApiError> {
    let mut health = SystemHealth::new();

    // Check database health (simple ping test)
    match state.db.get_agent_counts_by_state_and_type().await {
        Ok(_) => {
            let component = ComponentHealth::healthy("database");
            health.add_component(component);
        }
        Err(e) => {
            health.add_component(ComponentHealth::unhealthy(
                "database",
                format!("Database health check failed: {}", e),
            ));
        }
    }

    // Get active alerts count (firing alerts)
    let active_alerts = state
        .db
        .list_alerts_by_status(Some("firing"), None, 100, 0)
        .await
        .unwrap_or_default();
    health.active_alerts = active_alerts.len() as u32;

    // Use default metrics summary for now
    health.metrics_summary = MetricsSummary::default();

    Ok(Json(health))
}

/// POST /api/alerts/rules - Create alert rule
async fn create_alert_rule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAlertRuleRequest>,
) -> Result<Json<CreateAlertRuleResponse>, ApiError> {
    // Parse severity
    let severity = parse_alert_severity(&req.severity)?;

    // Create alert rule using builder pattern
    let mut rule = AlertRule::new(req.name.clone(), req.condition.clone(), severity);
    for channel in &req.channels {
        rule = rule.with_channel(channel);
    }
    rule.enabled = req.enabled;

    // Insert into database (using create_alert_rule which returns the rule ID)
    let rule_id = state
        .db
        .create_alert_rule(&rule)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create alert rule: {}", e)))?;

    let id = rule_id;

    // Create audit log entry
    let audit_entry = AuditEntry::new(
        "system",
        AuditAction::Custom("alert.rule.created".to_string()),
        "alert_rule",
        id.to_string(),
    )
    .with_detail("name", serde_json::json!(req.name))
    .with_detail("severity", serde_json::json!(req.severity));
    let _ = state.db.insert_audit_entry(&audit_entry).await;

    Ok(Json(CreateAlertRuleResponse {
        id,
        rule,
    }))
}

/// GET /api/audit - Query audit log
async fn query_audit_log(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuditLogQuery>,
) -> Result<Json<AuditLogResponse>, ApiError> {
    // Build audit query
    let mut query = AuditQuery::new()
        .with_pagination(params.limit, params.offset);

    // Check for filters before consuming params
    let has_filters = params.actor.is_some()
        || params.actor_type.is_some()
        || params.action.is_some()
        || params.resource_type.is_some();

    if let Some(actor) = params.actor {
        query = query.with_actor(actor);
    }

    if let Some(actor_type_str) = params.actor_type {
        let actor_type = parse_actor_type(&actor_type_str)?;
        query = query.with_actor_type(actor_type);
    }

    if let Some(action_str) = params.action {
        let action = parse_audit_action(&action_str)?;
        query = query.with_action(action);
    }

    if let Some(resource_type) = params.resource_type {
        let resource_id = params.resource_id.unwrap_or_else(|| String::from(""));
        query = query.with_resource(resource_type, resource_id);
    }

    if let Some(success) = params.success {
        query = query.with_success(success);
    }

    if let (Some(start), Some(end)) = (params.start, params.end) {
        query = query.with_timerange(start, end);
    }

    // Get total count
    let total = state
        .db
        .count_audit_entries(&query)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to count audit entries: {}", e)))?;

    // Get entries
    let entries = state
        .db
        .query_audit_log(&query)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to query audit log: {}", e)))?;

    // Get stats if no filters (for performance)
    let stats = if !has_filters {
        state.db.get_audit_stats().await.ok()
    } else {
        None
    };

    Ok(Json(AuditLogResponse {
        total,
        offset: query.offset.unwrap_or(0),
        limit: query.limit.unwrap_or(100),
        entries,
        stats,
    }))
}

/// GET /api/performance - Agent performance stats
async fn get_performance_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PerformanceQuery>,
) -> Result<Json<PerformanceResponse>, ApiError> {
    // Validate time range
    if query.start >= query.end {
        return Err(ApiError::validation("Start time must be before end time"));
    }

    // Get performance stats from database (returning empty for now)
    // TODO: Implement get_agent_performance_stats in database
    let stats: Vec<AgentPerformance> = vec![];

    Ok(Json(PerformanceResponse {
        period_start: query.start,
        period_end: query.end,
        stats,
    }))
}

/// GET /api/costs - Cost reports
async fn get_cost_reports(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CostQuery>,
) -> Result<Json<CostReportResponse>, ApiError> {
    // Parse period
    let period = parse_budget_period(&query.period)?;

    // Determine time range based on period
    let (start, end) = if let (Some(start), Some(end)) = (query.start, query.end) {
        (start, end)
    } else {
        calculate_period_range(period)
    };

    // Get cost report from database (using empty report for now)
    // TODO: Implement get_cost_report in database
    let report = orchestrate_core::monitoring::CostReport::new(start, end);

    Ok(Json(CostReportResponse {
        period: query.period,
        report,
    }))
}

/// Helper function to parse Prometheus metrics text into structured format
fn parse_prometheus_metrics(text: &str) -> Vec<MetricValue> {
    let mut metrics = Vec::new();

    for line in text.lines() {
        // Skip comments and empty lines
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        // Parse metric line: metric_name{labels} value
        if let Some((metric_part, value_str)) = line.rsplit_once(' ') {
            if let Ok(value) = value_str.parse::<f64>() {
                // Parse metric name and labels
                let (name, labels) = parse_metric_name_and_labels(metric_part);

                let mut metric = MetricValue::new(name, value);
                for (key, val) in labels {
                    metric = metric.with_label(key, val);
                }
                metrics.push(metric);
            }
        }
    }

    metrics
}

/// Helper function to parse metric name and labels from Prometheus format
fn parse_metric_name_and_labels(metric_part: &str) -> (String, HashMap<String, String>) {
    if let Some(brace_pos) = metric_part.find('{') {
        let name = metric_part[..brace_pos].to_string();
        let labels_str = &metric_part[brace_pos + 1..metric_part.len() - 1];

        let mut labels = HashMap::new();
        for label_pair in labels_str.split(',') {
            if let Some((key, value)) = label_pair.split_once('=') {
                // Remove quotes from value
                let value = value.trim_matches('"');
                labels.insert(key.to_string(), value.to_string());
            }
        }

        (name, labels)
    } else {
        (metric_part.to_string(), HashMap::new())
    }
}

/// Helper function to parse alert status from string
fn parse_alert_status(status: &str) -> Result<AlertStatus, ApiError> {
    match status.to_lowercase().as_str() {
        "pending" => Ok(AlertStatus::Pending),
        "active" | "firing" => Ok(AlertStatus::Firing),
        "acknowledged" => Ok(AlertStatus::Acknowledged),
        "resolved" => Ok(AlertStatus::Resolved),
        "silenced" => Ok(AlertStatus::Silenced),
        _ => Err(ApiError::validation(format!(
            "Invalid alert status: {}",
            status
        ))),
    }
}

/// Helper function to parse alert severity from string
fn parse_alert_severity(severity: &str) -> Result<AlertSeverity, ApiError> {
    use std::str::FromStr;
    AlertSeverity::from_str(severity)
        .map_err(|e| ApiError::validation(format!("Invalid alert severity: {}", e)))
}

/// Helper function to parse actor type from string
fn parse_actor_type(actor_type: &str) -> Result<ActorType, ApiError> {
    match actor_type.to_lowercase().as_str() {
        "user" => Ok(ActorType::User),
        "system" => Ok(ActorType::System),
        "agent" => Ok(ActorType::Agent),
        "apikey" | "api_key" => Ok(ActorType::ApiKey),
        "webhook" => Ok(ActorType::Webhook),
        _ => Err(ApiError::validation(format!(
            "Invalid actor type: {}",
            actor_type
        ))),
    }
}

/// Helper function to parse audit action from string
fn parse_audit_action(action: &str) -> Result<AuditAction, ApiError> {
    match action.to_lowercase().as_str() {
        "agent.spawned" | "agent_spawned" => Ok(AuditAction::AgentSpawned),
        "agent.terminated" | "agent_terminated" => Ok(AuditAction::AgentTerminated),
        "config.changed" | "configuration_changed" => Ok(AuditAction::ConfigurationChanged),
        "approval.granted" | "approval_granted" => Ok(AuditAction::ApprovalGranted),
        "approval.denied" | "approval_denied" => Ok(AuditAction::ApprovalDenied),
        "deployment.triggered" | "deployment_triggered" => Ok(AuditAction::DeploymentTriggered),
        "deployment.rolled_back" | "deployment_rolled_back" => {
            Ok(AuditAction::DeploymentRolledBack)
        }
        "alert.acknowledged" | "alert_acknowledged" => Ok(AuditAction::AlertAcknowledged),
        "alert.silenced" | "alert_silenced" => Ok(AuditAction::AlertSilenced),
        "user.login" | "user_login" => Ok(AuditAction::UserLogin),
        "user.logout" | "user_logout" => Ok(AuditAction::UserLogout),
        "apikey.created" | "api_key_created" => Ok(AuditAction::ApiKeyCreated),
        "apikey.revoked" | "api_key_revoked" => Ok(AuditAction::ApiKeyRevoked),
        custom => Ok(AuditAction::Custom(custom.to_string())),
    }
}

/// Helper function to parse budget period from string
fn parse_budget_period(period: &str) -> Result<BudgetPeriod, ApiError> {
    use std::str::FromStr;
    BudgetPeriod::from_str(period)
        .map_err(|e| ApiError::validation(format!("Invalid budget period: {}", e)))
}

/// Helper function to calculate period range
fn calculate_period_range(period: BudgetPeriod) -> (DateTime<Utc>, DateTime<Utc>) {
    let end = Utc::now();
    let start = match period {
        BudgetPeriod::Daily => end - Duration::days(1),
        BudgetPeriod::Weekly => end - Duration::weeks(1),
        BudgetPeriod::Monthly => end - Duration::days(30),
    };
    (start, end)
}

/// Create the monitoring router
pub fn create_monitoring_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/metrics", get(get_metrics_snapshot))
        .route("/api/metrics/history", get(get_metrics_history))
        .route("/api/alerts", get(list_alerts))
        .route("/api/alerts/:id/acknowledge", post(acknowledge_alert))
        .route("/api/alerts/rules", post(create_alert_rule))
        .route("/api/health", get(get_system_health))
        .route("/api/audit", get(query_audit_log))
        .route("/api/performance", get(get_performance_stats))
        .route("/api/costs", get(get_cost_reports))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricsCollector;
    use orchestrate_core::{Agent, AgentState, AgentType, Database};
    use secrecy::SecretString;

    async fn setup_test_state() -> Arc<AppState> {
        let db = Database::in_memory().await.unwrap();
        let metrics = Arc::new(MetricsCollector::new().unwrap());

        Arc::new(AppState {
            db,
            api_key: Some(SecretString::new("test-key".to_string())),
            metrics,
        })
    }

    #[tokio::test]
    async fn test_get_metrics_snapshot() {
        let state = setup_test_state().await;

        // Add some test agents to generate metrics
        let agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        state.db.insert_agent(&agent).await.unwrap();

        let result = get_metrics_snapshot(State(state.clone())).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(!response.metrics.is_empty() || response.summary.active_agents >= 0);
    }

    #[tokio::test]
    async fn test_get_metrics_history() {
        let state = setup_test_state().await;

        let query = MetricsHistoryQuery {
            start: Utc::now() - Duration::hours(1),
            end: Utc::now(),
            metric: None,
            interval: 60,
        };

        let result = get_metrics_history(State(state.clone()), Query(query)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_metrics_history_invalid_time_range() {
        let state = setup_test_state().await;

        let query = MetricsHistoryQuery {
            start: Utc::now(),
            end: Utc::now() - Duration::hours(1),
            metric: None,
            interval: 60,
        };

        let result = get_metrics_history(State(state.clone()), Query(query)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_alerts() {
        let state = setup_test_state().await;

        // Create a test alert rule
        let rule =
            AlertRule::new("test-rule", "test > 100", AlertSeverity::Warning, vec![]);
        let created_rule = state.db.create_alert_rule(rule).await.unwrap();
        let rule_id = created_rule.id.unwrap();

        // Create a test alert
        let alert = Alert::new(rule_id, "test-fingerprint");
        state.db.create_alert(alert).await.unwrap();

        let query = AlertsQuery {
            status: None,
            severity: None,
            limit: 100,
            offset: 0,
        };

        let result = list_alerts(State(state.clone()), Query(query)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.total >= 1);
        assert!(!response.alerts.is_empty());
    }

    #[tokio::test]
    async fn test_list_alerts_with_filters() {
        let state = setup_test_state().await;

        let query = AlertsQuery {
            status: Some("active".to_string()),
            severity: Some("warning".to_string()),
            limit: 50,
            offset: 0,
        };

        let result = list_alerts(State(state.clone()), Query(query)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_acknowledge_alert() {
        let state = setup_test_state().await;

        // Create a test alert
        let rule =
            AlertRule::new("test-rule", "test > 100", AlertSeverity::Critical, vec![]);
        let created_rule = state.db.create_alert_rule(rule).await.unwrap();
        let rule_id = created_rule.id.unwrap();

        let alert = Alert::new(rule_id, "test-fingerprint");
        let created_alert = state.db.create_alert(alert).await.unwrap();
        let alert_id = created_alert.id.unwrap();

        let req = AcknowledgeAlertRequest {
            acknowledged_by: "test@example.com".to_string(),
            notes: Some("Acknowledged in test".to_string()),
        };

        let result = acknowledge_alert(State(state.clone()), Path(alert_id), Json(req)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.status, AlertStatus::Acknowledged);
        assert!(response.acknowledged_at.is_some());
    }

    #[tokio::test]
    async fn test_acknowledge_nonexistent_alert() {
        let state = setup_test_state().await;

        let req = AcknowledgeAlertRequest {
            acknowledged_by: "test@example.com".to_string(),
            notes: None,
        };

        let result = acknowledge_alert(State(state.clone()), Path(99999), Json(req)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_system_health() {
        let state = setup_test_state().await;

        let result = get_system_health(State(state.clone())).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(!response.components.is_empty());
        assert_eq!(
            response.components[0].name, "database",
            "Should have database component"
        );
    }

    #[tokio::test]
    async fn test_create_alert_rule() {
        let state = setup_test_state().await;

        let req = CreateAlertRuleRequest {
            name: "High Queue Depth".to_string(),
            condition: "orchestrate_queue_depth{queue='webhook_events'} > 100".to_string(),
            severity: "warning".to_string(),
            channels: vec!["slack".to_string()],
            evaluation_interval_seconds: 30,
            enabled: true,
        };

        let result = create_alert_rule(State(state.clone()), Json(req)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.id > 0);
        assert_eq!(response.rule.name, "High Queue Depth");
        assert_eq!(response.rule.severity, AlertSeverity::Warning);
    }

    #[tokio::test]
    async fn test_create_alert_rule_invalid_severity() {
        let state = setup_test_state().await;

        let req = CreateAlertRuleRequest {
            name: "Test Rule".to_string(),
            condition: "test > 100".to_string(),
            severity: "invalid".to_string(),
            channels: vec![],
            evaluation_interval_seconds: 60,
            enabled: true,
        };

        let result = create_alert_rule(State(state.clone()), Json(req)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_audit_log() {
        let state = setup_test_state().await;

        // Insert test audit entries
        let entry1 = AuditEntry::new(
            "test-user",
            AuditAction::AgentSpawned,
            "agent",
            "agent-1",
        );
        state.db.insert_audit_entry(&entry1).await.unwrap();

        let entry2 = AuditEntry::new(
            "system",
            AuditAction::ConfigurationChanged,
            "config",
            "cfg-1",
        );
        state.db.insert_audit_entry(&entry2).await.unwrap();

        let params = AuditLogQuery {
            actor: None,
            actor_type: None,
            action: None,
            resource_type: None,
            resource_id: None,
            success: None,
            start: None,
            end: None,
            limit: 100,
            offset: 0,
        };

        let result = query_audit_log(State(state.clone()), Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.total >= 2);
        assert!(response.entries.len() >= 2);
    }

    #[tokio::test]
    async fn test_query_audit_log_with_filters() {
        let state = setup_test_state().await;

        // Insert test entry
        let entry = AuditEntry::new(
            "test-user",
            AuditAction::ApprovalGranted,
            "approval",
            "ap-1",
        );
        state.db.insert_audit_entry(&entry).await.unwrap();

        let params = AuditLogQuery {
            actor: Some("test-user".to_string()),
            actor_type: Some("user".to_string()),
            action: Some("approval.granted".to_string()),
            resource_type: None,
            resource_id: None,
            success: Some(true),
            start: None,
            end: None,
            limit: 100,
            offset: 0,
        };

        let result = query_audit_log(State(state.clone()), Query(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.total >= 1);
    }

    #[tokio::test]
    async fn test_get_performance_stats() {
        let state = setup_test_state().await;

        // Create test agents with completed state
        let mut agent1 = Agent::new(AgentType::StoryDeveloper, "Task 1");
        agent1.state = AgentState::Completed;
        state.db.insert_agent(&agent1).await.unwrap();

        let query = PerformanceQuery {
            agent_type: None,
            start: Utc::now() - Duration::days(7),
            end: Utc::now(),
        };

        let result = get_performance_stats(State(state.clone()), Query(query)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_performance_stats_invalid_time_range() {
        let state = setup_test_state().await;

        let query = PerformanceQuery {
            agent_type: None,
            start: Utc::now(),
            end: Utc::now() - Duration::days(7),
        };

        let result = get_performance_stats(State(state.clone()), Query(query)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_cost_reports() {
        let state = setup_test_state().await;

        let query = CostQuery {
            period: "monthly".to_string(),
            start: None,
            end: None,
            epic_id: None,
            agent_type: None,
        };

        let result = get_cost_reports(State(state.clone()), Query(query)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert_eq!(response.period, "monthly");
    }

    #[tokio::test]
    async fn test_parse_prometheus_metrics() {
        let text = r#"
# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",status="200"} 150
http_requests_total{method="POST",status="201"} 50
orchestrate_queue_depth{queue="webhook_events"} 25
"#;

        let metrics = parse_prometheus_metrics(text);
        assert_eq!(metrics.len(), 3);

        // Check first metric
        assert_eq!(metrics[0].name, "http_requests_total");
        assert_eq!(metrics[0].value, 150.0);
        assert_eq!(metrics[0].labels.get("method"), Some(&"GET".to_string()));
    }

    #[tokio::test]
    async fn test_parse_alert_status() {
        assert_eq!(
            parse_alert_status("active").unwrap(),
            AlertStatus::Active
        );
        assert_eq!(
            parse_alert_status("acknowledged").unwrap(),
            AlertStatus::Acknowledged
        );
        assert_eq!(
            parse_alert_status("resolved").unwrap(),
            AlertStatus::Resolved
        );
        assert!(parse_alert_status("invalid").is_err());
    }

    #[tokio::test]
    async fn test_parse_alert_severity() {
        assert_eq!(
            parse_alert_severity("info").unwrap(),
            AlertSeverity::Info
        );
        assert_eq!(
            parse_alert_severity("warning").unwrap(),
            AlertSeverity::Warning
        );
        assert_eq!(
            parse_alert_severity("critical").unwrap(),
            AlertSeverity::Critical
        );
        assert!(parse_alert_severity("invalid").is_err());
    }

    #[tokio::test]
    async fn test_parse_actor_type() {
        assert_eq!(parse_actor_type("user").unwrap(), ActorType::User);
        assert_eq!(parse_actor_type("system").unwrap(), ActorType::System);
        assert_eq!(parse_actor_type("agent").unwrap(), ActorType::Agent);
        assert_eq!(parse_actor_type("apikey").unwrap(), ActorType::ApiKey);
        assert_eq!(parse_actor_type("webhook").unwrap(), ActorType::Webhook);
        assert!(parse_actor_type("invalid").is_err());
    }

    #[tokio::test]
    async fn test_calculate_period_range() {
        let (start, end) = calculate_period_range(BudgetPeriod::Daily);
        let diff = end.signed_duration_since(start);
        assert!(diff.num_hours() >= 23 && diff.num_hours() <= 25);

        let (start, end) = calculate_period_range(BudgetPeriod::Weekly);
        let diff = end.signed_duration_since(start);
        assert!(diff.num_days() >= 6 && diff.num_days() <= 8);

        let (start, end) = calculate_period_range(BudgetPeriod::Monthly);
        let diff = end.signed_duration_since(start);
        assert!(diff.num_days() >= 29 && diff.num_days() <= 31);
    }
}
