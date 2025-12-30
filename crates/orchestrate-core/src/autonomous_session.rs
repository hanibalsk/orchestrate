//! Autonomous Session Management
//!
//! Types and utilities for tracking autonomous processing sessions.
//! Enables fully autonomous workflow orchestration from epic discovery
//! through PR merge.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Autonomous session states in the lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousSessionState {
    /// Session created but not started
    Idle,
    /// Analyzing project state and requirements
    Analyzing,
    /// Discovering epics and stories
    Discovering,
    /// Planning work execution order
    Planning,
    /// Executing development work
    Executing,
    /// Reviewing completed work
    Reviewing,
    /// Creating pull request
    PrCreation,
    /// Monitoring PR status (CI, reviews)
    PrMonitoring,
    /// Fixing PR issues
    PrFixing,
    /// Merging approved PR
    PrMerging,
    /// Completing session cleanup
    Completing,
    /// Session completed successfully
    Done,
    /// Session blocked, requires intervention
    Blocked,
    /// Session paused by user
    Paused,
}

impl AutonomousSessionState {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Analyzing => "analyzing",
            Self::Discovering => "discovering",
            Self::Planning => "planning",
            Self::Executing => "executing",
            Self::Reviewing => "reviewing",
            Self::PrCreation => "pr_creation",
            Self::PrMonitoring => "pr_monitoring",
            Self::PrFixing => "pr_fixing",
            Self::PrMerging => "pr_merging",
            Self::Completing => "completing",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Paused => "paused",
        }
    }


    /// Check if this state allows transition to another state
    pub fn can_transition_to(&self, target: AutonomousSessionState) -> bool {
        use AutonomousSessionState::*;
        match (self, target) {
            // Idle can go to Analyzing or be paused
            (Idle, Analyzing) => true,

            // Main workflow progression
            (Analyzing, Discovering) | (Analyzing, Blocked) => true,
            (Discovering, Planning) | (Discovering, Blocked) => true,
            (Planning, Executing) | (Planning, Blocked) => true,
            (Executing, Reviewing) | (Executing, Blocked) => true,
            (Reviewing, PrCreation) | (Reviewing, Executing) | (Reviewing, Blocked) => true,
            (PrCreation, PrMonitoring) | (PrCreation, Blocked) => true,
            (PrMonitoring, PrFixing) | (PrMonitoring, PrMerging) | (PrMonitoring, Blocked) => true,
            (PrFixing, PrMonitoring) | (PrFixing, Blocked) => true,
            (PrMerging, Completing) | (PrMerging, Blocked) => true,
            (Completing, Done) | (Completing, Discovering) | (Completing, Blocked) => true,

            // Any active state can be paused
            (Idle, Paused)
            | (Analyzing, Paused)
            | (Discovering, Paused)
            | (Planning, Paused)
            | (Executing, Paused)
            | (Reviewing, Paused)
            | (PrCreation, Paused)
            | (PrMonitoring, Paused)
            | (PrFixing, Paused)
            | (PrMerging, Paused)
            | (Completing, Paused) => true,

            // Paused can resume to Analyzing (restart) or go to Done
            (Paused, Analyzing) | (Paused, Done) => true,

            // Blocked can be unblocked to resume or go to Done
            (Blocked, Analyzing) | (Blocked, Done) | (Blocked, Paused) => true,

            // No other transitions allowed
            _ => false,
        }
    }

    /// Check if session is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done)
    }

    /// Check if session is in an active processing state
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Analyzing
                | Self::Discovering
                | Self::Planning
                | Self::Executing
                | Self::Reviewing
                | Self::PrCreation
                | Self::PrMonitoring
                | Self::PrFixing
                | Self::PrMerging
                | Self::Completing
        )
    }

    /// Check if session can accept new work
    pub fn can_accept_work(&self) -> bool {
        matches!(self, Self::Idle | Self::Planning | Self::Executing)
    }
}

impl std::str::FromStr for AutonomousSessionState {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(Self::Idle),
            "analyzing" => Ok(Self::Analyzing),
            "discovering" => Ok(Self::Discovering),
            "planning" => Ok(Self::Planning),
            "executing" => Ok(Self::Executing),
            "reviewing" => Ok(Self::Reviewing),
            "pr_creation" => Ok(Self::PrCreation),
            "pr_monitoring" => Ok(Self::PrMonitoring),
            "pr_fixing" => Ok(Self::PrFixing),
            "pr_merging" => Ok(Self::PrMerging),
            "completing" => Ok(Self::Completing),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            "paused" => Ok(Self::Paused),
            _ => Err(crate::Error::Other(format!(
                "Unknown autonomous session state: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for AutonomousSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Session metrics tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetrics {
    /// Number of stories completed successfully
    pub stories_completed: u32,
    /// Number of stories that failed
    pub stories_failed: u32,
    /// Number of reviews that passed
    pub reviews_passed: u32,
    /// Number of reviews that failed/required changes
    pub reviews_failed: u32,
    /// Total number of iterations (story attempts + review cycles)
    pub total_iterations: u32,
    /// Total number of agents spawned
    pub agents_spawned: u32,
    /// Total tokens used
    pub tokens_used: u64,
    /// Time spent in each state (in seconds)
    #[serde(default)]
    pub state_durations: std::collections::HashMap<String, u64>,
}

impl SessionMetrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a story completion
    pub fn record_story_completed(&mut self) {
        self.stories_completed += 1;
        self.total_iterations += 1;
    }

    /// Record a story failure
    pub fn record_story_failed(&mut self) {
        self.stories_failed += 1;
        self.total_iterations += 1;
    }

    /// Record a review pass
    pub fn record_review_passed(&mut self) {
        self.reviews_passed += 1;
        self.total_iterations += 1;
    }

    /// Record a review failure
    pub fn record_review_failed(&mut self) {
        self.reviews_failed += 1;
        self.total_iterations += 1;
    }

    /// Record an agent spawn
    pub fn record_agent_spawned(&mut self) {
        self.agents_spawned += 1;
    }

    /// Add tokens used
    pub fn add_tokens(&mut self, tokens: u64) {
        self.tokens_used += tokens;
    }

    /// Record time spent in a state
    pub fn record_state_duration(&mut self, state: &str, seconds: u64) {
        *self.state_durations.entry(state.to_string()).or_insert(0) += seconds;
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.stories_completed + self.stories_failed;
        if total == 0 {
            0.0
        } else {
            self.stories_completed as f64 / total as f64
        }
    }

    /// Calculate review pass rate
    pub fn review_pass_rate(&self) -> f64 {
        let total = self.reviews_passed + self.reviews_failed;
        if total == 0 {
            0.0
        } else {
            self.reviews_passed as f64 / total as f64
        }
    }
}

/// Work item in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    /// Work item ID
    pub id: String,
    /// Type of work
    pub work_type: WorkItemType,
    /// Epic ID
    pub epic_id: String,
    /// Story ID (optional)
    pub story_id: Option<String>,
    /// Priority (lower is higher priority)
    pub priority: u32,
    /// Dependencies (IDs of work items that must complete first)
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Type of work item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemType {
    /// Develop a story
    Story,
    /// Review completed work
    Review,
    /// Create a PR
    CreatePr,
    /// Fix PR issues
    FixPr,
    /// Merge a PR
    MergePr,
}

impl WorkItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Story => "story",
            Self::Review => "review",
            Self::CreatePr => "create_pr",
            Self::FixPr => "fix_pr",
            Self::MergePr => "merge_pr",
        }
    }
}

impl std::fmt::Display for WorkItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Completed work item record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedItem {
    /// Work item ID
    pub id: String,
    /// Type of work
    pub work_type: WorkItemType,
    /// Epic ID
    pub epic_id: String,
    /// Story ID (optional)
    pub story_id: Option<String>,
    /// Whether it succeeded
    pub success: bool,
    /// Completion timestamp
    pub completed_at: DateTime<Utc>,
    /// Error message if failed
    pub error: Option<String>,
    /// Agent ID that completed the work
    pub agent_id: Option<String>,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Maximum number of concurrent agents
    #[serde(default = "default_max_agents")]
    pub max_agents: u32,
    /// Epic pattern to match (e.g., "epic-016-*")
    pub epic_pattern: Option<String>,
    /// Preferred model for execution
    pub model: Option<String>,
    /// Whether this is a dry run
    #[serde(default)]
    pub dry_run: bool,
    /// Maximum retries per story
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Auto-merge approved PRs
    #[serde(default = "default_auto_merge")]
    pub auto_merge: bool,
}

fn default_max_agents() -> u32 {
    1
}

fn default_max_retries() -> u32 {
    3
}

fn default_auto_merge() -> bool {
    true
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_agents: default_max_agents(),
            epic_pattern: None,
            model: None,
            dry_run: false,
            max_retries: default_max_retries(),
            auto_merge: default_auto_merge(),
        }
    }
}

/// An autonomous processing session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousSession {
    /// Unique session ID
    pub id: String,
    /// Current state
    pub state: AutonomousSessionState,
    /// Session start timestamp
    pub started_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Current epic being processed
    pub current_epic_id: Option<String>,
    /// Current story being processed
    pub current_story_id: Option<String>,
    /// Current agent working on the session
    pub current_agent_id: Option<String>,
    /// Session configuration
    pub config: SessionConfig,
    /// Pending work queue
    pub work_queue: Vec<WorkItem>,
    /// Completed work items
    pub completed_items: Vec<CompletedItem>,
    /// Session metrics
    pub metrics: SessionMetrics,
    /// Error message if blocked
    pub error_message: Option<String>,
    /// Reason for being blocked
    pub blocked_reason: Option<String>,
    /// Reason for being paused
    pub pause_reason: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl AutonomousSession {
    /// Create a new autonomous session
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            state: AutonomousSessionState::Idle,
            started_at: now,
            updated_at: now,
            completed_at: None,
            current_epic_id: None,
            current_story_id: None,
            current_agent_id: None,
            config: SessionConfig::default(),
            work_queue: Vec::new(),
            completed_items: Vec::new(),
            metrics: SessionMetrics::new(),
            error_message: None,
            blocked_reason: None,
            pause_reason: None,
            created_at: now,
        }
    }

    /// Create a new session with specific ID
    pub fn with_id(id: impl Into<String>) -> Self {
        let mut session = Self::new();
        session.id = id.into();
        session
    }

    /// Set session configuration
    pub fn with_config(mut self, config: SessionConfig) -> Self {
        self.config = config;
        self
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: AutonomousSessionState) -> crate::Result<()> {
        if !self.state.can_transition_to(new_state) {
            return Err(crate::Error::InvalidStateTransition(
                self.state.as_str().to_string(),
                new_state.as_str().to_string(),
            ));
        }

        self.state = new_state;
        self.updated_at = Utc::now();

        if new_state.is_terminal() {
            self.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    /// Start the session (transition from Idle to Analyzing)
    pub fn start(&mut self) -> crate::Result<()> {
        self.transition_to(AutonomousSessionState::Analyzing)?;
        self.started_at = Utc::now();
        Ok(())
    }

    /// Pause the session
    pub fn pause(&mut self, reason: impl Into<String>) -> crate::Result<()> {
        self.transition_to(AutonomousSessionState::Paused)?;
        self.pause_reason = Some(reason.into());
        Ok(())
    }

    /// Resume a paused session
    pub fn resume(&mut self) -> crate::Result<()> {
        if self.state != AutonomousSessionState::Paused {
            return Err(crate::Error::InvalidStateTransition(
                self.state.as_str().to_string(),
                "analyzing".to_string(),
            ));
        }
        self.transition_to(AutonomousSessionState::Analyzing)?;
        self.pause_reason = None;
        Ok(())
    }

    /// Block the session with a reason
    pub fn block(&mut self, reason: impl Into<String>) -> crate::Result<()> {
        let reason_str = reason.into();
        self.blocked_reason = Some(reason_str.clone());
        self.error_message = Some(reason_str);
        self.transition_to(AutonomousSessionState::Blocked)
    }

    /// Unblock the session
    pub fn unblock(&mut self) -> crate::Result<()> {
        if self.state != AutonomousSessionState::Blocked {
            return Err(crate::Error::InvalidStateTransition(
                self.state.as_str().to_string(),
                "analyzing".to_string(),
            ));
        }
        self.transition_to(AutonomousSessionState::Analyzing)?;
        self.blocked_reason = None;
        self.error_message = None;
        Ok(())
    }

    /// Complete the session
    pub fn complete(&mut self) -> crate::Result<()> {
        self.transition_to(AutonomousSessionState::Done)
    }

    /// Set current epic
    pub fn set_current_epic(&mut self, epic_id: impl Into<String>) {
        self.current_epic_id = Some(epic_id.into());
        self.updated_at = Utc::now();
    }

    /// Set current story
    pub fn set_current_story(&mut self, story_id: impl Into<String>) {
        self.current_story_id = Some(story_id.into());
        self.updated_at = Utc::now();
    }

    /// Set current agent
    pub fn set_current_agent(&mut self, agent_id: impl Into<String>) {
        self.current_agent_id = Some(agent_id.into());
        self.updated_at = Utc::now();
    }

    /// Add a work item to the queue
    pub fn add_work_item(&mut self, item: WorkItem) {
        self.work_queue.push(item);
        self.updated_at = Utc::now();
    }

    /// Pop the next work item from the queue (highest priority)
    ///
    /// Uses O(n) min-search with swap_remove instead of O(n log n) sort.
    pub fn pop_work_item(&mut self) -> Option<WorkItem> {
        if self.work_queue.is_empty() {
            return None;
        }
        // Find the index of the item with lowest priority value (highest priority)
        // This is O(n) vs O(n log n) for sorting on every pop
        let min_idx = self
            .work_queue
            .iter()
            .enumerate()
            .min_by_key(|(_, item)| item.priority)
            .map(|(idx, _)| idx)?;

        // swap_remove is O(1) - swaps with last element and pops
        let item = self.work_queue.swap_remove(min_idx);
        self.updated_at = Utc::now();
        Some(item)
    }

    /// Record a completed work item
    pub fn record_completed(&mut self, item: CompletedItem) {
        if item.success {
            if matches!(item.work_type, WorkItemType::Story) {
                self.metrics.record_story_completed();
            } else if matches!(item.work_type, WorkItemType::Review) {
                self.metrics.record_review_passed();
            }
        } else if matches!(item.work_type, WorkItemType::Story) {
            self.metrics.record_story_failed();
        } else if matches!(item.work_type, WorkItemType::Review) {
            self.metrics.record_review_failed();
        }
        self.completed_items.push(item);
        self.updated_at = Utc::now();
    }

    /// Get the duration since session started
    pub fn duration(&self) -> chrono::Duration {
        let end_time = self.completed_at.unwrap_or_else(Utc::now);
        end_time - self.started_at
    }

    /// Check if the session has pending work
    pub fn has_pending_work(&self) -> bool {
        !self.work_queue.is_empty()
    }
}

impl Default for AutonomousSession {
    fn default() -> Self {
        Self::new()
    }
}

/// State transition history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStateHistory {
    /// History entry ID
    pub id: i64,
    /// Session ID
    pub session_id: String,
    /// Previous state
    pub from_state: AutonomousSessionState,
    /// New state
    pub to_state: AutonomousSessionState,
    /// Reason for transition
    pub reason: Option<String>,
    /// Transition timestamp
    pub transitioned_at: DateTime<Utc>,
    /// Additional metadata
    pub metadata: serde_json::Value,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== AutonomousSessionState Tests ====================

    #[test]
    fn test_state_as_str() {
        assert_eq!(AutonomousSessionState::Idle.as_str(), "idle");
        assert_eq!(AutonomousSessionState::Analyzing.as_str(), "analyzing");
        assert_eq!(AutonomousSessionState::Discovering.as_str(), "discovering");
        assert_eq!(AutonomousSessionState::Planning.as_str(), "planning");
        assert_eq!(AutonomousSessionState::Executing.as_str(), "executing");
        assert_eq!(AutonomousSessionState::Reviewing.as_str(), "reviewing");
        assert_eq!(AutonomousSessionState::PrCreation.as_str(), "pr_creation");
        assert_eq!(AutonomousSessionState::PrMonitoring.as_str(), "pr_monitoring");
        assert_eq!(AutonomousSessionState::PrFixing.as_str(), "pr_fixing");
        assert_eq!(AutonomousSessionState::PrMerging.as_str(), "pr_merging");
        assert_eq!(AutonomousSessionState::Completing.as_str(), "completing");
        assert_eq!(AutonomousSessionState::Done.as_str(), "done");
        assert_eq!(AutonomousSessionState::Blocked.as_str(), "blocked");
        assert_eq!(AutonomousSessionState::Paused.as_str(), "paused");
    }

    #[test]
    fn test_state_from_str() {
        assert_eq!(
            "idle".parse::<AutonomousSessionState>().unwrap(),
            AutonomousSessionState::Idle
        );
        assert_eq!(
            "analyzing".parse::<AutonomousSessionState>().unwrap(),
            AutonomousSessionState::Analyzing
        );
        assert_eq!(
            "pr_creation".parse::<AutonomousSessionState>().unwrap(),
            AutonomousSessionState::PrCreation
        );
        assert_eq!(
            "done".parse::<AutonomousSessionState>().unwrap(),
            AutonomousSessionState::Done
        );
        assert!("invalid".parse::<AutonomousSessionState>().is_err());
    }

    #[test]
    fn test_state_transitions_main_workflow() {
        use AutonomousSessionState::*;

        // Main workflow path
        assert!(Idle.can_transition_to(Analyzing));
        assert!(Analyzing.can_transition_to(Discovering));
        assert!(Discovering.can_transition_to(Planning));
        assert!(Planning.can_transition_to(Executing));
        assert!(Executing.can_transition_to(Reviewing));
        assert!(Reviewing.can_transition_to(PrCreation));
        assert!(PrCreation.can_transition_to(PrMonitoring));
        assert!(PrMonitoring.can_transition_to(PrMerging));
        assert!(PrMerging.can_transition_to(Completing));
        assert!(Completing.can_transition_to(Done));
    }

    #[test]
    fn test_state_transitions_pr_fix_cycle() {
        use AutonomousSessionState::*;

        // PR fix cycle
        assert!(PrMonitoring.can_transition_to(PrFixing));
        assert!(PrFixing.can_transition_to(PrMonitoring));
    }

    #[test]
    fn test_state_transitions_review_iteration() {
        use AutonomousSessionState::*;

        // Review iteration (back to executing if review fails)
        assert!(Reviewing.can_transition_to(Executing));
    }

    #[test]
    fn test_state_transitions_to_blocked() {
        use AutonomousSessionState::*;

        // Any active state can transition to Blocked
        assert!(Analyzing.can_transition_to(Blocked));
        assert!(Discovering.can_transition_to(Blocked));
        assert!(Planning.can_transition_to(Blocked));
        assert!(Executing.can_transition_to(Blocked));
        assert!(Reviewing.can_transition_to(Blocked));
        assert!(PrCreation.can_transition_to(Blocked));
        assert!(PrMonitoring.can_transition_to(Blocked));
        assert!(PrFixing.can_transition_to(Blocked));
        assert!(PrMerging.can_transition_to(Blocked));
        assert!(Completing.can_transition_to(Blocked));
    }

    #[test]
    fn test_state_transitions_to_paused() {
        use AutonomousSessionState::*;

        // Any active state can be paused
        assert!(Idle.can_transition_to(Paused));
        assert!(Analyzing.can_transition_to(Paused));
        assert!(Executing.can_transition_to(Paused));
        assert!(PrMonitoring.can_transition_to(Paused));
    }

    #[test]
    fn test_state_transitions_from_paused() {
        use AutonomousSessionState::*;

        // Paused can resume to Analyzing or go to Done
        assert!(Paused.can_transition_to(Analyzing));
        assert!(Paused.can_transition_to(Done));
    }

    #[test]
    fn test_state_transitions_from_blocked() {
        use AutonomousSessionState::*;

        // Blocked can be unblocked or terminated
        assert!(Blocked.can_transition_to(Analyzing));
        assert!(Blocked.can_transition_to(Done));
        assert!(Blocked.can_transition_to(Paused));
    }

    #[test]
    fn test_state_transitions_invalid() {
        use AutonomousSessionState::*;

        // Invalid transitions
        assert!(!Idle.can_transition_to(Done));
        assert!(!Idle.can_transition_to(Executing));
        assert!(!Done.can_transition_to(Idle));
        assert!(!Done.can_transition_to(Analyzing));
    }

    #[test]
    fn test_state_is_terminal() {
        assert!(AutonomousSessionState::Done.is_terminal());
        assert!(!AutonomousSessionState::Idle.is_terminal());
        assert!(!AutonomousSessionState::Blocked.is_terminal());
        assert!(!AutonomousSessionState::Paused.is_terminal());
    }

    #[test]
    fn test_state_is_active() {
        assert!(AutonomousSessionState::Analyzing.is_active());
        assert!(AutonomousSessionState::Executing.is_active());
        assert!(AutonomousSessionState::PrMonitoring.is_active());
        assert!(!AutonomousSessionState::Idle.is_active());
        assert!(!AutonomousSessionState::Done.is_active());
        assert!(!AutonomousSessionState::Blocked.is_active());
        assert!(!AutonomousSessionState::Paused.is_active());
    }

    #[test]
    fn test_state_can_accept_work() {
        assert!(AutonomousSessionState::Idle.can_accept_work());
        assert!(AutonomousSessionState::Planning.can_accept_work());
        assert!(AutonomousSessionState::Executing.can_accept_work());
        assert!(!AutonomousSessionState::Analyzing.can_accept_work());
        assert!(!AutonomousSessionState::Done.can_accept_work());
    }

    // ==================== SessionMetrics Tests ====================

    #[test]
    fn test_metrics_new() {
        let metrics = SessionMetrics::new();
        assert_eq!(metrics.stories_completed, 0);
        assert_eq!(metrics.stories_failed, 0);
        assert_eq!(metrics.reviews_passed, 0);
        assert_eq!(metrics.reviews_failed, 0);
        assert_eq!(metrics.total_iterations, 0);
    }

    #[test]
    fn test_metrics_record_story_completed() {
        let mut metrics = SessionMetrics::new();
        metrics.record_story_completed();
        metrics.record_story_completed();

        assert_eq!(metrics.stories_completed, 2);
        assert_eq!(metrics.total_iterations, 2);
    }

    #[test]
    fn test_metrics_record_story_failed() {
        let mut metrics = SessionMetrics::new();
        metrics.record_story_failed();

        assert_eq!(metrics.stories_failed, 1);
        assert_eq!(metrics.total_iterations, 1);
    }

    #[test]
    fn test_metrics_record_review_passed() {
        let mut metrics = SessionMetrics::new();
        metrics.record_review_passed();

        assert_eq!(metrics.reviews_passed, 1);
        assert_eq!(metrics.total_iterations, 1);
    }

    #[test]
    fn test_metrics_record_review_failed() {
        let mut metrics = SessionMetrics::new();
        metrics.record_review_failed();

        assert_eq!(metrics.reviews_failed, 1);
        assert_eq!(metrics.total_iterations, 1);
    }

    #[test]
    fn test_metrics_success_rate() {
        let mut metrics = SessionMetrics::new();
        assert_eq!(metrics.success_rate(), 0.0);

        metrics.record_story_completed();
        metrics.record_story_completed();
        metrics.record_story_failed();

        assert!((metrics.success_rate() - 0.666666).abs() < 0.001);
    }

    #[test]
    fn test_metrics_review_pass_rate() {
        let mut metrics = SessionMetrics::new();
        assert_eq!(metrics.review_pass_rate(), 0.0);

        metrics.record_review_passed();
        metrics.record_review_passed();
        metrics.record_review_passed();
        metrics.record_review_failed();

        assert_eq!(metrics.review_pass_rate(), 0.75);
    }

    #[test]
    fn test_metrics_agent_spawned() {
        let mut metrics = SessionMetrics::new();
        metrics.record_agent_spawned();
        metrics.record_agent_spawned();

        assert_eq!(metrics.agents_spawned, 2);
    }

    #[test]
    fn test_metrics_tokens() {
        let mut metrics = SessionMetrics::new();
        metrics.add_tokens(1000);
        metrics.add_tokens(500);

        assert_eq!(metrics.tokens_used, 1500);
    }

    #[test]
    fn test_metrics_state_duration() {
        let mut metrics = SessionMetrics::new();
        metrics.record_state_duration("executing", 100);
        metrics.record_state_duration("executing", 50);
        metrics.record_state_duration("reviewing", 30);

        assert_eq!(metrics.state_durations.get("executing"), Some(&150));
        assert_eq!(metrics.state_durations.get("reviewing"), Some(&30));
    }

    // ==================== SessionConfig Tests ====================

    #[test]
    fn test_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.max_agents, 1);
        assert_eq!(config.max_retries, 3);
        assert!(config.auto_merge);
        assert!(!config.dry_run);
        assert!(config.epic_pattern.is_none());
        assert!(config.model.is_none());
    }

    // ==================== AutonomousSession Tests ====================

    #[test]
    fn test_session_new() {
        let session = AutonomousSession::new();

        assert_eq!(session.state, AutonomousSessionState::Idle);
        assert!(session.current_epic_id.is_none());
        assert!(session.current_story_id.is_none());
        assert!(session.current_agent_id.is_none());
        assert!(session.completed_at.is_none());
        assert!(session.work_queue.is_empty());
        assert!(session.completed_items.is_empty());
    }

    #[test]
    fn test_session_with_id() {
        let session = AutonomousSession::with_id("test-session-123");
        assert_eq!(session.id, "test-session-123");
    }

    #[test]
    fn test_session_with_config() {
        let config = SessionConfig {
            max_agents: 5,
            epic_pattern: Some("epic-016-*".to_string()),
            ..Default::default()
        };
        let session = AutonomousSession::new().with_config(config);

        assert_eq!(session.config.max_agents, 5);
        assert_eq!(session.config.epic_pattern, Some("epic-016-*".to_string()));
    }

    #[test]
    fn test_session_start() {
        let mut session = AutonomousSession::new();
        assert!(session.start().is_ok());
        assert_eq!(session.state, AutonomousSessionState::Analyzing);
    }

    #[test]
    fn test_session_transition_success() {
        let mut session = AutonomousSession::new();

        assert!(session.transition_to(AutonomousSessionState::Analyzing).is_ok());
        assert_eq!(session.state, AutonomousSessionState::Analyzing);

        assert!(session.transition_to(AutonomousSessionState::Discovering).is_ok());
        assert_eq!(session.state, AutonomousSessionState::Discovering);
    }

    #[test]
    fn test_session_transition_failure() {
        let mut session = AutonomousSession::new();

        // Cannot go directly from Idle to Done
        assert!(session.transition_to(AutonomousSessionState::Done).is_err());
        assert_eq!(session.state, AutonomousSessionState::Idle);
    }

    #[test]
    fn test_session_pause_resume() {
        let mut session = AutonomousSession::new();
        session.start().unwrap();

        // Pause
        assert!(session.pause("User requested pause").is_ok());
        assert_eq!(session.state, AutonomousSessionState::Paused);
        assert_eq!(session.pause_reason, Some("User requested pause".to_string()));

        // Resume
        assert!(session.resume().is_ok());
        assert_eq!(session.state, AutonomousSessionState::Analyzing);
        assert!(session.pause_reason.is_none());
    }

    #[test]
    fn test_session_block_unblock() {
        let mut session = AutonomousSession::new();
        session.start().unwrap();

        // Block
        assert!(session.block("CI failure").is_ok());
        assert_eq!(session.state, AutonomousSessionState::Blocked);
        assert_eq!(session.blocked_reason, Some("CI failure".to_string()));
        assert_eq!(session.error_message, Some("CI failure".to_string()));

        // Unblock
        assert!(session.unblock().is_ok());
        assert_eq!(session.state, AutonomousSessionState::Analyzing);
        assert!(session.blocked_reason.is_none());
        assert!(session.error_message.is_none());
    }

    #[test]
    fn test_session_complete() {
        let mut session = AutonomousSession::new();
        session.start().unwrap();
        session.transition_to(AutonomousSessionState::Discovering).unwrap();
        session.transition_to(AutonomousSessionState::Planning).unwrap();
        session.transition_to(AutonomousSessionState::Executing).unwrap();
        session.transition_to(AutonomousSessionState::Reviewing).unwrap();
        session.transition_to(AutonomousSessionState::PrCreation).unwrap();
        session.transition_to(AutonomousSessionState::PrMonitoring).unwrap();
        session.transition_to(AutonomousSessionState::PrMerging).unwrap();
        session.transition_to(AutonomousSessionState::Completing).unwrap();

        assert!(session.complete().is_ok());
        assert_eq!(session.state, AutonomousSessionState::Done);
        assert!(session.completed_at.is_some());
    }

    #[test]
    fn test_session_set_current_epic() {
        let mut session = AutonomousSession::new();
        let initial_updated = session.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.set_current_epic("epic-016");

        assert_eq!(session.current_epic_id, Some("epic-016".to_string()));
        assert!(session.updated_at > initial_updated);
    }

    #[test]
    fn test_session_set_current_story() {
        let mut session = AutonomousSession::new();
        session.set_current_story("story-1");
        assert_eq!(session.current_story_id, Some("story-1".to_string()));
    }

    #[test]
    fn test_session_set_current_agent() {
        let mut session = AutonomousSession::new();
        session.set_current_agent("agent-abc");
        assert_eq!(session.current_agent_id, Some("agent-abc".to_string()));
    }

    #[test]
    fn test_session_work_queue() {
        let mut session = AutonomousSession::new();

        session.add_work_item(WorkItem {
            id: "work-1".to_string(),
            work_type: WorkItemType::Story,
            epic_id: "epic-1".to_string(),
            story_id: Some("story-1".to_string()),
            priority: 2,
            dependencies: vec![],
            metadata: serde_json::Value::Null,
        });

        session.add_work_item(WorkItem {
            id: "work-2".to_string(),
            work_type: WorkItemType::Story,
            epic_id: "epic-1".to_string(),
            story_id: Some("story-2".to_string()),
            priority: 1,
            dependencies: vec![],
            metadata: serde_json::Value::Null,
        });

        assert!(session.has_pending_work());
        assert_eq!(session.work_queue.len(), 2);

        // Pop should return highest priority (lowest number)
        let item = session.pop_work_item().unwrap();
        assert_eq!(item.id, "work-2");
        assert_eq!(item.priority, 1);

        let item = session.pop_work_item().unwrap();
        assert_eq!(item.id, "work-1");

        assert!(!session.has_pending_work());
        assert!(session.pop_work_item().is_none());
    }

    #[test]
    fn test_session_record_completed() {
        let mut session = AutonomousSession::new();

        // Record successful story
        session.record_completed(CompletedItem {
            id: "work-1".to_string(),
            work_type: WorkItemType::Story,
            epic_id: "epic-1".to_string(),
            story_id: Some("story-1".to_string()),
            success: true,
            completed_at: Utc::now(),
            error: None,
            agent_id: Some("agent-1".to_string()),
        });

        assert_eq!(session.completed_items.len(), 1);
        assert_eq!(session.metrics.stories_completed, 1);
        assert_eq!(session.metrics.stories_failed, 0);

        // Record failed story
        session.record_completed(CompletedItem {
            id: "work-2".to_string(),
            work_type: WorkItemType::Story,
            epic_id: "epic-1".to_string(),
            story_id: Some("story-2".to_string()),
            success: false,
            completed_at: Utc::now(),
            error: Some("Test failure".to_string()),
            agent_id: Some("agent-2".to_string()),
        });

        assert_eq!(session.completed_items.len(), 2);
        assert_eq!(session.metrics.stories_completed, 1);
        assert_eq!(session.metrics.stories_failed, 1);
    }

    #[test]
    fn test_session_duration() {
        let session = AutonomousSession::new();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let duration = session.duration();
        assert!(duration.num_milliseconds() >= 50);
    }

    // ==================== WorkItem Tests ====================

    #[test]
    fn test_work_item_type_as_str() {
        assert_eq!(WorkItemType::Story.as_str(), "story");
        assert_eq!(WorkItemType::Review.as_str(), "review");
        assert_eq!(WorkItemType::CreatePr.as_str(), "create_pr");
        assert_eq!(WorkItemType::FixPr.as_str(), "fix_pr");
        assert_eq!(WorkItemType::MergePr.as_str(), "merge_pr");
    }

    // ==================== Full Workflow Test ====================

    #[test]
    fn test_full_session_workflow() {
        let mut session = AutonomousSession::new();

        // Start session
        session.start().unwrap();
        assert_eq!(session.state, AutonomousSessionState::Analyzing);

        // Progress through workflow
        session.transition_to(AutonomousSessionState::Discovering).unwrap();
        session.set_current_epic("epic-016");

        session.transition_to(AutonomousSessionState::Planning).unwrap();

        // Add work items
        session.add_work_item(WorkItem {
            id: "story-1".to_string(),
            work_type: WorkItemType::Story,
            epic_id: "epic-016".to_string(),
            story_id: Some("story-1".to_string()),
            priority: 1,
            dependencies: vec![],
            metadata: serde_json::Value::Null,
        });

        session.transition_to(AutonomousSessionState::Executing).unwrap();

        // Execute work
        let work = session.pop_work_item().unwrap();
        session.set_current_story(&work.story_id.clone().unwrap());
        session.metrics.record_agent_spawned();

        // Complete work
        session.record_completed(CompletedItem {
            id: work.id,
            work_type: work.work_type,
            epic_id: work.epic_id,
            story_id: work.story_id,
            success: true,
            completed_at: Utc::now(),
            error: None,
            agent_id: Some("agent-1".to_string()),
        });

        // Continue through PR workflow
        session.transition_to(AutonomousSessionState::Reviewing).unwrap();
        session.metrics.record_review_passed();

        session.transition_to(AutonomousSessionState::PrCreation).unwrap();
        session.transition_to(AutonomousSessionState::PrMonitoring).unwrap();
        session.transition_to(AutonomousSessionState::PrMerging).unwrap();
        session.transition_to(AutonomousSessionState::Completing).unwrap();

        session.complete().unwrap();

        // Verify final state
        assert_eq!(session.state, AutonomousSessionState::Done);
        assert!(session.completed_at.is_some());
        assert_eq!(session.metrics.stories_completed, 1);
        assert_eq!(session.metrics.reviews_passed, 1);
        assert_eq!(session.metrics.agents_spawned, 1);
    }
}
