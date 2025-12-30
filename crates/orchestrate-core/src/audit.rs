//! Audit Logging Module
//!
//! Provides comprehensive audit logging for all agent actions, configuration changes,
//! and approval decisions with structured JSON format and retention policies.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::monitoring::{ActorType, AuditAction, AuditEntry};
use crate::{Database, Result};

/// Audit log query filters
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// Filter by time range
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    /// Filter by actor
    pub actor: Option<String>,
    /// Filter by actor type
    pub actor_type: Option<ActorType>,
    /// Filter by action type
    pub action: Option<AuditAction>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by resource ID
    pub resource_id: Option<String>,
    /// Filter by success/failure
    pub success: Option<bool>,
    /// Pagination limit
    pub limit: Option<i64>,
    /// Pagination offset
    pub offset: Option<i64>,
}

impl AuditQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timerange(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn with_actor_type(mut self, actor_type: ActorType) -> Self {
        self.actor_type = Some(actor_type);
        self
    }

    pub fn with_action(mut self, action: AuditAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_resource(mut self, resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        self.resource_type = Some(resource_type.into());
        self.resource_id = Some(resource_id.into());
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = Some(success);
        self
    }

    pub fn with_pagination(mut self, limit: i64, offset: i64) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}

/// Export format for audit logs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    JsonLines,
}

/// Audit log retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// How long to keep audit logs (in days)
    pub retention_days: u32,
    /// Whether to archive before deletion
    pub archive_before_delete: bool,
    /// Archive location (if archiving)
    pub archive_path: Option<String>,
    /// Whether to compress archives
    pub compress_archives: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            retention_days: 90,
            archive_before_delete: true,
            archive_path: None,
            compress_archives: true,
        }
    }
}

impl RetentionPolicy {
    pub fn new(retention_days: u32) -> Self {
        Self {
            retention_days,
            ..Default::default()
        }
    }

    pub fn with_archive(mut self, path: impl Into<String>) -> Self {
        self.archive_path = Some(path.into());
        self
    }

    pub fn cutoff_date(&self) -> DateTime<Utc> {
        Utc::now() - Duration::days(self.retention_days as i64)
    }
}

/// Statistics about audit log operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_entries: i64,
    pub entries_by_action: HashMap<String, i64>,
    pub entries_by_actor_type: HashMap<String, i64>,
    pub success_count: i64,
    pub failure_count: i64,
    pub first_entry_at: Option<DateTime<Utc>>,
    pub last_entry_at: Option<DateTime<Utc>>,
}

impl Default for AuditStats {
    fn default() -> Self {
        Self {
            total_entries: 0,
            entries_by_action: HashMap::new(),
            entries_by_actor_type: HashMap::new(),
            success_count: 0,
            failure_count: 0,
            first_entry_at: None,
            last_entry_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::AuditAction;

    #[test]
    fn test_audit_query_builder() {
        let start = Utc::now() - Duration::hours(24);
        let end = Utc::now();

        let query = AuditQuery::new()
            .with_timerange(start, end)
            .with_actor("user@example.com")
            .with_actor_type(ActorType::User)
            .with_action(AuditAction::ConfigurationChanged)
            .with_success(true)
            .with_pagination(100, 0);

        assert_eq!(query.actor, Some("user@example.com".to_string()));
        assert_eq!(query.actor_type, Some(ActorType::User));
        assert_eq!(query.action, Some(AuditAction::ConfigurationChanged));
        assert_eq!(query.success, Some(true));
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_retention_policy_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.retention_days, 90);
        assert!(policy.archive_before_delete);
        assert!(policy.compress_archives);
        assert!(policy.archive_path.is_none());
    }

    #[test]
    fn test_retention_policy_custom() {
        let policy = RetentionPolicy::new(30)
            .with_archive("/var/log/orchestrate/archives");

        assert_eq!(policy.retention_days, 30);
        assert_eq!(policy.archive_path, Some("/var/log/orchestrate/archives".to_string()));
    }

    #[test]
    fn test_retention_policy_cutoff_date() {
        let policy = RetentionPolicy::new(7);
        let cutoff = policy.cutoff_date();
        let expected = Utc::now() - Duration::days(7);

        // Allow 1 second variance due to test execution time
        assert!((cutoff.timestamp() - expected.timestamp()).abs() <= 1);
    }

    #[tokio::test]
    async fn test_audit_entry_creation() {
        let entry = AuditEntry::new(
            "test-agent",
            AuditAction::AgentSpawned,
            "agent",
            "agent-123",
        )
        .with_detail("agent_type", serde_json::json!("story-developer"))
        .with_ip("127.0.0.1");

        assert_eq!(entry.actor, "test-agent");
        assert_eq!(entry.action, AuditAction::AgentSpawned);
        assert_eq!(entry.resource_type, "agent");
        assert_eq!(entry.resource_id, "agent-123");
        assert!(entry.success);
        assert_eq!(entry.ip_address, Some("127.0.0.1".to_string()));
        assert!(entry.details.contains_key("agent_type"));
    }

    #[tokio::test]
    async fn test_insert_and_query_audit_entry() {
        let db = Database::in_memory().await.unwrap();

        let entry = AuditEntry::new(
            "user@example.com",
            AuditAction::ConfigurationChanged,
            "system",
            "config-1",
        )
        .with_detail("setting", serde_json::json!("max_workers"))
        .with_detail("old_value", serde_json::json!(5))
        .with_detail("new_value", serde_json::json!(10));

        db.insert_audit_entry(&entry).await.unwrap();

        let query = AuditQuery::new()
            .with_actor("user@example.com")
            .with_action(AuditAction::ConfigurationChanged);

        let entries = db.query_audit_log(&query).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].actor, "user@example.com");
        assert_eq!(entries[0].action, AuditAction::ConfigurationChanged);
    }

    #[tokio::test]
    async fn test_query_by_timerange() {
        let db = Database::in_memory().await.unwrap();

        let now = Utc::now();
        let hour_ago = now - Duration::hours(1);
        let two_hours_ago = now - Duration::hours(2);

        // Insert entries at different times
        let mut entry1 = AuditEntry::new(
            "agent-1",
            AuditAction::AgentSpawned,
            "agent",
            "id-1",
        );
        entry1.timestamp = two_hours_ago;
        db.insert_audit_entry(&entry1).await.unwrap();

        let mut entry2 = AuditEntry::new(
            "agent-2",
            AuditAction::AgentTerminated,
            "agent",
            "id-2",
        );
        entry2.timestamp = hour_ago;
        db.insert_audit_entry(&entry2).await.unwrap();

        let entry3 = AuditEntry::new(
            "agent-3",
            AuditAction::AgentSpawned,
            "agent",
            "id-3",
        );
        db.insert_audit_entry(&entry3).await.unwrap();

        // Query last hour
        let query = AuditQuery::new()
            .with_timerange(hour_ago - Duration::minutes(5), now + Duration::minutes(5));

        let entries = db.query_audit_log(&query).await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_query_by_actor_type() {
        let db = Database::in_memory().await.unwrap();

        let mut entry1 = AuditEntry::new(
            "user@example.com",
            AuditAction::ApprovalGranted,
            "approval",
            "ap-1",
        );
        entry1.actor_type = ActorType::User;
        db.insert_audit_entry(&entry1).await.unwrap();

        let mut entry2 = AuditEntry::new(
            "system",
            AuditAction::AgentSpawned,
            "agent",
            "ag-1",
        );
        entry2.actor_type = ActorType::System;
        db.insert_audit_entry(&entry2).await.unwrap();

        let query = AuditQuery::new()
            .with_actor_type(ActorType::User);

        let entries = db.query_audit_log(&query).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].actor_type, ActorType::User);
    }

    #[tokio::test]
    async fn test_query_by_resource() {
        let db = Database::in_memory().await.unwrap();

        let entry1 = AuditEntry::new(
            "agent-1",
            AuditAction::AgentSpawned,
            "agent",
            "agent-123",
        );
        db.insert_audit_entry(&entry1).await.unwrap();

        let entry2 = AuditEntry::new(
            "user@example.com",
            AuditAction::ConfigurationChanged,
            "config",
            "cfg-456",
        );
        db.insert_audit_entry(&entry2).await.unwrap();

        let query = AuditQuery::new()
            .with_resource("agent", "agent-123");

        let entries = db.query_audit_log(&query).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].resource_type, "agent");
        assert_eq!(entries[0].resource_id, "agent-123");
    }

    #[tokio::test]
    async fn test_query_with_pagination() {
        let db = Database::in_memory().await.unwrap();

        // Insert 10 entries
        for i in 0..10 {
            let entry = AuditEntry::new(
                format!("actor-{}", i),
                AuditAction::AgentSpawned,
                "agent",
                format!("id-{}", i),
            );
            db.insert_audit_entry(&entry).await.unwrap();
        }

        // Query first page
        let query = AuditQuery::new()
            .with_pagination(5, 0);
        let page1 = db.query_audit_log(&query).await.unwrap();
        assert_eq!(page1.len(), 5);

        // Query second page
        let query = AuditQuery::new()
            .with_pagination(5, 5);
        let page2 = db.query_audit_log(&query).await.unwrap();
        assert_eq!(page2.len(), 5);

        // Ensure pages are different
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[tokio::test]
    async fn test_query_success_failure() {
        let db = Database::in_memory().await.unwrap();

        let success_entry = AuditEntry::new(
            "agent-1",
            AuditAction::DeploymentTriggered,
            "deployment",
            "dep-1",
        );
        db.insert_audit_entry(&success_entry).await.unwrap();

        let failed_entry = AuditEntry::new(
            "agent-2",
            AuditAction::DeploymentTriggered,
            "deployment",
            "dep-2",
        )
        .as_failed("Connection timeout");
        db.insert_audit_entry(&failed_entry).await.unwrap();

        // Query successes
        let query = AuditQuery::new().with_success(true);
        let successes = db.query_audit_log(&query).await.unwrap();
        assert_eq!(successes.len(), 1);
        assert!(successes[0].success);

        // Query failures
        let query = AuditQuery::new().with_success(false);
        let failures = db.query_audit_log(&query).await.unwrap();
        assert_eq!(failures.len(), 1);
        assert!(!failures[0].success);
    }

    #[tokio::test]
    async fn test_audit_stats() {
        let db = Database::in_memory().await.unwrap();

        // Insert various entries
        db.insert_audit_entry(&AuditEntry::new(
            "agent-1",
            AuditAction::AgentSpawned,
            "agent",
            "id-1",
        )).await.unwrap();

        db.insert_audit_entry(&AuditEntry::new(
            "agent-2",
            AuditAction::AgentSpawned,
            "agent",
            "id-2",
        )).await.unwrap();

        db.insert_audit_entry(&AuditEntry::new(
            "user@example.com",
            AuditAction::ApprovalGranted,
            "approval",
            "ap-1",
        )).await.unwrap();

        db.insert_audit_entry(&AuditEntry::new(
            "system",
            AuditAction::ConfigurationChanged,
            "config",
            "cfg-1",
        ).as_failed("Validation error")).await.unwrap();

        let stats = db.get_audit_stats().await.unwrap();
        assert_eq!(stats.total_entries, 4);
        assert_eq!(stats.success_count, 3);
        assert_eq!(stats.failure_count, 1);
        assert!(stats.entries_by_action.get("agent.spawned").is_some());
    }

    #[tokio::test]
    async fn test_export_json() {
        let db = Database::in_memory().await.unwrap();

        let entry = AuditEntry::new(
            "test-actor",
            AuditAction::ConfigurationChanged,
            "config",
            "test-1",
        )
        .with_detail("key", serde_json::json!("value"));

        db.insert_audit_entry(&entry).await.unwrap();

        let query = AuditQuery::new();
        let json = db.export_audit_log(&query, ExportFormat::Json).await.unwrap();

        assert!(json.contains("test-actor"));
        assert!(json.contains("ConfigurationChanged") || json.contains("configuration_changed"));
        assert!(json.contains("\"key\""));
    }

    #[tokio::test]
    async fn test_export_csv() {
        let db = Database::in_memory().await.unwrap();

        let entry = AuditEntry::new(
            "test-actor",
            AuditAction::ApprovalGranted,
            "approval",
            "ap-1",
        );

        db.insert_audit_entry(&entry).await.unwrap();

        let query = AuditQuery::new();
        let csv = db.export_audit_log(&query, ExportFormat::Csv).await.unwrap();

        // Check CSV headers
        assert!(csv.contains("id,timestamp,actor,actor_type,action"));
        assert!(csv.contains("test-actor"));
        assert!(csv.contains("approval.granted"));
    }

    #[tokio::test]
    async fn test_apply_retention_policy() {
        let db = Database::in_memory().await.unwrap();

        let now = Utc::now();
        let old_date = now - Duration::days(100);

        // Insert old entry
        let mut old_entry = AuditEntry::new(
            "old-agent",
            AuditAction::AgentSpawned,
            "agent",
            "old-1",
        );
        old_entry.timestamp = old_date;
        db.insert_audit_entry(&old_entry).await.unwrap();

        // Insert recent entry
        let recent_entry = AuditEntry::new(
            "recent-agent",
            AuditAction::AgentSpawned,
            "agent",
            "recent-1",
        );
        db.insert_audit_entry(&recent_entry).await.unwrap();

        // Apply 90-day retention policy
        let policy = RetentionPolicy::new(90);
        let deleted = db.apply_retention_policy(&policy).await.unwrap();

        assert_eq!(deleted, 1);

        // Verify only recent entry remains
        let query = AuditQuery::new();
        let entries = db.query_audit_log(&query).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].actor, "recent-agent");
    }

    #[tokio::test]
    async fn test_count_audit_entries() {
        let db = Database::in_memory().await.unwrap();

        // Insert entries
        for i in 0..5 {
            db.insert_audit_entry(&AuditEntry::new(
                format!("actor-{}", i),
                AuditAction::AgentSpawned,
                "agent",
                format!("id-{}", i),
            )).await.unwrap();
        }

        let query = AuditQuery::new();
        let count = db.count_audit_entries(&query).await.unwrap();
        assert_eq!(count, 5);

        // Count with filter
        let query = AuditQuery::new()
            .with_action(AuditAction::AgentSpawned);
        let count = db.count_audit_entries(&query).await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_combined_filters() {
        let db = Database::in_memory().await.unwrap();

        let now = Utc::now();
        let hour_ago = now - Duration::hours(1);

        // Insert various entries
        let mut entry1 = AuditEntry::new(
            "user@example.com",
            AuditAction::ApprovalGranted,
            "approval",
            "ap-1",
        );
        entry1.actor_type = ActorType::User;
        entry1.timestamp = hour_ago;
        db.insert_audit_entry(&entry1).await.unwrap();

        let mut entry2 = AuditEntry::new(
            "user@example.com",
            AuditAction::ApprovalDenied,
            "approval",
            "ap-2",
        );
        entry2.actor_type = ActorType::User;
        db.insert_audit_entry(&entry2).await.unwrap();

        let entry3 = AuditEntry::new(
            "system",
            AuditAction::AgentSpawned,
            "agent",
            "ag-1",
        );
        db.insert_audit_entry(&entry3).await.unwrap();

        // Query with multiple filters
        let query = AuditQuery::new()
            .with_actor("user@example.com")
            .with_actor_type(ActorType::User)
            .with_timerange(hour_ago - Duration::minutes(5), now + Duration::minutes(5));

        let entries = db.query_audit_log(&query).await.unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.actor == "user@example.com"));
        assert!(entries.iter().all(|e| e.actor_type == ActorType::User));
    }

    #[tokio::test]
    async fn test_structured_json_output() {
        let entry = AuditEntry::new(
            "test-user",
            AuditAction::ConfigurationChanged,
            "system",
            "config-1",
        )
        .with_detail("module", serde_json::json!("agents"))
        .with_detail("changes", serde_json::json!({
            "max_workers": {"from": 5, "to": 10},
            "timeout_seconds": {"from": 30, "to": 60}
        }))
        .with_ip("192.168.1.100");

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&entry).unwrap();

        // Verify it contains expected fields
        assert!(json.contains("test-user"));
        assert!(json.contains("ConfigurationChanged") || json.contains("configuration_changed"));
        assert!(json.contains("max_workers"));
        assert!(json.contains("192.168.1.100"));

        // Verify it can be deserialized
        let deserialized: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.actor, "test-user");
        assert_eq!(deserialized.action, AuditAction::ConfigurationChanged);
    }
}
