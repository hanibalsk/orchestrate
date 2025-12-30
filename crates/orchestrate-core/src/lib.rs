//! Orchestrate Core - Core types and database layer
//!
//! This crate provides the fundamental types for the orchestrator system:
//! - Agent state machine and network
//! - Database operations
//! - Session management
//! - Message handling
//! - Agent network with state/skill dependencies

pub mod agent;
pub mod agent_continuation;
pub mod autonomous_session;
pub mod context_summary;
pub mod decision_engine;
pub mod approval;
pub mod approval_service;
pub mod condition_evaluator;
pub mod cron;
pub mod database;
#[cfg(test)]
mod database_approval_tests;
#[cfg(test)]
mod database_webhook_tests;
#[cfg(test)]
mod database_pipeline_tests;
#[cfg(test)]
mod database_autonomous_session_tests;
#[cfg(test)]
mod database_continuation_tests;
#[cfg(test)]
mod database_incident_tests;
pub mod documentation;
pub mod epic;
pub mod requirements;
pub mod multi_repo;
pub mod ci_integration;
pub mod incident;
pub mod test_generation;
pub mod deployment;
pub mod monitoring;
pub mod slack;
pub mod security;
pub mod security_gate;
pub mod security_report;
pub mod audit;
pub mod cost_analytics;
pub mod error;
pub mod experiment;
pub mod feedback;
pub mod instruction;
pub mod learning;
pub mod learning_automation;
pub mod message;
pub mod model_selection;
pub mod network;
pub mod pattern_export;
pub mod prompt_optimization;
pub mod pipeline;
pub mod pipeline_executor;
pub mod pipeline_parser;
pub mod pipeline_template;
pub mod pr;
pub mod schedule;
pub mod schedule_template;
pub mod session;
pub mod shell_state;
pub mod webhook;
pub mod webhook_config;
pub mod worktree;
pub mod test_stubs;
pub mod stuck_detection;
pub mod recovery;
pub mod work_evaluation;
pub mod code_review;
pub mod pr_workflow;
#[cfg(test)]
mod database_stuck_detection_tests;
#[cfg(test)]
mod database_recovery_tests;
#[cfg(test)]
mod database_work_evaluation_tests;

pub use agent::{Agent, AgentContext, AgentState, AgentType};
pub use database::{
    AgentStats, DailyTokenUsage, Database, EffectivenessAnalysisRow, EffectivenessSummary,
    TokenStats,
};
pub use epic::{BmadPhase, Epic, EpicStatus, Story, StoryStatus};
pub use error::{Error, Result};
pub use message::{Message, MessageRole};
pub use pr::{MergeStrategy, PrStatus, PullRequest};
pub use session::Session;
pub use worktree::{create_pr_worktree, Worktree, WorktreeStatus};

// Re-export instruction types
pub use instruction::{
    CustomInstruction, EffectivenessAnalysis, InstructionEffectiveness, InstructionScope,
    InstructionSource, LearningConfig, LearningPattern, PatternStatus, PatternType,
    SuccessPattern, SuccessPatternType,
};

// Re-export learning types
pub use learning::{CleanupResult, LearningEngine, SuccessRecommendations};

// Re-export feedback types
pub use feedback::{Feedback, FeedbackRating, FeedbackSource, FeedbackStats};

// Re-export experiment types
pub use experiment::{
    Experiment, ExperimentAssignment, ExperimentMetric, ExperimentObservation, ExperimentResults,
    ExperimentStatus, ExperimentType, ExperimentVariant, VariantResults,
};

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

// Re-export approval types
pub use approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
pub use approval_service::ApprovalService;

// Re-export pipeline template types
pub use pipeline_template::PipelineTemplate;

// Re-export model selection types
pub use model_selection::{
    classify_task_complexity, model_to_tier, models, AlternativeModel, AutoModelSelector,
    AutoSelectionFactors, AutoSelectionReason, ModelPerformance, ModelRecommendation,
    ModelSelectionConfig, ModelSelectionRule, ModelTier, OptimizationGoal, TaskComplexity,
};

// Re-export prompt optimization types
pub use prompt_optimization::{
    analyze_prompt_sections, prompt_similarity, PromptEffectiveness, PromptOptimizationConfig,
    PromptSection, PromptSuggestion, PromptVersion, SectionAnalysis, SuggestionStatus,
};

// Re-export pattern export types
pub use pattern_export::{
    filter_patterns, ExportMetadata, ExportablePattern, ImportDetail, ImportOptions, ImportResult,
    ImportStatus, InstructionPattern, PatternContext, PatternEffectiveness, PatternExport,
    PromptTemplatePattern, SuccessPatternExport, ToolSequencePattern,
};

// Re-export learning automation types
pub use learning_automation::{
    predict_task_outcome, ActionType, AreaForImprovement, AutomationAction, AutomationResults,
    AutomationRun, AutomationRunStatus, AutomationTrigger, DurationEstimate, Improvement,
    ImprovementCategory, LearningAutomationConfig, LearningReport, ReportSummary, RiskFactor,
    RiskSeverity, TaskPrediction, TokenEstimate,
};

// Re-export documentation types
pub use documentation::{
    Adr, AdrConsequence, AdrStatus, ApiContact, ApiDocumentation, ApiEndpoint, ApiInfo, ApiLicense,
    ApiParameter, ApiServer, Changelog, ChangelogEntry, ChangelogRelease, ChangeType, DocItemType,
    DocIssueType, DocType, DocValidationIssue, DocValidationResult, ParameterLocation, PropertyInfo,
    ReadmeContent, ReadmeSection, ReadmeSectionContent, SchemaInfo,
};

// Re-export requirements types
pub use requirements::{
    ArtifactType, ClarifyingQuestion, EffortEstimate, GeneratedStory, ImpactAnalysis, LinkType,
    Requirement, RequirementPriority, RequirementStatus, RequirementType, RiskLevel,
    StoryComplexity, TraceCoverage, TraceabilityLink, TraceabilityMatrix,
};

// Re-export multi-repo types
pub use multi_repo::{
    CoordinatedRelease, CrossRepoBranch, LinkedPr, LinkedPrGroup, LinkedPrStatus, ReleaseStatus,
    RepoBranchStatus, RepoConfig, RepoDependencyGraph, RepoProvider, RepoRelease, RepoStatus,
    Repository,
};

// Re-export CI integration types
pub use ci_integration::{
    CiArtifact, CiAuthType, CiConclusion, CiConfig, CiFailureAnalysis, CiJob, CiProvider, CiRun,
    CiRunStatus, CiStep, CiTriggerRequest, FailedJob, FailedTest,
};

// Re-export incident types
pub use incident::{
    ActionItem, ActionItemPriority, ActionResult, AnomalyMetric, EscalationCondition,
    EscalationRule, EscalationTarget, EscalationTargetType, Evidence, EvidenceType, Hypothesis,
    Incident, IncidentImpact, IncidentSeverity, IncidentStatus, Playbook, PlaybookAction,
    PlaybookExecution, PlaybookExecutionStatus, PlaybookTrigger, PostMortem, RelatedEvent,
    RootCauseAnalysis, TimelineEvent, TimelineEventType,
};

// Re-export test generation types
pub use test_generation::{
    AcceptanceCriterion, E2ETestPlatform, E2ETestResult, FunctionSignature, IntegrationTestResult,
    InterfaceInfo, InterfaceType, Language, ModuleInfo, Parameter, PropertyTestCase,
    PropertyTestResult, PropertyType, TestCase, TestCategory, TestFixture, TestGenerationResult,
    TestGenerationService, TestType,
};

// Re-export deployment types
pub use deployment::{
    CanaryMetrics, CanaryStage, CanaryStageStatus, ChangeItem, Deployment, DeploymentChangeType,
    DeploymentDiff, DeploymentLogEntry, DeploymentLogLevel, DeploymentMetrics, DeploymentProvider,
    DeploymentStatus, DeploymentStrategy, Environment, EnvironmentType, FeatureFlag,
    PostDeploymentVerification, PreDeploymentValidation, Release, ReleaseAsset,
    ReleaseType, ValidationCheck, ValidationCheckType, VerificationCheck, VerificationCheckType,
};

// Re-export monitoring types
pub use monitoring::{
    ActorType, AgentPerformance, Alert, AlertRule, AlertSeverity, AlertStatus, AuditAction,
    AuditEntry, ComponentHealth, ConditionType, CostRecord, CostReport, DailyCost, HealthStatus,
    HistogramBucket, HistogramValue, MetricDefinition, MetricType, MetricValue, MetricsSummary,
    NotificationChannel, NotificationChannelType, SystemHealth,
};

// Re-export audit types
pub use audit::{AuditQuery, AuditStats, ExportFormat, RetentionPolicy};

// Re-export cost analytics types
pub use cost_analytics::BudgetPeriod;

// Re-export Slack types
pub use slack::{
    ButtonStyle, ChannelConfig, DigestMode, InteractionAction, InteractionChannel,
    InteractionMessage, InteractionPayload, InteractionType, InteractionUser, NotificationSettings,
    NotificationType, NotificationTemplate, PrThread, ResponseType, SentMessage, SlackApprovalRequest,
    SlackBlock, SlackConnection, SlackContextElement, SlackElement, SlackMessage, SlackOption,
    SlackText, SlashCommand, SlashCommandResponse, TextType, UserMapping,
    ApprovalDecision as SlackApprovalDecision,
};

// Re-export security types
pub use security::{
    DetectedSecret, FixChange, FixStatus, FixType, LicenseCheckResult, LicenseIssue,
    LicenseIssueType, SarifArtifactLocation, SarifDriver, SarifLocation, SarifMessage,
    SarifPhysicalLocation, SarifRegion, SarifReport, SarifResult, SarifRun, SarifTool,
    ScanStatus, ScanSummary, ScanType, SecretType, SecurityException, SecurityFix,
    SecurityPolicy, SecurityScan, Severity, Vulnerability, VulnerabilityType,
};

// Re-export security gate types
pub use security_gate::{GateDecision, SecurityGate};

// Re-export security report types
pub use security_report::ReportFormat;

// Re-export test stub types (placeholders for incomplete features)
pub use test_stubs::{
    CoverageReport, FileCoverage, GeneratedTest, IssueSeverity, ModuleCoverage,
    SlackService, SlackUserService,
    TestFramework, TestQualityIssue, TestQualityIssueType, TestQualityReport,
    TestResult, TestResultStatus, TestRun, TestRunStatus,
};

// Re-export autonomous session types (Epic 016)
pub use autonomous_session::{
    AutonomousSession, AutonomousSessionState, CompletedItem, SessionConfig, SessionMetrics,
    SessionStateHistory, WorkItem, WorkItemType,
};

// Re-export decision engine types (Epic 016)
pub use decision_engine::{
    AgentStatus, Decision, DecisionEngine, DecisionEngineConfig, EscalationSeverity,
    EvaluationResult as DecisionEvaluationResult, ReviewType, StatusSignal, WaitType,
};

// Re-export agent continuation types (Epic 016)
pub use agent_continuation::{
    AgentContinuation, ContinuationBuilder, ContinuationReason, ContinuationResult,
    ContinuationStatus,
};

// Re-export context summary types (Epic 016)
pub use context_summary::{
    Blocker, BlockerSeverity, BlockerType, ContextSummary, DecisionCategory, FileChange,
    FileChangeType, KeyDecision, OutputSummarizer, TestAdded, TestType as SummaryTestType,
    WorkStatus,
};

// Re-export stuck detection types (Epic 016)
pub use stuck_detection::{
    AgentProgress, EvaluationStatus, EvaluationType, RateLimitBackoff, StuckDetection,
    StuckDetectionConfig, StuckDetector, StuckSeverity, StuckType, WorkEvaluation,
};

// Re-export recovery types (Epic 016)
pub use recovery::{
    FixerAgentType, FixerRequest, PlannedRecoveryAction, RecoveryActionType, RecoveryAttempt,
    RecoveryConfig, RecoveryOutcome, RecoverySelector,
};

// Re-export work evaluation types (Epic 016 - Story 8)
pub use work_evaluation::{
    CiCheckResult, CiStatus, CriterionCheck, FeedbackItem, FeedbackType, PrMergeStatus,
    ReviewIssue, ReviewIssueSeverity, ReviewResult, ReviewVerdict, StoryEvaluationRecord,
    WorkCompletionStatus, WorkEvaluationResult, WorkEvaluator, WorkEvaluatorConfig,
};

// Re-export code review types (Epic 016 - Story 9)
pub use code_review::{
    CodeReviewConfig, CodeReviewCoordinator, ReviewEscalationLevel, ReviewIteration,
    ReviewRequest, ReviewResponse, ReviewerType,
};

// Re-export PR workflow types (Epic 016 - Story 10)
pub use pr_workflow::{
    CiAggregateStatus, ConflictInfo, ConflictResolutionStrategy, MergeMethod, PrDescription,
    PrStateTransition, PrWorkflowAction, PrWorkflowConfig, PrWorkflowContext, PrWorkflowManager,
    PrWorkflowRecord, PrWorkflowState,
};
