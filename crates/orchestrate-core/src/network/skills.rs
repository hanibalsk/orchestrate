//! Skill registry with state requirements
//!
//! This module provides skill definitions that include:
//! - Required agent state for execution
//! - Required dependency states
//! - State changes produced by skill execution

use super::AgentId;
use crate::{AgentState, AgentType};
use std::collections::HashMap;

/// Definition of a skill that an agent can execute
#[derive(Debug, Clone)]
pub struct SkillDefinition {
    /// Unique skill name
    pub name: String,
    /// Agent types that can execute this skill
    pub agent_types: Vec<AgentType>,
    /// Required state for the executing agent
    pub required_state: AgentState,
    /// Required states of dependency agents
    pub dependency_states: Vec<(AgentType, AgentState)>,
    /// State produced after successful execution
    pub produces_state: Option<AgentState>,
    /// Timeout in seconds (None for no timeout)
    pub timeout_secs: Option<u64>,
    /// Whether this skill can be cancelled
    pub cancellable: bool,
    /// Priority (higher = more important)
    pub priority: u32,
}

impl SkillDefinition {
    /// Create a new skill definition
    pub fn new(name: impl Into<String>, agent_types: Vec<AgentType>) -> Self {
        Self {
            name: name.into(),
            agent_types,
            required_state: AgentState::Running,
            dependency_states: Vec::new(),
            produces_state: None,
            timeout_secs: None,
            cancellable: true,
            priority: 0,
        }
    }

    /// Set required agent state
    pub fn requires_state(mut self, state: AgentState) -> Self {
        self.required_state = state;
        self
    }

    /// Add a dependency state requirement
    pub fn requires_dependency(mut self, agent_type: AgentType, state: AgentState) -> Self {
        self.dependency_states.push((agent_type, state));
        self
    }

    /// Set the state produced after execution
    pub fn produces(mut self, state: AgentState) -> Self {
        self.produces_state = Some(state);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Set whether skill is cancellable
    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if an agent type can execute this skill
    pub fn can_execute(&self, agent_type: AgentType) -> bool {
        self.agent_types.contains(&agent_type)
    }

    /// Check if all requirements are met
    pub fn requirements_met(
        &self,
        agent_state: AgentState,
        dependency_states: &HashMap<AgentId, (AgentType, AgentState)>,
    ) -> bool {
        // Check agent's own state
        if agent_state != self.required_state {
            return false;
        }

        // Check dependency states
        for (required_type, required_state) in &self.dependency_states {
            let satisfied = dependency_states
                .values()
                .any(|(t, s)| t == required_type && s == required_state);
            if !satisfied {
                return false;
            }
        }

        true
    }
}

/// A skill instance that can be executed
#[derive(Debug, Clone)]
pub struct Skill {
    /// Skill definition
    pub definition: SkillDefinition,
    /// Current execution status
    pub status: SkillStatus,
    /// Agent executing this skill
    pub executor: Option<AgentId>,
    /// Start time
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// End time
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Result message
    pub result: Option<String>,
}

/// Status of a skill execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillStatus {
    /// Waiting for dependencies
    Pending,
    /// Ready to execute
    Ready,
    /// Currently executing
    Executing,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
    /// Timed out
    TimedOut,
}

impl Skill {
    /// Create a new skill instance
    pub fn new(definition: SkillDefinition) -> Self {
        Self {
            definition,
            status: SkillStatus::Pending,
            executor: None,
            started_at: None,
            ended_at: None,
            result: None,
        }
    }

    /// Start executing the skill
    pub fn start(&mut self, executor: AgentId) {
        self.status = SkillStatus::Executing;
        self.executor = Some(executor);
        self.started_at = Some(chrono::Utc::now());
    }

    /// Complete the skill successfully
    pub fn complete(&mut self, result: Option<String>) {
        self.status = SkillStatus::Completed;
        self.ended_at = Some(chrono::Utc::now());
        self.result = result;
    }

    /// Mark the skill as failed
    pub fn fail(&mut self, error: String) {
        self.status = SkillStatus::Failed;
        self.ended_at = Some(chrono::Utc::now());
        self.result = Some(error);
    }

    /// Cancel the skill
    pub fn cancel(&mut self) {
        if self.definition.cancellable {
            self.status = SkillStatus::Cancelled;
            self.ended_at = Some(chrono::Utc::now());
        }
    }

    /// Check if skill has timed out
    pub fn check_timeout(&mut self) -> bool {
        if let (Some(timeout), Some(started), SkillStatus::Executing) =
            (self.definition.timeout_secs, self.started_at, self.status)
        {
            let elapsed = chrono::Utc::now().signed_duration_since(started);
            if elapsed.num_seconds() as u64 > timeout {
                self.status = SkillStatus::TimedOut;
                self.ended_at = Some(chrono::Utc::now());
                return true;
            }
        }
        false
    }

    /// Get execution duration
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => Some(end.signed_duration_since(start)),
            (Some(start), None) => Some(chrono::Utc::now().signed_duration_since(start)),
            _ => None,
        }
    }
}

/// Registry of available skills
#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    /// All registered skills
    skills: HashMap<String, SkillDefinition>,
    /// Skills by agent type
    skills_by_type: HashMap<AgentType, Vec<String>>,
}

impl SkillRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a skill
    pub fn register(&mut self, skill: SkillDefinition) {
        let name = skill.name.clone();
        for agent_type in &skill.agent_types {
            self.skills_by_type
                .entry(*agent_type)
                .or_default()
                .push(name.clone());
        }
        self.skills.insert(name, skill);
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&SkillDefinition> {
        self.skills.get(name)
    }

    /// Get all skills for an agent type
    pub fn skills_for_type(&self, agent_type: AgentType) -> Vec<&SkillDefinition> {
        self.skills_by_type
            .get(&agent_type)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| self.skills.get(name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get available skills for an agent given its state and dependencies
    pub fn available_skills(
        &self,
        agent_type: AgentType,
        agent_state: AgentState,
        dependency_states: &HashMap<AgentId, (AgentType, AgentState)>,
    ) -> Vec<&SkillDefinition> {
        self.skills_for_type(agent_type)
            .into_iter()
            .filter(|skill| skill.requirements_met(agent_state, dependency_states))
            .collect()
    }

    /// Get all registered skill names
    pub fn all_skills(&self) -> impl Iterator<Item = &str> {
        self.skills.keys().map(|s| s.as_str())
    }
}

/// Create the default skill registry with standard agent skills
pub fn default_skill_registry() -> SkillRegistry {
    let mut registry = SkillRegistry::new();

    // Story Developer skills
    registry.register(
        SkillDefinition::new("develop", vec![AgentType::StoryDeveloper])
            .requires_state(AgentState::Running)
            .with_timeout(3600), // 1 hour
    );
    registry.register(
        SkillDefinition::new("test", vec![AgentType::StoryDeveloper])
            .requires_state(AgentState::Running)
            .with_timeout(1800), // 30 minutes
    );
    registry.register(
        SkillDefinition::new("refactor", vec![AgentType::StoryDeveloper])
            .requires_state(AgentState::Running)
            .with_timeout(3600),
    );

    // Code Reviewer skills
    registry.register(
        SkillDefinition::new("review", vec![AgentType::CodeReviewer])
            .requires_state(AgentState::Running)
            .requires_dependency(AgentType::StoryDeveloper, AgentState::Completed)
            .with_timeout(1800),
    );

    // Issue Fixer skills
    registry.register(
        SkillDefinition::new("diagnose", vec![AgentType::IssueFixer])
            .requires_state(AgentState::Running)
            .with_timeout(900), // 15 minutes
    );
    registry.register(
        SkillDefinition::new("fix", vec![AgentType::IssueFixer])
            .requires_state(AgentState::Running)
            .with_timeout(1800),
    );

    // Explorer skills
    registry.register(
        SkillDefinition::new("search", vec![AgentType::Explorer])
            .requires_state(AgentState::Running)
            .with_timeout(300), // 5 minutes
    );
    registry.register(
        SkillDefinition::new("analyze", vec![AgentType::Explorer])
            .requires_state(AgentState::Running)
            .with_timeout(600), // 10 minutes
    );

    // PR Shepherd skills
    registry.register(
        SkillDefinition::new("watch_pr", vec![AgentType::PrShepherd])
            .requires_state(AgentState::Running)
            .cancellable(false), // Long-running
    );
    registry.register(
        SkillDefinition::new("fix_review", vec![AgentType::PrShepherd])
            .requires_state(AgentState::Running)
            .with_timeout(1800),
    );
    registry.register(
        SkillDefinition::new(
            "resolve_conflicts",
            vec![AgentType::PrShepherd, AgentType::ConflictResolver],
        )
        .requires_state(AgentState::Running)
        .with_timeout(900),
    );

    // BMAD Orchestrator skills
    registry.register(
        SkillDefinition::new("orchestrate_epic", vec![AgentType::BmadOrchestrator])
            .requires_state(AgentState::Running)
            .cancellable(false),
    );
    registry.register(
        SkillDefinition::new("spawn_developer", vec![AgentType::BmadOrchestrator])
            .requires_state(AgentState::Running)
            .with_timeout(60), // Quick spawn
    );

    // BMAD Planner skills
    registry.register(
        SkillDefinition::new("plan_epic", vec![AgentType::BmadPlanner])
            .requires_state(AgentState::Running)
            .with_timeout(1800),
    );
    registry.register(
        SkillDefinition::new("create_stories", vec![AgentType::BmadPlanner])
            .requires_state(AgentState::Running)
            .with_timeout(1800),
    );

    // PR Controller skills
    registry.register(
        SkillDefinition::new("manage_queue", vec![AgentType::PrController])
            .requires_state(AgentState::Running)
            .cancellable(false),
    );
    registry.register(
        SkillDefinition::new("merge_pr", vec![AgentType::PrController])
            .requires_state(AgentState::Running)
            .requires_dependency(AgentType::CodeReviewer, AgentState::Completed)
            .with_timeout(300),
    );

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_registry() {
        let registry = default_skill_registry();

        // Check story developer skills
        let skills = registry.skills_for_type(AgentType::StoryDeveloper);
        assert!(skills.iter().any(|s| s.name == "develop"));
        assert!(skills.iter().any(|s| s.name == "test"));
        assert!(skills.iter().any(|s| s.name == "refactor"));

        // Check code reviewer skills
        let skills = registry.skills_for_type(AgentType::CodeReviewer);
        assert!(skills.iter().any(|s| s.name == "review"));
    }

    #[test]
    fn test_skill_requirements() {
        let skill = SkillDefinition::new("review", vec![AgentType::CodeReviewer])
            .requires_state(AgentState::Running)
            .requires_dependency(AgentType::StoryDeveloper, AgentState::Completed);

        let dev_id = AgentId::new();
        let mut deps = HashMap::new();

        // Without dependency, requirements not met
        assert!(!skill.requirements_met(AgentState::Running, &deps));

        // With dependency in wrong state
        deps.insert(dev_id, (AgentType::StoryDeveloper, AgentState::Running));
        assert!(!skill.requirements_met(AgentState::Running, &deps));

        // With dependency in correct state
        deps.insert(dev_id, (AgentType::StoryDeveloper, AgentState::Completed));
        assert!(skill.requirements_met(AgentState::Running, &deps));

        // With wrong agent state
        assert!(!skill.requirements_met(AgentState::Paused, &deps));
    }

    #[test]
    fn test_skill_execution() {
        let definition =
            SkillDefinition::new("test", vec![AgentType::StoryDeveloper]).with_timeout(60);

        let mut skill = Skill::new(definition);
        assert_eq!(skill.status, SkillStatus::Pending);

        let executor = AgentId::new();
        skill.start(executor);
        assert_eq!(skill.status, SkillStatus::Executing);
        assert!(skill.started_at.is_some());

        skill.complete(Some("All tests passed".to_string()));
        assert_eq!(skill.status, SkillStatus::Completed);
        assert!(skill.ended_at.is_some());
        assert!(skill.duration().is_some());
    }
}
