//! Monitoring & Alerting Module
//!
//! This module provides comprehensive monitoring capabilities including:
//! - Prometheus-compatible metrics
//! - Alerting rules engine
//! - Cost analytics
//! - Audit logging
//! - Agent performance tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Metric type for Prometheus-compatible metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// A metric definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub name: String,
    pub metric_type: MetricType,
    pub description: String,
    pub labels: Vec<String>,
    pub unit: Option<String>,
}

impl MetricDefinition {
    pub fn new(name: impl Into<String>, metric_type: MetricType, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metric_type,
            description: description.into(),
            labels: Vec::new(),
            unit: None,
        }
    }

    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }
}

/// A metric value with labels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

impl MetricValue {
    pub fn new(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            labels: HashMap::new(),
            value,
            timestamp: Utc::now(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Format as Prometheus text format
    pub fn to_prometheus(&self) -> String {
        if self.labels.is_empty() {
            format!("{} {}", self.name, self.value)
        } else {
            let labels: Vec<String> = self.labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            format!("{}{{{}}} {}", self.name, labels.join(","), self.value)
        }
    }
}

/// Histogram bucket for latency tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub le: f64,
    pub count: u64,
}

/// Histogram metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramValue {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub buckets: Vec<HistogramBucket>,
    pub count: u64,
    pub sum: f64,
}

impl HistogramValue {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            labels: HashMap::new(),
            buckets: vec![
                HistogramBucket { le: 0.005, count: 0 },
                HistogramBucket { le: 0.01, count: 0 },
                HistogramBucket { le: 0.025, count: 0 },
                HistogramBucket { le: 0.05, count: 0 },
                HistogramBucket { le: 0.1, count: 0 },
                HistogramBucket { le: 0.25, count: 0 },
                HistogramBucket { le: 0.5, count: 0 },
                HistogramBucket { le: 1.0, count: 0 },
                HistogramBucket { le: 2.5, count: 0 },
                HistogramBucket { le: 5.0, count: 0 },
                HistogramBucket { le: 10.0, count: 0 },
            ],
            count: 0,
            sum: 0.0,
        }
    }

    pub fn observe(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        for bucket in &mut self.buckets {
            if value <= bucket.le {
                bucket.count += 1;
            }
        }
    }
}

/// Alert severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl FromStr for AlertSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warning" | "warn" => Ok(Self::Warning),
            "critical" | "crit" => Ok(Self::Critical),
            _ => Err(format!("Unknown severity: {}", s)),
        }
    }
}

/// Alert status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AlertStatus {
    Pending,
    Firing,
    Acknowledged,
    Resolved,
    Silenced,
}

impl std::fmt::Display for AlertStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Firing => write!(f, "firing"),
            Self::Acknowledged => write!(f, "acknowledged"),
            Self::Resolved => write!(f, "resolved"),
            Self::Silenced => write!(f, "silenced"),
        }
    }
}

/// Condition type for alert rules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    Threshold,
    Rate,
    Absence,
    Custom,
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub condition: String,
    pub condition_type: ConditionType,
    pub threshold: Option<f64>,
    pub duration_seconds: Option<u64>,
    pub severity: AlertSeverity,
    pub channels: Vec<String>,
    pub enabled: bool,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AlertRule {
    pub fn new(
        name: impl Into<String>,
        condition: impl Into<String>,
        severity: AlertSeverity,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            condition: condition.into(),
            condition_type: ConditionType::Threshold,
            threshold: None,
            duration_seconds: None,
            severity,
            channels: Vec::new(),
            enabled: true,
            labels: HashMap::new(),
            annotations: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }

    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channels.push(channel.into());
        self
    }

    pub fn with_duration(mut self, seconds: u64) -> Self {
        self.duration_seconds = Some(seconds);
        self
    }
}

/// An active or historical alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub rule_name: String,
    pub status: AlertStatus,
    pub severity: AlertSeverity,
    pub message: String,
    pub current_value: Option<f64>,
    pub threshold: Option<f64>,
    pub labels: HashMap<String, String>,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub silenced_until: Option<DateTime<Utc>>,
}

impl Alert {
    pub fn new(rule: &AlertRule, message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            status: AlertStatus::Firing,
            severity: rule.severity.clone(),
            message: message.into(),
            current_value: None,
            threshold: rule.threshold,
            labels: rule.labels.clone(),
            triggered_at: Utc::now(),
            acknowledged_at: None,
            acknowledged_by: None,
            resolved_at: None,
            silenced_until: None,
        }
    }

    pub fn acknowledge(&mut self, by: impl Into<String>) {
        self.status = AlertStatus::Acknowledged;
        self.acknowledged_at = Some(Utc::now());
        self.acknowledged_by = Some(by.into());
    }

    pub fn resolve(&mut self) {
        self.status = AlertStatus::Resolved;
        self.resolved_at = Some(Utc::now());
    }

    pub fn silence(&mut self, until: DateTime<Utc>) {
        self.status = AlertStatus::Silenced;
        self.silenced_until = Some(until);
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, AlertStatus::Firing | AlertStatus::Acknowledged)
    }
}

/// Notification channel type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannelType {
    Slack,
    Email,
    PagerDuty,
    Webhook,
    Discord,
}

impl FromStr for NotificationChannelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "slack" => Ok(Self::Slack),
            "email" => Ok(Self::Email),
            "pagerduty" => Ok(Self::PagerDuty),
            "webhook" => Ok(Self::Webhook),
            "discord" => Ok(Self::Discord),
            _ => Err(format!("Unknown channel type: {}", s)),
        }
    }
}

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub id: String,
    pub name: String,
    pub channel_type: NotificationChannelType,
    pub config: HashMap<String, String>,
    pub enabled: bool,
    pub rate_limit_per_minute: u32,
    pub created_at: DateTime<Utc>,
}

impl NotificationChannel {
    pub fn new(
        name: impl Into<String>,
        channel_type: NotificationChannelType,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            channel_type,
            config: HashMap::new(),
            enabled: true,
            rate_limit_per_minute: 10,
            created_at: Utc::now(),
        }
    }

    pub fn with_config(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }
}

/// Cost tracking for API usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    pub id: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub epic_id: Option<String>,
    pub story_id: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

impl CostRecord {
    pub fn new(model: impl Into<String>, input_tokens: u64, output_tokens: u64, cost_usd: f64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            model: model.into(),
            input_tokens,
            output_tokens,
            cost_usd,
            agent_id: None,
            agent_type: None,
            epic_id: None,
            story_id: None,
            recorded_at: Utc::now(),
        }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>, agent_type: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self.agent_type = Some(agent_type.into());
        self
    }
}

/// Cost report aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub by_model: HashMap<String, f64>,
    pub by_agent_type: HashMap<String, f64>,
    pub by_epic: HashMap<String, f64>,
    pub daily_breakdown: Vec<DailyCost>,
    pub budget_usd: Option<f64>,
    pub forecast_usd: Option<f64>,
}

impl CostReport {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            period_start: start,
            period_end: end,
            total_cost_usd: 0.0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            by_model: HashMap::new(),
            by_agent_type: HashMap::new(),
            by_epic: HashMap::new(),
            daily_breakdown: Vec::new(),
            budget_usd: None,
            forecast_usd: None,
        }
    }

    pub fn add_record(&mut self, record: &CostRecord) {
        self.total_cost_usd += record.cost_usd;
        self.total_input_tokens += record.input_tokens;
        self.total_output_tokens += record.output_tokens;

        *self.by_model.entry(record.model.clone()).or_insert(0.0) += record.cost_usd;

        if let Some(agent_type) = &record.agent_type {
            *self.by_agent_type.entry(agent_type.clone()).or_insert(0.0) += record.cost_usd;
        }

        if let Some(epic_id) = &record.epic_id {
            *self.by_epic.entry(epic_id.clone()).or_insert(0.0) += record.cost_usd;
        }
    }

    pub fn budget_percentage(&self) -> Option<f64> {
        self.budget_usd.map(|budget| (self.total_cost_usd / budget) * 100.0)
    }

    pub fn to_summary(&self) -> String {
        let mut lines = vec![
            format!("Cost Report: {} to {}",
                self.period_start.format("%Y-%m-%d"),
                self.period_end.format("%Y-%m-%d")),
            String::new(),
            format!("Total: ${:.2}", self.total_cost_usd),
            format!("Tokens: {} input, {} output", self.total_input_tokens, self.total_output_tokens),
        ];

        if !self.by_model.is_empty() {
            lines.push(String::new());
            lines.push("By Model:".to_string());
            for (model, cost) in &self.by_model {
                let pct = (cost / self.total_cost_usd) * 100.0;
                lines.push(format!("  {}: ${:.2} ({:.0}%)", model, cost, pct));
            }
        }

        if let Some(budget) = self.budget_usd {
            lines.push(String::new());
            lines.push(format!("Budget: ${:.2} ({:.0}% used)", budget, self.budget_percentage().unwrap_or(0.0)));
        }

        lines.join("\n")
    }
}

/// Daily cost breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCost {
    pub date: String,
    pub cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Audit log action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    AgentSpawned,
    AgentTerminated,
    ConfigurationChanged,
    ApprovalGranted,
    ApprovalDenied,
    DeploymentTriggered,
    DeploymentRolledBack,
    AlertAcknowledged,
    AlertSilenced,
    UserLogin,
    UserLogout,
    ApiKeyCreated,
    ApiKeyRevoked,
    Custom(String),
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentSpawned => write!(f, "agent.spawned"),
            Self::AgentTerminated => write!(f, "agent.terminated"),
            Self::ConfigurationChanged => write!(f, "config.changed"),
            Self::ApprovalGranted => write!(f, "approval.granted"),
            Self::ApprovalDenied => write!(f, "approval.denied"),
            Self::DeploymentTriggered => write!(f, "deployment.triggered"),
            Self::DeploymentRolledBack => write!(f, "deployment.rolled_back"),
            Self::AlertAcknowledged => write!(f, "alert.acknowledged"),
            Self::AlertSilenced => write!(f, "alert.silenced"),
            Self::UserLogin => write!(f, "user.login"),
            Self::UserLogout => write!(f, "user.logout"),
            Self::ApiKeyCreated => write!(f, "apikey.created"),
            Self::ApiKeyRevoked => write!(f, "apikey.revoked"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub actor_type: ActorType,
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_id: String,
    pub details: HashMap<String, serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

impl AuditEntry {
    pub fn new(
        actor: impl Into<String>,
        action: AuditAction,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor: actor.into(),
            actor_type: ActorType::User,
            action,
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            details: HashMap::new(),
            ip_address: None,
            user_agent: None,
            success: true,
            error_message: None,
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.details.insert(key.into(), value);
        self
    }

    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn as_failed(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error_message = Some(error.into());
        self
    }
}

/// Actor type for audit logs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    User,
    System,
    Agent,
    ApiKey,
    Webhook,
}

/// Agent performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformance {
    pub agent_type: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub success_rate: f64,
    pub avg_duration_seconds: f64,
    pub avg_tokens_per_execution: u64,
    pub avg_cost_per_execution: f64,
    pub p50_duration_seconds: f64,
    pub p95_duration_seconds: f64,
    pub p99_duration_seconds: f64,
}

impl AgentPerformance {
    pub fn new(agent_type: impl Into<String>, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            agent_type: agent_type.into(),
            period_start: start,
            period_end: end,
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            success_rate: 0.0,
            avg_duration_seconds: 0.0,
            avg_tokens_per_execution: 0,
            avg_cost_per_execution: 0.0,
            p50_duration_seconds: 0.0,
            p95_duration_seconds: 0.0,
            p99_duration_seconds: 0.0,
        }
    }

    pub fn calculate_success_rate(&mut self) {
        if self.total_executions > 0 {
            self.success_rate = (self.successful_executions as f64 / self.total_executions as f64) * 100.0;
        }
    }
}

/// System health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub checked_at: DateTime<Utc>,
    pub components: Vec<ComponentHealth>,
    pub active_alerts: u32,
    pub metrics_summary: MetricsSummary,
}

impl SystemHealth {
    pub fn new() -> Self {
        Self {
            status: HealthStatus::Healthy,
            checked_at: Utc::now(),
            components: Vec::new(),
            active_alerts: 0,
            metrics_summary: MetricsSummary::default(),
        }
    }

    pub fn add_component(&mut self, component: ComponentHealth) {
        if component.status == HealthStatus::Unhealthy {
            self.status = HealthStatus::Unhealthy;
        } else if component.status == HealthStatus::Degraded && self.status == HealthStatus::Healthy {
            self.status = HealthStatus::Degraded;
        }
        self.components.push(component);
    }
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self::new()
    }
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Individual component health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub latency_ms: Option<u64>,
}

impl ComponentHealth {
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: None,
            latency_ms: None,
        }
    }

    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            latency_ms: None,
        }
    }
}

/// Summary of key metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub active_agents: u32,
    pub pending_prs: u32,
    pub queue_depth: u32,
    pub error_rate_percent: f64,
    pub avg_response_time_ms: f64,
    pub tokens_used_today: u64,
    pub cost_today_usd: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_value_to_prometheus() {
        let metric = MetricValue::new("http_requests_total", 150.0)
            .with_label("method", "POST")
            .with_label("status", "200");

        let prom = metric.to_prometheus();
        assert!(prom.contains("http_requests_total"));
        assert!(prom.contains("method=\"POST\""));
        assert!(prom.contains("status=\"200\""));
        assert!(prom.contains("150"));
    }

    #[test]
    fn test_histogram_observe() {
        let mut hist = HistogramValue::new("request_duration");
        hist.observe(0.05);
        hist.observe(0.15);
        hist.observe(0.5);

        assert_eq!(hist.count, 3);
        assert!((hist.sum - 0.7).abs() < 0.001);

        // Check bucket counts
        assert!(hist.buckets.iter().find(|b| b.le == 0.05).unwrap().count >= 1);
        assert!(hist.buckets.iter().find(|b| b.le == 0.25).unwrap().count >= 2);
    }

    #[test]
    fn test_alert_rule_creation() {
        let rule = AlertRule::new(
            "high-failure-rate",
            "rate(agent_failures_total[5m]) > 0.2",
            AlertSeverity::Critical,
        )
        .with_threshold(0.2)
        .with_channel("slack")
        .with_duration(300);

        assert_eq!(rule.name, "high-failure-rate");
        assert_eq!(rule.severity, AlertSeverity::Critical);
        assert_eq!(rule.threshold, Some(0.2));
        assert!(rule.channels.contains(&"slack".to_string()));
        assert!(rule.enabled);
    }

    #[test]
    fn test_alert_lifecycle() {
        let rule = AlertRule::new("test-rule", "test > 1", AlertSeverity::Warning);
        let mut alert = Alert::new(&rule, "Test condition exceeded");

        assert_eq!(alert.status, AlertStatus::Firing);
        assert!(alert.is_active());

        alert.acknowledge("user@example.com");
        assert_eq!(alert.status, AlertStatus::Acknowledged);
        assert!(alert.is_active());
        assert!(alert.acknowledged_at.is_some());

        alert.resolve();
        assert_eq!(alert.status, AlertStatus::Resolved);
        assert!(!alert.is_active());
        assert!(alert.resolved_at.is_some());
    }

    #[test]
    fn test_notification_channel() {
        let channel = NotificationChannel::new("team-alerts", NotificationChannelType::Slack)
            .with_config("webhook_url", "https://hooks.slack.com/xxx")
            .with_config("channel", "#alerts");

        assert_eq!(channel.channel_type, NotificationChannelType::Slack);
        assert_eq!(channel.config.get("webhook_url"), Some(&"https://hooks.slack.com/xxx".to_string()));
    }

    #[test]
    fn test_cost_report() {
        let start = Utc::now() - chrono::Duration::days(30);
        let end = Utc::now();
        let mut report = CostReport::new(start, end);
        report.budget_usd = Some(1000.0);

        let record1 = CostRecord::new("claude-3-opus", 10000, 2000, 0.50)
            .with_agent("agent-1", "story-developer");
        let record2 = CostRecord::new("claude-3-sonnet", 5000, 1000, 0.10);

        report.add_record(&record1);
        report.add_record(&record2);

        assert!((report.total_cost_usd - 0.60).abs() < 0.001);
        assert_eq!(report.total_input_tokens, 15000);
        assert!((report.budget_percentage().unwrap() - 0.06).abs() < 0.01);

        let summary = report.to_summary();
        assert!(summary.contains("Total: $0.60"));
    }

    #[test]
    fn test_audit_entry() {
        let entry = AuditEntry::new(
            "user@example.com",
            AuditAction::DeploymentTriggered,
            "environment",
            "production",
        )
        .with_detail("version", serde_json::json!("1.2.0"))
        .with_ip("192.168.1.100");

        assert_eq!(entry.action.to_string(), "deployment.triggered");
        assert!(entry.success);
        assert!(entry.details.contains_key("version"));
    }

    #[test]
    fn test_system_health() {
        let mut health = SystemHealth::new();
        health.add_component(ComponentHealth::healthy("database"));
        health.add_component(ComponentHealth::healthy("api"));

        assert_eq!(health.status, HealthStatus::Healthy);

        health.add_component(ComponentHealth::unhealthy("cache", "Connection timeout"));
        assert_eq!(health.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_agent_performance() {
        let start = Utc::now() - chrono::Duration::days(7);
        let mut perf = AgentPerformance::new("story-developer", start, Utc::now());
        perf.total_executions = 100;
        perf.successful_executions = 95;
        perf.failed_executions = 5;
        perf.calculate_success_rate();

        assert!((perf.success_rate - 95.0).abs() < 0.1);
    }
}
