//! Agent Network System
//!
//! This module provides a network of agents where states and skills are
//! interdependent across agents, enabling full automation with validation.
//!
//! ## Features
//!
//! - **Dependency-based visibility**: Agents only see states of agents they depend on
//! - **Compile-time validation**: Rust traits enforce capability contracts
//! - **Runtime validation**: NetworkCoordinator ensures consistency
//! - **Auto propagation**: State changes flow through dependency graph
//! - **Self-healing**: Automatic recovery from invalid states

pub mod coordinator;
pub mod dependency;
pub mod skills;
pub mod state;
pub mod validation;

pub use coordinator::{NetworkCoordinator, NetworkEvent, RecoveryAction};
pub use dependency::{DependencyCondition, DependencyGraph, DependencySet};
pub use skills::{Skill, SkillDefinition, SkillRegistry};
pub use state::{StateGraph, StateMachine, StatePropagation, StateTransition};
pub use validation::{NetworkValidator, ValidationError, ValidationResult};

use crate::AgentType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Unique identifier for an agent in the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    /// Create a new random agent ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// State requirement for a dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRequirement {
    /// The type of agent required
    pub agent_type: AgentType,
    /// The required state(s)
    pub required_states: Vec<crate::AgentState>,
    /// Whether this requirement is optional
    pub optional: bool,
}

impl StateRequirement {
    /// Create a new required state requirement
    pub fn required(agent_type: AgentType, states: Vec<crate::AgentState>) -> Self {
        Self {
            agent_type,
            required_states: states,
            optional: false,
        }
    }

    /// Create a new optional state requirement
    pub fn optional(agent_type: AgentType, states: Vec<crate::AgentState>) -> Self {
        Self {
            agent_type,
            required_states: states,
            optional: true,
        }
    }
}

/// Handle to an agent in the network
#[derive(Debug, Clone)]
pub struct AgentHandle {
    /// The agent's unique identifier
    pub id: AgentId,
    /// The agent's type
    pub agent_type: AgentType,
    /// Current state
    pub state: crate::AgentState,
    /// Agents this agent depends on
    pub dependencies: Vec<AgentId>,
    /// Agents that depend on this agent
    pub dependents: Vec<AgentId>,
}

impl AgentHandle {
    /// Create a new agent handle
    pub fn new(id: AgentId, agent_type: AgentType, state: crate::AgentState) -> Self {
        Self {
            id,
            agent_type,
            state,
            dependencies: Vec::new(),
            dependents: Vec::new(),
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, agent_id: AgentId) {
        if !self.dependencies.contains(&agent_id) {
            self.dependencies.push(agent_id);
        }
    }

    /// Add a dependent
    pub fn add_dependent(&mut self, agent_id: AgentId) {
        if !self.dependents.contains(&agent_id) {
            self.dependents.push(agent_id);
        }
    }

    /// Check if this agent can observe another agent
    pub fn can_observe(&self, other_id: AgentId) -> bool {
        self.dependencies.contains(&other_id)
    }
}

/// Trait for types that can act as agent capabilities
pub trait AgentCapability: Send + Sync {
    /// Get the agent type
    fn agent_type(&self) -> AgentType;

    /// Get the list of agent types this depends on
    fn dependencies(&self) -> Vec<AgentType>;

    /// Get the available skills
    fn skills(&self) -> Vec<&'static str>;

    /// Check if a state transition is valid
    fn can_transition(&self, from: crate::AgentState, to: crate::AgentState) -> bool;
}

/// Trait for defining state transition rules
pub trait StateTransitions {
    /// Get all valid state transitions
    fn transitions() -> Vec<StateTransition>;

    /// Get the initial state
    fn initial_state() -> crate::AgentState;

    /// Get all terminal states
    fn terminal_states() -> Vec<crate::AgentState>;
}

/// Trait for dependency visibility control
pub trait DependencyVisibility {
    /// Check if this agent type can observe another agent type
    fn can_see(agent_type: AgentType, other: AgentType) -> bool;

    /// Get all agent types this can observe
    fn observable_types(agent_type: AgentType) -> Vec<AgentType>;
}

// ==================== Step Output Types ====================

/// Type of output produced by a workflow step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutputType {
    /// Direct skill execution result
    SkillResult,
    /// State change metadata
    StateTransition,
    /// File or resource created
    Artifact,
    /// Error information
    Error,
}

impl StepOutputType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            StepOutputType::SkillResult => "skill_result",
            StepOutputType::StateTransition => "state_transition",
            StepOutputType::Artifact => "artifact",
            StepOutputType::Error => "error",
        }
    }
}

impl fmt::Display for StepOutputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for StepOutputType {
    type Err = crate::Error;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "skill_result" => Ok(StepOutputType::SkillResult),
            "state_transition" => Ok(StepOutputType::StateTransition),
            "artifact" => Ok(StepOutputType::Artifact),
            "error" => Ok(StepOutputType::Error),
            _ => Err(crate::Error::Other(format!(
                "Unknown step output type: {}",
                s
            ))),
        }
    }
}

/// Maximum allowed size for step output data payload (1MB)
pub const MAX_STEP_OUTPUT_DATA_SIZE: usize = 1024 * 1024;

/// Output from a workflow step that can be consumed by dependent agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    /// Unique output ID (from database, None if not yet persisted)
    pub id: Option<i64>,
    /// Agent that produced this output
    pub agent_id: AgentId,
    /// Skill that produced this output
    pub skill_name: String,
    /// Type of output
    pub output_type: StepOutputType,
    /// Output data (JSON)
    pub data: serde_json::Value,
    /// Whether this output has been consumed
    pub consumed: bool,
    /// Agent that consumed this output (if any)
    pub consumed_by: Option<AgentId>,
    /// When the output was consumed (if any)
    pub consumed_at: Option<DateTime<Utc>>,
    /// When the output was created
    pub created_at: DateTime<Utc>,
}

impl StepOutput {
    /// Create a new step output (without database ID)
    ///
    /// Returns an error if the data payload exceeds MAX_STEP_OUTPUT_DATA_SIZE.
    pub fn new(
        agent_id: AgentId,
        skill_name: impl Into<String>,
        output_type: StepOutputType,
        data: serde_json::Value,
    ) -> crate::Result<Self> {
        // Validate data size to prevent unbounded storage
        let data_size = serde_json::to_string(&data).map(|s| s.len()).unwrap_or(0);
        if data_size > MAX_STEP_OUTPUT_DATA_SIZE {
            return Err(crate::Error::Other(format!(
                "Step output data size {} exceeds maximum of {} bytes",
                data_size, MAX_STEP_OUTPUT_DATA_SIZE
            )));
        }

        Ok(Self {
            id: None, // Will be set by database
            agent_id,
            skill_name: skill_name.into(),
            output_type,
            data,
            consumed: false,
            consumed_by: None,
            consumed_at: None,
            created_at: Utc::now(),
        })
    }

    /// Create a skill result output
    pub fn skill_result(
        agent_id: AgentId,
        skill_name: impl Into<String>,
        data: serde_json::Value,
    ) -> crate::Result<Self> {
        Self::new(agent_id, skill_name, StepOutputType::SkillResult, data)
    }

    /// Create an error output
    pub fn error(
        agent_id: AgentId,
        skill_name: impl Into<String>,
        error_message: impl Into<String>,
    ) -> crate::Result<Self> {
        Self::new(
            agent_id,
            skill_name,
            StepOutputType::Error,
            serde_json::json!({ "error": error_message.into() }),
        )
    }

    /// Create an artifact output
    pub fn artifact(
        agent_id: AgentId,
        skill_name: impl Into<String>,
        artifact_path: impl Into<String>,
        metadata: serde_json::Value,
    ) -> crate::Result<Self> {
        Self::new(
            agent_id,
            skill_name,
            StepOutputType::Artifact,
            serde_json::json!({
                "path": artifact_path.into(),
                "metadata": metadata,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_output_creation() {
        let agent_id = AgentId::new();

        // Test skill result creation
        let output = StepOutput::skill_result(
            agent_id,
            "develop",
            serde_json::json!({"files_changed": 5, "tests_passed": true}),
        )
        .unwrap();

        assert_eq!(output.agent_id, agent_id);
        assert_eq!(output.skill_name, "develop");
        assert_eq!(output.output_type, StepOutputType::SkillResult);
        assert!(!output.consumed);
        assert!(output.consumed_by.is_none());
        assert!(output.consumed_at.is_none());
        assert!(output.id.is_none());
    }

    #[test]
    fn test_step_output_error() {
        let agent_id = AgentId::new();

        let output = StepOutput::error(agent_id, "test", "Tests failed").unwrap();

        assert_eq!(output.output_type, StepOutputType::Error);
        assert_eq!(output.data["error"], "Tests failed");
    }

    #[test]
    fn test_step_output_artifact() {
        let agent_id = AgentId::new();

        let output = StepOutput::artifact(
            agent_id,
            "build",
            "/path/to/artifact.zip",
            serde_json::json!({"size": 1024}),
        )
        .unwrap();

        assert_eq!(output.output_type, StepOutputType::Artifact);
        assert_eq!(output.data["path"], "/path/to/artifact.zip");
    }

    #[test]
    fn test_step_output_type_conversion() {
        assert_eq!(StepOutputType::SkillResult.as_str(), "skill_result");
        assert_eq!(
            "skill_result".parse::<StepOutputType>().unwrap(),
            StepOutputType::SkillResult
        );
        assert_eq!(
            "artifact".parse::<StepOutputType>().unwrap(),
            StepOutputType::Artifact
        );
        assert!("invalid".parse::<StepOutputType>().is_err());
    }

    #[test]
    fn test_step_output_type_display() {
        assert_eq!(format!("{}", StepOutputType::SkillResult), "skill_result");
        assert_eq!(format!("{}", StepOutputType::Error), "error");
    }

    #[test]
    fn test_step_output_data_size_validation() {
        let agent_id = AgentId::new();

        // Create data larger than MAX_STEP_OUTPUT_DATA_SIZE
        let large_string = "x".repeat(MAX_STEP_OUTPUT_DATA_SIZE + 1);
        let large_data = serde_json::json!({ "data": large_string });

        let result = StepOutput::new(agent_id, "test", StepOutputType::SkillResult, large_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_id_serialization() {
        let agent_id = AgentId::new();

        // Test serialization
        let json = serde_json::to_string(&agent_id).unwrap();
        assert!(json.contains(&agent_id.0.to_string()));

        // Test deserialization
        let deserialized: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, agent_id);
    }
}
