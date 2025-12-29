//! Cross-Project Pattern Learning
//!
//! Export and import learned patterns between projects for
//! knowledge sharing and bootstrapping new projects.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Exportable pattern type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExportablePattern {
    /// A custom instruction
    Instruction(InstructionPattern),
    /// A successful tool sequence
    ToolSequence(ToolSequencePattern),
    /// A prompt template
    PromptTemplate(PromptTemplatePattern),
    /// A success pattern
    SuccessPattern(SuccessPatternExport),
}

/// Exported instruction pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionPattern {
    pub name: String,
    pub content: String,
    pub scope: String,
    pub agent_types: Vec<String>,
    pub tags: Vec<String>,
    pub effectiveness: PatternEffectiveness,
    pub context: PatternContext,
}

/// Exported tool sequence pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSequencePattern {
    pub name: String,
    pub sequence: Vec<String>,
    pub task_type: String,
    pub agent_types: Vec<String>,
    pub effectiveness: PatternEffectiveness,
    pub context: PatternContext,
}

/// Exported prompt template pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplatePattern {
    pub name: String,
    pub agent_type: String,
    pub template: String,
    pub variables: Vec<String>,
    pub effectiveness: PatternEffectiveness,
    pub context: PatternContext,
}

/// Exported success pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPatternExport {
    pub pattern_type: String,
    pub signature: String,
    pub data: serde_json::Value,
    pub agent_types: Vec<String>,
    pub effectiveness: PatternEffectiveness,
    pub context: PatternContext,
}

/// Effectiveness metrics for an exported pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEffectiveness {
    pub success_rate: f64,
    pub sample_size: i64,
    pub avg_tokens: Option<f64>,
    pub avg_duration_secs: Option<f64>,
}

impl PatternEffectiveness {
    pub fn new(success_rate: f64, sample_size: i64) -> Self {
        Self {
            success_rate,
            sample_size,
            avg_tokens: None,
            avg_duration_secs: None,
        }
    }
}

/// Context information for a pattern
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatternContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub custom: HashMap<String, String>,
}

impl PatternContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_framework(mut self, framework: String) -> Self {
        self.framework = Some(framework);
        self
    }

    pub fn with_project_type(mut self, project_type: String) -> Self {
        self.project_type = Some(project_type);
        self
    }

    /// Check if this context matches another (for import filtering)
    pub fn matches(&self, other: &PatternContext) -> bool {
        // If source has a language requirement, target must match
        if let Some(ref lang) = self.language {
            if let Some(ref other_lang) = other.language {
                if lang != other_lang {
                    return false;
                }
            }
        }

        // Same for framework
        if let Some(ref fw) = self.framework {
            if let Some(ref other_fw) = other.framework {
                if fw != other_fw {
                    return false;
                }
            }
        }

        // Same for project type
        if let Some(ref pt) = self.project_type {
            if let Some(ref other_pt) = other.project_type {
                if pt != other_pt {
                    return false;
                }
            }
        }

        true
    }
}

/// Complete export bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternExport {
    pub version: String,
    pub exported_at: DateTime<Utc>,
    pub source_project: Option<String>,
    pub patterns: Vec<ExportablePattern>,
    pub metadata: ExportMetadata,
}

impl PatternExport {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            exported_at: Utc::now(),
            source_project: None,
            patterns: Vec::new(),
            metadata: ExportMetadata::default(),
        }
    }

    pub fn with_source_project(mut self, project: String) -> Self {
        self.source_project = Some(project);
        self
    }

    pub fn add_pattern(mut self, pattern: ExportablePattern) -> Self {
        self.patterns.push(pattern);
        self
    }

    /// Serialize to YAML
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Deserialize from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for PatternExport {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about the export
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportMetadata {
    pub total_patterns: usize,
    pub instruction_count: usize,
    pub tool_sequence_count: usize,
    pub prompt_template_count: usize,
    pub success_pattern_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
}

/// Import options for controlling how patterns are applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Only import patterns with success rate >= this threshold
    pub min_success_rate: f64,
    /// Only import patterns with sample size >= this threshold
    pub min_sample_size: i64,
    /// Adjust confidence for imported patterns (multiply by this factor)
    pub confidence_adjustment: f64,
    /// Skip patterns that already exist
    pub skip_existing: bool,
    /// Filter by context
    pub context_filter: Option<PatternContext>,
    /// Patterns types to import (empty = all)
    pub pattern_types: Vec<String>,
    /// Dry run - don't actually import
    pub dry_run: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            min_success_rate: 0.7,
            min_sample_size: 10,
            confidence_adjustment: 0.8,
            skip_existing: true,
            context_filter: None,
            pattern_types: Vec::new(),
            dry_run: false,
        }
    }
}

/// Result of an import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub total_patterns: usize,
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub details: Vec<ImportDetail>,
}

impl ImportResult {
    pub fn new() -> Self {
        Self {
            total_patterns: 0,
            imported: 0,
            skipped: 0,
            failed: 0,
            details: Vec::new(),
        }
    }
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Detail about a single pattern import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDetail {
    pub pattern_name: String,
    pub pattern_type: String,
    pub status: ImportStatus,
    pub message: Option<String>,
}

/// Status of a pattern import
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Imported,
    Skipped,
    Failed,
}

/// Filter patterns in an export based on options
pub fn filter_patterns<'a>(
    export: &'a PatternExport,
    options: &ImportOptions,
) -> Vec<&'a ExportablePattern> {
    export
        .patterns
        .iter()
        .filter(|p| {
            let effectiveness = match p {
                ExportablePattern::Instruction(i) => &i.effectiveness,
                ExportablePattern::ToolSequence(t) => &t.effectiveness,
                ExportablePattern::PromptTemplate(pt) => &pt.effectiveness,
                ExportablePattern::SuccessPattern(sp) => &sp.effectiveness,
            };

            // Check success rate
            if effectiveness.success_rate < options.min_success_rate {
                return false;
            }

            // Check sample size
            if effectiveness.sample_size < options.min_sample_size {
                return false;
            }

            // Check context filter
            if let Some(ref filter) = options.context_filter {
                let context = match p {
                    ExportablePattern::Instruction(i) => &i.context,
                    ExportablePattern::ToolSequence(t) => &t.context,
                    ExportablePattern::PromptTemplate(pt) => &pt.context,
                    ExportablePattern::SuccessPattern(sp) => &sp.context,
                };

                if !filter.matches(context) {
                    return false;
                }
            }

            // Check pattern type filter
            if !options.pattern_types.is_empty() {
                let type_name = match p {
                    ExportablePattern::Instruction(_) => "instruction",
                    ExportablePattern::ToolSequence(_) => "tool_sequence",
                    ExportablePattern::PromptTemplate(_) => "prompt_template",
                    ExportablePattern::SuccessPattern(_) => "success_pattern",
                };

                if !options.pattern_types.contains(&type_name.to_string()) {
                    return false;
                }
            }

            true
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_export_serialization() {
        let mut export = PatternExport::new()
            .with_source_project("test-project".to_string());

        let instruction = InstructionPattern {
            name: "null-check".to_string(),
            content: "Always check for null".to_string(),
            scope: "global".to_string(),
            agent_types: vec!["story-developer".to_string()],
            tags: vec!["safety".to_string()],
            effectiveness: PatternEffectiveness::new(0.87, 234),
            context: PatternContext::new().with_language("typescript".to_string()),
        };

        export = export.add_pattern(ExportablePattern::Instruction(instruction));

        let yaml = export.to_yaml().unwrap();
        assert!(yaml.contains("null-check"));
        assert!(yaml.contains("typescript"));

        let parsed = PatternExport::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.patterns.len(), 1);
    }

    #[test]
    fn test_context_matching() {
        let ts_context = PatternContext::new().with_language("typescript".to_string());
        let rust_context = PatternContext::new().with_language("rust".to_string());
        let empty_context = PatternContext::new();

        // Same language matches
        assert!(ts_context.matches(&ts_context));

        // Different languages don't match
        assert!(!ts_context.matches(&rust_context));

        // Empty context matches anything
        assert!(empty_context.matches(&ts_context));
        assert!(empty_context.matches(&rust_context));
    }

    #[test]
    fn test_filter_patterns() {
        let mut export = PatternExport::new();

        let good_pattern = ExportablePattern::Instruction(InstructionPattern {
            name: "good".to_string(),
            content: "Good pattern".to_string(),
            scope: "global".to_string(),
            agent_types: vec![],
            tags: vec![],
            effectiveness: PatternEffectiveness::new(0.9, 100),
            context: PatternContext::new(),
        });

        let bad_pattern = ExportablePattern::Instruction(InstructionPattern {
            name: "bad".to_string(),
            content: "Bad pattern".to_string(),
            scope: "global".to_string(),
            agent_types: vec![],
            tags: vec![],
            effectiveness: PatternEffectiveness::new(0.5, 5),
            context: PatternContext::new(),
        });

        export.patterns.push(good_pattern);
        export.patterns.push(bad_pattern);

        let options = ImportOptions::default();
        let filtered = filter_patterns(&export, &options);

        // Only the good pattern should pass
        assert_eq!(filtered.len(), 1);
    }
}
