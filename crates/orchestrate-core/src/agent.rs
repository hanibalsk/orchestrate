//! Agent state machine implementation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Agent states in the lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Agent created but not yet started
    Created,
    /// Agent is initializing (loading context, etc.)
    Initializing,
    /// Agent is actively running
    Running,
    /// Agent is waiting for user input
    WaitingForInput,
    /// Agent is waiting for external event (PR review, CI, etc.)
    WaitingForExternal,
    /// Agent is paused (can be resumed)
    Paused,
    /// Agent completed successfully
    Completed,
    /// Agent failed with error
    Failed,
    /// Agent was terminated
    Terminated,
}

impl AgentState {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentState::Created => "created",
            AgentState::Initializing => "initializing",
            AgentState::Running => "running",
            AgentState::WaitingForInput => "waiting_for_input",
            AgentState::WaitingForExternal => "waiting_for_external",
            AgentState::Paused => "paused",
            AgentState::Completed => "completed",
            AgentState::Failed => "failed",
            AgentState::Terminated => "terminated",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "created" => Ok(AgentState::Created),
            "initializing" => Ok(AgentState::Initializing),
            "running" => Ok(AgentState::Running),
            "waiting_for_input" => Ok(AgentState::WaitingForInput),
            "waiting_for_external" => Ok(AgentState::WaitingForExternal),
            "paused" => Ok(AgentState::Paused),
            "completed" => Ok(AgentState::Completed),
            "failed" => Ok(AgentState::Failed),
            "terminated" => Ok(AgentState::Terminated),
            _ => Err(crate::Error::Other(format!("Unknown agent state: {}", s))),
        }
    }

    /// Check if this state allows transition to another state
    pub fn can_transition_to(&self, target: AgentState) -> bool {
        use AgentState::*;
        match (self, target) {
            // Created can only go to Initializing
            (Created, Initializing) => true,

            // Initializing can go to Running or Failed
            (Initializing, Running) | (Initializing, Failed) => true,

            // Running can go to many states
            (Running, WaitingForInput)
            | (Running, WaitingForExternal)
            | (Running, Paused)
            | (Running, Completed)
            | (Running, Failed) => true,

            // Waiting states can go back to Running or fail
            (WaitingForInput, Running)
            | (WaitingForInput, Paused)
            | (WaitingForInput, Failed) => true,
            (WaitingForExternal, Running)
            | (WaitingForExternal, Paused)
            | (WaitingForExternal, Failed) => true,

            // Paused can resume or terminate
            (Paused, Running) | (Paused, Terminated) => true,

            // Any state can be terminated
            (_, Terminated) => true,

            // No other transitions allowed
            _ => false,
        }
    }

    /// Check if agent is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentState::Completed | AgentState::Failed | AgentState::Terminated
        )
    }

    /// Check if agent can accept new input
    pub fn accepts_input(&self) -> bool {
        matches!(self, AgentState::Running | AgentState::WaitingForInput)
    }
}

/// Types of agents in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    // Development agents
    StoryDeveloper,
    CodeReviewer,
    IssueFixer,
    Explorer,

    // BMAD agents
    BmadOrchestrator,
    BmadPlanner,

    // PR management
    PrShepherd,
    PrController,
    ConflictResolver,

    // System agents
    BackgroundController,
    Scheduler,
}

impl AgentType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::StoryDeveloper => "story_developer",
            AgentType::CodeReviewer => "code_reviewer",
            AgentType::IssueFixer => "issue_fixer",
            AgentType::Explorer => "explorer",
            AgentType::BmadOrchestrator => "bmad_orchestrator",
            AgentType::BmadPlanner => "bmad_planner",
            AgentType::PrShepherd => "pr_shepherd",
            AgentType::PrController => "pr_controller",
            AgentType::ConflictResolver => "conflict_resolver",
            AgentType::BackgroundController => "background_controller",
            AgentType::Scheduler => "scheduler",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "story_developer" => Ok(AgentType::StoryDeveloper),
            "code_reviewer" => Ok(AgentType::CodeReviewer),
            "issue_fixer" => Ok(AgentType::IssueFixer),
            "explorer" => Ok(AgentType::Explorer),
            "bmad_orchestrator" => Ok(AgentType::BmadOrchestrator),
            "bmad_planner" => Ok(AgentType::BmadPlanner),
            "pr_shepherd" => Ok(AgentType::PrShepherd),
            "pr_controller" => Ok(AgentType::PrController),
            "conflict_resolver" => Ok(AgentType::ConflictResolver),
            "background_controller" => Ok(AgentType::BackgroundController),
            "scheduler" => Ok(AgentType::Scheduler),
            _ => Err(crate::Error::Other(format!("Unknown agent type: {}", s))),
        }
    }

    /// Get the default model for this agent type
    pub fn default_model(&self) -> &'static str {
        match self {
            AgentType::Explorer => "claude-3-haiku-20240307",
            _ => "claude-sonnet-4-20250514",
        }
    }

    /// Get the allowed tools for this agent type
    pub fn allowed_tools(&self) -> Vec<&'static str> {
        match self {
            AgentType::StoryDeveloper => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep", "Task"]
            }
            AgentType::CodeReviewer => vec!["Bash", "Read", "Glob", "Grep"],
            AgentType::IssueFixer => vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep"],
            AgentType::Explorer => vec!["Read", "Glob", "Grep"],
            AgentType::BmadOrchestrator => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep", "Task"]
            }
            AgentType::BmadPlanner => vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep"],
            AgentType::PrShepherd => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep", "Task"]
            }
            AgentType::PrController => vec!["Bash", "Read"],
            AgentType::ConflictResolver => vec!["Bash", "Read", "Write", "Edit"],
            AgentType::BackgroundController => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep", "Task"]
            }
            AgentType::Scheduler => vec!["Bash", "Read"],
        }
    }

    /// Get maximum turns for this agent type
    pub fn default_max_turns(&self) -> u32 {
        match self {
            AgentType::Explorer => 20,
            AgentType::CodeReviewer => 30,
            AgentType::IssueFixer => 40,
            AgentType::ConflictResolver => 30,
            _ => 80,
        }
    }
}

/// Context data for an agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    /// Epic ID if working on an epic
    pub epic_id: Option<String>,
    /// Story ID if working on a story
    pub story_id: Option<String>,
    /// PR number if associated with a PR
    pub pr_number: Option<i32>,
    /// Branch name
    pub branch_name: Option<String>,
    /// Working directory
    pub working_directory: Option<String>,
    /// Custom context data
    #[serde(default)]
    pub custom: serde_json::Value,
}

/// An agent instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique agent ID
    pub id: Uuid,
    /// Type of agent
    pub agent_type: AgentType,
    /// Current state
    pub state: AgentState,
    /// Task description
    pub task: String,
    /// Agent context
    pub context: AgentContext,
    /// Associated session ID
    pub session_id: Option<String>,
    /// Parent agent ID (for forked agents)
    pub parent_agent_id: Option<Uuid>,
    /// Associated worktree ID
    pub worktree_id: Option<String>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

impl Agent {
    /// Create a new agent
    pub fn new(agent_type: AgentType, task: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            agent_type,
            state: AgentState::Created,
            task: task.into(),
            context: AgentContext::default(),
            session_id: None,
            parent_agent_id: None,
            worktree_id: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    /// Set agent context
    pub fn with_context(mut self, context: AgentContext) -> Self {
        self.context = context;
        self
    }

    /// Set parent agent (for forking)
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_agent_id = Some(parent_id);
        self
    }

    /// Set worktree
    pub fn with_worktree(mut self, worktree_id: impl Into<String>) -> Self {
        self.worktree_id = Some(worktree_id.into());
        self
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: AgentState) -> crate::Result<()> {
        if !self.state.can_transition_to(new_state) {
            return Err(crate::Error::InvalidStateTransition(
                format!("{:?}", self.state),
                format!("{:?}", new_state),
            ));
        }

        self.state = new_state;
        self.updated_at = Utc::now();

        if new_state.is_terminal() {
            self.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Mark agent as failed with error message
    pub fn fail(&mut self, error: impl Into<String>) -> crate::Result<()> {
        self.error_message = Some(error.into());
        self.transition_to(AgentState::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        assert!(AgentState::Created.can_transition_to(AgentState::Initializing));
        assert!(!AgentState::Created.can_transition_to(AgentState::Running));
        assert!(AgentState::Running.can_transition_to(AgentState::Completed));
        assert!(AgentState::Paused.can_transition_to(AgentState::Terminated));
    }

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new(AgentType::StoryDeveloper, "Implement auth feature");
        assert_eq!(agent.state, AgentState::Created);
        assert_eq!(agent.task, "Implement auth feature");
    }
}
