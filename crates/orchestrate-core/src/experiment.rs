//! A/B Testing Framework
//!
//! Provides experiment management for testing prompt variations,
//! model selection, and other agent configurations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

/// Status of an experiment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    /// Experiment is being set up
    Draft,
    /// Experiment is actively running
    Running,
    /// Experiment is paused
    Paused,
    /// Experiment has completed
    Completed,
    /// Experiment was cancelled
    Cancelled,
}

impl ExperimentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for ExperimentStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(crate::Error::Other(format!(
                "Invalid experiment status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ExperimentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type of experiment being conducted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentType {
    /// Testing different prompt variations
    Prompt,
    /// Testing different model selections
    Model,
    /// Testing different instruction sets
    Instruction,
    /// Testing different context configurations
    Context,
    /// Custom experiment type
    Custom,
}

impl ExperimentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::Model => "model",
            Self::Instruction => "instruction",
            Self::Context => "context",
            Self::Custom => "custom",
        }
    }
}

impl FromStr for ExperimentType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "prompt" => Ok(Self::Prompt),
            "model" => Ok(Self::Model),
            "instruction" => Ok(Self::Instruction),
            "context" => Ok(Self::Context),
            "custom" => Ok(Self::Custom),
            _ => Err(crate::Error::Other(format!(
                "Invalid experiment type: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ExperimentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metric used to evaluate experiment outcomes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentMetric {
    /// Task success rate
    SuccessRate,
    /// Time to completion
    CompletionTime,
    /// Token usage
    TokenUsage,
    /// Cost per task
    Cost,
    /// User feedback score
    FeedbackScore,
    /// Custom metric
    Custom,
}

impl ExperimentMetric {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SuccessRate => "success_rate",
            Self::CompletionTime => "completion_time",
            Self::TokenUsage => "token_usage",
            Self::Cost => "cost",
            Self::FeedbackScore => "feedback_score",
            Self::Custom => "custom",
        }
    }
}

impl FromStr for ExperimentMetric {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "success_rate" => Ok(Self::SuccessRate),
            "completion_time" => Ok(Self::CompletionTime),
            "token_usage" => Ok(Self::TokenUsage),
            "cost" => Ok(Self::Cost),
            "feedback_score" => Ok(Self::FeedbackScore),
            "custom" => Ok(Self::Custom),
            _ => Err(crate::Error::Other(format!(
                "Invalid experiment metric: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ExperimentMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An A/B experiment definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub hypothesis: Option<String>,
    pub experiment_type: ExperimentType,
    pub metric: ExperimentMetric,
    pub agent_type: Option<String>,
    pub status: ExperimentStatus,
    pub min_samples: i64,
    pub confidence_level: f64,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub winner_variant_id: Option<i64>,
}

impl Experiment {
    pub fn new(name: String, experiment_type: ExperimentType, metric: ExperimentMetric) -> Self {
        Self {
            id: 0,
            name,
            description: None,
            hypothesis: None,
            experiment_type,
            metric,
            agent_type: None,
            status: ExperimentStatus::Draft,
            min_samples: 100,
            confidence_level: 0.95,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            winner_variant_id: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_hypothesis(mut self, hypothesis: String) -> Self {
        self.hypothesis = Some(hypothesis);
        self
    }

    pub fn with_agent_type(mut self, agent_type: String) -> Self {
        self.agent_type = Some(agent_type);
        self
    }

    pub fn with_min_samples(mut self, min_samples: i64) -> Self {
        self.min_samples = min_samples;
        self
    }

    pub fn with_confidence_level(mut self, confidence_level: f64) -> Self {
        self.confidence_level = confidence_level;
        self
    }

    /// Check if the experiment has reached minimum sample size
    pub fn has_sufficient_samples(&self, total_samples: i64) -> bool {
        total_samples >= self.min_samples
    }

    /// Check if the experiment can be started
    pub fn can_start(&self) -> bool {
        self.status == ExperimentStatus::Draft
    }

    /// Check if the experiment is actively collecting data
    pub fn is_running(&self) -> bool {
        self.status == ExperimentStatus::Running
    }
}

/// A variant within an experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentVariant {
    pub id: i64,
    pub experiment_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub is_control: bool,
    pub weight: i32,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl ExperimentVariant {
    pub fn new(experiment_id: i64, name: String, is_control: bool) -> Self {
        Self {
            id: 0,
            experiment_id,
            name,
            description: None,
            is_control,
            weight: 50,
            config: serde_json::Value::Object(serde_json::Map::new()),
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_weight(mut self, weight: i32) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }
}

/// An assignment of an agent to a variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentAssignment {
    pub id: i64,
    pub experiment_id: i64,
    pub variant_id: i64,
    pub agent_id: Uuid,
    pub assigned_at: DateTime<Utc>,
}

/// A metric observation for an assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentObservation {
    pub id: i64,
    pub assignment_id: i64,
    pub metric_name: String,
    pub metric_value: f64,
    pub recorded_at: DateTime<Utc>,
}

/// Aggregated results for a variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResults {
    pub variant_id: i64,
    pub variant_name: String,
    pub is_control: bool,
    pub sample_count: i64,
    pub mean: f64,
    pub std_dev: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub success_count: Option<i64>,
    pub success_rate: Option<f64>,
}

impl VariantResults {
    /// Calculate the 95% confidence interval for the mean
    pub fn confidence_interval(&self) -> (f64, f64) {
        if self.sample_count == 0 {
            return (0.0, 0.0);
        }
        // 1.96 is the z-score for 95% confidence
        let margin = 1.96 * self.std_dev / (self.sample_count as f64).sqrt();
        (self.mean - margin, self.mean + margin)
    }
}

/// Results of an A/B test comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResults {
    pub experiment_id: i64,
    pub experiment_name: String,
    pub status: ExperimentStatus,
    pub variants: Vec<VariantResults>,
    pub total_samples: i64,
    pub is_significant: bool,
    pub p_value: Option<f64>,
    pub winner_variant_id: Option<i64>,
    pub improvement_percent: Option<f64>,
}

impl ExperimentResults {
    /// Perform a two-sample t-test between control and treatment
    pub fn calculate_significance(
        control: &VariantResults,
        treatment: &VariantResults,
        confidence_level: f64,
    ) -> (bool, f64) {
        if control.sample_count < 2 || treatment.sample_count < 2 {
            return (false, 1.0);
        }

        let n1 = control.sample_count as f64;
        let n2 = treatment.sample_count as f64;

        // Pooled standard error
        let se = ((control.std_dev.powi(2) / n1) + (treatment.std_dev.powi(2) / n2)).sqrt();

        if se == 0.0 {
            return (false, 1.0);
        }

        // t-statistic
        let t_stat = (treatment.mean - control.mean) / se;

        // Degrees of freedom (Welch's approximation)
        let df_num = ((control.std_dev.powi(2) / n1) + (treatment.std_dev.powi(2) / n2)).powi(2);
        let df_denom = (control.std_dev.powi(4) / (n1.powi(2) * (n1 - 1.0)))
            + (treatment.std_dev.powi(4) / (n2.powi(2) * (n2 - 1.0)));

        let _df = df_num / df_denom;

        // Simplified p-value approximation using normal distribution
        // For large samples (n > 30), t-distribution approximates normal
        let p_value = 2.0 * (1.0 - normal_cdf(t_stat.abs()));

        let alpha = 1.0 - confidence_level;
        let is_significant = p_value < alpha;

        (is_significant, p_value)
    }

    /// Calculate improvement percentage of treatment over control
    pub fn calculate_improvement(control_mean: f64, treatment_mean: f64) -> f64 {
        if control_mean == 0.0 {
            return 0.0;
        }
        ((treatment_mean - control_mean) / control_mean) * 100.0
    }
}

/// Standard normal CDF approximation
fn normal_cdf(x: f64) -> f64 {
    // Abramowitz and Stegun approximation
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs() / std::f64::consts::SQRT_2;

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    0.5 * (1.0 + sign * y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_status_roundtrip() {
        let statuses = [
            ExperimentStatus::Draft,
            ExperimentStatus::Running,
            ExperimentStatus::Paused,
            ExperimentStatus::Completed,
            ExperimentStatus::Cancelled,
        ];

        for status in statuses {
            let s = status.as_str();
            let parsed = ExperimentStatus::from_str(s).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_experiment_type_roundtrip() {
        let types = [
            ExperimentType::Prompt,
            ExperimentType::Model,
            ExperimentType::Instruction,
            ExperimentType::Context,
            ExperimentType::Custom,
        ];

        for exp_type in types {
            let s = exp_type.as_str();
            let parsed = ExperimentType::from_str(s).unwrap();
            assert_eq!(exp_type, parsed);
        }
    }

    #[test]
    fn test_experiment_metric_roundtrip() {
        let metrics = [
            ExperimentMetric::SuccessRate,
            ExperimentMetric::CompletionTime,
            ExperimentMetric::TokenUsage,
            ExperimentMetric::Cost,
            ExperimentMetric::FeedbackScore,
            ExperimentMetric::Custom,
        ];

        for metric in metrics {
            let s = metric.as_str();
            let parsed = ExperimentMetric::from_str(s).unwrap();
            assert_eq!(metric, parsed);
        }
    }

    #[test]
    fn test_confidence_interval() {
        let results = VariantResults {
            variant_id: 1,
            variant_name: "control".to_string(),
            is_control: true,
            sample_count: 100,
            mean: 0.75,
            std_dev: 0.1,
            min_value: 0.5,
            max_value: 1.0,
            success_count: Some(75),
            success_rate: Some(0.75),
        };

        let (lower, upper) = results.confidence_interval();
        // For n=100, std=0.1: margin = 1.96 * 0.1 / 10 = 0.0196
        assert!((lower - 0.7304).abs() < 0.001);
        assert!((upper - 0.7696).abs() < 0.001);
    }

    #[test]
    fn test_significance_calculation() {
        let control = VariantResults {
            variant_id: 1,
            variant_name: "control".to_string(),
            is_control: true,
            sample_count: 100,
            mean: 0.70,
            std_dev: 0.15,
            min_value: 0.3,
            max_value: 1.0,
            success_count: Some(70),
            success_rate: Some(0.70),
        };

        let treatment = VariantResults {
            variant_id: 2,
            variant_name: "treatment".to_string(),
            is_control: false,
            sample_count: 100,
            mean: 0.80,
            std_dev: 0.12,
            min_value: 0.4,
            max_value: 1.0,
            success_count: Some(80),
            success_rate: Some(0.80),
        };

        let (is_significant, p_value) = ExperimentResults::calculate_significance(
            &control,
            &treatment,
            0.95,
        );

        // With 10% improvement and these sample sizes, should be significant
        assert!(is_significant);
        assert!(p_value < 0.05);
    }

    #[test]
    fn test_improvement_calculation() {
        let improvement = ExperimentResults::calculate_improvement(0.70, 0.80);
        assert!((improvement - 14.285).abs() < 0.01);
    }
}
