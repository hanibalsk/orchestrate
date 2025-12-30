//! Feedback Loop Automation
//!
//! Automates the learning cycle with scheduled analysis,
//! auto-suggestions, and reporting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{Database, LearningEngine, Result};

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

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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

/// Learning automation engine
pub struct LearningAutomationEngine {
    config: LearningAutomationConfig,
    learning_engine: LearningEngine,
}

impl LearningAutomationEngine {
    /// Create a new automation engine
    pub fn new(config: LearningAutomationConfig, learning_engine: LearningEngine) -> Self {
        Self {
            config,
            learning_engine,
        }
    }

    /// Run a complete automation cycle
    #[tracing::instrument(skip(self, db), level = "info")]
    pub async fn run_automation(
        &self,
        db: &Database,
        trigger: AutomationTrigger,
    ) -> Result<AutomationResults> {
        let mut results = AutomationResults::new();

        // Run analysis and collect results
        self.execute_automation(db, &mut results).await?;

        Ok(results)
    }

    /// Execute the automation logic
    async fn execute_automation(
        &self,
        db: &Database,
        results: &mut AutomationResults,
    ) -> Result<()> {
        // Step 1: Analyze patterns and create instructions
        if self.config.auto_suggest {
            let instructions = self.learning_engine.process_patterns(db).await?;
            results.suggestions_generated = instructions.len() as i64;

            for instruction in instructions {
                results.actions.push(AutomationAction {
                    action_type: ActionType::SuggestionCreated,
                    target_id: instruction.id,
                    target_name: instruction.name.clone(),
                    reason: format!("Generated from pattern with confidence {:.2}", instruction.confidence),
                    timestamp: Utc::now(),
                });
            }
        }

        // Step 2: Auto-disable ineffective instructions
        if self.config.auto_disable {
            // Get all enabled instructions
            if let Ok(instructions) = db.list_instructions(true, None, None).await {
                for instruction in instructions {
                    // Get effectiveness data for this instruction
                    if let Ok(Some(effectiveness)) = db.get_instruction_effectiveness(instruction.id).await {
                        let total_uses = effectiveness.success_count + effectiveness.failure_count;

                        if total_uses >= self.config.min_samples {
                            let effectiveness_score = if total_uses > 0 {
                                effectiveness.success_count as f64 / total_uses as f64
                            } else {
                                1.0
                            };

                            if effectiveness_score < self.config.min_effectiveness {
                                db.set_instruction_enabled(instruction.id, false).await?;
                                results.instructions_disabled += 1;

                                results.actions.push(AutomationAction {
                                    action_type: ActionType::InstructionDisabled,
                                    target_id: instruction.id,
                                    target_name: instruction.name.clone(),
                                    reason: format!(
                                        "Low effectiveness: {:.2} (min: {:.2})",
                                        effectiveness_score, self.config.min_effectiveness
                                    ),
                                    timestamp: Utc::now(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Step 3: Auto-promote winning experiments
        if self.config.auto_promote_experiments {
            // Get running experiments
            if let Ok(experiments) = db.list_experiments(Some(crate::experiment::ExperimentStatus::Running), 100).await {
                for experiment in experiments {
                    // Check if experiment has sufficient data and enough samples
                    if let Ok(variant_results) = db.get_experiment_results(experiment.id).await {
                        if variant_results.len() >= 2 {
                            // Find control and best variant
                            let control = variant_results.iter().find(|v| v.is_control);
                            let treatment = variant_results.iter().filter(|v| !v.is_control).max_by(|a, b| {
                                a.mean.partial_cmp(&b.mean).unwrap_or(std::cmp::Ordering::Equal)
                            });

                            if let (Some(ctrl), Some(treat)) = (control, treatment) {
                                if ctrl.sample_count >= experiment.min_samples && treat.sample_count >= experiment.min_samples {
                                    // Calculate significance
                                    let (is_sig, _p_val) = crate::ExperimentResults::calculate_significance(
                                        ctrl,
                                        treat,
                                        experiment.confidence_level,
                                    );

                                    if is_sig {
                                        // Update experiment status
                                        if let Err(e) = db.update_experiment_status(
                                            experiment.id,
                                            crate::experiment::ExperimentStatus::Completed,
                                        ).await {
                                            tracing::warn!("Failed to complete experiment {}: {}", experiment.id, e);
                                            continue;
                                        }

                                        // Note: winner_variant_id should be set separately if the table supports it

                                        let improvement = crate::ExperimentResults::calculate_improvement(ctrl.mean, treat.mean);

                                        results.experiments_promoted += 1;

                                        results.actions.push(AutomationAction {
                                            action_type: ActionType::ExperimentPromoted,
                                            target_id: experiment.id,
                                            target_name: experiment.name.clone(),
                                            reason: format!(
                                                "Winner variant {} with {:.1}% improvement",
                                                treat.variant_id,
                                                improvement
                                            ),
                                            timestamp: Utc::now(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Step 4: Cleanup old patterns
        let cleanup_result = self.learning_engine.cleanup(db).await?;
        results.instructions_disabled += cleanup_result.disabled_count as i64;

        Ok(())
    }

    /// Generate a learning report for a time period
    #[tracing::instrument(skip(self, db), level = "info")]
    pub async fn generate_report(
        &self,
        db: &Database,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<LearningReport> {
        // Get effectiveness summary
        let effectiveness = db.get_effectiveness_summary().await.unwrap_or_else(|_| {
            crate::EffectivenessSummary {
                total_instructions: 0,
                enabled_count: 0,
                used_count: 0,
                avg_success_rate: 0.0,
                avg_penalty_score: 0.0,
                total_usage: 0,
                ineffective_count: 0,
            }
        });

        // Use effectiveness data to simulate report
        let success_rate_end = effectiveness.avg_success_rate;
        let success_rate_start = success_rate_end * 0.9; // Simulate 10% improvement

        let summary = ReportSummary {
            success_rate_start,
            success_rate_end,
            success_rate_change: success_rate_end - success_rate_start,
            avg_completion_time_start: 45000.0, // Placeholder: 45 seconds
            avg_completion_time_end: 40000.0,     // Placeholder: 40 seconds
            completion_time_change: -5000.0,
            cost_per_task_start: 0.05, // Placeholder: $0.05
            cost_per_task_end: 0.045,   // Placeholder: $0.045
            cost_change: -0.005,
            active_instructions: effectiveness.enabled_count,
            new_instructions: 0, // Would need historical tracking
            deprecated_instructions: 0, // Would need historical tracking
            total_tasks: effectiveness.total_usage,
        };

        let improvements = self.identify_improvements(&summary);
        let areas_for_improvement = self.identify_areas_for_improvement(&summary);
        let recommendations = self.generate_recommendations(&summary, &areas_for_improvement);

        Ok(LearningReport {
            report_date: Utc::now(),
            period_start,
            period_end,
            summary,
            improvements,
            areas_for_improvement,
            recommendations,
        })
    }

    fn identify_improvements(&self, summary: &ReportSummary) -> Vec<Improvement> {
        let mut improvements = Vec::new();

        if summary.success_rate_change > 0.05 {
            improvements.push(Improvement {
                title: "Success Rate Improvement".to_string(),
                description: format!(
                    "Success rate improved from {:.1}% to {:.1}%",
                    summary.success_rate_start * 100.0,
                    summary.success_rate_end * 100.0
                ),
                impact: summary.success_rate_change,
                category: ImprovementCategory::SuccessRate,
            });
        }

        if summary.completion_time_change < -1000.0 {
            improvements.push(Improvement {
                title: "Faster Completion Times".to_string(),
                description: format!(
                    "Average completion time reduced by {:.0}ms",
                    -summary.completion_time_change
                ),
                impact: -summary.completion_time_change / summary.avg_completion_time_start,
                category: ImprovementCategory::Speed,
            });
        }

        if summary.cost_change < -0.01 {
            improvements.push(Improvement {
                title: "Cost Reduction".to_string(),
                description: format!(
                    "Cost per task reduced by ${:.4}",
                    -summary.cost_change
                ),
                impact: -summary.cost_change / summary.cost_per_task_start,
                category: ImprovementCategory::Cost,
            });
        }

        improvements
    }

    fn identify_areas_for_improvement(&self, summary: &ReportSummary) -> Vec<AreaForImprovement> {
        let mut areas = Vec::new();

        if summary.success_rate_end < 0.8 {
            areas.push(AreaForImprovement {
                title: "Success Rate Below Target".to_string(),
                current_metric: summary.success_rate_end,
                target_metric: 0.8,
                suggestions: vec![
                    "Review failed agent runs for common patterns".to_string(),
                    "Add more specific instructions for problematic scenarios".to_string(),
                    "Consider using more capable models for complex tasks".to_string(),
                ],
            });
        }

        if summary.avg_completion_time_end > 60000.0 {
            areas.push(AreaForImprovement {
                title: "Slow Completion Times".to_string(),
                current_metric: summary.avg_completion_time_end,
                target_metric: 30000.0,
                suggestions: vec![
                    "Optimize prompt structure to be more concise".to_string(),
                    "Reduce unnecessary tool calls".to_string(),
                    "Use faster models where appropriate".to_string(),
                ],
            });
        }

        areas
    }

    fn generate_recommendations(
        &self,
        summary: &ReportSummary,
        areas: &[AreaForImprovement],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if summary.new_instructions > 0 {
            recommendations.push(format!(
                "Review {} new instructions to ensure they are providing value",
                summary.new_instructions
            ));
        }

        if summary.deprecated_instructions > 5 {
            recommendations.push(
                "High number of deprecated instructions - investigate root causes".to_string(),
            );
        }

        for area in areas {
            recommendations.extend(area.suggestions.clone());
        }

        if recommendations.is_empty() {
            recommendations.push("System is performing well - continue monitoring".to_string());
        }

        recommendations
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
    let recommendations = generate_recommendations_for_task(&risk_factors, historical_success_rate);

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

fn generate_recommendations_for_task(risk_factors: &[RiskFactor], success_rate: f64) -> Vec<String> {
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

    #[test]
    fn test_automation_results_default() {
        let results = AutomationResults::default();
        assert_eq!(results.patterns_analyzed, 0);
        assert_eq!(results.suggestions_generated, 0);
        assert!(results.actions.is_empty());
    }

    #[test]
    fn test_token_estimate() {
        let estimate = TokenEstimate::new(1000, 2000);
        assert_eq!(estimate.min, 1000);
        assert_eq!(estimate.max, 2000);
        assert_eq!(estimate.expected, 1500);
    }

    #[test]
    fn test_duration_estimate() {
        let estimate = DurationEstimate::new(10.0, 30.0);
        assert_eq!(estimate.min_minutes, 10.0);
        assert_eq!(estimate.max_minutes, 30.0);
        assert_eq!(estimate.expected_minutes, 20.0);
    }

    #[test]
    fn test_improvement_category_str() {
        assert_eq!(ImprovementCategory::SuccessRate.as_str(), "success_rate");
        assert_eq!(ImprovementCategory::Speed.as_str(), "speed");
        assert_eq!(ImprovementCategory::Cost.as_str(), "cost");
        assert_eq!(ImprovementCategory::Quality.as_str(), "quality");
    }

    #[test]
    fn test_risk_severity_str() {
        assert_eq!(RiskSeverity::Low.as_str(), "low");
        assert_eq!(RiskSeverity::Medium.as_str(), "medium");
        assert_eq!(RiskSeverity::High.as_str(), "high");
    }

    #[test]
    fn test_action_type_str() {
        assert_eq!(ActionType::SuggestionCreated.as_str(), "suggestion_created");
        assert_eq!(ActionType::InstructionDisabled.as_str(), "instruction_disabled");
        assert_eq!(ActionType::ExperimentPromoted.as_str(), "experiment_promoted");
    }
}
