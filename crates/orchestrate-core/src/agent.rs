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
            (WaitingForInput, Running) | (WaitingForInput, Paused) | (WaitingForInput, Failed) => {
                true
            }
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

    // Testing agents
    RegressionTester,

    // Issue management
    IssueTriager,

    // System agents
    BackgroundController,
    Scheduler,

    // Documentation agents (Epic 011)
    DocGenerator,

    // Requirements agents (Epic 012)
    RequirementsAnalyzer,

    // Multi-repo agents (Epic 013)
    MultiRepoCoordinator,

    // CI/CD agents (Epic 014)
    CiIntegrator,

    // Incident response agents (Epic 015)
    IncidentResponder,
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
            AgentType::RegressionTester => "regression_tester",
            AgentType::IssueTriager => "issue_triager",
            AgentType::BackgroundController => "background_controller",
            AgentType::Scheduler => "scheduler",
            AgentType::DocGenerator => "doc_generator",
            AgentType::RequirementsAnalyzer => "requirements_analyzer",
            AgentType::MultiRepoCoordinator => "multi_repo_coordinator",
            AgentType::CiIntegrator => "ci_integrator",
            AgentType::IncidentResponder => "incident_responder",
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
            "regression_tester" => Ok(AgentType::RegressionTester),
            "issue_triager" => Ok(AgentType::IssueTriager),
            "background_controller" => Ok(AgentType::BackgroundController),
            "scheduler" => Ok(AgentType::Scheduler),
            "doc_generator" => Ok(AgentType::DocGenerator),
            "requirements_analyzer" => Ok(AgentType::RequirementsAnalyzer),
            "multi_repo_coordinator" => Ok(AgentType::MultiRepoCoordinator),
            "ci_integrator" => Ok(AgentType::CiIntegrator),
            "incident_responder" => Ok(AgentType::IncidentResponder),
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
            AgentType::RegressionTester => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep"]
            }
            AgentType::IssueTriager => vec!["Bash", "Read", "Glob", "Grep"],
            AgentType::BackgroundController => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep", "Task"]
            }
            AgentType::Scheduler => vec!["Bash", "Read"],
            AgentType::DocGenerator => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep"]
            }
            AgentType::RequirementsAnalyzer => vec!["Bash", "Read", "Glob", "Grep"],
            AgentType::MultiRepoCoordinator => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep", "Task"]
            }
            AgentType::CiIntegrator => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep"]
            }
            AgentType::IncidentResponder => {
                vec!["Bash", "Read", "Write", "Edit", "Glob", "Grep", "Task"]
            }
        }
    }

    /// Get maximum turns for this agent type
    pub fn default_max_turns(&self) -> u32 {
        match self {
            AgentType::Explorer => 20,
            AgentType::CodeReviewer => 30,
            AgentType::IssueFixer => 40,
            AgentType::ConflictResolver => 30,
            AgentType::RegressionTester => 50,
            AgentType::IssueTriager => 30,
            AgentType::DocGenerator => 50,
            AgentType::RequirementsAnalyzer => 40,
            AgentType::CiIntegrator => 40,
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

    // ==================== AgentState Tests ====================

    #[test]
    fn test_state_transitions() {
        assert!(AgentState::Created.can_transition_to(AgentState::Initializing));
        assert!(!AgentState::Created.can_transition_to(AgentState::Running));
        assert!(AgentState::Running.can_transition_to(AgentState::Completed));
        assert!(AgentState::Paused.can_transition_to(AgentState::Terminated));
    }

    #[test]
    fn test_state_transitions_from_created() {
        let created = AgentState::Created;
        assert!(created.can_transition_to(AgentState::Initializing));
        assert!(created.can_transition_to(AgentState::Terminated));
        assert!(!created.can_transition_to(AgentState::Running));
        assert!(!created.can_transition_to(AgentState::Completed));
        assert!(!created.can_transition_to(AgentState::Paused));
    }

    #[test]
    fn test_state_transitions_from_running() {
        let running = AgentState::Running;
        assert!(running.can_transition_to(AgentState::WaitingForInput));
        assert!(running.can_transition_to(AgentState::WaitingForExternal));
        assert!(running.can_transition_to(AgentState::Paused));
        assert!(running.can_transition_to(AgentState::Completed));
        assert!(running.can_transition_to(AgentState::Failed));
        assert!(running.can_transition_to(AgentState::Terminated));
        assert!(!running.can_transition_to(AgentState::Created));
        assert!(!running.can_transition_to(AgentState::Initializing));
    }

    #[test]
    fn test_state_transitions_from_paused() {
        let paused = AgentState::Paused;
        assert!(paused.can_transition_to(AgentState::Running));
        assert!(paused.can_transition_to(AgentState::Terminated));
        assert!(!paused.can_transition_to(AgentState::Completed));
        assert!(!paused.can_transition_to(AgentState::Created));
    }

    #[test]
    fn test_terminal_states() {
        assert!(AgentState::Completed.is_terminal());
        assert!(AgentState::Failed.is_terminal());
        assert!(AgentState::Terminated.is_terminal());
        assert!(!AgentState::Created.is_terminal());
        assert!(!AgentState::Running.is_terminal());
        assert!(!AgentState::Paused.is_terminal());
    }

    #[test]
    fn test_accepts_input() {
        assert!(AgentState::Running.accepts_input());
        assert!(AgentState::WaitingForInput.accepts_input());
        assert!(!AgentState::Created.accepts_input());
        assert!(!AgentState::Paused.accepts_input());
        assert!(!AgentState::Completed.accepts_input());
    }

    #[test]
    fn test_state_as_str() {
        assert_eq!(AgentState::Created.as_str(), "created");
        assert_eq!(AgentState::Running.as_str(), "running");
        assert_eq!(AgentState::WaitingForInput.as_str(), "waiting_for_input");
        assert_eq!(AgentState::Completed.as_str(), "completed");
    }

    #[test]
    fn test_state_from_str() {
        assert_eq!(
            AgentState::from_str("created").unwrap(),
            AgentState::Created
        );
        assert_eq!(
            AgentState::from_str("running").unwrap(),
            AgentState::Running
        );
        assert_eq!(
            AgentState::from_str("waiting_for_input").unwrap(),
            AgentState::WaitingForInput
        );
        assert!(AgentState::from_str("invalid").is_err());
    }

    // ==================== AgentType Tests ====================

    #[test]
    fn test_agent_type_as_str() {
        assert_eq!(AgentType::StoryDeveloper.as_str(), "story_developer");
        assert_eq!(AgentType::CodeReviewer.as_str(), "code_reviewer");
        assert_eq!(AgentType::PrShepherd.as_str(), "pr_shepherd");
    }

    #[test]
    fn test_agent_type_from_str() {
        assert_eq!(
            AgentType::from_str("story_developer").unwrap(),
            AgentType::StoryDeveloper
        );
        assert_eq!(
            AgentType::from_str("code_reviewer").unwrap(),
            AgentType::CodeReviewer
        );
        assert!(AgentType::from_str("invalid").is_err());
    }

    #[test]
    fn test_agent_type_default_model() {
        assert_eq!(
            AgentType::Explorer.default_model(),
            "claude-3-haiku-20240307"
        );
        assert_eq!(
            AgentType::StoryDeveloper.default_model(),
            "claude-sonnet-4-20250514"
        );
    }

    #[test]
    fn test_agent_type_allowed_tools() {
        let explorer_tools = AgentType::Explorer.allowed_tools();
        assert!(explorer_tools.contains(&"Read"));
        assert!(explorer_tools.contains(&"Glob"));
        assert!(!explorer_tools.contains(&"Write"));

        let developer_tools = AgentType::StoryDeveloper.allowed_tools();
        assert!(developer_tools.contains(&"Write"));
        assert!(developer_tools.contains(&"Edit"));
        assert!(developer_tools.contains(&"Task"));
    }

    #[test]
    fn test_agent_type_max_turns() {
        assert_eq!(AgentType::Explorer.default_max_turns(), 20);
        assert_eq!(AgentType::CodeReviewer.default_max_turns(), 30);
        assert_eq!(AgentType::StoryDeveloper.default_max_turns(), 80);
    }

    // ==================== Agent Tests ====================

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new(AgentType::StoryDeveloper, "Implement auth feature");
        assert_eq!(agent.state, AgentState::Created);
        assert_eq!(agent.task, "Implement auth feature");
        assert_eq!(agent.agent_type, AgentType::StoryDeveloper);
        assert!(agent.error_message.is_none());
        assert!(agent.completed_at.is_none());
    }

    #[test]
    fn test_agent_with_context() {
        let context = AgentContext {
            epic_id: Some("epic-1".to_string()),
            story_id: Some("story-1".to_string()),
            pr_number: Some(42),
            branch_name: Some("feature/auth".to_string()),
            working_directory: Some("/tmp/work".to_string()),
            custom: serde_json::json!({"key": "value"}),
        };

        let agent = Agent::new(AgentType::StoryDeveloper, "Test task").with_context(context);
        assert_eq!(agent.context.epic_id, Some("epic-1".to_string()));
        assert_eq!(agent.context.pr_number, Some(42));
    }

    #[test]
    fn test_agent_with_parent() {
        let parent_id = Uuid::new_v4();
        let agent = Agent::new(AgentType::Explorer, "Sub task").with_parent(parent_id);
        assert_eq!(agent.parent_agent_id, Some(parent_id));
    }

    #[test]
    fn test_agent_with_worktree() {
        let agent = Agent::new(AgentType::StoryDeveloper, "Task").with_worktree("worktree-123");
        assert_eq!(agent.worktree_id, Some("worktree-123".to_string()));
    }

    #[test]
    fn test_agent_transition_success() {
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Task");
        assert!(agent.transition_to(AgentState::Initializing).is_ok());
        assert_eq!(agent.state, AgentState::Initializing);

        assert!(agent.transition_to(AgentState::Running).is_ok());
        assert_eq!(agent.state, AgentState::Running);

        assert!(agent.transition_to(AgentState::Completed).is_ok());
        assert_eq!(agent.state, AgentState::Completed);
        assert!(agent.completed_at.is_some());
    }

    #[test]
    fn test_agent_transition_failure() {
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Task");
        // Cannot go directly from Created to Running
        assert!(agent.transition_to(AgentState::Running).is_err());
        assert_eq!(agent.state, AgentState::Created);
    }

    #[test]
    fn test_agent_fail() {
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Task");
        agent.transition_to(AgentState::Initializing).unwrap();
        agent.transition_to(AgentState::Running).unwrap();

        assert!(agent.fail("Something went wrong").is_ok());
        assert_eq!(agent.state, AgentState::Failed);
        assert_eq!(
            agent.error_message,
            Some("Something went wrong".to_string())
        );
        assert!(agent.completed_at.is_some());
    }

    #[test]
    fn test_agent_terminate_from_any_state() {
        // Test that any state can transition to Terminated
        let states = [
            AgentState::Created,
            AgentState::Initializing,
            AgentState::Running,
            AgentState::WaitingForInput,
            AgentState::WaitingForExternal,
            AgentState::Paused,
        ];

        for state in states {
            assert!(
                state.can_transition_to(AgentState::Terminated),
                "Should be able to terminate from {:?}",
                state
            );
        }
    }

    #[test]
    fn test_agent_updated_at_changes() {
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Task");
        let initial_updated_at = agent.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        agent.transition_to(AgentState::Initializing).unwrap();
        assert!(agent.updated_at > initial_updated_at);
    }
}
