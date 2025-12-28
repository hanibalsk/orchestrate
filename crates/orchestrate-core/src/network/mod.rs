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
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an agent in the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
