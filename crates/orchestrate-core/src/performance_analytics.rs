//! Agent Performance Analytics
//!
//! This module provides comprehensive performance metrics for agents:
//! - Task completion rates
//! - Duration statistics (avg, min, max, p95)
//! - Token efficiency metrics
//! - Error pattern analysis
//! - Agent comparison analytics
//! - Performance trend detection

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::monitoring::AgentPerformance;
use crate::{AgentState, AgentType};

/// Performance analytics calculator
pub struct PerformanceAnalytics;

/// Agent execution data for analytics
#[derive(Debug, Clone)]
pub struct AgentExecution {
    pub agent_type: AgentType,
    pub state: AgentState,
    pub duration_seconds: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub error_message: Option<String>,
    pub completed_at: DateTime<Utc>,
}

/// Duration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationStats {
    pub min_seconds: f64,
    pub max_seconds: f64,
    pub avg_seconds: f64,
    pub p50_seconds: f64,
    pub p95_seconds: f64,
    pub p99_seconds: f64,
}

impl DurationStats {
    /// Calculate statistics from a list of durations
    pub fn from_durations(mut durations: Vec<f64>) -> Self {
        if durations.is_empty() {
            return Self::default();
        }

        durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = durations.len();
        let sum: f64 = durations.iter().sum();

        Self {
            min_seconds: durations[0],
            max_seconds: durations[count - 1],
            avg_seconds: sum / count as f64,
            p50_seconds: percentile(&durations, 50.0),
            p95_seconds: percentile(&durations, 95.0),
            p99_seconds: percentile(&durations, 99.0),
        }
    }
}

impl Default for DurationStats {
    fn default() -> Self {
        Self {
            min_seconds: 0.0,
            max_seconds: 0.0,
            avg_seconds: 0.0,
            p50_seconds: 0.0,
            p95_seconds: 0.0,
            p99_seconds: 0.0,
        }
    }
}

/// Token efficiency metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEfficiency {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tokens_per_successful_task: f64,
    pub avg_input_per_task: f64,
    pub avg_output_per_task: f64,
}

impl Default for TokenEfficiency {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            input_tokens: 0,
            output_tokens: 0,
            tokens_per_successful_task: 0.0,
            avg_input_per_task: 0.0,
            avg_output_per_task: 0.0,
        }
    }
}

/// Error pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern: String,
    pub occurrences: u64,
    pub percentage: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// Error analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAnalysis {
    pub total_errors: u64,
    pub error_rate: f64,
    pub top_patterns: Vec<ErrorPattern>,
    pub common_failures: HashMap<String, u64>,
}

impl Default for ErrorAnalysis {
    fn default() -> Self {
        Self {
            total_errors: 0,
            error_rate: 0.0,
            top_patterns: Vec::new(),
            common_failures: HashMap::new(),
        }
    }
}

/// Agent comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentComparison {
    pub agent_type: String,
    pub success_rate: f64,
    pub avg_duration_seconds: f64,
    pub tokens_per_task: f64,
    pub total_executions: u64,
    pub rank_by_success_rate: usize,
    pub rank_by_efficiency: usize,
}

/// Performance trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDataPoint {
    pub timestamp: DateTime<Utc>,
    pub success_rate: f64,
    pub avg_duration: f64,
    pub error_count: u64,
}

/// Performance trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub direction: TrendDirection,
    pub change_percentage: f64,
    pub data_points: Vec<TrendDataPoint>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

impl PerformanceAnalytics {
    /// Calculate performance metrics from agent executions
    pub fn calculate_metrics(
        agent_type: &str,
        executions: &[AgentExecution],
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> AgentPerformance {
        let total_executions = executions.len() as u64;

        if total_executions == 0 {
            return AgentPerformance::new(agent_type, period_start, period_end);
        }

        // Count successful and failed executions
        let successful_executions = executions
            .iter()
            .filter(|e| e.state == AgentState::Completed)
            .count() as u64;

        let failed_executions = total_executions - successful_executions;

        // Calculate success rate
        let success_rate = (successful_executions as f64 / total_executions as f64) * 100.0;

        // Calculate duration statistics
        let durations: Vec<f64> = executions.iter().map(|e| e.duration_seconds).collect();
        let duration_stats = DurationStats::from_durations(durations);

        // Calculate token statistics
        let total_tokens: u64 = executions
            .iter()
            .map(|e| e.input_tokens + e.output_tokens)
            .sum();

        let avg_tokens_per_execution = if total_executions > 0 {
            total_tokens / total_executions
        } else {
            0
        };

        // Calculate cost (placeholder - would need actual pricing)
        let avg_cost_per_execution = 0.0;

        AgentPerformance {
            agent_type: agent_type.to_string(),
            period_start,
            period_end,
            total_executions,
            successful_executions,
            failed_executions,
            success_rate,
            avg_duration_seconds: duration_stats.avg_seconds,
            avg_tokens_per_execution,
            avg_cost_per_execution,
            p50_duration_seconds: duration_stats.p50_seconds,
            p95_duration_seconds: duration_stats.p95_seconds,
            p99_duration_seconds: duration_stats.p99_seconds,
        }
    }

    /// Calculate token efficiency metrics
    pub fn calculate_token_efficiency(executions: &[AgentExecution]) -> TokenEfficiency {
        let successful = executions
            .iter()
            .filter(|e| e.state == AgentState::Completed)
            .collect::<Vec<_>>();

        if successful.is_empty() {
            return TokenEfficiency::default();
        }

        let total_input: u64 = executions.iter().map(|e| e.input_tokens).sum();
        let total_output: u64 = executions.iter().map(|e| e.output_tokens).sum();
        let total_tokens = total_input + total_output;

        let successful_count = successful.len() as f64;
        let total_count = executions.len() as f64;

        TokenEfficiency {
            total_tokens,
            input_tokens: total_input,
            output_tokens: total_output,
            tokens_per_successful_task: total_tokens as f64 / successful_count,
            avg_input_per_task: total_input as f64 / total_count,
            avg_output_per_task: total_output as f64 / total_count,
        }
    }

    /// Analyze error patterns from failed executions
    pub fn analyze_error_patterns(executions: &[AgentExecution]) -> ErrorAnalysis {
        let failed_executions: Vec<_> = executions
            .iter()
            .filter(|e| e.state == AgentState::Failed && e.error_message.is_some())
            .collect();

        if failed_executions.is_empty() {
            return ErrorAnalysis::default();
        }

        let total_errors = failed_executions.len() as u64;
        let total_executions = executions.len() as u64;
        let error_rate = (total_errors as f64 / total_executions as f64) * 100.0;

        // Group errors by pattern
        let mut error_groups: HashMap<String, Vec<&AgentExecution>> = HashMap::new();
        for exec in &failed_executions {
            if let Some(error) = &exec.error_message {
                // Extract error pattern (first line or key phrase)
                let pattern = extract_error_pattern(error);
                error_groups.entry(pattern).or_default().push(exec);
            }
        }

        // Create error patterns
        let mut top_patterns: Vec<ErrorPattern> = error_groups
            .iter()
            .map(|(pattern, execs)| {
                let occurrences = execs.len() as u64;
                let percentage = (occurrences as f64 / total_errors as f64) * 100.0;
                let timestamps: Vec<_> = execs.iter().map(|e| e.completed_at).collect();

                ErrorPattern {
                    pattern: pattern.clone(),
                    occurrences,
                    percentage,
                    first_seen: *timestamps.iter().min().unwrap(),
                    last_seen: *timestamps.iter().max().unwrap(),
                }
            })
            .collect();

        // Sort by occurrences (descending)
        top_patterns.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));

        // Take top 10
        top_patterns.truncate(10);

        ErrorAnalysis {
            total_errors,
            error_rate,
            top_patterns,
            common_failures: error_groups
                .into_iter()
                .map(|(k, v)| (k, v.len() as u64))
                .collect(),
        }
    }

    /// Compare performance across different agent types
    pub fn compare_agents(
        performances: &[AgentPerformance],
    ) -> Vec<AgentComparison> {
        if performances.is_empty() {
            return Vec::new();
        }

        // Calculate token efficiency for ranking
        let mut comparisons: Vec<AgentComparison> = performances
            .iter()
            .map(|p| AgentComparison {
                agent_type: p.agent_type.clone(),
                success_rate: p.success_rate,
                avg_duration_seconds: p.avg_duration_seconds,
                tokens_per_task: p.avg_tokens_per_execution as f64,
                total_executions: p.total_executions,
                rank_by_success_rate: 0,
                rank_by_efficiency: 0,
            })
            .collect();

        // Rank by success rate
        let mut by_success = comparisons.clone();
        by_success.sort_by(|a, b| {
            b.success_rate
                .partial_cmp(&a.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (rank, comp) in by_success.iter().enumerate() {
            if let Some(item) = comparisons.iter_mut().find(|c| c.agent_type == comp.agent_type) {
                item.rank_by_success_rate = rank + 1;
            }
        }

        // Rank by efficiency (tokens per task, lower is better)
        let mut by_efficiency = comparisons.clone();
        by_efficiency.sort_by(|a, b| {
            a.tokens_per_task
                .partial_cmp(&b.tokens_per_task)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (rank, comp) in by_efficiency.iter().enumerate() {
            if let Some(item) = comparisons.iter_mut().find(|c| c.agent_type == comp.agent_type) {
                item.rank_by_efficiency = rank + 1;
            }
        }

        comparisons
    }

    /// Detect performance trends over time
    pub fn detect_trend(
        data_points: Vec<TrendDataPoint>,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> PerformanceTrend {
        if data_points.len() < 2 {
            return PerformanceTrend {
                direction: TrendDirection::Stable,
                change_percentage: 0.0,
                data_points,
                period_start,
                period_end,
            };
        }

        // Calculate trend using simple linear regression on success rates
        let recent_half = &data_points[data_points.len() / 2..];
        let earlier_half = &data_points[..data_points.len() / 2];

        let recent_avg: f64 = recent_half.iter().map(|d| d.success_rate).sum::<f64>()
            / recent_half.len() as f64;
        let earlier_avg: f64 = earlier_half.iter().map(|d| d.success_rate).sum::<f64>()
            / earlier_half.len() as f64;

        let change_percentage = if earlier_avg > 0.0 {
            ((recent_avg - earlier_avg) / earlier_avg) * 100.0
        } else {
            0.0
        };

        let direction = if change_percentage > 5.0 {
            TrendDirection::Improving
        } else if change_percentage < -5.0 {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        };

        PerformanceTrend {
            direction,
            change_percentage,
            data_points,
            period_start,
            period_end,
        }
    }
}

/// Calculate percentile from sorted values
fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let index = (p / 100.0 * (sorted_values.len() - 1) as f64) as usize;
    sorted_values[index.min(sorted_values.len() - 1)]
}

/// Extract error pattern from error message
fn extract_error_pattern(error: &str) -> String {
    // Take first line or first 100 chars
    let first_line = error.lines().next().unwrap_or(error);
    if first_line.len() > 100 {
        format!("{}...", &first_line[..100])
    } else {
        first_line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_execution(
        agent_type: AgentType,
        state: AgentState,
        duration_seconds: f64,
        input_tokens: u64,
        output_tokens: u64,
        error_message: Option<String>,
    ) -> AgentExecution {
        AgentExecution {
            agent_type,
            state,
            duration_seconds,
            input_tokens,
            output_tokens,
            error_message,
            completed_at: Utc::now(),
        }
    }

    #[test]
    fn test_calculate_metrics_empty_executions() {
        let executions = vec![];
        let start = Utc::now();
        let end = Utc::now();

        let metrics = PerformanceAnalytics::calculate_metrics(
            "story-developer",
            &executions,
            start,
            end,
        );

        assert_eq!(metrics.total_executions, 0);
        assert_eq!(metrics.successful_executions, 0);
        assert_eq!(metrics.failed_executions, 0);
        assert_eq!(metrics.success_rate, 0.0);
    }

    #[test]
    fn test_calculate_metrics_all_successful() {
        let executions = vec![
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                100.0,
                1000,
                500,
                None,
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                150.0,
                1200,
                600,
                None,
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                200.0,
                1500,
                700,
                None,
            ),
        ];

        let start = Utc::now();
        let end = Utc::now();

        let metrics = PerformanceAnalytics::calculate_metrics(
            "story-developer",
            &executions,
            start,
            end,
        );

        assert_eq!(metrics.total_executions, 3);
        assert_eq!(metrics.successful_executions, 3);
        assert_eq!(metrics.failed_executions, 0);
        assert_eq!(metrics.success_rate, 100.0);
        assert_eq!(metrics.avg_duration_seconds, 150.0);
        assert_eq!(metrics.p50_duration_seconds, 150.0);
        assert!(metrics.avg_tokens_per_execution > 0);
    }

    #[test]
    fn test_calculate_metrics_mixed_results() {
        let executions = vec![
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                100.0,
                1000,
                500,
                None,
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Failed,
                50.0,
                500,
                200,
                Some("Test error".to_string()),
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                150.0,
                1200,
                600,
                None,
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Failed,
                30.0,
                300,
                100,
                Some("Another error".to_string()),
            ),
        ];

        let start = Utc::now();
        let end = Utc::now();

        let metrics = PerformanceAnalytics::calculate_metrics(
            "story-developer",
            &executions,
            start,
            end,
        );

        assert_eq!(metrics.total_executions, 4);
        assert_eq!(metrics.successful_executions, 2);
        assert_eq!(metrics.failed_executions, 2);
        assert_eq!(metrics.success_rate, 50.0);
    }

    #[test]
    fn test_duration_stats_calculation() {
        let durations = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        let stats = DurationStats::from_durations(durations);

        assert_eq!(stats.min_seconds, 10.0);
        assert_eq!(stats.max_seconds, 100.0);
        assert_eq!(stats.avg_seconds, 55.0);
        assert_eq!(stats.p50_seconds, 50.0);
        assert!(stats.p95_seconds >= 90.0);
    }

    #[test]
    fn test_duration_stats_empty() {
        let durations = vec![];
        let stats = DurationStats::from_durations(durations);

        assert_eq!(stats.min_seconds, 0.0);
        assert_eq!(stats.max_seconds, 0.0);
        assert_eq!(stats.avg_seconds, 0.0);
    }

    #[test]
    fn test_token_efficiency_calculation() {
        let executions = vec![
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                100.0,
                1000,
                500,
                None,
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                150.0,
                2000,
                1000,
                None,
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Failed,
                50.0,
                500,
                200,
                Some("Error".to_string()),
            ),
        ];

        let efficiency = PerformanceAnalytics::calculate_token_efficiency(&executions);

        assert_eq!(efficiency.input_tokens, 3500);
        assert_eq!(efficiency.output_tokens, 1700);
        assert_eq!(efficiency.total_tokens, 5200);
        assert_eq!(efficiency.tokens_per_successful_task, 2600.0); // 5200 / 2 successful
        assert!((efficiency.avg_input_per_task - 1166.67).abs() < 1.0); // 3500 / 3
    }

    #[test]
    fn test_token_efficiency_no_successful() {
        let executions = vec![
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Failed,
                50.0,
                500,
                200,
                Some("Error".to_string()),
            ),
        ];

        let efficiency = PerformanceAnalytics::calculate_token_efficiency(&executions);

        // When no successful tasks, returns default with zeros
        assert_eq!(efficiency.total_tokens, 0);
        assert_eq!(efficiency.tokens_per_successful_task, 0.0);
    }

    #[test]
    fn test_error_pattern_analysis() {
        let executions = vec![
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Failed,
                50.0,
                500,
                200,
                Some("Database connection timeout".to_string()),
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Failed,
                50.0,
                500,
                200,
                Some("Database connection timeout".to_string()),
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Failed,
                50.0,
                500,
                200,
                Some("File not found: test.txt".to_string()),
            ),
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                100.0,
                1000,
                500,
                None,
            ),
        ];

        let analysis = PerformanceAnalytics::analyze_error_patterns(&executions);

        assert_eq!(analysis.total_errors, 3);
        assert_eq!(analysis.error_rate, 75.0); // 3 out of 4
        assert_eq!(analysis.top_patterns.len(), 2);

        // Most common error should be first
        let top_error = &analysis.top_patterns[0];
        assert_eq!(top_error.pattern, "Database connection timeout");
        assert_eq!(top_error.occurrences, 2);
        assert!((top_error.percentage - 66.67).abs() < 0.1);
    }

    #[test]
    fn test_error_pattern_analysis_no_errors() {
        let executions = vec![
            create_test_execution(
                AgentType::StoryDeveloper,
                AgentState::Completed,
                100.0,
                1000,
                500,
                None,
            ),
        ];

        let analysis = PerformanceAnalytics::analyze_error_patterns(&executions);

        assert_eq!(analysis.total_errors, 0);
        assert_eq!(analysis.error_rate, 0.0);
        assert_eq!(analysis.top_patterns.len(), 0);
    }

    #[test]
    fn test_compare_agents() {
        let start = Utc::now();
        let end = Utc::now();

        let performances = vec![
            AgentPerformance {
                agent_type: "story-developer".to_string(),
                period_start: start,
                period_end: end,
                total_executions: 100,
                successful_executions: 95,
                failed_executions: 5,
                success_rate: 95.0,
                avg_duration_seconds: 120.0,
                avg_tokens_per_execution: 1500,
                avg_cost_per_execution: 0.05,
                p50_duration_seconds: 100.0,
                p95_duration_seconds: 200.0,
                p99_duration_seconds: 250.0,
            },
            AgentPerformance {
                agent_type: "code-reviewer".to_string(),
                period_start: start,
                period_end: end,
                total_executions: 50,
                successful_executions: 48,
                failed_executions: 2,
                success_rate: 96.0,
                avg_duration_seconds: 60.0,
                avg_tokens_per_execution: 800,
                avg_cost_per_execution: 0.02,
                p50_duration_seconds: 50.0,
                p95_duration_seconds: 90.0,
                p99_duration_seconds: 100.0,
            },
        ];

        let comparisons = PerformanceAnalytics::compare_agents(&performances);

        assert_eq!(comparisons.len(), 2);

        // Find code-reviewer
        let code_reviewer = comparisons
            .iter()
            .find(|c| c.agent_type == "code-reviewer")
            .unwrap();

        // Should rank first by success rate (96.0 > 95.0)
        assert_eq!(code_reviewer.rank_by_success_rate, 1);
        // Should rank first by efficiency (800 < 1500 tokens)
        assert_eq!(code_reviewer.rank_by_efficiency, 1);
    }

    #[test]
    fn test_compare_agents_empty() {
        let comparisons = PerformanceAnalytics::compare_agents(&[]);
        assert_eq!(comparisons.len(), 0);
    }

    #[test]
    fn test_detect_trend_improving() {
        let start = Utc::now();
        let end = Utc::now();

        let data_points = vec![
            TrendDataPoint {
                timestamp: start,
                success_rate: 70.0,
                avg_duration: 100.0,
                error_count: 30,
            },
            TrendDataPoint {
                timestamp: start,
                success_rate: 75.0,
                avg_duration: 95.0,
                error_count: 25,
            },
            TrendDataPoint {
                timestamp: end,
                success_rate: 85.0,
                avg_duration: 90.0,
                error_count: 15,
            },
            TrendDataPoint {
                timestamp: end,
                success_rate: 90.0,
                avg_duration: 85.0,
                error_count: 10,
            },
        ];

        let trend = PerformanceAnalytics::detect_trend(data_points, start, end);

        assert_eq!(trend.direction, TrendDirection::Improving);
        assert!(trend.change_percentage > 5.0);
    }

    #[test]
    fn test_detect_trend_degrading() {
        let start = Utc::now();
        let end = Utc::now();

        let data_points = vec![
            TrendDataPoint {
                timestamp: start,
                success_rate: 90.0,
                avg_duration: 100.0,
                error_count: 10,
            },
            TrendDataPoint {
                timestamp: start,
                success_rate: 85.0,
                avg_duration: 110.0,
                error_count: 15,
            },
            TrendDataPoint {
                timestamp: end,
                success_rate: 75.0,
                avg_duration: 120.0,
                error_count: 25,
            },
            TrendDataPoint {
                timestamp: end,
                success_rate: 70.0,
                avg_duration: 130.0,
                error_count: 30,
            },
        ];

        let trend = PerformanceAnalytics::detect_trend(data_points, start, end);

        assert_eq!(trend.direction, TrendDirection::Degrading);
        assert!(trend.change_percentage < -5.0);
    }

    #[test]
    fn test_detect_trend_stable() {
        let start = Utc::now();
        let end = Utc::now();

        let data_points = vec![
            TrendDataPoint {
                timestamp: start,
                success_rate: 80.0,
                avg_duration: 100.0,
                error_count: 20,
            },
            TrendDataPoint {
                timestamp: end,
                success_rate: 82.0,
                avg_duration: 100.0,
                error_count: 18,
            },
        ];

        let trend = PerformanceAnalytics::detect_trend(data_points, start, end);

        assert_eq!(trend.direction, TrendDirection::Stable);
        assert!(trend.change_percentage.abs() <= 5.0);
    }

    #[test]
    fn test_detect_trend_insufficient_data() {
        let start = Utc::now();
        let end = Utc::now();

        let data_points = vec![TrendDataPoint {
            timestamp: start,
            success_rate: 80.0,
            avg_duration: 100.0,
            error_count: 20,
        }];

        let trend = PerformanceAnalytics::detect_trend(data_points, start, end);

        assert_eq!(trend.direction, TrendDirection::Stable);
        assert_eq!(trend.change_percentage, 0.0);
    }

    #[test]
    fn test_percentile_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        assert_eq!(percentile(&values, 0.0), 1.0);
        assert_eq!(percentile(&values, 50.0), 5.0);
        assert_eq!(percentile(&values, 100.0), 10.0);
    }

    #[test]
    fn test_percentile_empty() {
        let values = vec![];
        assert_eq!(percentile(&values, 50.0), 0.0);
    }

    #[test]
    fn test_extract_error_pattern() {
        let error = "Database connection timeout\nRetry failed\nGiving up";
        assert_eq!(extract_error_pattern(error), "Database connection timeout");

        let long_error = "x".repeat(150);
        let pattern = extract_error_pattern(&long_error);
        assert!(pattern.len() <= 103); // 100 chars + "..."
        assert!(pattern.ends_with("..."));
    }
}
