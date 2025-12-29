//! Conditional Execution Evaluator
//!
//! This module provides condition evaluation for pipeline stages.
//! Conditions determine whether a stage should be executed based on
//! runtime context such as branch, paths, labels, and variables.

use crate::{pipeline_parser::StageCondition, Result};
use std::collections::HashMap;
use tracing::{debug, info};

/// Runtime context for condition evaluation
#[derive(Debug, Clone, Default)]
pub struct ConditionContext {
    /// Current branch name
    pub branch: Option<String>,
    /// Changed file paths (for path-based conditions)
    pub changed_paths: Vec<String>,
    /// Labels associated with the event (e.g., PR labels)
    pub labels: Vec<String>,
    /// Runtime variables
    pub variables: HashMap<String, String>,
}

impl ConditionContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the branch
    pub fn with_branch(mut self, branch: String) -> Self {
        self.branch = Some(branch);
        self
    }

    /// Set changed paths
    pub fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.changed_paths = paths;
        self
    }

    /// Set labels
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// Set variables
    pub fn with_variables(mut self, variables: HashMap<String, String>) -> Self {
        self.variables = variables;
        self
    }
}

/// Reason why a stage was skipped
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// Branch condition not met
    BranchMismatch(String),
    /// Path condition not met
    PathMismatch(String),
    /// Label condition not met
    LabelMismatch(String),
    /// Variable condition not met
    VariableMismatch(String),
    /// Complex condition (and/or) not met
    ComplexCondition(String),
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkipReason::BranchMismatch(msg) => write!(f, "Branch condition not met: {}", msg),
            SkipReason::PathMismatch(msg) => write!(f, "Path condition not met: {}", msg),
            SkipReason::LabelMismatch(msg) => write!(f, "Label condition not met: {}", msg),
            SkipReason::VariableMismatch(msg) => write!(f, "Variable condition not met: {}", msg),
            SkipReason::ComplexCondition(msg) => write!(f, "Complex condition not met: {}", msg),
        }
    }
}

/// Evaluation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationResult {
    /// Stage should be executed
    Execute,
    /// Stage should be skipped with reason
    Skip(SkipReason),
}

/// Condition evaluator for pipeline stages
pub struct ConditionEvaluator;

impl ConditionEvaluator {
    /// Create a new condition evaluator
    pub fn new() -> Self {
        Self
    }

    /// Evaluate a stage condition against the runtime context
    pub fn evaluate(
        &self,
        condition: &StageCondition,
        context: &ConditionContext,
    ) -> Result<EvaluationResult> {
        debug!("Evaluating stage condition");

        // Evaluate all top-level conditions (implicit AND)
        let mut all_conditions_met = true;
        let mut skip_reason: Option<SkipReason> = None;

        // Check branch condition
        if let Some(ref branches) = condition.branch {
            if !self.evaluate_branch(branches, context) {
                all_conditions_met = false;
                skip_reason = Some(SkipReason::BranchMismatch(format!(
                    "Branch '{}' not in allowed list: {:?}",
                    context.branch.as_deref().unwrap_or("none"),
                    branches
                )));
            }
        }

        // Check path condition
        if all_conditions_met {
            if let Some(ref paths) = condition.paths {
                if !self.evaluate_paths(paths, context) {
                    all_conditions_met = false;
                    skip_reason = Some(SkipReason::PathMismatch(format!(
                        "No changed paths match patterns: {:?}",
                        paths
                    )));
                }
            }
        }

        // Check label condition
        if all_conditions_met {
            if let Some(ref labels) = condition.labels {
                if !self.evaluate_labels(labels, context) {
                    all_conditions_met = false;
                    skip_reason = Some(SkipReason::LabelMismatch(format!(
                        "Required labels not found: {:?}",
                        labels
                    )));
                }
            }
        }

        // Check variable condition
        if all_conditions_met {
            if let Some(ref variables) = condition.variable {
                if !self.evaluate_variables(variables, context) {
                    all_conditions_met = false;
                    skip_reason = Some(SkipReason::VariableMismatch(
                        "Variable conditions not met".to_string(),
                    ));
                }
            }
        }

        // Check OR condition (alternative)
        if !all_conditions_met {
            if let Some(ref or_condition) = condition.or {
                let or_result = self.evaluate(or_condition, context)?;
                if matches!(or_result, EvaluationResult::Execute) {
                    info!("Stage will execute due to OR condition");
                    return Ok(EvaluationResult::Execute);
                }
            }
        }

        if all_conditions_met {
            Ok(EvaluationResult::Execute)
        } else {
            Ok(EvaluationResult::Skip(
                skip_reason.unwrap_or_else(|| {
                    SkipReason::ComplexCondition("Condition not met".to_string())
                }),
            ))
        }
    }

    /// Evaluate branch condition
    fn evaluate_branch(&self, allowed_branches: &[String], context: &ConditionContext) -> bool {
        if let Some(ref branch) = context.branch {
            allowed_branches.iter().any(|pattern| {
                // Support simple wildcard matching
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    branch.starts_with(prefix)
                } else {
                    branch == pattern
                }
            })
        } else {
            false
        }
    }

    /// Evaluate path condition using glob patterns
    fn evaluate_paths(&self, patterns: &[String], context: &ConditionContext) -> bool {
        if context.changed_paths.is_empty() {
            return false;
        }

        patterns.iter().any(|pattern| {
            context.changed_paths.iter().any(|path| {
                self.matches_glob(path, pattern)
            })
        })
    }

    /// Evaluate label condition
    fn evaluate_labels(&self, required_labels: &[String], context: &ConditionContext) -> bool {
        required_labels
            .iter()
            .all(|label| context.labels.contains(label))
    }

    /// Evaluate variable condition
    fn evaluate_variables(
        &self,
        required_vars: &HashMap<String, String>,
        context: &ConditionContext,
    ) -> bool {
        required_vars.iter().all(|(key, expected_value)| {
            context
                .variables
                .get(key)
                .map(|actual_value| actual_value == expected_value)
                .unwrap_or(false)
        })
    }

    /// Simple glob pattern matching
    fn matches_glob(&self, path: &str, pattern: &str) -> bool {
        // Support simple glob patterns
        // ** matches any number of directories
        // * matches any characters in a path segment

        if pattern == "**" {
            return true;
        }

        if pattern.contains("**") {
            // Handle ** pattern
            let parts: Vec<&str> = pattern.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];

                let matches_prefix = if prefix.is_empty() {
                    true
                } else {
                    path.starts_with(prefix)
                };

                let matches_suffix = if suffix.is_empty() {
                    true
                } else {
                    path.ends_with(suffix.trim_start_matches('/'))
                };

                return matches_prefix && matches_suffix;
            }
        }

        if pattern.contains('*') {
            // Handle * pattern
            let parts: Vec<&str> = pattern.split('*').collect();
            let mut search_from = 0;

            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }

                if i == 0 {
                    // First part must match the start
                    if !path.starts_with(part) {
                        return false;
                    }
                    search_from = part.len();
                } else if i == parts.len() - 1 {
                    // Last part must match the end
                    if !path.ends_with(part) {
                        return false;
                    }
                } else {
                    // Middle parts must exist in order
                    if let Some(pos) = path[search_from..].find(part) {
                        search_from += pos + part.len();
                    } else {
                        return false;
                    }
                }
            }
            return true;
        }

        // Exact match
        path == pattern
    }
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_no_condition() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new();

        // Empty condition (no fields set) should execute
        let condition = StageCondition {
            branch: None,
            paths: None,
            labels: None,
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_branch_exact_match() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_branch("main".to_string());

        let condition = StageCondition {
            branch: Some(vec!["main".to_string(), "develop".to_string()]),
            paths: None,
            labels: None,
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_branch_mismatch() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_branch("feature".to_string());

        let condition = StageCondition {
            branch: Some(vec!["main".to_string(), "develop".to_string()]),
            paths: None,
            labels: None,
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert!(matches!(result, EvaluationResult::Skip(_)));
    }

    #[test]
    fn test_evaluate_branch_wildcard() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_branch("feature/xyz".to_string());

        let condition = StageCondition {
            branch: Some(vec!["feature/*".to_string()]),
            paths: None,
            labels: None,
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_paths_exact_match() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_paths(vec![
            "docs/README.md".to_string(),
            "src/main.rs".to_string(),
        ]);

        let condition = StageCondition {
            branch: None,
            paths: Some(vec!["docs/README.md".to_string()]),
            labels: None,
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_paths_glob_pattern() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_paths(vec![
            "docs/guide.md".to_string(),
            "src/main.rs".to_string(),
        ]);

        let condition = StageCondition {
            branch: None,
            paths: Some(vec!["docs/**".to_string()]),
            labels: None,
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_paths_wildcard() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_paths(vec!["README.md".to_string()]);

        let condition = StageCondition {
            branch: None,
            paths: Some(vec!["*.md".to_string()]),
            labels: None,
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_paths_no_match() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_paths(vec!["src/main.rs".to_string()]);

        let condition = StageCondition {
            branch: None,
            paths: Some(vec!["docs/**".to_string()]),
            labels: None,
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert!(matches!(result, EvaluationResult::Skip(_)));
    }

    #[test]
    fn test_evaluate_labels_all_present() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_labels(vec![
            "needs-full-test".to_string(),
            "security".to_string(),
        ]);

        let condition = StageCondition {
            branch: None,
            paths: None,
            labels: Some(vec!["needs-full-test".to_string()]),
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_labels_missing() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_labels(vec!["security".to_string()]);

        let condition = StageCondition {
            branch: None,
            paths: None,
            labels: Some(vec!["needs-full-test".to_string()]),
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert!(matches!(result, EvaluationResult::Skip(_)));
    }

    #[test]
    fn test_evaluate_variables_match() {
        let evaluator = ConditionEvaluator::new();
        let mut vars = HashMap::new();
        vars.insert("environment".to_string(), "production".to_string());
        vars.insert("region".to_string(), "us-west-2".to_string());

        let context = ConditionContext::new().with_variables(vars);

        let mut required_vars = HashMap::new();
        required_vars.insert("environment".to_string(), "production".to_string());

        let condition = StageCondition {
            branch: None,
            paths: None,
            labels: None,
            variable: Some(required_vars),
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_variables_mismatch() {
        let evaluator = ConditionEvaluator::new();
        let mut vars = HashMap::new();
        vars.insert("environment".to_string(), "staging".to_string());

        let context = ConditionContext::new().with_variables(vars);

        let mut required_vars = HashMap::new();
        required_vars.insert("environment".to_string(), "production".to_string());

        let condition = StageCondition {
            branch: None,
            paths: None,
            labels: None,
            variable: Some(required_vars),
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert!(matches!(result, EvaluationResult::Skip(_)));
    }

    #[test]
    fn test_evaluate_complex_and_all_match() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new()
            .with_branch("main".to_string())
            .with_paths(vec!["docs/README.md".to_string()])
            .with_labels(vec!["needs-docs-deploy".to_string()]);

        let condition = StageCondition {
            branch: Some(vec!["main".to_string()]),
            paths: Some(vec!["docs/**".to_string()]),
            labels: Some(vec!["needs-docs-deploy".to_string()]),
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_complex_and_one_fails() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new()
            .with_branch("develop".to_string()) // Wrong branch
            .with_paths(vec!["docs/README.md".to_string()])
            .with_labels(vec!["needs-docs-deploy".to_string()]);

        let condition = StageCondition {
            branch: Some(vec!["main".to_string()]),
            paths: Some(vec!["docs/**".to_string()]),
            labels: Some(vec!["needs-docs-deploy".to_string()]),
            variable: None,
            or: None,
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert!(matches!(result, EvaluationResult::Skip(_)));
    }

    #[test]
    fn test_evaluate_or_condition_first_succeeds() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_labels(vec!["needs-full-test".to_string()]);

        let condition = StageCondition {
            branch: None,
            paths: None,
            labels: Some(vec!["needs-full-test".to_string()]),
            variable: None,
            or: Some(Box::new(StageCondition {
                branch: None,
                paths: Some(vec!["src/core/**".to_string()]),
                labels: None,
                variable: None,
                or: None,
            })),
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_or_condition_second_succeeds() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_paths(vec!["src/core/main.rs".to_string()]);

        let condition = StageCondition {
            branch: None,
            paths: None,
            labels: Some(vec!["needs-full-test".to_string()]), // This will fail
            variable: None,
            or: Some(Box::new(StageCondition {
                branch: None,
                paths: Some(vec!["src/core/**".to_string()]), // This will succeed
                labels: None,
                variable: None,
                or: None,
            })),
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert_eq!(result, EvaluationResult::Execute);
    }

    #[test]
    fn test_evaluate_or_condition_both_fail() {
        let evaluator = ConditionEvaluator::new();
        let context = ConditionContext::new().with_paths(vec!["docs/README.md".to_string()]);

        let condition = StageCondition {
            branch: None,
            paths: None,
            labels: Some(vec!["needs-full-test".to_string()]), // Fails
            variable: None,
            or: Some(Box::new(StageCondition {
                branch: None,
                paths: Some(vec!["src/core/**".to_string()]), // Also fails
                labels: None,
                variable: None,
                or: None,
            })),
        };

        let result = evaluator.evaluate(&condition, &context).unwrap();
        assert!(matches!(result, EvaluationResult::Skip(_)));
    }

    #[test]
    fn test_glob_exact_match() {
        let evaluator = ConditionEvaluator::new();
        assert!(evaluator.matches_glob("docs/README.md", "docs/README.md"));
        assert!(!evaluator.matches_glob("docs/guide.md", "docs/README.md"));
    }

    #[test]
    fn test_glob_star_wildcard() {
        let evaluator = ConditionEvaluator::new();
        assert!(evaluator.matches_glob("README.md", "*.md"));
        assert!(evaluator.matches_glob("guide.md", "*.md"));
        assert!(!evaluator.matches_glob("README.txt", "*.md"));
    }

    #[test]
    fn test_glob_double_star() {
        let evaluator = ConditionEvaluator::new();
        assert!(evaluator.matches_glob("docs/guide.md", "docs/**"));
        assert!(evaluator.matches_glob("docs/api/reference.md", "docs/**"));
        assert!(!evaluator.matches_glob("src/main.rs", "docs/**"));
    }

    #[test]
    fn test_glob_double_star_suffix() {
        let evaluator = ConditionEvaluator::new();
        assert!(evaluator.matches_glob("src/core/main.rs", "**/main.rs"));
        assert!(evaluator.matches_glob("main.rs", "**/main.rs"));
        assert!(!evaluator.matches_glob("src/core/lib.rs", "**/main.rs"));
    }

    #[test]
    fn test_skip_reason_display() {
        let reason = SkipReason::BranchMismatch("test".to_string());
        assert_eq!(
            reason.to_string(),
            "Branch condition not met: test"
        );

        let reason = SkipReason::PathMismatch("test".to_string());
        assert_eq!(reason.to_string(), "Path condition not met: test");
    }
}
