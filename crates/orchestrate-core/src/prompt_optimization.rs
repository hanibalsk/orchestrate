//! Prompt Optimization
//!
//! Provides automated prompt improvement based on outcome analysis,
//! versioning, and A/B testing integration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// A versioned prompt with effectiveness tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVersion {
    pub id: i64,
    pub agent_type: String,
    pub version: i32,
    pub content: String,
    pub description: Option<String>,
    pub parent_version_id: Option<i64>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub deactivated_at: Option<DateTime<Utc>>,
}

impl PromptVersion {
    pub fn new(agent_type: String, version: i32, content: String) -> Self {
        Self {
            id: 0,
            agent_type,
            version,
            content,
            description: None,
            parent_version_id: None,
            is_active: false,
            created_at: Utc::now(),
            activated_at: None,
            deactivated_at: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_parent(mut self, parent_id: i64) -> Self {
        self.parent_version_id = Some(parent_id);
        self
    }
}

/// Effectiveness metrics for a prompt version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEffectiveness {
    pub prompt_version_id: i64,
    pub usage_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub success_rate: f64,
    pub avg_tokens: f64,
    pub avg_duration_secs: f64,
    pub avg_feedback_score: f64,
    pub updated_at: DateTime<Utc>,
}

impl PromptEffectiveness {
    pub fn new(prompt_version_id: i64) -> Self {
        Self {
            prompt_version_id,
            usage_count: 0,
            success_count: 0,
            failure_count: 0,
            success_rate: 0.0,
            avg_tokens: 0.0,
            avg_duration_secs: 0.0,
            avg_feedback_score: 0.0,
            updated_at: Utc::now(),
        }
    }
}

/// Prompt section type for analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptSection {
    /// System instructions
    SystemInstructions,
    /// Role definition
    RoleDefinition,
    /// Task description
    TaskDescription,
    /// Constraints and rules
    Constraints,
    /// Output format specification
    OutputFormat,
    /// Examples provided
    Examples,
    /// Context information
    Context,
    /// Custom section
    Custom,
}

impl PromptSection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SystemInstructions => "system_instructions",
            Self::RoleDefinition => "role_definition",
            Self::TaskDescription => "task_description",
            Self::Constraints => "constraints",
            Self::OutputFormat => "output_format",
            Self::Examples => "examples",
            Self::Context => "context",
            Self::Custom => "custom",
        }
    }
}

impl FromStr for PromptSection {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "system_instructions" | "system" => Ok(Self::SystemInstructions),
            "role_definition" | "role" => Ok(Self::RoleDefinition),
            "task_description" | "task" => Ok(Self::TaskDescription),
            "constraints" | "rules" => Ok(Self::Constraints),
            "output_format" | "output" | "format" => Ok(Self::OutputFormat),
            "examples" => Ok(Self::Examples),
            "context" => Ok(Self::Context),
            "custom" => Ok(Self::Custom),
            _ => Err(crate::Error::Other(format!(
                "Invalid prompt section: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for PromptSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Analysis of a prompt section's effectiveness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionAnalysis {
    pub section: PromptSection,
    pub content_hash: String,
    pub correlation_with_success: f64,
    pub correlation_with_failure: f64,
    pub sample_count: i64,
    pub suggestions: Vec<String>,
}

/// Improvement suggestion for a prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSuggestion {
    pub id: i64,
    pub agent_type: String,
    pub section: PromptSection,
    pub current_content: Option<String>,
    pub suggested_content: String,
    pub reasoning: String,
    pub expected_improvement: f64,
    pub confidence: f64,
    pub status: SuggestionStatus,
    pub created_at: DateTime<Utc>,
    pub applied_at: Option<DateTime<Utc>>,
}

/// Status of a prompt suggestion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionStatus {
    /// Pending review
    Pending,
    /// Approved for testing
    Approved,
    /// Rejected
    Rejected,
    /// Being tested via A/B
    Testing,
    /// Applied to production
    Applied,
    /// Rolled back after testing
    RolledBack,
}

impl SuggestionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Testing => "testing",
            Self::Applied => "applied",
            Self::RolledBack => "rolled_back",
        }
    }
}

impl FromStr for SuggestionStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "testing" => Ok(Self::Testing),
            "applied" => Ok(Self::Applied),
            "rolled_back" | "rolledback" => Ok(Self::RolledBack),
            _ => Err(crate::Error::Other(format!(
                "Invalid suggestion status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for SuggestionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Prompt optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOptimizationConfig {
    pub enabled: bool,
    pub auto_suggest: bool,
    pub auto_test: bool,
    pub min_samples_for_analysis: i64,
    pub min_improvement_threshold: f64,
    pub confidence_threshold: f64,
}

impl Default for PromptOptimizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_suggest: true,
            auto_test: false,
            min_samples_for_analysis: 20,
            min_improvement_threshold: 0.05,
            confidence_threshold: 0.7,
        }
    }
}

/// Analyze prompt sections and generate suggestions
pub fn analyze_prompt_sections(
    prompt: &str,
    success_patterns: &[String],
    failure_patterns: &[String],
) -> Vec<SectionAnalysis> {
    let mut analyses = Vec::new();

    // Simple heuristic analysis based on common patterns
    let sections = extract_sections(prompt);

    for (section, content) in sections {
        let content_hash = format!("{:x}", md5::compute(&content));

        // Check correlation with success/failure patterns
        let success_matches = success_patterns
            .iter()
            .filter(|p| content.contains(p.as_str()))
            .count();
        let failure_matches = failure_patterns
            .iter()
            .filter(|p| content.contains(p.as_str()))
            .count();

        let total_patterns = success_patterns.len() + failure_patterns.len();
        let correlation_with_success = if total_patterns > 0 {
            success_matches as f64 / total_patterns as f64
        } else {
            0.5
        };
        let correlation_with_failure = if total_patterns > 0 {
            failure_matches as f64 / total_patterns as f64
        } else {
            0.5
        };

        let suggestions = generate_section_suggestions(&section, &content, correlation_with_failure);

        analyses.push(SectionAnalysis {
            section,
            content_hash,
            correlation_with_success,
            correlation_with_failure,
            sample_count: 0,
            suggestions,
        });
    }

    analyses
}

/// Extract sections from a prompt
fn extract_sections(prompt: &str) -> HashMap<PromptSection, String> {
    let mut sections = HashMap::new();

    // Simple heuristic extraction based on common markers
    let lines: Vec<&str> = prompt.lines().collect();
    let mut current_section = PromptSection::SystemInstructions;
    let mut current_content = String::new();

    for line in lines {
        let line_lower = line.to_lowercase();

        // Detect section changes based on common headers
        let new_section = if line_lower.contains("role:") || line_lower.contains("you are") {
            Some(PromptSection::RoleDefinition)
        } else if line_lower.contains("task:") || line_lower.contains("your task") {
            Some(PromptSection::TaskDescription)
        } else if line_lower.contains("constraint") || line_lower.contains("rule") || line_lower.contains("must") {
            Some(PromptSection::Constraints)
        } else if line_lower.contains("format:") || line_lower.contains("output:") {
            Some(PromptSection::OutputFormat)
        } else if line_lower.contains("example") {
            Some(PromptSection::Examples)
        } else if line_lower.contains("context:") {
            Some(PromptSection::Context)
        } else {
            None
        };

        if let Some(section) = new_section {
            if !current_content.is_empty() {
                sections.insert(current_section, current_content.trim().to_string());
            }
            current_section = section;
            current_content = String::new();
        }

        current_content.push_str(line);
        current_content.push('\n');
    }

    if !current_content.is_empty() {
        sections.insert(current_section, current_content.trim().to_string());
    }

    sections
}

/// Generate suggestions for a section based on analysis
fn generate_section_suggestions(
    section: &PromptSection,
    content: &str,
    failure_correlation: f64,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    // Only generate suggestions if there's significant failure correlation
    if failure_correlation < 0.3 {
        return suggestions;
    }

    match section {
        PromptSection::Constraints => {
            if content.len() < 100 {
                suggestions.push("Consider adding more specific constraints".to_string());
            }
            if !content.contains("must") && !content.contains("always") && !content.contains("never") {
                suggestions.push("Add explicit requirements using 'must', 'always', or 'never'".to_string());
            }
        }
        PromptSection::OutputFormat => {
            if !content.contains("```") && !content.contains("json") && !content.contains("format") {
                suggestions.push("Consider adding explicit output format examples".to_string());
            }
        }
        PromptSection::Examples => {
            let example_count = content.matches("example").count();
            if example_count < 2 {
                suggestions.push("Add more examples (recommend 2-3 diverse examples)".to_string());
            }
        }
        PromptSection::TaskDescription => {
            if content.len() < 50 {
                suggestions.push("Task description may be too brief - add more detail".to_string());
            }
        }
        _ => {}
    }

    suggestions
}

/// Calculate prompt similarity for version comparison
pub fn prompt_similarity(prompt1: &str, prompt2: &str) -> f64 {
    let words1: std::collections::HashSet<&str> = prompt1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = prompt2.split_whitespace().collect();

    if words1.is_empty() && words2.is_empty() {
        return 1.0;
    }

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_section_roundtrip() {
        let sections = [
            PromptSection::SystemInstructions,
            PromptSection::RoleDefinition,
            PromptSection::TaskDescription,
            PromptSection::Constraints,
            PromptSection::OutputFormat,
            PromptSection::Examples,
            PromptSection::Context,
            PromptSection::Custom,
        ];

        for section in sections {
            let s = section.as_str();
            let parsed = PromptSection::from_str(s).unwrap();
            assert_eq!(section, parsed);
        }
    }

    #[test]
    fn test_suggestion_status_roundtrip() {
        let statuses = [
            SuggestionStatus::Pending,
            SuggestionStatus::Approved,
            SuggestionStatus::Rejected,
            SuggestionStatus::Testing,
            SuggestionStatus::Applied,
            SuggestionStatus::RolledBack,
        ];

        for status in statuses {
            let s = status.as_str();
            let parsed = SuggestionStatus::from_str(s).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_prompt_similarity() {
        let prompt1 = "You are a helpful assistant";
        let prompt2 = "You are a helpful coding assistant";

        let similarity = prompt_similarity(prompt1, prompt2);
        assert!(similarity > 0.5 && similarity < 1.0);

        let identical = prompt_similarity(prompt1, prompt1);
        assert!((identical - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_extract_sections() {
        let prompt = r#"
You are a code reviewer.

Task: Review the code for bugs.

Constraints:
- Must check for null pointers
- Always verify input validation

Output format:
```json
{"issues": [...]}
```
"#;

        let sections = extract_sections(prompt);
        assert!(sections.contains_key(&PromptSection::RoleDefinition));
        assert!(sections.contains_key(&PromptSection::TaskDescription));
        assert!(sections.contains_key(&PromptSection::Constraints));
        assert!(sections.contains_key(&PromptSection::OutputFormat));
    }
}
