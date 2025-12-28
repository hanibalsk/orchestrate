//! State machine with dependency-aware transitions
//!
//! This module provides state transition management that respects
//! dependencies between agents in the network.

use super::{AgentId, DependencyCondition};
use crate::{AgentState, AgentType};
use std::collections::{HashMap, HashSet};

/// A state transition definition
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// Source state
    pub from: AgentState,
    /// Target state
    pub to: AgentState,
    /// Required conditions from dependencies
    pub requires: Vec<DependencyCondition>,
    /// State changes to propagate after this transition
    pub propagates: Vec<StatePropagation>,
    /// Optional guard condition name (for runtime evaluation)
    pub guard: Option<String>,
}

impl StateTransition {
    /// Create a new state transition
    pub fn new(from: AgentState, to: AgentState) -> Self {
        Self {
            from,
            to,
            requires: Vec::new(),
            propagates: Vec::new(),
            guard: None,
        }
    }

    /// Add a dependency requirement
    pub fn when(mut self, condition: DependencyCondition) -> Self {
        self.requires.push(condition);
        self
    }

    /// Add a state propagation
    pub fn propagate(mut self, propagation: StatePropagation) -> Self {
        self.propagates.push(propagation);
        self
    }

    /// Add a guard condition
    pub fn with_guard(mut self, guard: impl Into<String>) -> Self {
        self.guard = Some(guard.into());
        self
    }

    /// Check if this transition can be taken given current dependency states
    pub fn can_take(&self, dependency_states: &HashMap<AgentId, (AgentType, AgentState)>) -> bool {
        self.requires.iter().all(|cond| cond.is_satisfied(dependency_states))
    }
}

/// State propagation to downstream agents
#[derive(Debug, Clone)]
pub struct StatePropagation {
    /// Target agent type to notify
    pub target_type: AgentType,
    /// Event to send
    pub event: PropagationEvent,
}

impl StatePropagation {
    /// Create a propagation that signals state change
    pub fn signal(target_type: AgentType, event: PropagationEvent) -> Self {
        Self { target_type, event }
    }
}

/// Events that can be propagated to dependent agents
#[derive(Debug, Clone)]
pub enum PropagationEvent {
    /// Dependency is now ready
    DependencyReady,
    /// Dependency completed successfully
    DependencyCompleted,
    /// Dependency failed
    DependencyFailed,
    /// Dependency was blocked
    DependencyBlocked,
    /// Custom event
    Custom(String),
}

/// State graph representing all possible states and transitions
#[derive(Debug, Clone)]
pub struct StateGraph {
    /// All transitions indexed by (from_state)
    transitions: HashMap<AgentState, Vec<StateTransition>>,
    /// Initial state
    initial: AgentState,
    /// Terminal states
    terminals: HashSet<AgentState>,
}

impl StateGraph {
    /// Create a new state graph
    pub fn new(initial: AgentState) -> Self {
        Self {
            transitions: HashMap::new(),
            initial,
            terminals: HashSet::new(),
        }
    }

    /// Add a transition
    pub fn add_transition(&mut self, transition: StateTransition) {
        self.transitions
            .entry(transition.from)
            .or_default()
            .push(transition);
    }

    /// Mark a state as terminal
    pub fn add_terminal(&mut self, state: AgentState) {
        self.terminals.insert(state);
    }

    /// Get the initial state
    pub fn initial_state(&self) -> AgentState {
        self.initial
    }

    /// Check if a state is terminal
    pub fn is_terminal(&self, state: AgentState) -> bool {
        self.terminals.contains(&state)
    }

    /// Get all possible transitions from a state
    pub fn transitions_from(&self, state: AgentState) -> &[StateTransition] {
        self.transitions.get(&state).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Find a valid transition from one state to another
    pub fn find_transition(
        &self,
        from: AgentState,
        to: AgentState,
        dependency_states: &HashMap<AgentId, (AgentType, AgentState)>,
    ) -> Option<&StateTransition> {
        self.transitions_from(from)
            .iter()
            .find(|t| t.to == to && t.can_take(dependency_states))
    }

    /// Get all reachable states from current state
    pub fn reachable_states(
        &self,
        from: AgentState,
        dependency_states: &HashMap<AgentId, (AgentType, AgentState)>,
    ) -> Vec<AgentState> {
        self.transitions_from(from)
            .iter()
            .filter(|t| t.can_take(dependency_states))
            .map(|t| t.to)
            .collect()
    }
}

/// State machine for a single agent
#[derive(Debug)]
pub struct StateMachine {
    /// Current state
    current: AgentState,
    /// State graph defining valid transitions
    graph: StateGraph,
    /// History of state transitions
    history: Vec<StateTransitionRecord>,
}

/// Record of a state transition
#[derive(Debug, Clone)]
pub struct StateTransitionRecord {
    /// Previous state
    pub from: AgentState,
    /// New state
    pub to: AgentState,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Trigger event (if any)
    pub trigger: Option<String>,
}

impl StateMachine {
    /// Create a new state machine
    pub fn new(graph: StateGraph) -> Self {
        let initial = graph.initial_state();
        Self {
            current: initial,
            graph,
            history: Vec::new(),
        }
    }

    /// Get current state
    pub fn current_state(&self) -> AgentState {
        self.current
    }

    /// Check if the machine is in a terminal state
    pub fn is_terminal(&self) -> bool {
        self.graph.is_terminal(self.current)
    }

    /// Attempt a state transition
    pub fn transition(
        &mut self,
        to: AgentState,
        dependency_states: &HashMap<AgentId, (AgentType, AgentState)>,
        trigger: Option<String>,
    ) -> Result<Vec<StatePropagation>, StateMachineError> {
        // Check if transition exists and is valid
        let transition = self
            .graph
            .find_transition(self.current, to, dependency_states)
            .ok_or_else(|| StateMachineError::InvalidTransition {
                from: self.current,
                to,
            })?;

        // Record the transition
        let record = StateTransitionRecord {
            from: self.current,
            to,
            timestamp: chrono::Utc::now(),
            trigger,
        };
        self.history.push(record);

        // Update state
        let propagations = transition.propagates.clone();
        self.current = to;

        Ok(propagations)
    }

    /// Get transition history
    pub fn history(&self) -> &[StateTransitionRecord] {
        &self.history
    }

    /// Get available transitions from current state
    pub fn available_transitions(
        &self,
        dependency_states: &HashMap<AgentId, (AgentType, AgentState)>,
    ) -> Vec<AgentState> {
        self.graph.reachable_states(self.current, dependency_states)
    }
}

/// Errors from state machine operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum StateMachineError {
    #[error("Invalid transition from {from:?} to {to:?}")]
    InvalidTransition { from: AgentState, to: AgentState },

    #[error("Dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),

    #[error("Guard condition failed: {0}")]
    GuardFailed(String),
}

/// Builder for creating state graphs
pub struct StateGraphBuilder {
    graph: StateGraph,
}

impl StateGraphBuilder {
    /// Create a new builder with initial state
    pub fn new(initial: AgentState) -> Self {
        Self {
            graph: StateGraph::new(initial),
        }
    }

    /// Add a simple transition (no conditions)
    pub fn transition(mut self, from: AgentState, to: AgentState) -> Self {
        self.graph.add_transition(StateTransition::new(from, to));
        self
    }

    /// Add a conditional transition
    pub fn conditional_transition(mut self, transition: StateTransition) -> Self {
        self.graph.add_transition(transition);
        self
    }

    /// Mark a state as terminal
    pub fn terminal(mut self, state: AgentState) -> Self {
        self.graph.add_terminal(state);
        self
    }

    /// Build the state graph
    pub fn build(self) -> StateGraph {
        self.graph
    }
}

/// Create a default state graph for agents
pub fn default_agent_state_graph() -> StateGraph {
    StateGraphBuilder::new(AgentState::Created)
        // Normal flow
        .transition(AgentState::Created, AgentState::Initializing)
        .transition(AgentState::Initializing, AgentState::Running)
        .transition(AgentState::Running, AgentState::Completed)
        // Pause/resume
        .transition(AgentState::Running, AgentState::Paused)
        .transition(AgentState::Paused, AgentState::Running)
        // Failure paths
        .transition(AgentState::Running, AgentState::Failed)
        .transition(AgentState::Initializing, AgentState::Failed)
        // Termination (from any active state)
        .transition(AgentState::Running, AgentState::Terminated)
        .transition(AgentState::Paused, AgentState::Terminated)
        .transition(AgentState::Initializing, AgentState::Terminated)
        // Terminal states
        .terminal(AgentState::Completed)
        .terminal(AgentState::Failed)
        .terminal(AgentState::Terminated)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transition() {
        let graph = default_agent_state_graph();
        let mut machine = StateMachine::new(graph);

        assert_eq!(machine.current_state(), AgentState::Created);

        // Transition to initializing
        let deps = HashMap::new();
        machine
            .transition(AgentState::Initializing, &deps, None)
            .unwrap();
        assert_eq!(machine.current_state(), AgentState::Initializing);

        // Transition to running
        machine
            .transition(AgentState::Running, &deps, None)
            .unwrap();
        assert_eq!(machine.current_state(), AgentState::Running);

        // Transition to completed
        machine
            .transition(AgentState::Completed, &deps, None)
            .unwrap();
        assert_eq!(machine.current_state(), AgentState::Completed);
        assert!(machine.is_terminal());
    }

    #[test]
    fn test_invalid_transition() {
        let graph = default_agent_state_graph();
        let mut machine = StateMachine::new(graph);

        let deps = HashMap::new();
        // Can't go directly from Created to Running
        let result = machine.transition(AgentState::Running, &deps, None);
        assert!(result.is_err());
    }
}
