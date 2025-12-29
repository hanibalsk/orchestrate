//! Orchestrate Core - Core types and database layer
//!
//! This crate provides the fundamental types for the orchestrator system:
//! - Agent state machine and network
//! - Database operations
//! - Session management
//! - Message handling
//! - Agent network with state/skill dependencies

pub mod agent;
pub mod condition_evaluator;
pub mod database;
#[cfg(test)]
mod database_webhook_tests;
#[cfg(test)]
mod database_pipeline_tests;
pub mod epic;
pub mod error;
pub mod instruction;
pub mod learning;
pub mod message;
pub mod network;
pub mod pipeline;
pub mod pipeline_executor;
pub mod pipeline_parser;
pub mod pr;
pub mod session;
pub mod shell_state;
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

// Re-export pipeline types
pub use pipeline::{Pipeline, PipelineRun, PipelineRunStatus, PipelineStage, PipelineStageStatus};
pub use pipeline_executor::{ExecutionContext, PipelineExecutor};
pub use pipeline_parser::{
    FailureAction, PipelineDefinition, StageCondition, StageDefinition, TriggerDefinition,
};

// Re-export condition evaluator types
pub use condition_evaluator::{ConditionContext, ConditionEvaluator, EvaluationResult, SkipReason};
