//! Network Coordinator for state propagation and self-healing
//!
//! The NetworkCoordinator is the central hub for:
//! - Agent registration and discovery
//! - State change propagation
//! - Network-wide validation
//! - Self-healing capabilities

use super::{
    AgentHandle, AgentId, DependencyGraph, SkillRegistry,
    StateGraph, StateMachine, StatePropagation, ValidationResult,
};
use super::state::default_agent_state_graph;
use super::skills::default_skill_registry;
use super::validation::{NetworkValidator, ValidationErrorCode};
use crate::{AgentState, AgentType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Events emitted by the network
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// Agent registered in the network
    AgentRegistered {
        agent_id: AgentId,
        agent_type: AgentType,
    },
    /// Agent removed from the network
    AgentRemoved {
        agent_id: AgentId,
    },
    /// Agent state changed
    StateChanged {
        agent_id: AgentId,
        from: AgentState,
        to: AgentState,
    },
    /// Dependency added
    DependencyAdded {
        from: AgentId,
        to: AgentId,
    },
    /// Validation completed
    ValidationCompleted {
        result: ValidationResult,
    },
    /// Self-healing action taken
    SelfHealingAction {
        action: RecoveryAction,
    },
}

/// Recovery action for self-healing
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Restart a failed agent
    RestartAgent {
        agent_id: AgentId,
        reason: String,
    },
    /// Pause an agent waiting for dependencies
    PauseAgent {
        agent_id: AgentId,
        reason: String,
    },
    /// Terminate a stuck agent
    TerminateAgent {
        agent_id: AgentId,
        reason: String,
    },
    /// Spawn a missing dependency
    SpawnDependency {
        for_agent: AgentId,
        agent_type: AgentType,
        reason: String,
    },
    /// Retry a failed transition
    RetryTransition {
        agent_id: AgentId,
        target_state: AgentState,
    },
    /// No action needed
    None,
}

/// Configuration for the network coordinator
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Enable automatic state propagation
    pub auto_propagate: bool,
    /// Enable self-healing
    pub self_healing_enabled: bool,
    /// Validation interval in seconds
    pub validation_interval_secs: u64,
    /// Maximum retry attempts for recovery
    pub max_recovery_attempts: u32,
    /// Event channel capacity
    pub event_channel_capacity: usize,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            auto_propagate: true,
            self_healing_enabled: true,
            validation_interval_secs: 60,
            max_recovery_attempts: 3,
            event_channel_capacity: 1000,
        }
    }
}

/// The network coordinator
pub struct NetworkCoordinator {
    /// All registered agents
    agents: RwLock<HashMap<AgentId, AgentHandle>>,
    /// State machines for each agent
    state_machines: RwLock<HashMap<AgentId, StateMachine>>,
    /// Dependency graph
    dependency_graph: RwLock<DependencyGraph>,
    /// Skill registry
    skill_registry: Arc<SkillRegistry>,
    /// Network validator
    validator: NetworkValidator,
    /// State graph template
    state_graph: StateGraph,
    /// Event broadcaster
    event_tx: broadcast::Sender<NetworkEvent>,
    /// Configuration
    config: CoordinatorConfig,
    /// Recovery attempt counts
    recovery_attempts: RwLock<HashMap<AgentId, u32>>,
}

impl NetworkCoordinator {
    /// Create a new network coordinator
    pub fn new(config: CoordinatorConfig) -> Self {
        let (event_tx, _) = broadcast::channel(config.event_channel_capacity);

        Self {
            agents: RwLock::new(HashMap::new()),
            state_machines: RwLock::new(HashMap::new()),
            dependency_graph: RwLock::new(DependencyGraph::new()),
            skill_registry: Arc::new(default_skill_registry()),
            validator: NetworkValidator::default(),
            state_graph: default_agent_state_graph(),
            event_tx,
            config,
            recovery_attempts: RwLock::new(HashMap::new()),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CoordinatorConfig::default())
    }

    /// Subscribe to network events
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkEvent> {
        self.event_tx.subscribe()
    }

    /// Register an agent in the network
    pub async fn register_agent(
        &self,
        agent_id: AgentId,
        agent_type: AgentType,
        initial_state: AgentState,
    ) -> Result<(), CoordinatorError> {
        let handle = AgentHandle::new(agent_id, agent_type, initial_state);
        let state_machine = StateMachine::new(self.state_graph.clone());

        {
            let mut agents = self.agents.write().await;
            if agents.contains_key(&agent_id) {
                return Err(CoordinatorError::AgentAlreadyExists(agent_id));
            }
            agents.insert(agent_id, handle);
        }

        {
            let mut machines = self.state_machines.write().await;
            machines.insert(agent_id, state_machine);
        }

        {
            let mut graph = self.dependency_graph.write().await;
            graph.register_agent(agent_id, agent_type);
        }

        let _ = self.event_tx.send(NetworkEvent::AgentRegistered {
            agent_id,
            agent_type,
        });

        Ok(())
    }

    /// Remove an agent from the network
    pub async fn remove_agent(&self, agent_id: AgentId) -> Result<(), CoordinatorError> {
        {
            let mut agents = self.agents.write().await;
            agents.remove(&agent_id);
        }

        {
            let mut machines = self.state_machines.write().await;
            machines.remove(&agent_id);
        }

        {
            let mut graph = self.dependency_graph.write().await;
            graph.remove_agent(agent_id);
        }

        let _ = self.event_tx.send(NetworkEvent::AgentRemoved { agent_id });

        Ok(())
    }

    /// Add a dependency between agents
    pub async fn add_dependency(
        &self,
        from: AgentId,
        to: AgentId,
    ) -> Result<(), CoordinatorError> {
        {
            let mut graph = self.dependency_graph.write().await;
            graph.add_dependency(from, to)
                .map_err(|e| CoordinatorError::DependencyError(e.to_string()))?;
        }

        {
            let mut agents = self.agents.write().await;
            if let Some(from_agent) = agents.get_mut(&from) {
                from_agent.add_dependency(to);
            }
            if let Some(to_agent) = agents.get_mut(&to) {
                to_agent.add_dependent(from);
            }
        }

        let _ = self.event_tx.send(NetworkEvent::DependencyAdded { from, to });

        Ok(())
    }

    /// Transition an agent to a new state
    pub async fn transition_state(
        &self,
        agent_id: AgentId,
        new_state: AgentState,
        trigger: Option<String>,
    ) -> Result<(), CoordinatorError> {
        let old_state;
        let propagations;

        // Get dependency states for validation
        let dependency_states = self.get_dependency_states(agent_id).await?;

        // Perform the transition
        {
            let mut machines = self.state_machines.write().await;
            let machine = machines
                .get_mut(&agent_id)
                .ok_or(CoordinatorError::AgentNotFound(agent_id))?;

            old_state = machine.current_state();
            propagations = machine
                .transition(new_state, &dependency_states, trigger)
                .map_err(|e| CoordinatorError::TransitionError(e.to_string()))?;
        }

        // Update agent handle
        {
            let mut agents = self.agents.write().await;
            if let Some(agent) = agents.get_mut(&agent_id) {
                agent.state = new_state;
            }
        }

        // Emit state change event
        let _ = self.event_tx.send(NetworkEvent::StateChanged {
            agent_id,
            from: old_state,
            to: new_state,
        });

        // Propagate to dependents if enabled
        if self.config.auto_propagate {
            self.propagate_state_changes(agent_id, propagations).await?;
        }

        Ok(())
    }

    /// Get dependency states for an agent
    async fn get_dependency_states(
        &self,
        agent_id: AgentId,
    ) -> Result<HashMap<AgentId, (AgentType, AgentState)>, CoordinatorError> {
        let agents = self.agents.read().await;
        let agent = agents
            .get(&agent_id)
            .ok_or(CoordinatorError::AgentNotFound(agent_id))?;

        let mut states = HashMap::new();
        for dep_id in &agent.dependencies {
            if let Some(dep) = agents.get(dep_id) {
                states.insert(*dep_id, (dep.agent_type, dep.state));
            }
        }

        Ok(states)
    }

    /// Propagate state changes to dependent agents
    async fn propagate_state_changes(
        &self,
        source_agent: AgentId,
        propagations: Vec<StatePropagation>,
    ) -> Result<(), CoordinatorError> {
        if propagations.is_empty() {
            return Ok(());
        }

        let affected_agents = {
            let graph = self.dependency_graph.read().await;
            graph.affected_agents(source_agent)
        };

        // Queue propagation events for affected agents
        let agents = self.agents.read().await;
        for propagation in propagations {
            for &affected_id in &affected_agents {
                if let Some(agent) = agents.get(&affected_id) {
                    if agent.agent_type == propagation.target_type {
                        // Notify the agent about the propagation event
                        // In a real implementation, this would trigger the agent's event handler
                        tracing::debug!(
                            "Propagating {:?} from {} to {}",
                            propagation.event,
                            source_agent,
                            affected_id
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate the entire network
    pub async fn validate_network(&self) -> ValidationResult {
        let agents = self.agents.read().await;
        let graph = self.dependency_graph.read().await;

        let result = self.validator.validate(&agents, &graph);

        let _ = self.event_tx.send(NetworkEvent::ValidationCompleted {
            result: result.clone(),
        });

        result
    }

    /// Perform self-healing based on current network state
    pub async fn self_heal(&self) -> Vec<RecoveryAction> {
        if !self.config.self_healing_enabled {
            return vec![];
        }

        let mut actions = Vec::new();

        // First, validate the network
        let validation = self.validate_network().await;

        // Generate recovery actions for each error
        for error in validation.errors {
            let action = self.generate_recovery_action(&error).await;
            if !matches!(action, RecoveryAction::None) {
                actions.push(action);
            }
        }

        // Emit events for actions taken
        for action in &actions {
            let _ = self.event_tx.send(NetworkEvent::SelfHealingAction {
                action: action.clone(),
            });
        }

        actions
    }

    /// Generate a recovery action for a validation error
    async fn generate_recovery_action(
        &self,
        error: &super::validation::ValidationError,
    ) -> RecoveryAction {

        let agent_id = match error.agent_id {
            Some(id) => id,
            None => return RecoveryAction::None,
        };

        // Check recovery attempts
        {
            let attempts = self.recovery_attempts.read().await;
            if let Some(&count) = attempts.get(&agent_id) {
                if count >= self.config.max_recovery_attempts {
                    return RecoveryAction::TerminateAgent {
                        agent_id,
                        reason: "Max recovery attempts exceeded".to_string(),
                    };
                }
            }
        }

        // Increment recovery attempts
        {
            let mut attempts = self.recovery_attempts.write().await;
            *attempts.entry(agent_id).or_insert(0) += 1;
        }

        match error.code {
            ValidationErrorCode::DependencyStateInvalid => {
                RecoveryAction::PauseAgent {
                    agent_id,
                    reason: error.message.clone(),
                }
            }
            ValidationErrorCode::InvalidState => {
                RecoveryAction::RestartAgent {
                    agent_id,
                    reason: error.message.clone(),
                }
            }
            ValidationErrorCode::MissingDependency => {
                // Try to determine what type of agent is needed
                RecoveryAction::SpawnDependency {
                    for_agent: agent_id,
                    agent_type: AgentType::Explorer, // Default fallback
                    reason: error.message.clone(),
                }
            }
            ValidationErrorCode::TimeoutExceeded => {
                RecoveryAction::TerminateAgent {
                    agent_id,
                    reason: error.message.clone(),
                }
            }
            _ => RecoveryAction::None,
        }
    }

    /// Execute a recovery action
    pub async fn execute_recovery(&self, action: &RecoveryAction) -> Result<(), CoordinatorError> {
        match action {
            RecoveryAction::RestartAgent { agent_id, .. } => {
                // Transition to Created state and then to Initializing
                self.transition_state(*agent_id, AgentState::Created, Some("recovery".to_string())).await?;
                self.transition_state(*agent_id, AgentState::Initializing, Some("recovery".to_string())).await?;
            }
            RecoveryAction::PauseAgent { agent_id, .. } => {
                self.transition_state(*agent_id, AgentState::Paused, Some("recovery".to_string())).await?;
            }
            RecoveryAction::TerminateAgent { agent_id, .. } => {
                self.transition_state(*agent_id, AgentState::Terminated, Some("recovery".to_string())).await?;
            }
            RecoveryAction::SpawnDependency { for_agent, agent_type, .. } => {
                // Create and register a new agent
                let new_id = AgentId::new();
                self.register_agent(new_id, *agent_type, AgentState::Created).await?;
                self.add_dependency(*for_agent, new_id).await?;
            }
            RecoveryAction::RetryTransition { agent_id, target_state } => {
                self.transition_state(*agent_id, *target_state, Some("retry".to_string())).await?;
            }
            RecoveryAction::None => {}
        }
        Ok(())
    }

    /// Get the current state of an agent
    pub async fn get_agent_state(&self, agent_id: AgentId) -> Option<AgentState> {
        let agents = self.agents.read().await;
        agents.get(&agent_id).map(|a| a.state)
    }

    /// Get all agents of a specific type
    pub async fn get_agents_by_type(&self, agent_type: AgentType) -> Vec<AgentId> {
        let agents = self.agents.read().await;
        agents
            .iter()
            .filter(|(_, a)| a.agent_type == agent_type)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get available skills for an agent
    pub async fn available_skills(&self, agent_id: AgentId) -> Vec<String> {
        let agents = self.agents.read().await;
        let agent = match agents.get(&agent_id) {
            Some(a) => a,
            None => return vec![],
        };

        let dependency_states: HashMap<AgentId, (AgentType, AgentState)> = agent
            .dependencies
            .iter()
            .filter_map(|dep_id| {
                agents.get(dep_id).map(|dep| (*dep_id, (dep.agent_type, dep.state)))
            })
            .collect();

        self.skill_registry
            .available_skills(agent.agent_type, agent.state, &dependency_states)
            .iter()
            .map(|s| s.name.clone())
            .collect()
    }

    /// Get the skill registry
    pub fn skill_registry(&self) -> Arc<SkillRegistry> {
        Arc::clone(&self.skill_registry)
    }

    /// Get dependency agent IDs for an agent
    pub async fn get_dependency_ids(&self, agent_id: AgentId) -> Vec<AgentId> {
        let agents = self.agents.read().await;
        agents
            .get(&agent_id)
            .map(|a| a.dependencies.clone())
            .unwrap_or_default()
    }

    /// Get network statistics
    pub async fn stats(&self) -> NetworkStats {
        let agents = self.agents.read().await;

        let mut by_type: HashMap<AgentType, usize> = HashMap::new();
        let mut by_state: HashMap<AgentState, usize> = HashMap::new();

        for agent in agents.values() {
            *by_type.entry(agent.agent_type).or_default() += 1;
            *by_state.entry(agent.state).or_default() += 1;
        }

        NetworkStats {
            total_agents: agents.len(),
            agents_by_type: by_type,
            agents_by_state: by_state,
        }
    }
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub total_agents: usize,
    pub agents_by_type: HashMap<AgentType, usize>,
    pub agents_by_state: HashMap<AgentState, usize>,
}

/// Coordinator errors
#[derive(Debug, thiserror::Error)]
pub enum CoordinatorError {
    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("Agent already exists: {0}")]
    AgentAlreadyExists(AgentId),

    #[error("Dependency error: {0}")]
    DependencyError(String),

    #[error("Transition error: {0}")]
    TransitionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_basic() {
        let coordinator = NetworkCoordinator::with_defaults();

        let agent_id = AgentId::new();
        coordinator
            .register_agent(agent_id, AgentType::StoryDeveloper, AgentState::Created)
            .await
            .unwrap();

        // Transition through states
        coordinator
            .transition_state(agent_id, AgentState::Initializing, None)
            .await
            .unwrap();

        let state = coordinator.get_agent_state(agent_id).await;
        assert_eq!(state, Some(AgentState::Initializing));

        coordinator
            .transition_state(agent_id, AgentState::Running, None)
            .await
            .unwrap();

        let state = coordinator.get_agent_state(agent_id).await;
        assert_eq!(state, Some(AgentState::Running));
    }

    #[tokio::test]
    async fn test_coordinator_dependencies() {
        let coordinator = NetworkCoordinator::with_defaults();

        let dev_id = AgentId::new();
        let reviewer_id = AgentId::new();

        coordinator
            .register_agent(dev_id, AgentType::StoryDeveloper, AgentState::Created)
            .await
            .unwrap();
        coordinator
            .register_agent(reviewer_id, AgentType::CodeReviewer, AgentState::Created)
            .await
            .unwrap();

        // Reviewer depends on developer
        coordinator.add_dependency(reviewer_id, dev_id).await.unwrap();

        let stats = coordinator.stats().await;
        assert_eq!(stats.total_agents, 2);
    }

    #[tokio::test]
    async fn test_coordinator_validation() {
        let coordinator = NetworkCoordinator::with_defaults();

        let agent_id = AgentId::new();
        coordinator
            .register_agent(agent_id, AgentType::Explorer, AgentState::Running)
            .await
            .unwrap();

        let result = coordinator.validate_network().await;
        // Single agent with no dependencies should be valid (but may have warnings)
        assert!(result.is_valid);
    }
}
