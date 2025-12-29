//! Requirements Capture Module
//!
//! Types and utilities for requirements gathering, story generation,
//! and traceability tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Requirement type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementType {
    Functional,
    NonFunctional,
    Constraint,
    Interface,
    Security,
    Performance,
    Usability,
}

impl RequirementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Functional => "functional",
            Self::NonFunctional => "non_functional",
            Self::Constraint => "constraint",
            Self::Interface => "interface",
            Self::Security => "security",
            Self::Performance => "performance",
            Self::Usability => "usability",
        }
    }
}

impl std::fmt::Display for RequirementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Requirement priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl RequirementPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Requirement status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    Draft,
    Proposed,
    Approved,
    InProgress,
    Implemented,
    Verified,
    Rejected,
    Deferred,
}

impl RequirementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::InProgress => "in_progress",
            Self::Implemented => "implemented",
            Self::Verified => "verified",
            Self::Rejected => "rejected",
            Self::Deferred => "deferred",
        }
    }
}

impl std::str::FromStr for RequirementStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "proposed" => Ok(Self::Proposed),
            "approved" => Ok(Self::Approved),
            "in_progress" | "inprogress" => Ok(Self::InProgress),
            "implemented" => Ok(Self::Implemented),
            "verified" => Ok(Self::Verified),
            "rejected" => Ok(Self::Rejected),
            "deferred" => Ok(Self::Deferred),
            _ => Err(format!("Invalid requirement status: {}", s)),
        }
    }
}

/// A captured requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub requirement_type: RequirementType,
    pub priority: RequirementPriority,
    pub status: RequirementStatus,
    pub stakeholders: Vec<String>,
    pub actors: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub dependencies: Vec<String>,
    pub related_requirements: Vec<String>,
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

impl Requirement {
    /// Create a new requirement
    pub fn new(id: &str, title: &str, description: &str, req_type: RequirementType) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            requirement_type: req_type,
            priority: RequirementPriority::Medium,
            status: RequirementStatus::Draft,
            stakeholders: vec![],
            actors: vec![],
            acceptance_criteria: vec![],
            dependencies: vec![],
            related_requirements: vec![],
            tags: vec![],
            source: None,
            created_at: now,
            updated_at: now,
            version: 1,
        }
    }

    /// Generate markdown representation
    pub fn to_markdown(&self) -> String {
        let mut output = format!("# {}: {}\n\n", self.id, self.title);
        output.push_str(&format!("**Type:** {}\n", self.requirement_type));
        output.push_str(&format!("**Priority:** {}\n", self.priority.as_str()));
        output.push_str(&format!("**Status:** {}\n\n", self.status.as_str()));

        output.push_str("## Description\n\n");
        output.push_str(&self.description);
        output.push_str("\n\n");

        if !self.actors.is_empty() {
            output.push_str("## Actors\n\n");
            for actor in &self.actors {
                output.push_str(&format!("- {}\n", actor));
            }
            output.push('\n');
        }

        if !self.acceptance_criteria.is_empty() {
            output.push_str("## Acceptance Criteria\n\n");
            for criteria in &self.acceptance_criteria {
                output.push_str(&format!("- [ ] {}\n", criteria));
            }
            output.push('\n');
        }

        if !self.dependencies.is_empty() {
            output.push_str("## Dependencies\n\n");
            for dep in &self.dependencies {
                output.push_str(&format!("- {}\n", dep));
            }
            output.push('\n');
        }

        if !self.tags.is_empty() {
            output.push_str(&format!("**Tags:** {}\n", self.tags.join(", ")));
        }

        output
    }
}

/// A clarifying question for requirements refinement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyingQuestion {
    pub id: String,
    pub requirement_id: String,
    pub question: String,
    pub context: String,
    pub options: Vec<String>,
    pub answer: Option<String>,
    pub answered_at: Option<DateTime<Utc>>,
}

/// Generated user story from requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedStory {
    pub title: String,
    pub user_type: String,
    pub goal: String,
    pub benefit: String,
    pub acceptance_criteria: Vec<String>,
    pub complexity: StoryComplexity,
    pub related_requirements: Vec<String>,
    pub suggested_epic: Option<String>,
}

/// Story complexity estimate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryComplexity {
    Simple,
    Medium,
    Complex,
    Epic,
}

impl StoryComplexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Medium => "medium",
            Self::Complex => "complex",
            Self::Epic => "epic",
        }
    }

    pub fn story_points(&self) -> u32 {
        match self {
            Self::Simple => 1,
            Self::Medium => 3,
            Self::Complex => 5,
            Self::Epic => 13,
        }
    }
}

impl GeneratedStory {
    /// Generate user story in standard format
    pub fn to_markdown(&self) -> String {
        let mut output = format!("# Story: {}\n\n", self.title);
        output.push_str(&format!(
            "As a {}\nI want to {}\nSo that {}\n\n",
            self.user_type, self.goal, self.benefit
        ));

        output.push_str("## Acceptance Criteria\n\n");
        for criteria in &self.acceptance_criteria {
            output.push_str(&format!("- [ ] {}\n", criteria));
        }
        output.push('\n');

        output.push_str(&format!("## Complexity: {}\n", self.complexity.as_str()));
        output.push_str(&format!(
            "## Story Points: {}\n",
            self.complexity.story_points()
        ));

        if !self.related_requirements.is_empty() {
            output.push_str(&format!(
                "## Related Requirements: {}\n",
                self.related_requirements.join(", ")
            ));
        }

        if let Some(ref epic) = self.suggested_epic {
            output.push_str(&format!("## Suggested Epic: {}\n", epic));
        }

        output
    }
}

/// Traceability link between artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceabilityLink {
    pub source_type: ArtifactType,
    pub source_id: String,
    pub target_type: ArtifactType,
    pub target_id: String,
    pub link_type: LinkType,
    pub created_at: DateTime<Utc>,
}

/// Artifact type for traceability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Requirement,
    Epic,
    Story,
    Task,
    Commit,
    Test,
    CodeFile,
}

impl ArtifactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Epic => "epic",
            Self::Story => "story",
            Self::Task => "task",
            Self::Commit => "commit",
            Self::Test => "test",
            Self::CodeFile => "code_file",
        }
    }
}

/// Type of traceability link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    DerivedFrom,
    ImplementedBy,
    TestedBy,
    DependsOn,
    RelatedTo,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DerivedFrom => "derived_from",
            Self::ImplementedBy => "implemented_by",
            Self::TestedBy => "tested_by",
            Self::DependsOn => "depends_on",
            Self::RelatedTo => "related_to",
        }
    }
}

/// Traceability matrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceabilityMatrix {
    pub requirements: Vec<String>,
    pub stories: Vec<String>,
    pub links: Vec<TraceabilityLink>,
    pub coverage: HashMap<String, TraceCoverage>,
    pub generated_at: DateTime<Utc>,
}

/// Coverage information for a requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCoverage {
    pub requirement_id: String,
    pub stories_count: usize,
    pub tests_count: usize,
    pub code_files_count: usize,
    pub is_fully_covered: bool,
}

impl TraceabilityMatrix {
    /// Create a new empty traceability matrix
    pub fn new() -> Self {
        Self {
            requirements: vec![],
            stories: vec![],
            links: vec![],
            coverage: HashMap::new(),
            generated_at: Utc::now(),
        }
    }

    /// Add a link to the matrix
    pub fn add_link(&mut self, link: TraceabilityLink) {
        if matches!(link.source_type, ArtifactType::Requirement) {
            if !self.requirements.contains(&link.source_id) {
                self.requirements.push(link.source_id.clone());
            }
        }
        if matches!(link.target_type, ArtifactType::Story) {
            if !self.stories.contains(&link.target_id) {
                self.stories.push(link.target_id.clone());
            }
        }
        self.links.push(link);
    }

    /// Calculate coverage for all requirements
    pub fn calculate_coverage(&mut self) {
        self.coverage.clear();

        for req_id in &self.requirements {
            let stories_count = self
                .links
                .iter()
                .filter(|l| {
                    l.source_id == *req_id
                        && matches!(l.target_type, ArtifactType::Story)
                        && matches!(l.link_type, LinkType::ImplementedBy)
                })
                .count();

            let tests_count = self
                .links
                .iter()
                .filter(|l| {
                    l.source_id == *req_id
                        && matches!(l.target_type, ArtifactType::Test)
                        && matches!(l.link_type, LinkType::TestedBy)
                })
                .count();

            let code_files_count = self
                .links
                .iter()
                .filter(|l| {
                    l.source_id == *req_id && matches!(l.target_type, ArtifactType::CodeFile)
                })
                .count();

            self.coverage.insert(
                req_id.clone(),
                TraceCoverage {
                    requirement_id: req_id.clone(),
                    stories_count,
                    tests_count,
                    code_files_count,
                    is_fully_covered: stories_count > 0 && tests_count > 0,
                },
            );
        }
    }

    /// Generate markdown representation
    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# Requirements Traceability Matrix\n\n");
        output.push_str(&format!(
            "Generated: {}\n\n",
            self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        output.push_str("## Coverage Summary\n\n");
        output.push_str("| Requirement | Stories | Tests | Code Files | Covered |\n");
        output.push_str("|-------------|---------|-------|------------|--------|\n");

        let mut sorted_reqs: Vec<_> = self.coverage.values().collect();
        sorted_reqs.sort_by(|a, b| a.requirement_id.cmp(&b.requirement_id));

        for cov in sorted_reqs {
            let covered = if cov.is_fully_covered { "✓" } else { "✗" };
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                cov.requirement_id,
                cov.stories_count,
                cov.tests_count,
                cov.code_files_count,
                covered
            ));
        }

        output
    }
}

impl Default for TraceabilityMatrix {
    fn default() -> Self {
        Self::new()
    }
}

/// Impact analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    pub requirement_id: String,
    pub affected_stories: Vec<String>,
    pub affected_code_files: Vec<String>,
    pub affected_tests: Vec<String>,
    pub estimated_effort: EffortEstimate,
    pub risk_level: RiskLevel,
    pub recommendations: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

/// Effort estimate for changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffortEstimate {
    Minimal,
    Low,
    Medium,
    High,
    VeryHigh,
}

impl EffortEstimate {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::VeryHigh => "very_high",
        }
    }
}

/// Risk level for changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_requirement_creation() {
        let req = Requirement::new(
            "REQ-001",
            "User Login",
            "Users should be able to log in with email and password",
            RequirementType::Functional,
        );

        assert_eq!(req.id, "REQ-001");
        assert_eq!(req.title, "User Login");
        assert_eq!(req.requirement_type, RequirementType::Functional);
        assert_eq!(req.status, RequirementStatus::Draft);
        assert_eq!(req.version, 1);
    }

    #[test]
    fn test_requirement_markdown() {
        let mut req = Requirement::new(
            "REQ-001",
            "User Login",
            "Users should be able to log in with email and password",
            RequirementType::Functional,
        );
        req.actors.push("Registered User".to_string());
        req.acceptance_criteria
            .push("User can enter email".to_string());
        req.acceptance_criteria
            .push("User can enter password".to_string());

        let md = req.to_markdown();
        assert!(md.contains("# REQ-001: User Login"));
        assert!(md.contains("**Type:** functional"));
        assert!(md.contains("## Actors"));
        assert!(md.contains("- Registered User"));
        assert!(md.contains("## Acceptance Criteria"));
        assert!(md.contains("- [ ] User can enter email"));
    }

    #[test]
    fn test_generated_story() {
        let story = GeneratedStory {
            title: "User Login".to_string(),
            user_type: "registered user".to_string(),
            goal: "log in with my email and password".to_string(),
            benefit: "I can access my account".to_string(),
            acceptance_criteria: vec![
                "User can enter email".to_string(),
                "User can enter password".to_string(),
            ],
            complexity: StoryComplexity::Medium,
            related_requirements: vec!["REQ-001".to_string()],
            suggested_epic: Some("Authentication".to_string()),
        };

        let md = story.to_markdown();
        assert!(md.contains("As a registered user"));
        assert!(md.contains("I want to log in with my email and password"));
        assert!(md.contains("So that I can access my account"));
        assert!(md.contains("## Complexity: medium"));
        assert!(md.contains("## Story Points: 3"));
    }

    #[test]
    fn test_traceability_matrix() {
        let mut matrix = TraceabilityMatrix::new();

        matrix.add_link(TraceabilityLink {
            source_type: ArtifactType::Requirement,
            source_id: "REQ-001".to_string(),
            target_type: ArtifactType::Story,
            target_id: "STORY-001".to_string(),
            link_type: LinkType::ImplementedBy,
            created_at: Utc::now(),
        });

        matrix.add_link(TraceabilityLink {
            source_type: ArtifactType::Requirement,
            source_id: "REQ-001".to_string(),
            target_type: ArtifactType::Test,
            target_id: "TEST-001".to_string(),
            link_type: LinkType::TestedBy,
            created_at: Utc::now(),
        });

        matrix.calculate_coverage();

        assert_eq!(matrix.requirements.len(), 1);
        assert_eq!(matrix.stories.len(), 1);
        assert_eq!(matrix.links.len(), 2);

        let cov = matrix.coverage.get("REQ-001").unwrap();
        assert_eq!(cov.stories_count, 1);
        assert_eq!(cov.tests_count, 1);
        assert!(cov.is_fully_covered);
    }

    #[test]
    fn test_story_complexity_points() {
        assert_eq!(StoryComplexity::Simple.story_points(), 1);
        assert_eq!(StoryComplexity::Medium.story_points(), 3);
        assert_eq!(StoryComplexity::Complex.story_points(), 5);
        assert_eq!(StoryComplexity::Epic.story_points(), 13);
    }
}
