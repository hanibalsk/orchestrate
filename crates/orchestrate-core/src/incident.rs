//! Incident Response Module
//!
//! Types and utilities for incident detection, response, and remediation.
//! Enables autonomous incident handling with human escalation when needed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Incident severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl IncidentSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    /// Check if severity requires immediate escalation
    pub fn requires_escalation(&self) -> bool {
        matches!(self, Self::Critical)
    }
}

impl std::str::FromStr for IncidentSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" | "crit" | "p0" => Ok(Self::Critical),
            "high" | "p1" => Ok(Self::High),
            "medium" | "med" | "p2" => Ok(Self::Medium),
            "low" | "p3" => Ok(Self::Low),
            _ => Err(format!("Unknown severity: {}", s)),
        }
    }
}

/// Incident status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Detected,
    Investigating,
    Mitigating,
    Resolved,
    PostMortem,
}

impl IncidentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Detected => "detected",
            Self::Investigating => "investigating",
            Self::Mitigating => "mitigating",
            Self::Resolved => "resolved",
            Self::PostMortem => "post_mortem",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Detected | Self::Investigating | Self::Mitigating)
    }
}

impl std::str::FromStr for IncidentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "detected" => Ok(Self::Detected),
            "investigating" => Ok(Self::Investigating),
            "mitigating" => Ok(Self::Mitigating),
            "resolved" => Ok(Self::Resolved),
            "post_mortem" | "postmortem" => Ok(Self::PostMortem),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

/// Core incident type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub status: IncidentStatus,
    pub detected_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub timeline: Vec<TimelineEvent>,
    pub affected_services: Vec<String>,
    pub related_incidents: Vec<String>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl Incident {
    /// Create a new incident
    pub fn new(id: &str, title: &str, severity: IncidentSeverity) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: String::new(),
            severity,
            status: IncidentStatus::Detected,
            detected_at: Utc::now(),
            acknowledged_at: None,
            resolved_at: None,
            timeline: vec![TimelineEvent {
                timestamp: Utc::now(),
                event_type: TimelineEventType::Detected,
                description: format!("Incident detected: {}", title),
                actor: None,
                metadata: HashMap::new(),
            }],
            affected_services: vec![],
            related_incidents: vec![],
            tags: vec![],
            metadata: HashMap::new(),
        }
    }

    /// Acknowledge the incident
    pub fn acknowledge(&mut self, actor: Option<&str>) {
        self.acknowledged_at = Some(Utc::now());
        self.add_timeline_event(
            TimelineEventType::Acknowledged,
            "Incident acknowledged",
            actor,
        );
    }

    /// Transition to investigating
    pub fn start_investigation(&mut self, actor: Option<&str>) {
        self.status = IncidentStatus::Investigating;
        self.add_timeline_event(
            TimelineEventType::InvestigationStarted,
            "Investigation started",
            actor,
        );
    }

    /// Transition to mitigating
    pub fn start_mitigation(&mut self, actor: Option<&str>) {
        self.status = IncidentStatus::Mitigating;
        self.add_timeline_event(
            TimelineEventType::MitigationStarted,
            "Mitigation started",
            actor,
        );
    }

    /// Resolve the incident
    pub fn resolve(&mut self, resolution: &str, actor: Option<&str>) {
        self.status = IncidentStatus::Resolved;
        self.resolved_at = Some(Utc::now());
        self.add_timeline_event(
            TimelineEventType::Resolved,
            &format!("Incident resolved: {}", resolution),
            actor,
        );
    }

    /// Add a timeline event
    pub fn add_timeline_event(
        &mut self,
        event_type: TimelineEventType,
        description: &str,
        actor: Option<&str>,
    ) {
        self.timeline.push(TimelineEvent {
            timestamp: Utc::now(),
            event_type,
            description: description.to_string(),
            actor: actor.map(|s| s.to_string()),
            metadata: HashMap::new(),
        });
    }

    /// Calculate incident duration
    pub fn duration(&self) -> Option<i64> {
        self.resolved_at
            .map(|resolved| (resolved - self.detected_at).num_seconds())
    }

    /// Mean Time to Resolution (in minutes)
    pub fn mttr_minutes(&self) -> Option<f64> {
        self.duration().map(|secs| secs as f64 / 60.0)
    }
}

/// Timeline event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventType {
    Detected,
    Acknowledged,
    InvestigationStarted,
    RootCauseIdentified,
    MitigationStarted,
    PlaybookExecuted,
    Escalated,
    Resolved,
    PostMortemCreated,
    Comment,
}

/// Timeline event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: TimelineEventType,
    pub description: String,
    pub actor: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Root cause analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCauseAnalysis {
    pub incident_id: String,
    pub primary_cause: String,
    pub evidence: Vec<Evidence>,
    pub contributing_factors: Vec<String>,
    pub hypotheses: Vec<Hypothesis>,
    pub related_events: Vec<RelatedEvent>,
    pub analyzed_at: DateTime<Utc>,
}

impl RootCauseAnalysis {
    /// Create a new analysis
    pub fn new(incident_id: &str) -> Self {
        Self {
            incident_id: incident_id.to_string(),
            primary_cause: String::new(),
            evidence: vec![],
            contributing_factors: vec![],
            hypotheses: vec![],
            related_events: vec![],
            analyzed_at: Utc::now(),
        }
    }

    /// Set primary cause
    pub fn set_primary_cause(&mut self, cause: &str) {
        self.primary_cause = cause.to_string();
    }

    /// Add evidence
    pub fn add_evidence(&mut self, evidence_type: EvidenceType, description: &str, source: &str) {
        self.evidence.push(Evidence {
            evidence_type,
            description: description.to_string(),
            source: source.to_string(),
            timestamp: Some(Utc::now()),
        });
    }

    /// Add hypothesis
    pub fn add_hypothesis(&mut self, description: &str, confidence: f64) {
        self.hypotheses.push(Hypothesis {
            description: description.to_string(),
            confidence,
            evidence_for: vec![],
            evidence_against: vec![],
        });
    }

    /// Generate summary
    pub fn to_summary(&self) -> String {
        let mut output = format!("# Root Cause Analysis: {}\n\n", self.incident_id);
        output.push_str(&format!("Analyzed at: {}\n\n", self.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC")));

        output.push_str("## Primary Cause\n");
        output.push_str(&format!("{}\n\n", self.primary_cause));

        if !self.evidence.is_empty() {
            output.push_str("## Evidence\n");
            for e in &self.evidence {
                output.push_str(&format!("- [{}] {}\n", e.evidence_type.as_str(), e.description));
            }
            output.push('\n');
        }

        if !self.contributing_factors.is_empty() {
            output.push_str("## Contributing Factors\n");
            for f in &self.contributing_factors {
                output.push_str(&format!("- {}\n", f));
            }
            output.push('\n');
        }

        if !self.related_events.is_empty() {
            output.push_str("## Related Events\n");
            for e in &self.related_events {
                output.push_str(&format!("- {} at {}\n", e.event_type, e.timestamp.format("%H:%M:%S")));
            }
        }

        output
    }
}

/// Evidence type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    LogPattern,
    MetricSpike,
    Deployment,
    ConfigChange,
    ErrorMessage,
    Correlation,
}

impl EvidenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LogPattern => "log",
            Self::MetricSpike => "metric",
            Self::Deployment => "deploy",
            Self::ConfigChange => "config",
            Self::ErrorMessage => "error",
            Self::Correlation => "correlation",
        }
    }
}

/// Evidence item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: EvidenceType,
    pub description: String,
    pub source: String,
    pub timestamp: Option<DateTime<Utc>>,
}

/// Hypothesis for root cause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub description: String,
    pub confidence: f64,
    pub evidence_for: Vec<String>,
    pub evidence_against: Vec<String>,
}

/// Related event during incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub description: String,
    pub source: String,
}

/// Remediation playbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playbook {
    pub id: String,
    pub name: String,
    pub description: String,
    pub triggers: Vec<PlaybookTrigger>,
    pub actions: Vec<PlaybookAction>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Playbook {
    /// Create a new playbook
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            triggers: vec![],
            actions: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Add a trigger
    pub fn add_trigger(&mut self, condition: &str, threshold: Option<f64>) {
        self.triggers.push(PlaybookTrigger {
            condition: condition.to_string(),
            threshold,
        });
    }

    /// Add an action
    pub fn add_action(&mut self, name: &str, command: &str, requires_approval: bool) {
        self.actions.push(PlaybookAction {
            name: name.to_string(),
            command: command.to_string(),
            requires_approval,
            condition: None,
            timeout_seconds: 300,
        });
    }
}

/// Playbook trigger condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookTrigger {
    pub condition: String,
    pub threshold: Option<f64>,
}

/// Playbook action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookAction {
    pub name: String,
    pub command: String,
    pub requires_approval: bool,
    pub condition: Option<String>,
    pub timeout_seconds: u32,
}

/// Playbook execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybookExecution {
    pub id: String,
    pub playbook_id: String,
    pub incident_id: Option<String>,
    pub status: PlaybookExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub action_results: Vec<ActionResult>,
    pub triggered_by: Option<String>,
}

/// Playbook execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookExecutionStatus {
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Cancelled,
}

impl PlaybookExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Action result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_name: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

/// Escalation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRule {
    pub name: String,
    pub condition: EscalationCondition,
    pub targets: Vec<EscalationTarget>,
    pub delay_seconds: u32,
    pub repeat_interval_seconds: Option<u32>,
}

/// Escalation condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationCondition {
    SeverityCritical,
    NoAcknowledgmentWithin { seconds: u32 },
    RemediationFailed,
    ApprovalTimeout { seconds: u32 },
    Custom { expression: String },
}

/// Escalation target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationTarget {
    pub target_type: EscalationTargetType,
    pub destination: String,
}

/// Escalation target type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationTargetType {
    Slack,
    PagerDuty,
    Email,
    Webhook,
}

impl EscalationTargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Slack => "slack",
            Self::PagerDuty => "pagerduty",
            Self::Email => "email",
            Self::Webhook => "webhook",
        }
    }
}

/// Post-mortem document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMortem {
    pub incident_id: String,
    pub title: String,
    pub summary: String,
    pub impact: IncidentImpact,
    pub timeline: Vec<TimelineEvent>,
    pub root_cause: String,
    pub contributing_factors: Vec<String>,
    pub resolution: String,
    pub action_items: Vec<ActionItem>,
    pub lessons_learned: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub authors: Vec<String>,
}

impl PostMortem {
    /// Create a new post-mortem from an incident
    pub fn from_incident(incident: &Incident) -> Self {
        Self {
            incident_id: incident.id.clone(),
            title: format!("Post-Mortem: {}", incident.title),
            summary: String::new(),
            impact: IncidentImpact::default(),
            timeline: incident.timeline.clone(),
            root_cause: String::new(),
            contributing_factors: vec![],
            resolution: String::new(),
            action_items: vec![],
            lessons_learned: vec![],
            created_at: Utc::now(),
            authors: vec![],
        }
    }

    /// Add an action item
    pub fn add_action_item(&mut self, description: &str, priority: ActionItemPriority, assignee: Option<&str>) {
        self.action_items.push(ActionItem {
            description: description.to_string(),
            priority,
            assignee: assignee.map(|s| s.to_string()),
            due_date: None,
            completed: false,
        });
    }

    /// Generate markdown
    pub fn to_markdown(&self) -> String {
        let mut output = format!("# {}\n\n", self.title);

        output.push_str("## Summary\n");
        output.push_str(&format!("{}\n\n", self.summary));

        output.push_str("## Impact\n");
        if let Some(duration) = &self.impact.duration_minutes {
            output.push_str(&format!("- Duration: {} minutes\n", duration));
        }
        if let Some(users) = &self.impact.users_affected {
            output.push_str(&format!("- Users affected: ~{}\n", users));
        }
        if let Some(revenue) = &self.impact.revenue_impact {
            output.push_str(&format!("- Revenue impact: ~${}\n", revenue));
        }
        output.push('\n');

        output.push_str("## Timeline\n");
        for event in &self.timeline {
            output.push_str(&format!(
                "- {} - {}\n",
                event.timestamp.format("%H:%M:%S"),
                event.description
            ));
        }
        output.push('\n');

        output.push_str("## Root Cause\n");
        output.push_str(&format!("{}\n\n", self.root_cause));

        if !self.contributing_factors.is_empty() {
            output.push_str("## Contributing Factors\n");
            for factor in &self.contributing_factors {
                output.push_str(&format!("- {}\n", factor));
            }
            output.push('\n');
        }

        output.push_str("## Resolution\n");
        output.push_str(&format!("{}\n\n", self.resolution));

        if !self.action_items.is_empty() {
            output.push_str("## Action Items\n");
            for item in &self.action_items {
                let check = if item.completed { "x" } else { " " };
                let assignee = item.assignee.as_deref().unwrap_or("unassigned");
                output.push_str(&format!("- [{}] {} ({})\n", check, item.description, assignee));
            }
            output.push('\n');
        }

        if !self.lessons_learned.is_empty() {
            output.push_str("## Lessons Learned\n");
            for lesson in &self.lessons_learned {
                output.push_str(&format!("- {}\n", lesson));
            }
        }

        output
    }
}

/// Incident impact
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncidentImpact {
    pub duration_minutes: Option<u32>,
    pub users_affected: Option<u32>,
    pub revenue_impact: Option<f64>,
    pub services_affected: Vec<String>,
}

/// Action item from post-mortem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub description: String,
    pub priority: ActionItemPriority,
    pub assignee: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub completed: bool,
}

/// Action item priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionItemPriority {
    High,
    Medium,
    Low,
}

impl ActionItemPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Anomaly detection metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyMetric {
    pub name: String,
    pub current_value: f64,
    pub baseline_value: f64,
    pub threshold: f64,
    pub deviation_percent: f64,
    pub is_anomaly: bool,
    pub timestamp: DateTime<Utc>,
}

impl AnomalyMetric {
    /// Calculate if value is anomalous
    pub fn calculate_anomaly(name: &str, current: f64, baseline: f64, threshold_percent: f64) -> Self {
        let deviation = if baseline != 0.0 {
            ((current - baseline) / baseline * 100.0).abs()
        } else if current != 0.0 {
            100.0
        } else {
            0.0
        };

        Self {
            name: name.to_string(),
            current_value: current,
            baseline_value: baseline,
            threshold: threshold_percent,
            deviation_percent: deviation,
            is_anomaly: deviation > threshold_percent,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_severity_from_str() {
        use std::str::FromStr;

        assert_eq!(IncidentSeverity::from_str("critical").unwrap(), IncidentSeverity::Critical);
        assert_eq!(IncidentSeverity::from_str("P0").unwrap(), IncidentSeverity::Critical);
        assert_eq!(IncidentSeverity::from_str("high").unwrap(), IncidentSeverity::High);
        assert_eq!(IncidentSeverity::from_str("low").unwrap(), IncidentSeverity::Low);
        assert!(IncidentSeverity::from_str("unknown").is_err());
    }

    #[test]
    fn test_incident_creation() {
        let incident = Incident::new("INC-001", "Database connection exhaustion", IncidentSeverity::High);

        assert_eq!(incident.id, "INC-001");
        assert_eq!(incident.severity, IncidentSeverity::High);
        assert_eq!(incident.status, IncidentStatus::Detected);
        assert!(incident.status.is_active());
        assert_eq!(incident.timeline.len(), 1);
    }

    #[test]
    fn test_incident_lifecycle() {
        let mut incident = Incident::new("INC-002", "API latency spike", IncidentSeverity::Medium);

        incident.acknowledge(Some("on-call"));
        assert!(incident.acknowledged_at.is_some());

        incident.start_investigation(Some("sre-team"));
        assert_eq!(incident.status, IncidentStatus::Investigating);

        incident.start_mitigation(Some("sre-team"));
        assert_eq!(incident.status, IncidentStatus::Mitigating);

        incident.resolve("Scaled up pods", Some("sre-team"));
        assert_eq!(incident.status, IncidentStatus::Resolved);
        assert!(incident.resolved_at.is_some());
        assert!(incident.duration().is_some());
    }

    #[test]
    fn test_root_cause_analysis() {
        let mut rca = RootCauseAnalysis::new("INC-001");

        rca.set_primary_cause("Database connection pool exhaustion");
        rca.add_evidence(EvidenceType::LogPattern, "145 connection timeout errors", "app.log");
        rca.add_hypothesis("Connection pool too small", 0.85);

        assert_eq!(rca.primary_cause, "Database connection pool exhaustion");
        assert_eq!(rca.evidence.len(), 1);
        assert_eq!(rca.hypotheses.len(), 1);

        let summary = rca.to_summary();
        assert!(summary.contains("Database connection pool exhaustion"));
    }

    #[test]
    fn test_playbook_creation() {
        let mut playbook = Playbook::new("pb-001", "db-connection-exhaustion");

        playbook.add_trigger("db_pool_usage > 90%", Some(90.0));
        playbook.add_action("Increase pool size", "kubectl set env deployment/app DB_POOL_SIZE=50", false);
        playbook.add_action("Restart pods", "kubectl rollout restart deployment/app", true);

        assert_eq!(playbook.triggers.len(), 1);
        assert_eq!(playbook.actions.len(), 2);
        assert!(!playbook.actions[0].requires_approval);
        assert!(playbook.actions[1].requires_approval);
    }

    #[test]
    fn test_post_mortem_generation() {
        let incident = Incident::new("INC-003", "Service outage", IncidentSeverity::Critical);

        let mut pm = PostMortem::from_incident(&incident);
        pm.summary = "Complete service outage for 10 minutes".to_string();
        pm.root_cause = "Misconfigured load balancer".to_string();
        pm.add_action_item("Review LB configuration process", ActionItemPriority::High, Some("platform-team"));
        pm.lessons_learned.push("Need better staging environment parity".to_string());

        let md = pm.to_markdown();
        assert!(md.contains("Post-Mortem: Service outage"));
        assert!(md.contains("Misconfigured load balancer"));
        assert!(md.contains("Review LB configuration"));
    }

    #[test]
    fn test_anomaly_detection() {
        let normal = AnomalyMetric::calculate_anomaly("error_rate", 2.0, 2.0, 50.0);
        assert!(!normal.is_anomaly);

        let anomaly = AnomalyMetric::calculate_anomaly("error_rate", 10.0, 2.0, 50.0);
        assert!(anomaly.is_anomaly);
        assert!(anomaly.deviation_percent > 50.0);
    }

    #[test]
    fn test_escalation_conditions() {
        assert!(IncidentSeverity::Critical.requires_escalation());
        assert!(!IncidentSeverity::High.requires_escalation());
        assert!(!IncidentSeverity::Medium.requires_escalation());
    }
}
