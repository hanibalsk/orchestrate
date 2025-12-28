//! Dependency management for the agent network
//!
//! This module handles dependencies between agents, including:
//! - Dependency conditions for state transitions
//! - Dependency graph for tracking relationships
//! - Visibility control based on dependencies

use super::AgentId;
use crate::{AgentState, AgentType};
use std::collections::{HashMap, HashSet, VecDeque};

/// Condition that must be satisfied by dependencies
#[derive(Debug, Clone)]
pub enum DependencyCondition {
    /// A specific agent must be in a specific state
    AgentInState {
        agent_id: AgentId,
        state: AgentState,
    },
    /// All agents of a type must be in a specific state
    AllOfType {
        agent_type: AgentType,
        state: AgentState,
    },
    /// Any agent of a type must be in one of the specified states
    AnyOfType {
        agent_type: AgentType,
        states: Vec<AgentState>,
    },
    /// At least N agents of a type must be in the specified state
    AtLeastN {
        agent_type: AgentType,
        state: AgentState,
        count: usize,
    },
    /// No agents of a type are in the specified state
    NoneOfType {
        agent_type: AgentType,
        state: AgentState,
    },
    /// Compound condition: all conditions must be true
    And(Vec<DependencyCondition>),
    /// Compound condition: at least one condition must be true
    Or(Vec<DependencyCondition>),
}

impl DependencyCondition {
    /// Check if this condition is satisfied given current dependency states
    pub fn is_satisfied(&self, states: &HashMap<AgentId, (AgentType, AgentState)>) -> bool {
        match self {
            DependencyCondition::AgentInState { agent_id, state } => {
                states.get(agent_id).map(|(_, s)| s == state).unwrap_or(false)
            }
            DependencyCondition::AllOfType { agent_type, state } => {
                let matching: Vec<_> = states
                    .values()
                    .filter(|(t, _)| t == agent_type)
                    .collect();
                !matching.is_empty() && matching.iter().all(|(_, s)| s == state)
            }
            DependencyCondition::AnyOfType { agent_type, states: required_states } => {
                states
                    .values()
                    .filter(|(t, _)| t == agent_type)
                    .any(|(_, s)| required_states.contains(s))
            }
            DependencyCondition::AtLeastN { agent_type, state, count } => {
                states
                    .values()
                    .filter(|(t, s)| t == agent_type && s == state)
                    .count()
                    >= *count
            }
            DependencyCondition::NoneOfType { agent_type, state } => {
                !states
                    .values()
                    .any(|(t, s)| t == agent_type && s == state)
            }
            DependencyCondition::And(conditions) => {
                conditions.iter().all(|c| c.is_satisfied(states))
            }
            DependencyCondition::Or(conditions) => {
                conditions.iter().any(|c| c.is_satisfied(states))
            }
        }
    }

    /// Get all agent types referenced by this condition
    pub fn referenced_types(&self) -> HashSet<AgentType> {
        let mut types = HashSet::new();
        self.collect_types(&mut types);
        types
    }

    fn collect_types(&self, types: &mut HashSet<AgentType>) {
        match self {
            DependencyCondition::AgentInState { .. } => {}
            DependencyCondition::AllOfType { agent_type, .. }
            | DependencyCondition::AnyOfType { agent_type, .. }
            | DependencyCondition::AtLeastN { agent_type, .. }
            | DependencyCondition::NoneOfType { agent_type, .. } => {
                types.insert(*agent_type);
            }
            DependencyCondition::And(conditions) | DependencyCondition::Or(conditions) => {
                for c in conditions {
                    c.collect_types(types);
                }
            }
        }
    }
}

/// Trait for types that define their dependencies
pub trait DependencySet {
    /// Check if this type can observe a target agent
    fn can_observe(&self, target: &AgentId) -> bool;

    /// Get required state conditions for dependencies
    fn required_states(&self) -> Vec<super::StateRequirement>;

    /// Get the agent types this depends on
    fn depends_on_types(&self) -> Vec<AgentType>;
}

/// Dependency graph for the agent network
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Forward edges: agent -> agents it depends on
    dependencies: HashMap<AgentId, HashSet<AgentId>>,
    /// Reverse edges: agent -> agents that depend on it
    dependents: HashMap<AgentId, HashSet<AgentId>>,
    /// Agent type information
    agent_types: HashMap<AgentId, AgentType>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an agent in the graph
    pub fn register_agent(&mut self, agent_id: AgentId, agent_type: AgentType) {
        self.agent_types.insert(agent_id, agent_type);
        self.dependencies.entry(agent_id).or_default();
        self.dependents.entry(agent_id).or_default();
    }

    /// Add a dependency relationship
    pub fn add_dependency(&mut self, from: AgentId, to: AgentId) -> Result<(), DependencyError> {
        // Check for cycles before adding
        if self.would_create_cycle(from, to) {
            return Err(DependencyError::CycleDetected { from, to });
        }

        self.dependencies.entry(from).or_default().insert(to);
        self.dependents.entry(to).or_default().insert(from);
        Ok(())
    }

    /// Remove a dependency relationship
    pub fn remove_dependency(&mut self, from: AgentId, to: AgentId) {
        if let Some(deps) = self.dependencies.get_mut(&from) {
            deps.remove(&to);
        }
        if let Some(deps) = self.dependents.get_mut(&to) {
            deps.remove(&from);
        }
    }

    /// Remove an agent from the graph
    pub fn remove_agent(&mut self, agent_id: AgentId) {
        // Remove from dependencies
        if let Some(deps) = self.dependencies.remove(&agent_id) {
            for dep in deps {
                if let Some(dependents) = self.dependents.get_mut(&dep) {
                    dependents.remove(&agent_id);
                }
            }
        }

        // Remove from dependents
        if let Some(deps) = self.dependents.remove(&agent_id) {
            for dep in deps {
                if let Some(dependencies) = self.dependencies.get_mut(&dep) {
                    dependencies.remove(&agent_id);
                }
            }
        }

        self.agent_types.remove(&agent_id);
    }

    /// Get agents that the given agent depends on
    pub fn get_dependencies(&self, agent_id: AgentId) -> impl Iterator<Item = AgentId> + '_ {
        self.dependencies
            .get(&agent_id)
            .into_iter()
            .flat_map(|s| s.iter().copied())
    }

    /// Get agents that depend on the given agent
    pub fn get_dependents(&self, agent_id: AgentId) -> impl Iterator<Item = AgentId> + '_ {
        self.dependents
            .get(&agent_id)
            .into_iter()
            .flat_map(|s| s.iter().copied())
    }

    /// Get the type of an agent
    pub fn get_agent_type(&self, agent_id: AgentId) -> Option<AgentType> {
        self.agent_types.get(&agent_id).copied()
    }

    /// Check if adding an edge would create a cycle
    fn would_create_cycle(&self, from: AgentId, to: AgentId) -> bool {
        if from == to {
            return true;
        }

        // BFS from 'to' to see if we can reach 'from'
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(to);

        while let Some(current) = queue.pop_front() {
            if current == from {
                return true;
            }
            if visited.insert(current) {
                for dep in self.get_dependencies(current) {
                    queue.push_back(dep);
                }
            }
        }

        false
    }

    /// Get topological ordering of agents (for propagation order)
    pub fn topological_order(&self) -> Result<Vec<AgentId>, DependencyError> {
        let mut result = Vec::new();
        let mut in_degree: HashMap<AgentId, usize> = HashMap::new();

        // Calculate in-degrees
        for agent_id in self.agent_types.keys() {
            let count = self
                .dependencies
                .get(agent_id)
                .map(|s| s.len())
                .unwrap_or(0);
            in_degree.insert(*agent_id, count);
        }

        // Start with nodes that have no dependencies
        let mut queue: VecDeque<AgentId> = in_degree
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(&id, _)| id)
            .collect();

        while let Some(agent_id) = queue.pop_front() {
            result.push(agent_id);

            // Reduce in-degree of dependents
            for dependent in self.get_dependents(agent_id) {
                if let Some(count) = in_degree.get_mut(&dependent) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }

        if result.len() != self.agent_types.len() {
            Err(DependencyError::CycleInGraph)
        } else {
            Ok(result)
        }
    }

    /// Get all agents that would be affected by a state change
    pub fn affected_agents(&self, agent_id: AgentId) -> HashSet<AgentId> {
        let mut affected = HashSet::new();
        let mut queue = VecDeque::new();

        for dep in self.get_dependents(agent_id) {
            queue.push_back(dep);
        }

        while let Some(current) = queue.pop_front() {
            if affected.insert(current) {
                for dep in self.get_dependents(current) {
                    queue.push_back(dep);
                }
            }
        }

        affected
    }

    /// Check if agent A can observe agent B (B is a dependency of A)
    pub fn can_observe(&self, observer: AgentId, target: AgentId) -> bool {
        self.dependencies
            .get(&observer)
            .map(|deps| deps.contains(&target))
            .unwrap_or(false)
    }

    /// Get all agents
    pub fn all_agents(&self) -> impl Iterator<Item = AgentId> + '_ {
        self.agent_types.keys().copied()
    }
}

/// Errors from dependency operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum DependencyError {
    #[error("Adding dependency from {from} to {to} would create a cycle")]
    CycleDetected { from: AgentId, to: AgentId },

    #[error("Dependency graph contains a cycle")]
    CycleInGraph,

    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("Dependency not satisfied: {0}")]
    NotSatisfied(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        let a = AgentId::new();
        let b = AgentId::new();
        let c = AgentId::new();

        graph.register_agent(a, AgentType::StoryDeveloper);
        graph.register_agent(b, AgentType::CodeReviewer);
        graph.register_agent(c, AgentType::BmadOrchestrator);

        // a depends on b, b depends on c
        graph.add_dependency(a, b).unwrap();
        graph.add_dependency(b, c).unwrap();

        // a can observe b, b can observe c
        assert!(graph.can_observe(a, b));
        assert!(graph.can_observe(b, c));
        assert!(!graph.can_observe(a, c)); // a cannot directly observe c
        assert!(!graph.can_observe(c, a)); // c cannot observe a

        // Topological order should be [c, b, a]
        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], c);
        assert_eq!(order[1], b);
        assert_eq!(order[2], a);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        let a = AgentId::new();
        let b = AgentId::new();

        graph.register_agent(a, AgentType::StoryDeveloper);
        graph.register_agent(b, AgentType::CodeReviewer);

        graph.add_dependency(a, b).unwrap();

        // Adding b -> a would create a cycle
        let result = graph.add_dependency(b, a);
        assert!(matches!(result, Err(DependencyError::CycleDetected { .. })));
    }

    #[test]
    fn test_dependency_condition() {
        let a = AgentId::new();
        let b = AgentId::new();

        let mut states = HashMap::new();
        states.insert(a, (AgentType::StoryDeveloper, AgentState::Running));
        states.insert(b, (AgentType::CodeReviewer, AgentState::Completed));

        // Test specific agent condition
        let cond = DependencyCondition::AgentInState {
            agent_id: a,
            state: AgentState::Running,
        };
        assert!(cond.is_satisfied(&states));

        // Test all of type condition
        let cond = DependencyCondition::AllOfType {
            agent_type: AgentType::CodeReviewer,
            state: AgentState::Completed,
        };
        assert!(cond.is_satisfied(&states));

        // Test any of type condition
        let cond = DependencyCondition::AnyOfType {
            agent_type: AgentType::StoryDeveloper,
            states: vec![AgentState::Running, AgentState::Completed],
        };
        assert!(cond.is_satisfied(&states));
    }
}
