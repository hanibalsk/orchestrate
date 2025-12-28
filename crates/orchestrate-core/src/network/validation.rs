//! Network validation for compile-time and runtime checks
//!
//! This module provides validation capabilities:
//! - Runtime state invariant checks
//! - Dependency satisfaction verification
//! - Network consistency validation

use super::{AgentHandle, AgentId, DependencyGraph};
use crate::AgentState;
use std::collections::HashMap;

/// Result of network validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the network is valid
    pub is_valid: bool,
    /// List of validation errors
    pub errors: Vec<ValidationError>,
    /// List of warnings (non-fatal issues)
    pub warnings: Vec<ValidationWarning>,
    /// Timestamp of validation
    pub validated_at: chrono::DateTime<chrono::Utc>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            validated_at: chrono::Utc::now(),
        }
    }

    /// Create a failed validation result
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
            validated_at: chrono::Utc::now(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: ValidationWarning) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Add warnings
    pub fn with_warnings(mut self, warnings: Vec<ValidationWarning>) -> Self {
        self.warnings.extend(warnings);
        self
    }
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error code
    pub code: ValidationErrorCode,
    /// Affected agent (if applicable)
    pub agent_id: Option<AgentId>,
    /// Human-readable message
    pub message: String,
    /// Suggested fix
    pub suggestion: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(code: ValidationErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            agent_id: None,
            message: message.into(),
            suggestion: None,
        }
    }

    /// Set the affected agent
    pub fn for_agent(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Set a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Error codes for validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationErrorCode {
    /// Dependency cycle detected
    CyclicDependency,
    /// Required dependency missing
    MissingDependency,
    /// Agent in invalid state
    InvalidState,
    /// State transition not allowed
    InvalidTransition,
    /// Dependency in wrong state
    DependencyStateInvalid,
    /// Orphaned agent (no connections)
    OrphanedAgent,
    /// Conflicting states
    ConflictingStates,
    /// Timeout exceeded
    TimeoutExceeded,
    /// Skill requirements not met
    SkillRequirementsNotMet,
}

/// Validation warning (non-fatal issue)
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning code
    pub code: ValidationWarningCode,
    /// Affected agent (if applicable)
    pub agent_id: Option<AgentId>,
    /// Human-readable message
    pub message: String,
}

impl ValidationWarning {
    /// Create a new warning
    pub fn new(code: ValidationWarningCode, message: impl Into<String>) -> Self {
        Self {
            code,
            agent_id: None,
            message: message.into(),
        }
    }

    /// Set the affected agent
    pub fn for_agent(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
}

/// Warning codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationWarningCode {
    /// Agent has been running for a long time
    LongRunningAgent,
    /// Agent has no dependents
    NoDependents,
    /// Agent has many dependencies
    ManyDependencies,
    /// State unchanged for a long time
    StaleState,
}

/// Network validator
#[derive(Debug)]
pub struct NetworkValidator {
    /// Maximum allowed running time before warning (seconds)
    pub max_running_time_secs: u64,
    /// Maximum allowed dependencies before warning
    pub max_dependencies: usize,
    /// Stale state threshold (seconds)
    pub stale_threshold_secs: u64,
}

impl Default for NetworkValidator {
    fn default() -> Self {
        Self {
            max_running_time_secs: 3600, // 1 hour
            max_dependencies: 10,
            stale_threshold_secs: 1800, // 30 minutes
        }
    }
}

impl NetworkValidator {
    /// Create a new validator with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate the entire network
    pub fn validate(
        &self,
        agents: &HashMap<AgentId, AgentHandle>,
        dependency_graph: &DependencyGraph,
    ) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check for cycles in dependency graph
        if let Err(_) = dependency_graph.topological_order() {
            errors.push(ValidationError::new(
                ValidationErrorCode::CyclicDependency,
                "Dependency graph contains cycles",
            ));
        }

        // Validate each agent
        for (agent_id, agent) in agents {
            // Check dependencies exist
            for dep_id in &agent.dependencies {
                if !agents.contains_key(dep_id) {
                    errors.push(
                        ValidationError::new(
                            ValidationErrorCode::MissingDependency,
                            format!("Agent {} depends on non-existent agent {}", agent_id, dep_id),
                        )
                        .for_agent(*agent_id),
                    );
                }
            }

            // Check for orphaned agents (no dependencies and no dependents)
            if agent.dependencies.is_empty() && agent.dependents.is_empty() {
                warnings.push(
                    ValidationWarning::new(
                        ValidationWarningCode::NoDependents,
                        format!("Agent {} has no connections in the network", agent_id),
                    )
                    .for_agent(*agent_id),
                );
            }

            // Check for too many dependencies
            if agent.dependencies.len() > self.max_dependencies {
                warnings.push(
                    ValidationWarning::new(
                        ValidationWarningCode::ManyDependencies,
                        format!(
                            "Agent {} has {} dependencies (threshold: {})",
                            agent_id,
                            agent.dependencies.len(),
                            self.max_dependencies
                        ),
                    )
                    .for_agent(*agent_id),
                );
            }

            // Validate state consistency
            self.validate_agent_state(agent, agents, &mut errors, &mut warnings);
        }

        if errors.is_empty() {
            ValidationResult::success().with_warnings(warnings)
        } else {
            ValidationResult::failure(errors).with_warnings(warnings)
        }
    }

    /// Validate a single agent's state
    fn validate_agent_state(
        &self,
        agent: &AgentHandle,
        all_agents: &HashMap<AgentId, AgentHandle>,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut Vec<ValidationWarning>,
    ) {
        // Check that running agents have their dependencies satisfied
        if agent.state == AgentState::Running {
            for dep_id in &agent.dependencies {
                if let Some(dep) = all_agents.get(dep_id) {
                    // Dependency should not be in a failed state
                    if dep.state == AgentState::Failed {
                        errors.push(
                            ValidationError::new(
                                ValidationErrorCode::DependencyStateInvalid,
                                format!(
                                    "Agent {} is running but dependency {} has failed",
                                    agent.id, dep_id
                                ),
                            )
                            .for_agent(agent.id)
                            .with_suggestion(format!("Pause agent {} or restart dependency {}", agent.id, dep_id)),
                        );
                    }
                    // Dependency should not be terminated
                    if dep.state == AgentState::Terminated {
                        errors.push(
                            ValidationError::new(
                                ValidationErrorCode::DependencyStateInvalid,
                                format!(
                                    "Agent {} is running but dependency {} was terminated",
                                    agent.id, dep_id
                                ),
                            )
                            .for_agent(agent.id)
                            .with_suggestion(format!("Terminate agent {} or restart dependency {}", agent.id, dep_id)),
                        );
                    }
                }
            }
        }

        // Check for conflicting states among dependents
        if agent.state == AgentState::Completed {
            for dep_id in &agent.dependents {
                if let Some(dep) = all_agents.get(dep_id) {
                    // If we're completed, dependents should not still be in Created state
                    if dep.state == AgentState::Created {
                        // This is not an error, just a note that propagation might be needed
                    }
                }
            }
        }
    }

    /// Validate a proposed state transition
    pub fn validate_transition(
        &self,
        agent: &AgentHandle,
        new_state: AgentState,
        dependency_states: &HashMap<AgentId, AgentState>,
    ) -> Result<(), ValidationError> {
        // Check that dependencies are in valid states for the transition
        match new_state {
            AgentState::Running => {
                // Can only run if no dependencies are failed/terminated
                for dep_id in &agent.dependencies {
                    if let Some(state) = dependency_states.get(dep_id) {
                        if *state == AgentState::Failed || *state == AgentState::Terminated {
                            return Err(
                                ValidationError::new(
                                    ValidationErrorCode::DependencyStateInvalid,
                                    format!("Cannot run: dependency {} is in state {:?}", dep_id, state),
                                )
                                .for_agent(agent.id),
                            );
                        }
                    }
                }
            }
            AgentState::Completed => {
                // Can only complete if currently running
                if agent.state != AgentState::Running {
                    return Err(
                        ValidationError::new(
                            ValidationErrorCode::InvalidTransition,
                            format!("Cannot complete from state {:?}", agent.state),
                        )
                        .for_agent(agent.id),
                    );
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Invariant that must hold in the network
pub trait NetworkInvariant: Send + Sync {
    /// Check if the invariant holds
    fn check(
        &self,
        agents: &HashMap<AgentId, AgentHandle>,
        dependency_graph: &DependencyGraph,
    ) -> Result<(), ValidationError>;

    /// Get a description of this invariant
    fn description(&self) -> &str;
}

/// Invariant: No cycles in dependency graph
pub struct NoCyclesInvariant;

impl NetworkInvariant for NoCyclesInvariant {
    fn check(
        &self,
        _agents: &HashMap<AgentId, AgentHandle>,
        dependency_graph: &DependencyGraph,
    ) -> Result<(), ValidationError> {
        dependency_graph
            .topological_order()
            .map(|_| ())
            .map_err(|_| {
                ValidationError::new(ValidationErrorCode::CyclicDependency, "Dependency cycle detected")
            })
    }

    fn description(&self) -> &str {
        "No cycles in dependency graph"
    }
}

/// Invariant: Running agents have no failed dependencies
pub struct NoFailedDependenciesInvariant;

impl NetworkInvariant for NoFailedDependenciesInvariant {
    fn check(
        &self,
        agents: &HashMap<AgentId, AgentHandle>,
        _dependency_graph: &DependencyGraph,
    ) -> Result<(), ValidationError> {
        for (agent_id, agent) in agents {
            if agent.state == AgentState::Running {
                for dep_id in &agent.dependencies {
                    if let Some(dep) = agents.get(dep_id) {
                        if dep.state == AgentState::Failed {
                            return Err(
                                ValidationError::new(
                                    ValidationErrorCode::DependencyStateInvalid,
                                    format!("Running agent {} has failed dependency {}", agent_id, dep_id),
                                )
                                .for_agent(*agent_id),
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Running agents have no failed dependencies"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AgentType;

    #[test]
    fn test_validation_success() {
        let mut agents = HashMap::new();
        let mut graph = DependencyGraph::new();

        let a = AgentId::new();
        let b = AgentId::new();

        agents.insert(a, AgentHandle::new(a, AgentType::StoryDeveloper, AgentState::Running));
        agents.insert(b, AgentHandle::new(b, AgentType::CodeReviewer, AgentState::Running));

        graph.register_agent(a, AgentType::StoryDeveloper);
        graph.register_agent(b, AgentType::CodeReviewer);
        graph.add_dependency(b, a).unwrap(); // b depends on a

        // Update handles with dependency info
        agents.get_mut(&b).unwrap().add_dependency(a);
        agents.get_mut(&a).unwrap().add_dependent(b);

        let validator = NetworkValidator::new();
        let result = validator.validate(&agents, &graph);

        assert!(result.is_valid);
    }

    #[test]
    fn test_validation_failed_dependency() {
        let mut agents = HashMap::new();
        let mut graph = DependencyGraph::new();

        let a = AgentId::new();
        let b = AgentId::new();

        // a is failed, b is running
        agents.insert(a, AgentHandle::new(a, AgentType::StoryDeveloper, AgentState::Failed));
        let mut b_handle = AgentHandle::new(b, AgentType::CodeReviewer, AgentState::Running);
        b_handle.add_dependency(a);
        agents.insert(b, b_handle);

        graph.register_agent(a, AgentType::StoryDeveloper);
        graph.register_agent(b, AgentType::CodeReviewer);
        graph.add_dependency(b, a).unwrap();

        let validator = NetworkValidator::new();
        let result = validator.validate(&agents, &graph);

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.code == ValidationErrorCode::DependencyStateInvalid));
    }
}
