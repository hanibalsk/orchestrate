//! Orchestrate Core - Core types and database layer
//!
//! This crate provides the fundamental types for the orchestrator system:
//! - Agent state machine and network
//! - Database operations
//! - Session management
//! - Message handling
//! - Agent network with state/skill dependencies

pub mod agent;
pub mod alerting;
pub mod approval;
pub mod approval_service;
pub mod condition_evaluator;
pub mod cron;
pub mod database;
#[cfg(test)]
mod database_alerting_tests;
#[cfg(test)]
mod database_approval_tests;
#[cfg(test)]
mod database_webhook_tests;
#[cfg(test)]
mod database_pipeline_tests;
#[cfg(test)]
mod database_environment_tests;
pub mod deployment_executor;
pub mod deployment_rollback;
pub mod deployment_strategy;
pub mod environment;
pub mod epic;
pub mod release_management;
pub mod post_deploy_verification;
pub mod pre_deploy_validation;
pub mod error;
pub mod feature_flags;
pub mod feedback;
pub mod instruction;
pub mod learning;
pub mod message;
pub mod network;
pub mod pipeline;
pub mod pipeline_executor;
pub mod pipeline_parser;
pub mod pipeline_template;
pub mod pr;
pub mod schedule;
pub mod secrets;
pub mod schedule_template;
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
    LearningConfig, LearningPattern, PatternStatus, PatternType, SuccessPattern,
    SuccessPatternType,
};

// Re-export learning types
pub use learning::{CleanupResult, LearningEngine, SuccessRecommendations};

// Re-export feedback types
pub use feedback::{Feedback, FeedbackRating, FeedbackSource, FeedbackStats};

// Re-export network types
pub use network::{
    AgentCapability, AgentHandle, AgentId, DependencyCondition, DependencyGraph, DependencySet,
    NetworkCoordinator, NetworkEvent, NetworkValidator, RecoveryAction, Skill, SkillDefinition,
    SkillRegistry, StateGraph, StateMachine, StatePropagation, StateRequirement, StateTransition,
    StepOutput, StepOutputType, ValidationError, ValidationResult, MAX_STEP_OUTPUT_DATA_SIZE,
};

// Re-export shell state types
pub use shell_state::{QueueEntry, ShellState, ShepherdLock};

// Re-export schedule types
pub use schedule::{Schedule, ScheduleRun, ScheduleRunStatus};

// Re-export schedule template types
pub use schedule_template::ScheduleTemplate;

// Re-export cron types
pub use cron::CronSchedule;

// Re-export webhook types
pub use webhook::{WebhookEvent, WebhookEventStatus};
pub use webhook_config::{EventConfig, EventFilter, WebhookConfig};

// Re-export pipeline types
pub use pipeline::{
    Pipeline, PipelineRun, PipelineRunStatus, PipelineStage, PipelineStageStatus, RollbackEvent,
    RollbackStatus, RollbackTriggerType,
};
pub use pipeline_executor::{ExecutionContext, PipelineExecutor};
pub use pipeline_parser::{
    FailureAction, PipelineDefinition, StageCondition, StageDefinition, TriggerDefinition,
};

// Re-export condition evaluator types
pub use condition_evaluator::{ConditionContext, ConditionEvaluator, EvaluationResult, SkipReason};

// Re-export alerting types
pub use alerting::{
    Alert, AlertEvaluator, AlertRule, AlertSeverity, AlertStatus, ConditionParser, ConditionType,
    ThresholdOperator, generate_fingerprint,
};

// Re-export approval types
pub use approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
pub use approval_service::ApprovalService;

// Re-export pipeline template types
pub use pipeline_template::PipelineTemplate;

// Re-export environment types
pub use environment::{CreateEnvironment, Environment, EnvironmentType};

// Re-export secrets types
pub use secrets::{get_encryption_key, SecretsManager};

// Re-export deployment strategy types
pub use deployment_strategy::{
    BatchSize, BlueGreenConfig, CanaryConfig, DeploymentStrategy, Environment as BlueGreenEnvironment,
    HealthCheck, RecreateConfig, RollingConfig, StrategyType,
};

// Re-export pre-deployment validation types
pub use pre_deploy_validation::{
    DeploymentValidation, PreDeployValidator, ValidationCheck, ValidationStatus,
};

// Re-export deployment executor types
pub use deployment_executor::{
    Deployment, DeploymentExecutor, DeploymentProgress, DeploymentProvider, DeploymentRequest,
    DeploymentStatus,
};

// Re-export post-deployment verification types
pub use post_deploy_verification::{
    PostDeployVerifier, VerificationCheck, VerificationCheckStatus, VerificationCheckType,
    VerificationResult,
};

// Re-export deployment rollback types
pub use deployment_rollback::{
    DeploymentRollback, RollbackEvent as DeploymentRollbackEvent, RollbackNotification,
    RollbackRequest, RollbackStatus as DeploymentRollbackStatus, RollbackType,
};

// Re-export release management types
pub use release_management::{
    BumpType, Changelog, ChangelogEntry, Commit, CommitType, ReleaseAsset, ReleaseManager,
    ReleasePreparation, ReleaseRequest, Version,
};

// Re-export feature flags types
pub use feature_flags::{
    CreateFeatureFlag, FeatureFlag, FlagStatus, UpdateFeatureFlag,
};
