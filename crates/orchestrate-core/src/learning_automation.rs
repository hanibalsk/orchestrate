//! Feedback Loop Automation
//!
//! Automates the learning cycle with scheduled analysis,
//! auto-suggestions, and reporting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Learning automation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningAutomationConfig {
    pub enabled: bool,
    /// Run analysis on this schedule (cron expression)
    pub analysis_schedule: String,
    /// Auto-generate instruction suggestions
    pub auto_suggest: bool,
    /// Auto-disable ineffective instructions
    pub auto_disable: bool,
    /// Auto-promote winning experiment variants
    pub auto_promote_experiments: bool,
    /// Generate learning reports
    pub generate_reports: bool,
    /// Minimum effectiveness score to keep instruction enabled
    pub min_effectiveness: f64,
    /// Minimum sample size before making auto decisions
    pub min_samples: i64,
}

impl Default for LearningAutomationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            analysis_schedule: "0 0 * * *".to_string(), // Daily at midnight
            auto_suggest: true,
            auto_disable: false,
            auto_promote_experiments: false,
            generate_reports: true,
            min_effectiveness: 0.5,
            min_samples: 10,
        }
    }
}

/// Status of a learning automation run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRunStatus {
    /// Run is in progress
    Running,
    /// Run completed successfully
    Completed,
    /// Run failed
    Failed,
    /// Run was cancelled
    Cancelled,
}

impl AutomationRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for AutomationRunStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(crate::Error::Other(format!(
                "Invalid automation run status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for AutomationRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Record of a learning automation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRun {
    pub id: i64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: AutomationRunStatus,
    pub trigger: AutomationTrigger,
    pub results: Option<AutomationResults>,
    pub error_message: Option<String>,
}

/// What triggered the automation run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationTrigger {
    /// Scheduled run
    Scheduled,
    /// Manual trigger
    Manual,
    /// Triggered by event
    Event,
}

impl AutomationTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Manual => "manual",
            Self::Event => "event",
        }
    }
}

impl FromStr for AutomationTrigger {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "scheduled" => Ok(Self::Scheduled),
            "manual" => Ok(Self::Manual),
            "event" => Ok(Self::Event),
            _ => Err(crate::Error::Other(format!(
                "Invalid automation trigger: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for AutomationTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Results of an automation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationResults {
    /// Number of patterns analyzed
    pub patterns_analyzed: i64,
    /// New patterns identified
    pub new_patterns: i64,
    /// Suggestions generated
    pub suggestions_generated: i64,
    /// Instructions disabled
    pub instructions_disabled: i64,
    /// Experiments promoted
    pub experiments_promoted: i64,
    /// Learning score change
    pub score_delta: f64,
    /// Details for each action taken
    pub actions: Vec<AutomationAction>,
}

impl AutomationResults {
    pub fn new() -> Self {
        Self {
            patterns_analyzed: 0,
            new_patterns: 0,
            suggestions_generated: 0,
            instructions_disabled: 0,
            experiments_promoted: 0,
            score_delta: 0.0,
            actions: Vec::new(),
        }
    }
}

impl Default for AutomationResults {
    fn default() -> Self {
        Self::new()
    }
}

/// An action taken during automation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationAction {
    pub action_type: ActionType,
    pub target_id: i64,
    pub target_name: String,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Type of automation action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Created a new suggestion
    SuggestionCreated,
    /// Disabled an instruction
    InstructionDisabled,
    /// Promoted an experiment variant
    ExperimentPromoted,
    /// Created a new pattern
    PatternCreated,
    /// Updated effectiveness scores
    ScoresUpdated,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SuggestionCreated => "suggestion_created",
            Self::InstructionDisabled => "instruction_disabled",
            Self::ExperimentPromoted => "experiment_promoted",
            Self::PatternCreated => "pattern_created",
            Self::ScoresUpdated => "scores_updated",
        }
    }
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Learning report summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningReport {
    pub report_date: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub summary: ReportSummary,
    pub improvements: Vec<Improvement>,
    pub areas_for_improvement: Vec<AreaForImprovement>,
    pub recommendations: Vec<String>,
}

/// Summary statistics for a learning report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub success_rate_start: f64,
    pub success_rate_end: f64,
    pub success_rate_change: f64,
    pub avg_completion_time_start: f64,
    pub avg_completion_time_end: f64,
    pub completion_time_change: f64,
    pub cost_per_task_start: f64,
    pub cost_per_task_end: f64,
    pub cost_change: f64,
    pub active_instructions: i64,
    pub new_instructions: i64,
    pub deprecated_instructions: i64,
    pub total_tasks: i64,
}

/// An improvement achieved during the period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub title: String,
    pub description: String,
    pub impact: f64,
    pub category: ImprovementCategory,
}

/// Category of improvement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementCategory {
    SuccessRate,
    Speed,
    Cost,
    Quality,
}

impl ImprovementCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SuccessRate => "success_rate",
            Self::Speed => "speed",
            Self::Cost => "cost",
            Self::Quality => "quality",
        }
    }
}

/// An area identified for improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaForImprovement {
    pub title: String,
    pub current_metric: f64,
    pub target_metric: f64,
    pub suggestions: Vec<String>,
}

/// Performance prediction for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPrediction {
    pub task_description: String,
    pub success_probability: f64,
    pub confidence: f64,
    pub estimated_tokens: TokenEstimate,
    pub estimated_duration: DurationEstimate,
    pub recommended_model: String,
    pub risk_factors: Vec<RiskFactor>,
    pub recommendations: Vec<String>,
}

/// Token usage estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEstimate {
    pub min: i64,
    pub max: i64,
    pub expected: i64,
}

impl TokenEstimate {
    pub fn new(min: i64, max: i64) -> Self {
        Self {
            min,
            max,
            expected: (min + max) / 2,
        }
    }
}

/// Duration estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationEstimate {
    pub min_minutes: f64,
    pub max_minutes: f64,
    pub expected_minutes: f64,
}

impl DurationEstimate {
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min_minutes: min,
            max_minutes: max,
            expected_minutes: (min + max) / 2.0,
        }
    }
}

/// A risk factor for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub name: String,
    pub description: String,
    pub severity: RiskSeverity,
    pub impact_on_success: f64,
}

/// Severity of a risk factor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
}

impl RiskSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl std::fmt::Display for RiskSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Generate a prediction for a task based on historical data
pub fn predict_task_outcome(
    task_description: &str,
    historical_success_rate: f64,
    avg_tokens: i64,
    avg_duration_mins: f64,
    similar_task_count: i64,
) -> TaskPrediction {
    // Calculate confidence based on sample size
    let confidence = calculate_confidence(similar_task_count);

    // Estimate tokens with variance
    let token_variance = (avg_tokens as f64 * 0.3) as i64;
    let token_estimate = TokenEstimate::new(
        avg_tokens - token_variance,
        avg_tokens + token_variance,
    );

    // Estimate duration with variance
    let duration_variance = avg_duration_mins * 0.3;
    let duration_estimate = DurationEstimate::new(
        avg_duration_mins - duration_variance,
        avg_duration_mins + duration_variance,
    );

    // Identify risk factors
    let risk_factors = identify_risk_factors(task_description, historical_success_rate);

    // Generate recommendations
    let recommendations = generate_recommendations(&risk_factors, historical_success_rate);

    // Determine recommended model
    let recommended_model = if historical_success_rate < 0.6 {
        "claude-3-opus-20240229"
    } else if historical_success_rate < 0.8 {
        "claude-3-5-sonnet-20241022"
    } else {
        "claude-3-5-sonnet-20241022"
    };

    TaskPrediction {
        task_description: task_description.to_string(),
        success_probability: historical_success_rate,
        confidence,
        estimated_tokens: token_estimate,
        estimated_duration: duration_estimate,
        recommended_model: recommended_model.to_string(),
        risk_factors,
        recommendations,
    }
}

fn calculate_confidence(sample_count: i64) -> f64 {
    // Confidence increases with sample size, maxing out around 100 samples
    let normalized = (sample_count as f64 / 100.0).min(1.0);
    0.5 + (normalized * 0.5)
}

fn identify_risk_factors(task_description: &str, success_rate: f64) -> Vec<RiskFactor> {
    let mut factors = Vec::new();
    let desc_lower = task_description.to_lowercase();

    // Check for complexity indicators
    if desc_lower.contains("refactor") || desc_lower.contains("rewrite") {
        factors.push(RiskFactor {
            name: "High Complexity".to_string(),
            description: "Task involves significant code restructuring".to_string(),
            severity: RiskSeverity::High,
            impact_on_success: -0.15,
        });
    }

    if desc_lower.contains("migrate") || desc_lower.contains("upgrade") {
        factors.push(RiskFactor {
            name: "Migration Risk".to_string(),
            description: "Migrations can have unexpected side effects".to_string(),
            severity: RiskSeverity::Medium,
            impact_on_success: -0.10,
        });
    }

    if success_rate < 0.6 {
        factors.push(RiskFactor {
            name: "Low Historical Success".to_string(),
            description: "Similar tasks have had lower success rates".to_string(),
            severity: RiskSeverity::High,
            impact_on_success: -0.20,
        });
    }

    factors
}

fn generate_recommendations(risk_factors: &[RiskFactor], success_rate: f64) -> Vec<String> {
    let mut recommendations = Vec::new();

    if success_rate < 0.7 {
        recommendations.push("Consider breaking this into smaller subtasks".to_string());
    }

    if risk_factors.iter().any(|r| r.severity == RiskSeverity::High) {
        recommendations.push("Use a more capable model for this task".to_string());
        recommendations.push("Ensure comprehensive test coverage before starting".to_string());
    }

    if risk_factors.is_empty() && success_rate > 0.8 {
        recommendations.push("This task has good success indicators".to_string());
    }

    recommendations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_run_status_roundtrip() {
        let statuses = [
            AutomationRunStatus::Running,
            AutomationRunStatus::Completed,
            AutomationRunStatus::Failed,
            AutomationRunStatus::Cancelled,
        ];

        for status in statuses {
            let s = status.as_str();
            let parsed = AutomationRunStatus::from_str(s).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_predict_task_outcome() {
        let prediction = predict_task_outcome(
            "Implement user authentication",
            0.75,
            50000,
            30.0,
            50,
        );

        assert!(prediction.success_probability > 0.7);
        assert!(prediction.confidence > 0.5);
        assert!(prediction.estimated_tokens.expected > 0);
        assert!(prediction.estimated_duration.expected_minutes > 0.0);
    }

    #[test]
    fn test_identify_risk_factors() {
        let factors = identify_risk_factors("Refactor the entire authentication module", 0.5);
        assert!(!factors.is_empty());
        assert!(factors.iter().any(|f| f.name == "High Complexity"));
    }

    #[test]
    fn test_confidence_calculation() {
        assert!(calculate_confidence(0) < calculate_confidence(50));
        assert!(calculate_confidence(50) < calculate_confidence(100));
        assert!((calculate_confidence(100) - calculate_confidence(200)).abs() < 0.01);
    }
}
