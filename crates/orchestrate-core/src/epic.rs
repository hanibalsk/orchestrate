//! BMAD Epic and Story types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Epic status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EpicStatus {
    /// Epic is pending, not started
    Pending,
    /// Epic is in progress
    InProgress,
    /// Epic is completed
    Completed,
    /// Epic is blocked
    Blocked,
    /// Epic was skipped
    Skipped,
}

impl EpicStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            EpicStatus::Pending => "pending",
            EpicStatus::InProgress => "in_progress",
            EpicStatus::Completed => "completed",
            EpicStatus::Blocked => "blocked",
            EpicStatus::Skipped => "skipped",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "pending" => Ok(EpicStatus::Pending),
            "in_progress" => Ok(EpicStatus::InProgress),
            "completed" => Ok(EpicStatus::Completed),
            "blocked" => Ok(EpicStatus::Blocked),
            "skipped" => Ok(EpicStatus::Skipped),
            _ => Err(crate::Error::Other(format!("Unknown epic status: {}", s))),
        }
    }
}

/// Story status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum StoryStatus {
    /// Story is pending
    Pending,
    /// Story is in progress
    InProgress,
    /// Story is completed
    Completed,
    /// Story is blocked
    Blocked,
    /// Story was skipped
    Skipped,
}

impl StoryStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Skipped => "skipped",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, crate::Error> {
        match s {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "blocked" => Ok(Self::Blocked),
            "skipped" => Ok(Self::Skipped),
            _ => Err(crate::Error::Other(format!("Invalid story status: {}", s))),
        }
    }
}

/// BMAD workflow phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BmadPhase {
    FindEpic,
    CreateBranch,
    DevelopStories,
    CodeReview,
    CreatePr,
    WaitCopilot,
    FixIssues,
    MergePr,
    Done,
    Blocked,
}

impl std::fmt::Display for BmadPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BmadPhase::FindEpic => write!(f, "FIND_EPIC"),
            BmadPhase::CreateBranch => write!(f, "CREATE_BRANCH"),
            BmadPhase::DevelopStories => write!(f, "DEVELOP_STORIES"),
            BmadPhase::CodeReview => write!(f, "CODE_REVIEW"),
            BmadPhase::CreatePr => write!(f, "CREATE_PR"),
            BmadPhase::WaitCopilot => write!(f, "WAIT_COPILOT"),
            BmadPhase::FixIssues => write!(f, "FIX_ISSUES"),
            BmadPhase::MergePr => write!(f, "MERGE_PR"),
            BmadPhase::Done => write!(f, "DONE"),
            BmadPhase::Blocked => write!(f, "BLOCKED"),
        }
    }
}

/// A BMAD epic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epic {
    /// Epic ID (e.g., "7A", "10B-SSO")
    pub id: String,
    /// Epic title
    pub title: String,
    /// Source file where epic was found
    pub source_file: Option<String>,
    /// Pattern used to match this epic
    pub pattern: Option<String>,
    /// Current status
    pub status: EpicStatus,
    /// Current workflow phase
    pub current_phase: Option<BmadPhase>,
    /// Agent working on this epic
    pub agent_id: Option<Uuid>,
    /// Associated PR ID
    pub pr_id: Option<i64>,
    /// Error message if blocked
    pub error_message: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

impl Epic {
    /// Create a new epic
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            source_file: None,
            pattern: None,
            status: EpicStatus::Pending,
            current_phase: None,
            agent_id: None,
            pr_id: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    /// Set source file
    pub fn with_source(mut self, file: impl Into<String>) -> Self {
        self.source_file = Some(file.into());
        self
    }

    /// Start the epic
    pub fn start(&mut self, agent_id: Uuid) {
        self.status = EpicStatus::InProgress;
        self.agent_id = Some(agent_id);
        self.current_phase = Some(BmadPhase::CreateBranch);
        self.updated_at = Utc::now();
    }

    /// Complete the epic
    pub fn complete(&mut self) {
        self.status = EpicStatus::Completed;
        self.current_phase = Some(BmadPhase::Done);
        self.updated_at = Utc::now();
        self.completed_at = Some(Utc::now());
    }

    /// Block the epic
    pub fn block(&mut self, error: impl Into<String>) {
        self.status = EpicStatus::Blocked;
        self.current_phase = Some(BmadPhase::Blocked);
        self.error_message = Some(error.into());
        self.updated_at = Utc::now();
    }

    /// Update phase
    pub fn set_phase(&mut self, phase: BmadPhase) {
        self.current_phase = Some(phase);
        self.updated_at = Utc::now();
    }
}

/// A story within an epic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    /// Story ID
    pub id: String,
    /// Parent epic ID
    pub epic_id: String,
    /// Story title
    pub title: String,
    /// Story description
    pub description: Option<String>,
    /// Acceptance criteria
    pub acceptance_criteria: Option<serde_json::Value>,
    /// Current status
    pub status: StoryStatus,
    /// Agent working on this story
    pub agent_id: Option<Uuid>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

impl Story {
    /// Create a new story
    pub fn new(
        id: impl Into<String>,
        epic_id: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            epic_id: epic_id.into(),
            title: title.into(),
            description: None,
            acceptance_criteria: None,
            status: StoryStatus::Pending,
            agent_id: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
        }
    }

    /// Set acceptance criteria
    pub fn with_criteria(mut self, criteria: serde_json::Value) -> Self {
        self.acceptance_criteria = Some(criteria);
        self
    }

    /// Start the story
    pub fn start(&mut self, agent_id: Uuid) {
        self.status = StoryStatus::InProgress;
        self.agent_id = Some(agent_id);
        self.updated_at = Utc::now();
    }

    /// Complete the story
    pub fn complete(&mut self) {
        self.status = StoryStatus::Completed;
        self.updated_at = Utc::now();
        self.completed_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== EpicStatus Tests ====================

    #[test]
    fn test_epic_status_as_str() {
        assert_eq!(EpicStatus::Pending.as_str(), "pending");
        assert_eq!(EpicStatus::InProgress.as_str(), "in_progress");
        assert_eq!(EpicStatus::Completed.as_str(), "completed");
        assert_eq!(EpicStatus::Blocked.as_str(), "blocked");
    }

    #[test]
    fn test_epic_status_from_str() {
        assert_eq!(
            EpicStatus::from_str("pending").unwrap(),
            EpicStatus::Pending
        );
        assert_eq!(
            EpicStatus::from_str("in_progress").unwrap(),
            EpicStatus::InProgress
        );
        assert_eq!(
            EpicStatus::from_str("completed").unwrap(),
            EpicStatus::Completed
        );
        assert!(EpicStatus::from_str("invalid").is_err());
    }

    // ==================== BmadPhase Tests ====================

    #[test]
    fn test_bmad_phase_display() {
        assert_eq!(format!("{}", BmadPhase::FindEpic), "FIND_EPIC");
        assert_eq!(format!("{}", BmadPhase::CreateBranch), "CREATE_BRANCH");
        assert_eq!(format!("{}", BmadPhase::DevelopStories), "DEVELOP_STORIES");
        assert_eq!(format!("{}", BmadPhase::Done), "DONE");
    }

    // ==================== Epic Tests ====================

    #[test]
    fn test_epic_new() {
        let epic = Epic::new("7A", "Implement authentication");

        assert_eq!(epic.id, "7A");
        assert_eq!(epic.title, "Implement authentication");
        assert_eq!(epic.status, EpicStatus::Pending);
        assert!(epic.current_phase.is_none());
        assert!(epic.agent_id.is_none());
        assert!(epic.completed_at.is_none());
    }

    #[test]
    fn test_epic_with_source() {
        let epic = Epic::new("7A", "Auth").with_source("docs/epics/auth.md");

        assert_eq!(epic.source_file, Some("docs/epics/auth.md".to_string()));
    }

    #[test]
    fn test_epic_start() {
        let mut epic = Epic::new("7A", "Auth");
        let agent_id = Uuid::new_v4();
        let initial_updated = epic.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        epic.start(agent_id);

        assert_eq!(epic.status, EpicStatus::InProgress);
        assert_eq!(epic.agent_id, Some(agent_id));
        assert_eq!(epic.current_phase, Some(BmadPhase::CreateBranch));
        assert!(epic.updated_at > initial_updated);
    }

    #[test]
    fn test_epic_complete() {
        let mut epic = Epic::new("7A", "Auth");
        epic.complete();

        assert_eq!(epic.status, EpicStatus::Completed);
        assert_eq!(epic.current_phase, Some(BmadPhase::Done));
        assert!(epic.completed_at.is_some());
    }

    #[test]
    fn test_epic_block() {
        let mut epic = Epic::new("7A", "Auth");
        epic.block("CI failed");

        assert_eq!(epic.status, EpicStatus::Blocked);
        assert_eq!(epic.current_phase, Some(BmadPhase::Blocked));
        assert_eq!(epic.error_message, Some("CI failed".to_string()));
    }

    #[test]
    fn test_epic_set_phase() {
        let mut epic = Epic::new("7A", "Auth");
        let initial_updated = epic.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        epic.set_phase(BmadPhase::CodeReview);

        assert_eq!(epic.current_phase, Some(BmadPhase::CodeReview));
        assert!(epic.updated_at > initial_updated);
    }

    // ==================== Story Tests ====================

    #[test]
    fn test_story_new() {
        let story = Story::new("7A-1", "7A", "Add login form");

        assert_eq!(story.id, "7A-1");
        assert_eq!(story.epic_id, "7A");
        assert_eq!(story.title, "Add login form");
        assert_eq!(story.status, StoryStatus::Pending);
        assert!(story.agent_id.is_none());
        assert!(story.completed_at.is_none());
    }

    #[test]
    fn test_story_with_criteria() {
        let criteria = serde_json::json!([
            "User can enter email",
            "User can enter password",
            "Form validates input"
        ]);

        let story = Story::new("7A-1", "7A", "Add login form").with_criteria(criteria.clone());

        assert_eq!(story.acceptance_criteria, Some(criteria));
    }

    #[test]
    fn test_story_start() {
        let mut story = Story::new("7A-1", "7A", "Add login form");
        let agent_id = Uuid::new_v4();
        let initial_updated = story.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        story.start(agent_id);

        assert_eq!(story.status, StoryStatus::InProgress);
        assert_eq!(story.agent_id, Some(agent_id));
        assert!(story.updated_at > initial_updated);
    }

    #[test]
    fn test_story_complete() {
        let mut story = Story::new("7A-1", "7A", "Add login form");
        story.complete();

        assert_eq!(story.status, StoryStatus::Completed);
        assert!(story.completed_at.is_some());
    }

    #[test]
    fn test_story_lifecycle() {
        let mut story = Story::new("7A-1", "7A", "Add login form");
        let agent_id = Uuid::new_v4();

        // Start story
        story.start(agent_id);
        assert_eq!(story.status, StoryStatus::InProgress);
        assert!(story.completed_at.is_none());

        // Complete story
        story.complete();
        assert_eq!(story.status, StoryStatus::Completed);
        assert!(story.completed_at.is_some());
    }
}
