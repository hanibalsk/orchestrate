//! Dynamic Model Selection
//!
//! Provides intelligent model selection based on task characteristics,
//! historical performance, and cost/quality tradeoffs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Task complexity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskComplexity {
    /// Simple, straightforward tasks (typos, small fixes)
    Simple,
    /// Medium complexity tasks (feature additions, bug fixes)
    Medium,
    /// Complex tasks (refactoring, architecture changes)
    Complex,
    /// Very complex tasks (multi-file changes, new systems)
    VeryComplex,
}

impl TaskComplexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Medium => "medium",
            Self::Complex => "complex",
            Self::VeryComplex => "very_complex",
        }
    }

    /// Suggested minimum model tier for this complexity
    pub fn suggested_model_tier(&self) -> ModelTier {
        match self {
            Self::Simple => ModelTier::Fast,
            Self::Medium => ModelTier::Balanced,
            Self::Complex => ModelTier::Smart,
            Self::VeryComplex => ModelTier::Premium,
        }
    }
}

impl FromStr for TaskComplexity {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "simple" => Ok(Self::Simple),
            "medium" => Ok(Self::Medium),
            "complex" => Ok(Self::Complex),
            "very_complex" | "verycomplex" => Ok(Self::VeryComplex),
            _ => Err(crate::Error::Other(format!(
                "Invalid task complexity: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Model tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    /// Fastest, cheapest models (e.g., Haiku)
    Fast,
    /// Balanced performance/cost (e.g., Sonnet)
    Balanced,
    /// Smarter models for complex tasks (e.g., Sonnet 3.5)
    Smart,
    /// Premium models for the hardest tasks (e.g., Opus)
    Premium,
}

impl ModelTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Smart => "smart",
            Self::Premium => "premium",
        }
    }

    /// Get the default model for this tier
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Fast => "claude-3-haiku-20240307",
            Self::Balanced => "claude-3-5-sonnet-20241022",
            Self::Smart => "claude-3-5-sonnet-20241022",
            Self::Premium => "claude-3-opus-20240229",
        }
    }
}

impl FromStr for ModelTier {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "smart" => Ok(Self::Smart),
            "premium" => Ok(Self::Premium),
            _ => Err(crate::Error::Other(format!("Invalid model tier: {}", s))),
        }
    }
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Optimization goal for model selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationGoal {
    /// Minimize cost (use cheaper models when possible)
    Cost,
    /// Maximize quality (use better models)
    Quality,
    /// Balance cost and quality
    Balanced,
}

impl OptimizationGoal {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cost => "cost",
            Self::Quality => "quality",
            Self::Balanced => "balanced",
        }
    }
}

impl FromStr for OptimizationGoal {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cost" => Ok(Self::Cost),
            "quality" => Ok(Self::Quality),
            "balanced" => Ok(Self::Balanced),
            _ => Err(crate::Error::Other(format!(
                "Invalid optimization goal: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for OptimizationGoal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Performance statistics for a model on a specific task type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    pub model: String,
    pub task_type: String,
    pub agent_type: Option<String>,
    pub success_count: i64,
    pub failure_count: i64,
    pub success_rate: f64,
    pub avg_tokens: f64,
    pub avg_cost: f64,
    pub avg_duration_secs: f64,
    pub sample_count: i64,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl ModelPerformance {
    /// Calculate a score balancing success rate and cost
    pub fn balanced_score(&self) -> f64 {
        if self.sample_count == 0 {
            return 0.0;
        }
        // Higher success rate is better, lower cost is better
        // Normalize cost to a 0-1 scale (assuming max $1 per task)
        let cost_factor = 1.0 - (self.avg_cost / 1.0).min(1.0);
        0.6 * self.success_rate + 0.4 * cost_factor
    }

    /// Calculate quality-focused score
    pub fn quality_score(&self) -> f64 {
        self.success_rate
    }

    /// Calculate cost-focused score
    pub fn cost_score(&self) -> f64 {
        if self.sample_count == 0 {
            return 0.0;
        }
        // Must have at least 50% success rate to be considered
        if self.success_rate < 0.5 {
            return 0.0;
        }
        // Prefer lower cost
        1.0 - (self.avg_cost / 1.0).min(1.0)
    }
}

/// A model selection rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionRule {
    pub id: i64,
    pub name: String,
    pub task_type: Option<String>,
    pub agent_type: Option<String>,
    pub complexity: Option<TaskComplexity>,
    pub preferred_model: String,
    pub fallback_model: Option<String>,
    pub max_cost: Option<f64>,
    pub min_success_rate: Option<f64>,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl ModelSelectionRule {
    pub fn new(name: String, preferred_model: String) -> Self {
        Self {
            id: 0,
            name,
            task_type: None,
            agent_type: None,
            complexity: None,
            preferred_model,
            fallback_model: None,
            max_cost: None,
            min_success_rate: None,
            priority: 0,
            enabled: true,
            created_at: Utc::now(),
        }
    }

    pub fn with_task_type(mut self, task_type: String) -> Self {
        self.task_type = Some(task_type);
        self
    }

    pub fn with_agent_type(mut self, agent_type: String) -> Self {
        self.agent_type = Some(agent_type);
        self
    }

    pub fn with_complexity(mut self, complexity: TaskComplexity) -> Self {
        self.complexity = Some(complexity);
        self
    }

    pub fn with_fallback(mut self, fallback: String) -> Self {
        self.fallback_model = Some(fallback);
        self
    }

    pub fn with_max_cost(mut self, max_cost: f64) -> Self {
        self.max_cost = Some(max_cost);
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

/// Model selection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelectionConfig {
    pub optimization_goal: OptimizationGoal,
    pub max_cost_per_task: Option<f64>,
    pub min_success_rate: f64,
    pub min_samples_for_auto: i64,
    pub enabled: bool,
}

impl Default for ModelSelectionConfig {
    fn default() -> Self {
        Self {
            optimization_goal: OptimizationGoal::Balanced,
            max_cost_per_task: Some(0.50),
            min_success_rate: 0.6,
            min_samples_for_auto: 10,
            enabled: true,
        }
    }
}

/// Model recommendation with reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecommendation {
    pub recommended_model: String,
    pub model_tier: ModelTier,
    pub confidence: f64,
    pub reasoning: Vec<String>,
    pub alternatives: Vec<AlternativeModel>,
    pub estimated_cost: Option<f64>,
    pub estimated_success_rate: Option<f64>,
}

/// An alternative model option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeModel {
    pub model: String,
    pub tier: ModelTier,
    pub reason: String,
    pub estimated_cost: Option<f64>,
    pub estimated_success_rate: Option<f64>,
}

/// Classify task complexity based on heuristics
pub fn classify_task_complexity(
    task_description: &str,
    file_count: Option<usize>,
    estimated_lines: Option<usize>,
) -> TaskComplexity {
    let description_lower = task_description.to_lowercase();

    // Keyword-based heuristics
    let simple_keywords = ["typo", "rename", "update comment", "fix typo", "small"];
    let complex_keywords = [
        "refactor",
        "redesign",
        "architecture",
        "migrate",
        "rewrite",
        "system",
    ];
    let very_complex_keywords = [
        "multi-file",
        "cross-cutting",
        "breaking change",
        "major",
        "complete overhaul",
    ];

    // Check for very complex indicators
    if very_complex_keywords
        .iter()
        .any(|k| description_lower.contains(k))
    {
        return TaskComplexity::VeryComplex;
    }

    // Check file count
    if let Some(count) = file_count {
        if count > 10 {
            return TaskComplexity::VeryComplex;
        } else if count > 5 {
            return TaskComplexity::Complex;
        }
    }

    // Check estimated lines
    if let Some(lines) = estimated_lines {
        if lines > 500 {
            return TaskComplexity::VeryComplex;
        } else if lines > 200 {
            return TaskComplexity::Complex;
        }
    }

    // Check for complex indicators
    if complex_keywords
        .iter()
        .any(|k| description_lower.contains(k))
    {
        return TaskComplexity::Complex;
    }

    // Check for simple indicators
    if simple_keywords
        .iter()
        .any(|k| description_lower.contains(k))
    {
        return TaskComplexity::Simple;
    }

    // Default to medium
    TaskComplexity::Medium
}

/// Get the tier for a model identifier
pub fn model_to_tier(model: &str) -> ModelTier {
    let model_lower = model.to_lowercase();

    if model_lower.contains("haiku") {
        ModelTier::Fast
    } else if model_lower.contains("opus") {
        ModelTier::Premium
    } else if model_lower.contains("sonnet") {
        ModelTier::Smart
    } else {
        ModelTier::Balanced
    }
}

// ==================== Epic 016 Extensions ====================

/// Model identifier constants for easy reference
pub mod models {
    pub const OPUS: &str = "claude-opus-4-20250514";
    pub const SONNET: &str = "claude-sonnet-4-20250514";
    pub const HAIKU: &str = "claude-3-5-haiku-20241022";
}

impl ModelTier {
    /// Get the next tier up (for escalation)
    pub fn escalate(&self) -> Option<Self> {
        match self {
            Self::Fast => Some(Self::Balanced),
            Self::Balanced => Some(Self::Smart),
            Self::Smart => Some(Self::Premium),
            Self::Premium => None,
        }
    }

    /// Get the next tier down (for cost optimization)
    pub fn deescalate(&self) -> Option<Self> {
        match self {
            Self::Premium => Some(Self::Smart),
            Self::Smart => Some(Self::Balanced),
            Self::Balanced => Some(Self::Fast),
            Self::Fast => None,
        }
    }

    /// Get the latest model ID for this tier
    pub fn latest_model_id(&self) -> &'static str {
        match self {
            Self::Fast => models::HAIKU,
            Self::Balanced | Self::Smart => models::SONNET,
            Self::Premium => models::OPUS,
        }
    }

    /// Get approximate cost factor relative to Fast tier
    pub fn cost_factor(&self) -> f64 {
        match self {
            Self::Fast => 1.0,
            Self::Balanced => 3.0,
            Self::Smart => 3.0,
            Self::Premium => 15.0,
        }
    }
}

/// Factors that influence model selection for autonomous processing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoSelectionFactors {
    /// Story points (1-13 typical)
    pub story_points: Option<u8>,
    /// Number of files involved
    pub file_count: Option<u32>,
    /// Depth of dependencies
    pub dependency_depth: Option<u32>,
    /// Number of failed retries
    pub retry_count: u32,
    /// Is there a critical review issue?
    pub critical_review_issue: bool,
    /// Is context size expected to be large?
    pub large_context: bool,
    /// Is this security sensitive?
    pub security_sensitive: bool,
}

impl AutoSelectionFactors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_story_points(mut self, points: u8) -> Self {
        self.story_points = Some(points);
        self
    }

    pub fn with_file_count(mut self, count: u32) -> Self {
        self.file_count = Some(count);
        self
    }

    pub fn with_retries(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    pub fn critical_issue(mut self) -> Self {
        self.critical_review_issue = true;
        self
    }

    pub fn large_context(mut self) -> Self {
        self.large_context = true;
        self
    }

    pub fn security_sensitive(mut self) -> Self {
        self.security_sensitive = true;
        self
    }

    /// Calculate a complexity score (0-100)
    pub fn complexity_score(&self) -> u32 {
        let mut score = 0u32;

        if let Some(points) = self.story_points {
            score += (points as u32).min(13) * 3;
        }

        if let Some(count) = self.file_count {
            score += (count / 2).min(20);
        }

        if let Some(depth) = self.dependency_depth {
            score += (depth * 3).min(15);
        }

        if self.security_sensitive {
            score += 15;
        }

        score.min(100)
    }
}

/// Selection reasons for audit/tracking
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoSelectionReason {
    /// Default selection
    Default,
    /// Based on task complexity
    TaskComplexity,
    /// Escalation due to retries
    RetryEscalation,
    /// Critical review issue requires highest tier
    CriticalIssue,
    /// Large context requires larger model
    LargeContext,
    /// Security sensitive task
    SecuritySensitive,
    /// Manual override by user
    Override,
}

/// Autonomous model selector for epic processing
#[derive(Debug, Clone)]
pub struct AutoModelSelector {
    /// Default tier if no factors indicate otherwise
    pub default_tier: ModelTier,
    /// Escalate after this many retries
    pub escalation_threshold: u32,
    /// Score threshold for Premium tier
    pub premium_threshold: u32,
    /// Score threshold for Smart tier
    pub smart_threshold: u32,
}

impl Default for AutoModelSelector {
    fn default() -> Self {
        Self {
            default_tier: ModelTier::Smart,
            escalation_threshold: 2,
            premium_threshold: 70,
            smart_threshold: 30,
        }
    }
}

impl AutoModelSelector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select model based on factors
    pub fn select(&self, factors: &AutoSelectionFactors) -> (ModelTier, AutoSelectionReason) {
        // Critical issue always uses Premium
        if factors.critical_review_issue {
            return (ModelTier::Premium, AutoSelectionReason::CriticalIssue);
        }

        // Security sensitive uses Premium
        if factors.security_sensitive {
            return (ModelTier::Premium, AutoSelectionReason::SecuritySensitive);
        }

        // Check retry escalation
        if factors.retry_count >= self.escalation_threshold {
            let base = self.tier_for_score(factors.complexity_score());
            if let Some(escalated) = base.escalate() {
                return (escalated, AutoSelectionReason::RetryEscalation);
            }
            return (base, AutoSelectionReason::RetryEscalation);
        }

        // Large context prefers Smart tier
        if factors.large_context {
            return (ModelTier::Smart, AutoSelectionReason::LargeContext);
        }

        // Select based on complexity score
        let score = factors.complexity_score();
        let tier = self.tier_for_score(score);
        (tier, AutoSelectionReason::TaskComplexity)
    }

    fn tier_for_score(&self, score: u32) -> ModelTier {
        if score >= self.premium_threshold {
            ModelTier::Premium
        } else if score >= self.smart_threshold {
            ModelTier::Smart
        } else {
            ModelTier::Fast
        }
    }

    /// Escalate current tier
    pub fn escalate(&self, current: ModelTier) -> Option<ModelTier> {
        current.escalate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_complexity_roundtrip() {
        let complexities = [
            TaskComplexity::Simple,
            TaskComplexity::Medium,
            TaskComplexity::Complex,
            TaskComplexity::VeryComplex,
        ];

        for complexity in complexities {
            let s = complexity.as_str();
            let parsed = TaskComplexity::from_str(s).unwrap();
            assert_eq!(complexity, parsed);
        }
    }

    #[test]
    fn test_classify_simple_task() {
        let complexity = classify_task_complexity("Fix typo in README", None, None);
        assert_eq!(complexity, TaskComplexity::Simple);
    }

    #[test]
    fn test_classify_complex_task() {
        let complexity = classify_task_complexity("Refactor authentication system", None, None);
        assert_eq!(complexity, TaskComplexity::Complex);
    }

    #[test]
    fn test_classify_by_file_count() {
        let complexity = classify_task_complexity("Update config", Some(15), None);
        assert_eq!(complexity, TaskComplexity::VeryComplex);
    }

    #[test]
    fn test_model_tier_ordering() {
        assert!(ModelTier::Fast < ModelTier::Balanced);
        assert!(ModelTier::Balanced < ModelTier::Smart);
        assert!(ModelTier::Smart < ModelTier::Premium);
    }

    #[test]
    fn test_model_to_tier() {
        assert_eq!(model_to_tier("claude-3-haiku"), ModelTier::Fast);
        assert_eq!(model_to_tier("claude-3-opus"), ModelTier::Premium);
        assert_eq!(model_to_tier("claude-3-5-sonnet"), ModelTier::Smart);
    }

    #[test]
    fn test_model_performance_scores() {
        let perf = ModelPerformance {
            model: "test".to_string(),
            task_type: "test".to_string(),
            agent_type: None,
            success_count: 80,
            failure_count: 20,
            success_rate: 0.8,
            avg_tokens: 1000.0,
            avg_cost: 0.10,
            avg_duration_secs: 30.0,
            sample_count: 100,
            last_used_at: None,
        };

        // Quality score should be success rate
        assert!((perf.quality_score() - 0.8).abs() < 0.01);

        // Cost score should be high (low cost)
        assert!(perf.cost_score() > 0.8);

        // Balanced score should be in between
        let balanced = perf.balanced_score();
        assert!(balanced > 0.5 && balanced < 1.0);
    }

    // ==================== Epic 016 Extension Tests ====================

    #[test]
    fn test_model_tier_escalate() {
        assert_eq!(ModelTier::Fast.escalate(), Some(ModelTier::Balanced));
        assert_eq!(ModelTier::Balanced.escalate(), Some(ModelTier::Smart));
        assert_eq!(ModelTier::Smart.escalate(), Some(ModelTier::Premium));
        assert_eq!(ModelTier::Premium.escalate(), None);
    }

    #[test]
    fn test_model_tier_deescalate() {
        assert_eq!(ModelTier::Premium.deescalate(), Some(ModelTier::Smart));
        assert_eq!(ModelTier::Smart.deescalate(), Some(ModelTier::Balanced));
        assert_eq!(ModelTier::Balanced.deescalate(), Some(ModelTier::Fast));
        assert_eq!(ModelTier::Fast.deescalate(), None);
    }

    #[test]
    fn test_model_tier_latest_model_id() {
        assert_eq!(ModelTier::Fast.latest_model_id(), models::HAIKU);
        assert_eq!(ModelTier::Balanced.latest_model_id(), models::SONNET);
        assert_eq!(ModelTier::Smart.latest_model_id(), models::SONNET);
        assert_eq!(ModelTier::Premium.latest_model_id(), models::OPUS);
    }

    #[test]
    fn test_model_tier_cost_factor() {
        assert_eq!(ModelTier::Fast.cost_factor(), 1.0);
        assert_eq!(ModelTier::Balanced.cost_factor(), 3.0);
        assert_eq!(ModelTier::Smart.cost_factor(), 3.0);
        assert_eq!(ModelTier::Premium.cost_factor(), 15.0);
    }

    #[test]
    fn test_auto_selection_factors_builder() {
        let factors = AutoSelectionFactors::new()
            .with_story_points(8)
            .with_file_count(10)
            .with_retries(1)
            .critical_issue()
            .security_sensitive();

        assert_eq!(factors.story_points, Some(8));
        assert_eq!(factors.file_count, Some(10));
        assert_eq!(factors.retry_count, 1);
        assert!(factors.critical_review_issue);
        assert!(factors.security_sensitive);
    }

    #[test]
    fn test_auto_selection_factors_complexity_score() {
        // Empty factors = 0
        let empty = AutoSelectionFactors::default();
        assert_eq!(empty.complexity_score(), 0);

        // Story points contribute
        let with_points = AutoSelectionFactors::new().with_story_points(8);
        assert_eq!(with_points.complexity_score(), 24); // 8 * 3 = 24

        // File count contributes
        let with_files = AutoSelectionFactors::new().with_file_count(20);
        assert_eq!(with_files.complexity_score(), 10); // 20 / 2 = 10

        // Security sensitive adds 15
        let security = AutoSelectionFactors::new().security_sensitive();
        assert_eq!(security.complexity_score(), 15);

        // Combined score capped at 100
        let high_complexity = AutoSelectionFactors::new()
            .with_story_points(13)
            .with_file_count(50)
            .security_sensitive();
        assert!(high_complexity.complexity_score() <= 100);
    }

    #[test]
    fn test_auto_model_selector_critical_issue_uses_premium() {
        let selector = AutoModelSelector::new();
        let factors = AutoSelectionFactors::new().critical_issue();

        let (tier, reason) = selector.select(&factors);
        assert_eq!(tier, ModelTier::Premium);
        assert_eq!(reason, AutoSelectionReason::CriticalIssue);
    }

    #[test]
    fn test_auto_model_selector_security_sensitive_uses_premium() {
        let selector = AutoModelSelector::new();
        let factors = AutoSelectionFactors::new().security_sensitive();

        let (tier, reason) = selector.select(&factors);
        assert_eq!(tier, ModelTier::Premium);
        assert_eq!(reason, AutoSelectionReason::SecuritySensitive);
    }

    #[test]
    fn test_auto_model_selector_retry_escalation() {
        let selector = AutoModelSelector::new();

        // 1 retry - no escalation (threshold is 2)
        let factors_1 = AutoSelectionFactors::new().with_retries(1);
        let (tier_1, reason_1) = selector.select(&factors_1);
        assert_ne!(reason_1, AutoSelectionReason::RetryEscalation);

        // 2 retries - should escalate
        let factors_2 = AutoSelectionFactors::new().with_retries(2);
        let (tier_2, reason_2) = selector.select(&factors_2);
        assert_eq!(reason_2, AutoSelectionReason::RetryEscalation);
        // With no other factors, base tier would be Fast, escalated to Balanced
        assert_eq!(tier_2, ModelTier::Balanced);
    }

    #[test]
    fn test_auto_model_selector_large_context() {
        let selector = AutoModelSelector::new();
        let factors = AutoSelectionFactors::new().large_context();

        let (tier, reason) = selector.select(&factors);
        assert_eq!(tier, ModelTier::Smart);
        assert_eq!(reason, AutoSelectionReason::LargeContext);
    }

    #[test]
    fn test_auto_model_selector_complexity_based() {
        let selector = AutoModelSelector::new();

        // Low complexity -> Fast
        let low = AutoSelectionFactors::new().with_story_points(2);
        let (tier_low, reason_low) = selector.select(&low);
        assert_eq!(tier_low, ModelTier::Fast);
        assert_eq!(reason_low, AutoSelectionReason::TaskComplexity);

        // Medium complexity -> Smart (>= 30)
        let medium = AutoSelectionFactors::new().with_story_points(13);
        let (tier_medium, _) = selector.select(&medium);
        assert_eq!(tier_medium, ModelTier::Smart);

        // Higher complexity via more files -> Smart (score ~64)
        // 13*3=39 (story) + 20 (max file contribution) = 59
        let higher = AutoSelectionFactors::new()
            .with_story_points(13)
            .with_file_count(50);
        let (tier_higher, _) = selector.select(&higher);
        assert_eq!(tier_higher, ModelTier::Smart);

        // To get Premium via complexity, need score >= 70
        // Need to also add dependency_depth: 13*3=39 + 20 + 15 = 74
        let mut premium = AutoSelectionFactors::new()
            .with_story_points(13)
            .with_file_count(50);
        premium.dependency_depth = Some(5);
        let (tier_premium, reason_premium) = selector.select(&premium);
        assert_eq!(tier_premium, ModelTier::Premium);
        assert_eq!(reason_premium, AutoSelectionReason::TaskComplexity);
    }

    #[test]
    fn test_auto_model_selector_escalate() {
        let selector = AutoModelSelector::new();

        assert_eq!(selector.escalate(ModelTier::Fast), Some(ModelTier::Balanced));
        assert_eq!(selector.escalate(ModelTier::Smart), Some(ModelTier::Premium));
        assert_eq!(selector.escalate(ModelTier::Premium), None);
    }

    #[test]
    fn test_models_constants() {
        assert!(models::OPUS.contains("opus"));
        assert!(models::SONNET.contains("sonnet"));
        assert!(models::HAIKU.contains("haiku"));
    }
}
