//! Alerting Rules Engine
//!
//! This module provides alerting functionality including:
//! - Alert rule management (create, enable, disable, delete)
//! - Alert condition evaluation (threshold, rate, absence)
//! - Alert deduplication and tracking
//! - Alert lifecycle management (trigger, acknowledge, resolve)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for AlertSeverity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("Invalid severity: {}", s)),
        }
    }
}

/// Alert status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
}

impl std::fmt::Display for AlertStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Acknowledged => write!(f, "acknowledged"),
            Self::Resolved => write!(f, "resolved"),
        }
    }
}

impl std::str::FromStr for AlertStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "acknowledged" => Ok(Self::Acknowledged),
            "resolved" => Ok(Self::Resolved),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: Option<i64>,
    pub name: String,
    pub condition: String,
    pub severity: AlertSeverity,
    pub channels: Vec<String>,
    pub enabled: bool,
    pub evaluation_interval_seconds: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl AlertRule {
    /// Create a new alert rule
    pub fn new(
        name: impl Into<String>,
        condition: impl Into<String>,
        severity: AlertSeverity,
        channels: Vec<String>,
    ) -> Self {
        Self {
            id: None,
            name: name.into(),
            condition: condition.into(),
            severity,
            channels,
            enabled: true,
            evaluation_interval_seconds: 60,
            created_at: None,
            updated_at: None,
        }
    }

    /// Set evaluation interval
    pub fn with_interval(mut self, interval_seconds: i64) -> Self {
        self.evaluation_interval_seconds = interval_seconds;
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Triggered alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Option<i64>,
    pub rule_id: i64,
    pub status: AlertStatus,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub trigger_value: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub fingerprint: String,
    pub last_notified_at: Option<DateTime<Utc>>,
    pub notification_count: i64,
}

impl Alert {
    /// Create a new alert instance
    pub fn new(rule_id: i64, fingerprint: impl Into<String>) -> Self {
        Self {
            id: None,
            rule_id,
            status: AlertStatus::Active,
            triggered_at: Utc::now(),
            resolved_at: None,
            acknowledged_at: None,
            acknowledged_by: None,
            trigger_value: None,
            metadata: None,
            fingerprint: fingerprint.into(),
            last_notified_at: None,
            notification_count: 0,
        }
    }

    /// Set trigger value
    pub fn with_trigger_value(mut self, value: serde_json::Value) -> Self {
        self.trigger_value = Some(value);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Check if the alert is active
    pub fn is_active(&self) -> bool {
        self.status == AlertStatus::Active
    }

    /// Check if the alert is resolved
    pub fn is_resolved(&self) -> bool {
        self.status == AlertStatus::Resolved
    }
}

/// Alert condition type
#[derive(Debug, Clone, PartialEq)]
pub enum ConditionType {
    /// Threshold condition: metric > threshold, metric < threshold, metric == threshold
    Threshold {
        metric: String,
        operator: ThresholdOperator,
        threshold: f64,
    },
    /// Rate condition: increase by X% in Y minutes
    Rate {
        metric: String,
        window_minutes: i64,
        threshold_percent: f64,
    },
    /// Absence condition: no data for X minutes
    Absence {
        metric: String,
        duration_minutes: i64,
    },
}

/// Threshold comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdOperator {
    GreaterThan,
    LessThan,
    Equal,
}

impl ThresholdOperator {
    /// Evaluate the operator against two values
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            Self::GreaterThan => value > threshold,
            Self::LessThan => value < threshold,
            Self::Equal => (value - threshold).abs() < f64::EPSILON,
        }
    }
}

/// Alert condition parser - parses condition strings into structured conditions
pub struct ConditionParser;

impl ConditionParser {
    /// Parse a condition string into a ConditionType
    ///
    /// Examples:
    /// - "orchestrate_queue_depth{queue='webhook_events'} > 100" -> Threshold
    /// - "rate(orchestrate_agent_failures_total[5m]) > 0.2" -> Rate
    /// - "absence(orchestrate_heartbeat[10m])" -> Absence
    pub fn parse(condition: &str) -> Result<ConditionType, String> {
        let condition = condition.trim();

        // Check for absence condition first (has no operator)
        if condition.starts_with("absence(") {
            if let Some(absence_match) = Self::parse_absence_condition(condition) {
                return Ok(absence_match);
            }
        }

        // Check for rate condition: rate(metric[window]) > threshold
        if condition.starts_with("rate(") {
            if let Some(rate_match) = Self::parse_rate_condition(condition) {
                return Ok(rate_match);
            }
        }

        // Default to threshold condition: metric > threshold
        Self::parse_threshold_condition(condition)
    }

    fn parse_rate_condition(condition: &str) -> Option<ConditionType> {
        use regex::Regex;

        // Pattern: rate(metric[5m]) > 0.2
        // Capture: rate( <metric> [ <window>m ] ) > <threshold>
        let re = Regex::new(r"^rate\((.+?)\[(\d+)m\]\)\s*>\s*([0-9.]+)$").ok()?;
        let caps = re.captures(condition)?;

        let metric = caps.get(1)?.as_str().trim().to_string();
        let window_minutes: i64 = caps.get(2)?.as_str().parse().ok()?;
        let threshold_percent: f64 = caps.get(3)?.as_str().parse().ok()?;

        Some(ConditionType::Rate {
            metric,
            window_minutes,
            threshold_percent,
        })
    }

    fn parse_absence_condition(condition: &str) -> Option<ConditionType> {
        use regex::Regex;

        // Pattern: absence(metric[10m])
        // Capture: absence( <metric> [ <duration>m ] )
        let re = Regex::new(r"^absence\((.+?)\[(\d+)m\]\)$").ok()?;
        let caps = re.captures(condition)?;

        let metric = caps.get(1)?.as_str().trim().to_string();
        let duration_minutes: i64 = caps.get(2)?.as_str().parse().ok()?;

        Some(ConditionType::Absence {
            metric,
            duration_minutes,
        })
    }

    fn parse_threshold_condition(condition: &str) -> Result<ConditionType, String> {
        use regex::Regex;

        // Pattern: metric{labels} > threshold or metric > threshold
        // We need to be careful to capture the full metric name including optional {labels}
        let re = Regex::new(r"^(.+?)\s*([><=]+)\s*([0-9.]+)$")
            .map_err(|e| format!("Regex error: {}", e))?;

        let caps = re
            .captures(condition)
            .ok_or_else(|| format!("Invalid threshold condition: {}", condition))?;

        let metric = caps.get(1).unwrap().as_str().trim().to_string();
        let operator_str = caps.get(2).unwrap().as_str();
        let threshold: f64 = caps
            .get(3)
            .unwrap()
            .as_str()
            .parse()
            .map_err(|e| format!("Invalid threshold value: {}", e))?;

        let operator = match operator_str {
            ">" => ThresholdOperator::GreaterThan,
            "<" => ThresholdOperator::LessThan,
            "==" => ThresholdOperator::Equal,
            _ => return Err(format!("Invalid operator: {}", operator_str)),
        };

        Ok(ConditionType::Threshold {
            metric,
            operator,
            threshold,
        })
    }
}

/// Alert evaluator - evaluates conditions against metrics
pub struct AlertEvaluator;

impl AlertEvaluator {
    /// Evaluate a condition against current metrics
    pub fn evaluate(
        condition: &ConditionType,
        metrics: &HashMap<String, f64>,
        metric_history: &HashMap<String, Vec<(DateTime<Utc>, f64)>>,
    ) -> bool {
        match condition {
            ConditionType::Threshold {
                metric,
                operator,
                threshold,
            } => {
                if let Some(&value) = metrics.get(metric) {
                    operator.evaluate(value, *threshold)
                } else {
                    false
                }
            }
            ConditionType::Rate {
                metric,
                window_minutes,
                threshold_percent,
            } => {
                Self::evaluate_rate(metric, *window_minutes, *threshold_percent, metric_history)
            }
            ConditionType::Absence {
                metric,
                duration_minutes,
            } => Self::evaluate_absence(metric, *duration_minutes, metric_history),
        }
    }

    fn evaluate_rate(
        metric: &str,
        window_minutes: i64,
        threshold_percent: f64,
        metric_history: &HashMap<String, Vec<(DateTime<Utc>, f64)>>,
    ) -> bool {
        if let Some(history) = metric_history.get(metric) {
            if history.len() < 2 {
                return false;
            }

            let now = Utc::now();
            let window_start = now - chrono::Duration::minutes(window_minutes);

            // Get values in the window
            let values_in_window: Vec<f64> = history
                .iter()
                .filter(|(timestamp, _)| *timestamp >= window_start)
                .map(|(_, value)| *value)
                .collect();

            if values_in_window.len() < 2 {
                return false;
            }

            let first_value = values_in_window.first().unwrap();
            let last_value = values_in_window.last().unwrap();

            if *first_value == 0.0 {
                return false;
            }

            let rate = (last_value - first_value) / first_value;
            rate > threshold_percent
        } else {
            false
        }
    }

    fn evaluate_absence(
        metric: &str,
        duration_minutes: i64,
        metric_history: &HashMap<String, Vec<(DateTime<Utc>, f64)>>,
    ) -> bool {
        if let Some(history) = metric_history.get(metric) {
            if let Some((last_timestamp, _)) = history.last() {
                let now = Utc::now();
                let absence_threshold = now - chrono::Duration::minutes(duration_minutes);
                *last_timestamp < absence_threshold
            } else {
                true // No data ever = absence
            }
        } else {
            true // Metric not found = absence
        }
    }
}

/// Generate fingerprint for alert deduplication
pub fn generate_fingerprint(rule_name: &str, condition: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(rule_name.as_bytes());
    hasher.update(condition.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_severity_to_string() {
        assert_eq!(AlertSeverity::Info.to_string(), "info");
        assert_eq!(AlertSeverity::Warning.to_string(), "warning");
        assert_eq!(AlertSeverity::Critical.to_string(), "critical");
    }

    #[test]
    fn test_alert_severity_from_string() {
        use std::str::FromStr;
        assert_eq!(AlertSeverity::from_str("info").unwrap(), AlertSeverity::Info);
        assert_eq!(AlertSeverity::from_str("warning").unwrap(), AlertSeverity::Warning);
        assert_eq!(AlertSeverity::from_str("critical").unwrap(), AlertSeverity::Critical);
        assert!(AlertSeverity::from_str("invalid").is_err());
    }

    #[test]
    fn test_alert_rule_creation() {
        let rule = AlertRule::new(
            "high-failure-rate",
            "rate(orchestrate_agent_failures_total[5m]) > 0.2",
            AlertSeverity::Critical,
            vec!["slack".to_string(), "pagerduty".to_string()],
        );

        assert_eq!(rule.name, "high-failure-rate");
        assert_eq!(rule.severity, AlertSeverity::Critical);
        assert_eq!(rule.channels.len(), 2);
        assert!(rule.enabled);
        assert_eq!(rule.evaluation_interval_seconds, 60);
    }

    #[test]
    fn test_alert_rule_with_interval() {
        let rule = AlertRule::new(
            "test-rule",
            "metric > 100",
            AlertSeverity::Warning,
            vec!["email".to_string()],
        )
        .with_interval(30);

        assert_eq!(rule.evaluation_interval_seconds, 30);
    }

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(1, "fingerprint123");

        assert_eq!(alert.rule_id, 1);
        assert_eq!(alert.fingerprint, "fingerprint123");
        assert_eq!(alert.status, AlertStatus::Active);
        assert!(alert.is_active());
        assert!(!alert.is_resolved());
        assert_eq!(alert.notification_count, 0);
    }

    #[test]
    fn test_threshold_operator_evaluate() {
        assert!(ThresholdOperator::GreaterThan.evaluate(150.0, 100.0));
        assert!(!ThresholdOperator::GreaterThan.evaluate(50.0, 100.0));

        assert!(ThresholdOperator::LessThan.evaluate(50.0, 100.0));
        assert!(!ThresholdOperator::LessThan.evaluate(150.0, 100.0));

        assert!(ThresholdOperator::Equal.evaluate(100.0, 100.0));
        assert!(!ThresholdOperator::Equal.evaluate(100.1, 100.0));
    }

    #[test]
    fn test_parse_threshold_condition() {
        let condition = ConditionParser::parse("orchestrate_queue_depth{queue='webhook_events'} > 100").unwrap();

        match condition {
            ConditionType::Threshold {
                metric,
                operator,
                threshold,
            } => {
                assert!(metric.contains("orchestrate_queue_depth"));
                assert_eq!(operator, ThresholdOperator::GreaterThan);
                assert_eq!(threshold, 100.0);
            }
            _ => panic!("Expected threshold condition"),
        }
    }

    #[test]
    fn test_parse_rate_condition() {
        let condition = ConditionParser::parse("rate(orchestrate_agent_failures_total[5m]) > 0.2").unwrap();

        match condition {
            ConditionType::Rate {
                metric,
                window_minutes,
                threshold_percent,
            } => {
                assert!(metric.contains("orchestrate_agent_failures_total"));
                assert_eq!(window_minutes, 5);
                assert_eq!(threshold_percent, 0.2);
            }
            _ => panic!("Expected rate condition"),
        }
    }

    #[test]
    fn test_parse_absence_condition() {
        let condition = ConditionParser::parse("absence(orchestrate_heartbeat[10m])").unwrap();

        match condition {
            ConditionType::Absence {
                metric,
                duration_minutes,
            } => {
                assert!(metric.contains("orchestrate_heartbeat"));
                assert_eq!(duration_minutes, 10);
            }
            _ => panic!("Expected absence condition"),
        }
    }

    #[test]
    fn test_evaluate_threshold_condition() {
        let mut metrics = HashMap::new();
        metrics.insert("queue_depth".to_string(), 150.0);

        let condition = ConditionType::Threshold {
            metric: "queue_depth".to_string(),
            operator: ThresholdOperator::GreaterThan,
            threshold: 100.0,
        };

        let metric_history = HashMap::new();
        let result = AlertEvaluator::evaluate(&condition, &metrics, &metric_history);
        assert!(result);
    }

    #[test]
    fn test_evaluate_threshold_condition_not_triggered() {
        let mut metrics = HashMap::new();
        metrics.insert("queue_depth".to_string(), 50.0);

        let condition = ConditionType::Threshold {
            metric: "queue_depth".to_string(),
            operator: ThresholdOperator::GreaterThan,
            threshold: 100.0,
        };

        let metric_history = HashMap::new();
        let result = AlertEvaluator::evaluate(&condition, &metrics, &metric_history);
        assert!(!result);
    }

    #[test]
    fn test_evaluate_rate_condition() {
        let now = Utc::now();
        let mut metric_history = HashMap::new();
        // Create data points within the 5-minute window
        // Window will be from (now - 5min) to now
        // First point at now - 4 minutes: 100.0
        // Last point at now - 1 minute: 150.0
        // This is a 50% increase (150-100)/100 = 0.5, which is > 0.2 threshold
        metric_history.insert(
            "failures_total".to_string(),
            vec![
                (now - chrono::Duration::minutes(6), 80.0),   // Outside window
                (now - chrono::Duration::minutes(4), 100.0),  // Inside window - first
                (now - chrono::Duration::minutes(2), 130.0),  // Inside window
                (now - chrono::Duration::minutes(1), 150.0),  // Inside window - last
            ],
        );

        let condition = ConditionType::Rate {
            metric: "failures_total".to_string(),
            window_minutes: 5,
            threshold_percent: 0.2, // 20% increase
        };

        let metrics = HashMap::new();
        let result = AlertEvaluator::evaluate(&condition, &metrics, &metric_history);
        assert!(result); // 50% increase (150-100)/100 > 20% threshold
    }

    #[test]
    fn test_evaluate_absence_condition() {
        let old_time = Utc::now() - chrono::Duration::minutes(15);
        let mut metric_history = HashMap::new();
        metric_history.insert("heartbeat".to_string(), vec![(old_time, 1.0)]);

        let condition = ConditionType::Absence {
            metric: "heartbeat".to_string(),
            duration_minutes: 10,
        };

        let metrics = HashMap::new();
        let result = AlertEvaluator::evaluate(&condition, &metrics, &metric_history);
        assert!(result); // No data for 15 minutes > 10 minute threshold
    }

    #[test]
    fn test_generate_fingerprint() {
        let fp1 = generate_fingerprint("rule1", "condition1");
        let fp2 = generate_fingerprint("rule1", "condition1");
        let fp3 = generate_fingerprint("rule2", "condition1");

        assert_eq!(fp1, fp2); // Same inputs = same fingerprint
        assert_ne!(fp1, fp3); // Different inputs = different fingerprint
        assert_eq!(fp1.len(), 64); // SHA256 = 64 hex chars
    }
}
