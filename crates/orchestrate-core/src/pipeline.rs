//! Pipeline event-driven workflow system
//!
//! This module handles pipeline definitions, runs, and stage execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{Error, Result};

/// Status of a pipeline run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineRunStatus {
    /// Run is pending execution
    Pending,
    /// Run is currently executing
    Running,
    /// Run is waiting for approval
    WaitingApproval,
    /// Run completed successfully
    Succeeded,
    /// Run failed
    Failed,
    /// Run was cancelled
    Cancelled,
}

impl PipelineRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for PipelineRunStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "waiting_approval" => Ok(Self::WaitingApproval),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(Error::Other(format!("Invalid pipeline run status: {}", s))),
        }
    }
}

/// Status of a pipeline stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStageStatus {
    /// Stage is pending execution
    Pending,
    /// Stage is currently executing
    Running,
    /// Stage is waiting for approval
    WaitingApproval,
    /// Stage completed successfully
    Succeeded,
    /// Stage failed
    Failed,
    /// Stage was skipped
    Skipped,
    /// Stage was cancelled
    Cancelled,
}

impl PipelineStageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for PipelineStageStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "waiting_approval" => Ok(Self::WaitingApproval),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(Error::Other(format!("Invalid pipeline stage status: {}", s))),
        }
    }
}

/// A pipeline definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Database ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Pipeline name (unique identifier)
    pub name: String,
    /// Pipeline definition in YAML format
    pub definition: String,
    /// Whether the pipeline is enabled
    pub enabled: bool,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl Pipeline {
    /// Create a new pipeline
    pub fn new(name: String, definition: String) -> Self {
        Self {
            id: None,
            name,
            definition,
            enabled: true,
            created_at: Utc::now(),
        }
    }
}

/// A pipeline run instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    /// Database ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Pipeline ID this run belongs to
    pub pipeline_id: i64,
    /// Current status of the run
    pub status: PipelineRunStatus,
    /// Event that triggered this run
    pub trigger_event: Option<String>,
    /// When the run started
    pub started_at: Option<DateTime<Utc>>,
    /// When the run completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl PipelineRun {
    /// Create a new pipeline run
    pub fn new(pipeline_id: i64, trigger_event: Option<String>) -> Self {
        Self {
            id: None,
            pipeline_id,
            status: PipelineRunStatus::Pending,
            trigger_event,
            started_at: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    /// Mark run as running
    pub fn mark_running(&mut self) {
        self.status = PipelineRunStatus::Running;
        if self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }
    }

    /// Mark run as waiting for approval
    pub fn mark_waiting_approval(&mut self) {
        self.status = PipelineRunStatus::WaitingApproval;
    }

    /// Mark run as succeeded
    pub fn mark_succeeded(&mut self) {
        self.status = PipelineRunStatus::Succeeded;
        self.completed_at = Some(Utc::now());
    }

    /// Mark run as failed
    pub fn mark_failed(&mut self) {
        self.status = PipelineRunStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark run as cancelled
    pub fn mark_cancelled(&mut self) {
        self.status = PipelineRunStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }
}

/// A stage within a pipeline run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    /// Database ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Run ID this stage belongs to
    pub run_id: i64,
    /// Stage name from pipeline definition
    pub stage_name: String,
    /// Current status of the stage
    pub status: PipelineStageStatus,
    /// Agent ID executing this stage
    pub agent_id: Option<String>,
    /// When the stage started
    pub started_at: Option<DateTime<Utc>>,
    /// When the stage completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl PipelineStage {
    /// Create a new pipeline stage
    pub fn new(run_id: i64, stage_name: String) -> Self {
        Self {
            id: None,
            run_id,
            stage_name,
            status: PipelineStageStatus::Pending,
            agent_id: None,
            started_at: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    /// Mark stage as running
    pub fn mark_running(&mut self, agent_id: Option<String>) {
        self.status = PipelineStageStatus::Running;
        self.agent_id = agent_id;
        if self.started_at.is_none() {
            self.started_at = Some(Utc::now());
        }
    }

    /// Mark stage as waiting for approval
    pub fn mark_waiting_approval(&mut self) {
        self.status = PipelineStageStatus::WaitingApproval;
    }

    /// Mark stage as succeeded
    pub fn mark_succeeded(&mut self) {
        self.status = PipelineStageStatus::Succeeded;
        self.completed_at = Some(Utc::now());
    }

    /// Mark stage as failed
    pub fn mark_failed(&mut self) {
        self.status = PipelineStageStatus::Failed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark stage as skipped
    pub fn mark_skipped(&mut self) {
        self.status = PipelineStageStatus::Skipped;
        self.completed_at = Some(Utc::now());
    }

    /// Mark stage as cancelled
    pub fn mark_cancelled(&mut self) {
        self.status = PipelineStageStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_new() {
        let pipeline = Pipeline::new(
            "test-pipeline".to_string(),
            "name: test\nstages: []".to_string(),
        );

        assert_eq!(pipeline.name, "test-pipeline");
        assert!(pipeline.enabled);
        assert!(pipeline.id.is_none());
    }

    #[test]
    fn test_pipeline_run_new() {
        let run = PipelineRun::new(1, Some("pull_request.merged".to_string()));

        assert_eq!(run.pipeline_id, 1);
        assert_eq!(run.status, PipelineRunStatus::Pending);
        assert!(run.started_at.is_none());
        assert!(run.completed_at.is_none());
    }

    #[test]
    fn test_pipeline_run_mark_running() {
        let mut run = PipelineRun::new(1, None);
        run.mark_running();

        assert_eq!(run.status, PipelineRunStatus::Running);
        assert!(run.started_at.is_some());
    }

    #[test]
    fn test_pipeline_run_mark_succeeded() {
        let mut run = PipelineRun::new(1, None);
        run.mark_succeeded();

        assert_eq!(run.status, PipelineRunStatus::Succeeded);
        assert!(run.completed_at.is_some());
    }

    #[test]
    fn test_pipeline_run_mark_failed() {
        let mut run = PipelineRun::new(1, None);
        run.mark_failed();

        assert_eq!(run.status, PipelineRunStatus::Failed);
        assert!(run.completed_at.is_some());
    }

    #[test]
    fn test_pipeline_run_mark_cancelled() {
        let mut run = PipelineRun::new(1, None);
        run.mark_cancelled();

        assert_eq!(run.status, PipelineRunStatus::Cancelled);
        assert!(run.completed_at.is_some());
    }

    #[test]
    fn test_pipeline_run_mark_waiting_approval() {
        let mut run = PipelineRun::new(1, None);
        run.mark_waiting_approval();

        assert_eq!(run.status, PipelineRunStatus::WaitingApproval);
    }

    #[test]
    fn test_pipeline_stage_new() {
        let stage = PipelineStage::new(1, "deploy".to_string());

        assert_eq!(stage.run_id, 1);
        assert_eq!(stage.stage_name, "deploy");
        assert_eq!(stage.status, PipelineStageStatus::Pending);
        assert!(stage.agent_id.is_none());
    }

    #[test]
    fn test_pipeline_stage_mark_running() {
        let mut stage = PipelineStage::new(1, "test".to_string());
        stage.mark_running(Some("agent-123".to_string()));

        assert_eq!(stage.status, PipelineStageStatus::Running);
        assert_eq!(stage.agent_id, Some("agent-123".to_string()));
        assert!(stage.started_at.is_some());
    }

    #[test]
    fn test_pipeline_stage_mark_succeeded() {
        let mut stage = PipelineStage::new(1, "test".to_string());
        stage.mark_succeeded();

        assert_eq!(stage.status, PipelineStageStatus::Succeeded);
        assert!(stage.completed_at.is_some());
    }

    #[test]
    fn test_pipeline_stage_mark_failed() {
        let mut stage = PipelineStage::new(1, "test".to_string());
        stage.mark_failed();

        assert_eq!(stage.status, PipelineStageStatus::Failed);
        assert!(stage.completed_at.is_some());
    }

    #[test]
    fn test_pipeline_stage_mark_skipped() {
        let mut stage = PipelineStage::new(1, "test".to_string());
        stage.mark_skipped();

        assert_eq!(stage.status, PipelineStageStatus::Skipped);
        assert!(stage.completed_at.is_some());
    }

    #[test]
    fn test_pipeline_stage_mark_cancelled() {
        let mut stage = PipelineStage::new(1, "test".to_string());
        stage.mark_cancelled();

        assert_eq!(stage.status, PipelineStageStatus::Cancelled);
        assert!(stage.completed_at.is_some());
    }

    #[test]
    fn test_pipeline_run_status_parsing() {
        assert_eq!(
            PipelineRunStatus::from_str("pending").unwrap(),
            PipelineRunStatus::Pending
        );
        assert_eq!(
            PipelineRunStatus::from_str("running").unwrap(),
            PipelineRunStatus::Running
        );
        assert_eq!(
            PipelineRunStatus::from_str("waiting_approval").unwrap(),
            PipelineRunStatus::WaitingApproval
        );
        assert_eq!(
            PipelineRunStatus::from_str("succeeded").unwrap(),
            PipelineRunStatus::Succeeded
        );
        assert_eq!(
            PipelineRunStatus::from_str("failed").unwrap(),
            PipelineRunStatus::Failed
        );
        assert_eq!(
            PipelineRunStatus::from_str("cancelled").unwrap(),
            PipelineRunStatus::Cancelled
        );

        assert!(PipelineRunStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_pipeline_run_status_as_str() {
        assert_eq!(PipelineRunStatus::Pending.as_str(), "pending");
        assert_eq!(PipelineRunStatus::Running.as_str(), "running");
        assert_eq!(PipelineRunStatus::WaitingApproval.as_str(), "waiting_approval");
        assert_eq!(PipelineRunStatus::Succeeded.as_str(), "succeeded");
        assert_eq!(PipelineRunStatus::Failed.as_str(), "failed");
        assert_eq!(PipelineRunStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_pipeline_stage_status_parsing() {
        assert_eq!(
            PipelineStageStatus::from_str("pending").unwrap(),
            PipelineStageStatus::Pending
        );
        assert_eq!(
            PipelineStageStatus::from_str("running").unwrap(),
            PipelineStageStatus::Running
        );
        assert_eq!(
            PipelineStageStatus::from_str("waiting_approval").unwrap(),
            PipelineStageStatus::WaitingApproval
        );
        assert_eq!(
            PipelineStageStatus::from_str("succeeded").unwrap(),
            PipelineStageStatus::Succeeded
        );
        assert_eq!(
            PipelineStageStatus::from_str("failed").unwrap(),
            PipelineStageStatus::Failed
        );
        assert_eq!(
            PipelineStageStatus::from_str("skipped").unwrap(),
            PipelineStageStatus::Skipped
        );
        assert_eq!(
            PipelineStageStatus::from_str("cancelled").unwrap(),
            PipelineStageStatus::Cancelled
        );

        assert!(PipelineStageStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_pipeline_stage_status_as_str() {
        assert_eq!(PipelineStageStatus::Pending.as_str(), "pending");
        assert_eq!(PipelineStageStatus::Running.as_str(), "running");
        assert_eq!(PipelineStageStatus::WaitingApproval.as_str(), "waiting_approval");
        assert_eq!(PipelineStageStatus::Succeeded.as_str(), "succeeded");
        assert_eq!(PipelineStageStatus::Failed.as_str(), "failed");
        assert_eq!(PipelineStageStatus::Skipped.as_str(), "skipped");
        assert_eq!(PipelineStageStatus::Cancelled.as_str(), "cancelled");
    }
}
