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
}
