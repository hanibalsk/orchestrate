//! Orchestrate Core - Core types and database layer
//!
//! This crate provides the fundamental types for the orchestrator system:
//! - Agent state machine and network
//! - Database operations
//! - Session management
//! - Message handling
//! - Agent network with state/skill dependencies

pub mod agent;
pub mod coverage;
pub mod database;
#[cfg(test)]
mod database_webhook_tests;
pub mod epic;
pub mod error;
pub mod instruction;
pub mod learning;
pub mod message;
pub mod network;
pub mod pr;
pub mod session;
pub mod shell_state;
pub mod test_generation;
pub mod test_quality;
pub mod webhook;
pub mod webhook_config;
pub mod worktree;

pub use agent::{Agent, AgentContext, AgentState, AgentType};
pub use database::{AgentStats, DailyTokenUsage, Database, TokenStats};
pub use epic::{BmadPhase, Epic, EpicStatus, Story, StoryStatus};
pub use error::{Error, Result};
pub use message::{Message, MessageRole};
pub use pr::{MergeStrategy, PrStatus, PullRequest};
pub use session::Session;
pub use worktree::{create_pr_worktree, Worktree, WorktreeStatus};

// Re-export instruction types
pub use instruction::{
    CustomInstruction, InstructionEffectiveness, InstructionScope, InstructionSource,
    LearningConfig, LearningPattern, PatternStatus, PatternType,
};

// Re-export learning types
pub use learning::{CleanupResult, LearningEngine};

// Re-export network types
pub use network::{
    AgentCapability, AgentHandle, AgentId, DependencyCondition, DependencyGraph, DependencySet,
    NetworkCoordinator, NetworkEvent, NetworkValidator, RecoveryAction, Skill, SkillDefinition,
    SkillRegistry, StateGraph, StateMachine, StatePropagation, StateRequirement, StateTransition,
    StepOutput, StepOutputType, ValidationError, ValidationResult, MAX_STEP_OUTPUT_DATA_SIZE,
};

// Re-export shell state types
pub use shell_state::{QueueEntry, ShellState, ShepherdLock};

// Re-export webhook types
pub use webhook::{WebhookEvent, WebhookEventStatus};
pub use webhook_config::{EventConfig, EventFilter, WebhookConfig};

// Re-export test generation types
pub use test_generation::{
    AcceptanceCriterion, E2ETestPlatform, E2ETestResult, FunctionSignature,
    IntegrationTestResult, InterfaceInfo, InterfaceType, Language, ModuleInfo, Parameter,
    TestCase, TestCategory, TestFixture, TestGenerationResult, TestGenerationService, TestType,
};

// Re-export coverage types
pub use coverage::{
    CoverageFormat, CoverageReport, CoverageService, FileCoverage, ModuleCoverage,
};

// Re-export test quality types
pub use test_quality::{
    MutationDetail, MutationTestResult, MutationType, TestIssueType, TestQualityIssue,
    TestQualityReport, TestQualityService,
};
