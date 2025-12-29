//! Pipeline domain models
//!
//! This module defines the core types for event-driven pipelines:
//! - Pipeline definitions (YAML-based)
//! - Pipeline runs (execution instances)
//! - Pipeline stages (individual steps)
//! - Approval gates
//! - Rollback support

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

use crate::{Error, Result};

// ==================== Pipeline Definition ====================

/// Pipeline definition stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub definition: String,              // YAML definition
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
}

/// Parsed pipeline configuration from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub name: String,
    pub description: Option<String>,
    pub version: i32,
    #[serde(default)]
    pub triggers: Vec<PipelineTrigger>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    pub stages: Vec<StageConfig>,
}

/// Pipeline trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTrigger {
    pub event: String,                   // e.g., "pull_request.merged"
    #[serde(default)]
    pub branches: Vec<String>,           // Branch filters
    #[serde(default)]
    pub paths: Vec<String>,              // Path filters
    #[serde(default)]
    pub labels: Vec<String>,             // Label filters
}

/// Stage configuration in pipeline definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageConfig {
    pub name: String,
    pub agent: String,                   // Agent type or ID
    pub task: String,                    // Task description for agent
    #[serde(default)]
    pub environment: Option<String>,     // Environment variables
    #[serde(default)]
    pub timeout: Option<String>,         // e.g., "30m"
    #[serde(default)]
    pub on_failure: FailureAction,
    #[serde(default)]
    pub rollback_to: Option<String>,     // Stage name to roll back to
    #[serde(default)]
    pub depends_on: Vec<String>,         // Stage dependencies
    #[serde(default)]
    pub parallel_with: Option<String>,   // Stage to run in parallel with
    #[serde(default)]
    pub when: Option<StageCondition>,    // Conditional execution
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub approvers: Vec<String>,          // Who can approve
    #[serde(default)]
    pub approval_timeout: Option<i64>,   // Minutes
    #[serde(default)]
    pub approval_quorum: Option<i64>,    // Number of approvals needed
}

/// Action to take on stage failure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FailureAction {
    Halt,
    Continue,
    Rollback,
}

impl Default for FailureAction {
    fn default() -> Self {
        FailureAction::Halt
    }
}

impl FromStr for FailureAction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "halt" => Ok(FailureAction::Halt),
            "continue" => Ok(FailureAction::Continue),
            "rollback" => Ok(FailureAction::Rollback),
            _ => Err(Error::Other(format!("Invalid failure action: {}", s))),
        }
    }
}

impl FailureAction {
    pub fn as_str(&self) -> &str {
        match self {
            FailureAction::Halt => "halt",
            FailureAction::Continue => "continue",
            FailureAction::Rollback => "rollback",
        }
    }
}

/// Conditional execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageCondition {
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub and: Vec<StageCondition>,
    #[serde(default)]
    pub or: Vec<StageCondition>,
    #[serde(default)]
    pub not: Option<Box<StageCondition>>,
}

// ==================== Pipeline Run ====================

/// Pipeline run (execution instance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: String,
    pub pipeline_id: String,
    pub status: PipelineRunStatus,
    pub trigger_event: Option<String>,
    pub trigger_data: Option<serde_json::Value>,
    pub variables: Option<HashMap<String, String>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl PipelineRun {
    pub fn new(pipeline_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pipeline_id,
            status: PipelineRunStatus::Pending,
            trigger_event: None,
            trigger_data: None,
            variables: None,
            started_at: None,
            completed_at: None,
            error_message: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_trigger(mut self, event: String, data: Option<serde_json::Value>) -> Self {
        self.trigger_event = Some(event);
        self.trigger_data = data;
        self
    }

    pub fn with_variables(mut self, variables: HashMap<String, String>) -> Self {
        self.variables = Some(variables);
        self
    }
}

/// Pipeline run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineRunStatus {
    Pending,
    Running,
    WaitingApproval,
    Succeeded,
    Failed,
    Cancelled,
}

impl FromStr for PipelineRunStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(PipelineRunStatus::Pending),
            "running" => Ok(PipelineRunStatus::Running),
            "waiting_approval" => Ok(PipelineRunStatus::WaitingApproval),
            "succeeded" => Ok(PipelineRunStatus::Succeeded),
            "failed" => Ok(PipelineRunStatus::Failed),
            "cancelled" => Ok(PipelineRunStatus::Cancelled),
            _ => Err(Error::Other(format!("Invalid pipeline run status: {}", s))),
        }
    }
}

impl PipelineRunStatus {
    pub fn as_str(&self) -> &str {
        match self {
            PipelineRunStatus::Pending => "pending",
            PipelineRunStatus::Running => "running",
            PipelineRunStatus::WaitingApproval => "waiting_approval",
            PipelineRunStatus::Succeeded => "succeeded",
            PipelineRunStatus::Failed => "failed",
            PipelineRunStatus::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineRunStatus::Succeeded
                | PipelineRunStatus::Failed
                | PipelineRunStatus::Cancelled
        )
    }
}

// ==================== Pipeline Stage ====================

/// Pipeline stage (execution step)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub id: String,
    pub run_id: String,
    pub stage_name: String,
    pub status: PipelineStageStatus,
    pub agent_id: Option<Uuid>,
    pub stage_config: Option<serde_json::Value>,
    pub output: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl PipelineStage {
    pub fn new(run_id: String, stage_name: String, config: Option<StageConfig>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            run_id,
            stage_name,
            status: PipelineStageStatus::Pending,
            agent_id: None,
            stage_config: config.and_then(|c| serde_json::to_value(c).ok()),
            output: None,
            error_message: None,
            started_at: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }
}

/// Pipeline stage status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStageStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

impl FromStr for PipelineStageStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(PipelineStageStatus::Pending),
            "running" => Ok(PipelineStageStatus::Running),
            "succeeded" => Ok(PipelineStageStatus::Succeeded),
            "failed" => Ok(PipelineStageStatus::Failed),
            "skipped" => Ok(PipelineStageStatus::Skipped),
            "cancelled" => Ok(PipelineStageStatus::Cancelled),
            _ => Err(Error::Other(format!(
                "Invalid pipeline stage status: {}",
                s
            ))),
        }
    }
}

impl PipelineStageStatus {
    pub fn as_str(&self) -> &str {
        match self {
            PipelineStageStatus::Pending => "pending",
            PipelineStageStatus::Running => "running",
            PipelineStageStatus::Succeeded => "succeeded",
            PipelineStageStatus::Failed => "failed",
            PipelineStageStatus::Skipped => "skipped",
            PipelineStageStatus::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineStageStatus::Succeeded
                | PipelineStageStatus::Failed
                | PipelineStageStatus::Skipped
                | PipelineStageStatus::Cancelled
        )
    }
}

// ==================== Pipeline Approval ====================

/// Pipeline approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineApproval {
    pub id: String,
    pub run_id: String,
    pub stage_id: String,
    pub status: ApprovalStatus,
    pub approvers: Vec<String>,
    pub required_approvals: i64,
    pub timeout_minutes: Option<i64>,
    pub default_action: Option<String>,
    pub delegated_to: Option<String>,
    pub approved_by: Option<Vec<String>>,
    pub rejected_by: Option<String>,
    pub comment: Option<String>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl PipelineApproval {
    pub fn new(
        run_id: String,
        stage_id: String,
        approvers: Vec<String>,
        required_approvals: i64,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            run_id,
            stage_id,
            status: ApprovalStatus::Pending,
            approvers,
            required_approvals,
            timeout_minutes: None,
            default_action: None,
            delegated_to: None,
            approved_by: None,
            rejected_by: None,
            comment: None,
            reason: None,
            created_at: Utc::now(),
            resolved_at: None,
            expires_at: None,
        }
    }

    pub fn with_timeout(mut self, minutes: i64, default_action: String) -> Self {
        self.timeout_minutes = Some(minutes);
        self.default_action = Some(default_action);
        self.expires_at = Some(Utc::now() + chrono::Duration::minutes(minutes));
        self
    }
}

/// Approval status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Delegated,
}

impl FromStr for ApprovalStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(ApprovalStatus::Pending),
            "approved" => Ok(ApprovalStatus::Approved),
            "rejected" => Ok(ApprovalStatus::Rejected),
            "expired" => Ok(ApprovalStatus::Expired),
            "delegated" => Ok(ApprovalStatus::Delegated),
            _ => Err(Error::Other(format!("Invalid approval status: {}", s))),
        }
    }
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected => "rejected",
            ApprovalStatus::Expired => "expired",
            ApprovalStatus::Delegated => "delegated",
        }
    }

    pub fn is_resolved(&self) -> bool {
        matches!(
            self,
            ApprovalStatus::Approved | ApprovalStatus::Rejected | ApprovalStatus::Expired
        )
    }
}

// ==================== Pipeline Rollback ====================

/// Pipeline rollback record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRollback {
    pub id: String,
    pub run_id: String,
    pub stage_id: String,
    pub rollback_target_stage_id: Option<String>,
    pub status: RollbackStatus,
    pub trigger_reason: String,
    pub agent_id: Option<Uuid>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl PipelineRollback {
    pub fn new(run_id: String, stage_id: String, reason: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            run_id,
            stage_id,
            rollback_target_stage_id: None,
            status: RollbackStatus::Pending,
            trigger_reason: reason,
            agent_id: None,
            error_message: None,
            started_at: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_target(mut self, target_stage_id: String) -> Self {
        self.rollback_target_stage_id = Some(target_stage_id);
        self
    }
}

/// Rollback status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollbackStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl FromStr for RollbackStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(RollbackStatus::Pending),
            "running" => Ok(RollbackStatus::Running),
            "succeeded" => Ok(RollbackStatus::Succeeded),
            "failed" => Ok(RollbackStatus::Failed),
            _ => Err(Error::Other(format!("Invalid rollback status: {}", s))),
        }
    }
}

impl RollbackStatus {
    pub fn as_str(&self) -> &str {
        match self {
            RollbackStatus::Pending => "pending",
            RollbackStatus::Running => "running",
            RollbackStatus::Succeeded => "succeeded",
            RollbackStatus::Failed => "failed",
        }
    }
}

// ==================== Stage Dependency ====================

/// Stage dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageDependency {
    pub stage_id: String,
    pub depends_on_stage_id: String,
    pub dependency_type: DependencyType,
    pub created_at: DateTime<Utc>,
}

/// Dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Required,
    Optional,
}

impl FromStr for DependencyType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "required" => Ok(DependencyType::Required),
            "optional" => Ok(DependencyType::Optional),
            _ => Err(Error::Other(format!("Invalid dependency type: {}", s))),
        }
    }
}

impl DependencyType {
    pub fn as_str(&self) -> &str {
        match self {
            DependencyType::Required => "required",
            DependencyType::Optional => "optional",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_parse() {
        let yaml = r#"
name: test-pipeline
description: Test pipeline
version: 1
triggers:
  - event: pull_request.merged
    branches: [main]
variables:
  environment: staging
stages:
  - name: test
    agent: tester
    task: Run tests
    on_failure: halt
"#;

        let config: PipelineConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "test-pipeline");
        assert_eq!(config.stages.len(), 1);
        assert_eq!(config.stages[0].name, "test");
    }

    #[test]
    fn test_pipeline_run_status() {
        assert_eq!(PipelineRunStatus::Pending.as_str(), "pending");
        assert!(!PipelineRunStatus::Running.is_terminal());
        assert!(PipelineRunStatus::Succeeded.is_terminal());
    }

    #[test]
    fn test_stage_status() {
        assert_eq!(PipelineStageStatus::Succeeded.as_str(), "succeeded");
        assert!(PipelineStageStatus::Failed.is_terminal());
        assert!(!PipelineStageStatus::Running.is_terminal());
    }
}
