//! Orchestrate Core - Core types and database layer
//!
//! This crate provides the fundamental types for the orchestrator system:
//! - Agent state machine and network
//! - Database operations
//! - Session management
//! - Message handling
//! - Agent network with state/skill dependencies

pub mod agent;
pub mod database;
pub mod error;
pub mod message;
pub mod network;
pub mod session;
pub mod worktree;
pub mod pr;
pub mod epic;

pub use agent::{Agent, AgentState, AgentType};
pub use database::Database;
pub use error::{Error, Result};
pub use message::{Message, MessageRole};
pub use session::Session;
pub use worktree::{Worktree, WorktreeStatus};
pub use pr::{PullRequest, PrStatus, MergeStrategy};
pub use epic::{Epic, Story, EpicStatus, StoryStatus};

// Re-export network types
pub use network::{
    AgentCapability, AgentHandle, AgentId, DependencyCondition, DependencyGraph,
    DependencySet, NetworkCoordinator, NetworkEvent, NetworkValidator,
    RecoveryAction, Skill, SkillDefinition, SkillRegistry, StateGraph,
    StateMachine, StatePropagation, StateRequirement, StateTransition,
    StepOutput, StepOutputType, ValidationError, ValidationResult,
    MAX_STEP_OUTPUT_DATA_SIZE,
};
