//! Documentation Generation Module
//!
//! Types and utilities for automated documentation generation,
//! changelog automation, and Architecture Decision Records (ADRs).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Documentation generation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    /// OpenAPI/Swagger API documentation
    Api,
    /// README file
    Readme,
    /// CHANGELOG file
    Changelog,
    /// Architecture Decision Record
    Adr,
    /// General documentation
    General,
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Readme => "readme",
            Self::Changelog => "changelog",
            Self::Adr => "adr",
            Self::General => "general",
        }
    }
}

impl std::fmt::Display for DocType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// API endpoint information for documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub request_body: Option<SchemaInfo>,
    pub response: Option<SchemaInfo>,
    pub parameters: Vec<ApiParameter>,
    pub tags: Vec<String>,
}

/// API parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiParameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub description: Option<String>,
    pub schema_type: String,
}

/// Parameter location (path, query, header, cookie)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

impl ParameterLocation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Query => "query",
            Self::Header => "header",
            Self::Cookie => "cookie",
        }
    }
}

/// Schema information for request/response bodies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub schema_type: String,
    pub properties: HashMap<String, PropertyInfo>,
    pub required: Vec<String>,
    pub example: Option<serde_json::Value>,
}

/// Property information for schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyInfo {
    pub property_type: String,
    pub description: Option<String>,
    pub format: Option<String>,
    pub nullable: bool,
}

/// Generated API documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDocumentation {
    pub openapi_version: String,
    pub info: ApiInfo,
    pub servers: Vec<ApiServer>,
    pub endpoints: Vec<ApiEndpoint>,
    pub schemas: HashMap<String, SchemaInfo>,
    pub generated_at: DateTime<Utc>,
}

/// API info section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub contact: Option<ApiContact>,
    pub license: Option<ApiLicense>,
}

/// API contact info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiContact {
    pub name: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
}

/// API license info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiLicense {
    pub name: String,
    pub url: Option<String>,
}

/// API server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiServer {
    pub url: String,
    pub description: Option<String>,
}

impl Default for ApiDocumentation {
    fn default() -> Self {
        Self {
            openapi_version: "3.0.0".to_string(),
            info: ApiInfo {
                title: "API".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                contact: None,
                license: None,
            },
            servers: vec![],
            endpoints: vec![],
            schemas: HashMap::new(),
            generated_at: Utc::now(),
        }
    }
}

impl ApiDocumentation {
    /// Create a new API documentation with project info
    pub fn new(title: &str, version: &str, description: Option<&str>) -> Self {
        Self {
            info: ApiInfo {
                title: title.to_string(),
                version: version.to_string(),
                description: description.map(|s| s.to_string()),
                contact: None,
                license: None,
            },
            ..Default::default()
        }
    }

    /// Add a server URL
    pub fn add_server(&mut self, url: &str, description: Option<&str>) {
        self.servers.push(ApiServer {
            url: url.to_string(),
            description: description.map(|s| s.to_string()),
        });
    }

    /// Add an endpoint
    pub fn add_endpoint(&mut self, endpoint: ApiEndpoint) {
        self.endpoints.push(endpoint);
    }

    /// Add a schema definition
    pub fn add_schema(&mut self, name: &str, schema: SchemaInfo) {
        self.schemas.insert(name.to_string(), schema);
    }

    /// Generate OpenAPI 3.0 YAML output
    pub fn to_openapi_yaml(&self) -> String {
        let mut output = format!(
            "openapi: '{}'\n\
             info:\n\
             \x20 title: '{}'\n\
             \x20 version: '{}'\n",
            self.openapi_version, self.info.title, self.info.version
        );

        if let Some(ref desc) = self.info.description {
            output.push_str(&format!("  description: |\n    {}\n", desc.replace('\n', "\n    ")));
        }

        if let Some(ref contact) = self.info.contact {
            output.push_str("  contact:\n");
            if let Some(ref name) = contact.name {
                output.push_str(&format!("    name: '{}'\n", name));
            }
            if let Some(ref email) = contact.email {
                output.push_str(&format!("    email: '{}'\n", email));
            }
            if let Some(ref url) = contact.url {
                output.push_str(&format!("    url: '{}'\n", url));
            }
        }

        if let Some(ref license) = self.info.license {
            output.push_str(&format!("  license:\n    name: '{}'\n", license.name));
            if let Some(ref url) = license.url {
                output.push_str(&format!("    url: '{}'\n", url));
            }
        }

        // Servers
        if !self.servers.is_empty() {
            output.push_str("servers:\n");
            for server in &self.servers {
                output.push_str(&format!("  - url: '{}'\n", server.url));
                if let Some(ref desc) = server.description {
                    output.push_str(&format!("    description: '{}'\n", desc));
                }
            }
        }

        // Paths (endpoints grouped by path)
        if !self.endpoints.is_empty() {
            output.push_str("paths:\n");
            let mut paths_by_path: HashMap<&str, Vec<&ApiEndpoint>> = HashMap::new();
            for endpoint in &self.endpoints {
                paths_by_path
                    .entry(&endpoint.path)
                    .or_default()
                    .push(endpoint);
            }

            let mut sorted_paths: Vec<_> = paths_by_path.keys().collect();
            sorted_paths.sort();

            for path in sorted_paths {
                output.push_str(&format!("  '{}':\n", path));
                for endpoint in &paths_by_path[path] {
                    output.push_str(&format!("    {}:\n", endpoint.method.to_lowercase()));
                    if let Some(ref summary) = endpoint.summary {
                        output.push_str(&format!("      summary: '{}'\n", summary));
                    }
                    if let Some(ref desc) = endpoint.description {
                        output.push_str(&format!("      description: |\n        {}\n", desc.replace('\n', "\n        ")));
                    }
                    if !endpoint.tags.is_empty() {
                        output.push_str("      tags:\n");
                        for tag in &endpoint.tags {
                            output.push_str(&format!("        - '{}'\n", tag));
                        }
                    }
                    if !endpoint.parameters.is_empty() {
                        output.push_str("      parameters:\n");
                        for param in &endpoint.parameters {
                            output.push_str(&format!(
                                "        - name: '{}'\n\
                                 \x20         in: '{}'\n\
                                 \x20         required: {}\n\
                                 \x20         schema:\n\
                                 \x20           type: '{}'\n",
                                param.name,
                                param.location.as_str(),
                                param.required,
                                param.schema_type
                            ));
                            if let Some(ref desc) = param.description {
                                output.push_str(&format!("          description: '{}'\n", desc));
                            }
                        }
                    }
                    if let Some(ref req_body) = endpoint.request_body {
                        output.push_str("      requestBody:\n        required: true\n        content:\n          application/json:\n            schema:\n");
                        output.push_str(&format!("              type: '{}'\n", req_body.schema_type));
                        if !req_body.properties.is_empty() {
                            output.push_str("              properties:\n");
                            for (name, prop) in &req_body.properties {
                                output.push_str(&format!("                '{}':\n                  type: '{}'\n", name, prop.property_type));
                                if let Some(ref desc) = prop.description {
                                    output.push_str(&format!("                  description: '{}'\n", desc));
                                }
                            }
                        }
                    }
                    output.push_str("      responses:\n        '200':\n          description: Successful response\n");
                    if let Some(ref resp) = endpoint.response {
                        output.push_str("          content:\n            application/json:\n              schema:\n");
                        output.push_str(&format!("                type: '{}'\n", resp.schema_type));
                    }
                }
            }
        }

        // Component schemas
        if !self.schemas.is_empty() {
            output.push_str("components:\n  schemas:\n");
            let mut sorted_schemas: Vec<_> = self.schemas.keys().collect();
            sorted_schemas.sort();

            for name in sorted_schemas {
                let schema = &self.schemas[name];
                output.push_str(&format!("    '{}':\n      type: '{}'\n", name, schema.schema_type));
                if !schema.properties.is_empty() {
                    output.push_str("      properties:\n");
                    for (prop_name, prop) in &schema.properties {
                        output.push_str(&format!("        '{}':\n          type: '{}'\n", prop_name, prop.property_type));
                        if let Some(ref desc) = prop.description {
                            output.push_str(&format!("          description: '{}'\n", desc));
                        }
                        if let Some(ref format) = prop.format {
                            output.push_str(&format!("          format: '{}'\n", format));
                        }
                        if prop.nullable {
                            output.push_str("          nullable: true\n");
                        }
                    }
                }
                if !schema.required.is_empty() {
                    output.push_str("      required:\n");
                    for req in &schema.required {
                        output.push_str(&format!("        - '{}'\n", req));
                    }
                }
            }
        }

        output
    }

    /// Generate OpenAPI 3.0 JSON output
    pub fn to_openapi_json(&self) -> serde_json::Value {
        let mut root = serde_json::json!({
            "openapi": self.openapi_version,
            "info": {
                "title": self.info.title,
                "version": self.info.version,
            }
        });

        if let Some(ref desc) = self.info.description {
            root["info"]["description"] = serde_json::Value::String(desc.clone());
        }

        if !self.servers.is_empty() {
            root["servers"] = serde_json::json!(
                self.servers.iter().map(|s| {
                    let mut server = serde_json::json!({"url": s.url});
                    if let Some(ref desc) = s.description {
                        server["description"] = serde_json::Value::String(desc.clone());
                    }
                    server
                }).collect::<Vec<_>>()
            );
        }

        // Build paths
        let mut paths = serde_json::Map::new();
        for endpoint in &self.endpoints {
            let path_entry = paths
                .entry(&endpoint.path)
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

            if let serde_json::Value::Object(path_obj) = path_entry {
                let mut operation = serde_json::Map::new();

                if let Some(ref summary) = endpoint.summary {
                    operation.insert("summary".to_string(), serde_json::Value::String(summary.clone()));
                }
                if let Some(ref desc) = endpoint.description {
                    operation.insert("description".to_string(), serde_json::Value::String(desc.clone()));
                }
                if !endpoint.tags.is_empty() {
                    operation.insert("tags".to_string(), serde_json::json!(endpoint.tags));
                }
                if !endpoint.parameters.is_empty() {
                    let params: Vec<serde_json::Value> = endpoint.parameters.iter().map(|p| {
                        let mut param = serde_json::json!({
                            "name": p.name,
                            "in": p.location.as_str(),
                            "required": p.required,
                            "schema": {"type": p.schema_type}
                        });
                        if let Some(ref desc) = p.description {
                            param["description"] = serde_json::Value::String(desc.clone());
                        }
                        param
                    }).collect();
                    operation.insert("parameters".to_string(), serde_json::Value::Array(params));
                }

                operation.insert("responses".to_string(), serde_json::json!({
                    "200": {"description": "Successful response"}
                }));

                path_obj.insert(endpoint.method.to_lowercase(), serde_json::Value::Object(operation));
            }
        }

        if !paths.is_empty() {
            root["paths"] = serde_json::Value::Object(paths);
        }

        root
    }
}

impl ApiEndpoint {
    /// Create a new API endpoint
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_uppercase(),
            path: path.to_string(),
            summary: None,
            description: None,
            request_body: None,
            response: None,
            parameters: vec![],
            tags: vec![],
        }
    }

    /// Set the summary
    pub fn with_summary(mut self, summary: &str) -> Self {
        self.summary = Some(summary.to_string());
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Add a path parameter
    pub fn with_path_param(mut self, name: &str, description: Option<&str>) -> Self {
        self.parameters.push(ApiParameter {
            name: name.to_string(),
            location: ParameterLocation::Path,
            required: true,
            description: description.map(|s| s.to_string()),
            schema_type: "string".to_string(),
        });
        self
    }

    /// Add a query parameter
    pub fn with_query_param(mut self, name: &str, required: bool, description: Option<&str>) -> Self {
        self.parameters.push(ApiParameter {
            name: name.to_string(),
            location: ParameterLocation::Query,
            required,
            description: description.map(|s| s.to_string()),
            schema_type: "string".to_string(),
        });
        self
    }
}

// ==================== Changelog Types ====================

/// Changelog entry type based on conventional commits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Added,
    Changed,
    Deprecated,
    Removed,
    Fixed,
    Security,
}

impl ChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Changed => "Changed",
            Self::Deprecated => "Deprecated",
            Self::Removed => "Removed",
            Self::Fixed => "Fixed",
            Self::Security => "Security",
        }
    }

    /// Parse from conventional commit type
    pub fn from_commit_type(commit_type: &str) -> Option<Self> {
        match commit_type.to_lowercase().as_str() {
            "feat" | "feature" => Some(Self::Added),
            "fix" | "bugfix" => Some(Self::Fixed),
            "refactor" | "perf" => Some(Self::Changed),
            "docs" | "style" | "chore" => Some(Self::Changed),
            "deprecated" => Some(Self::Deprecated),
            "remove" | "removed" => Some(Self::Removed),
            "security" | "sec" => Some(Self::Security),
            _ => None,
        }
    }
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single changelog entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub change_type: ChangeType,
    pub description: String,
    pub commit_hash: Option<String>,
    pub pr_number: Option<i64>,
    pub issue_number: Option<i64>,
    pub author: Option<String>,
    pub scope: Option<String>,
    pub breaking: bool,
}

/// A changelog release/version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogRelease {
    pub version: String,
    pub date: DateTime<Utc>,
    pub entries: Vec<ChangelogEntry>,
    pub yanked: bool,
}

impl ChangelogRelease {
    /// Group entries by change type
    pub fn entries_by_type(&self) -> HashMap<ChangeType, Vec<&ChangelogEntry>> {
        let mut grouped: HashMap<ChangeType, Vec<&ChangelogEntry>> = HashMap::new();
        for entry in &self.entries {
            grouped.entry(entry.change_type).or_default().push(entry);
        }
        grouped
    }

    /// Format as Keep a Changelog markdown
    pub fn to_markdown(&self) -> String {
        let mut output = format!(
            "## [{}] - {}\n\n",
            self.version,
            self.date.format("%Y-%m-%d")
        );

        if self.yanked {
            output.push_str("[YANKED]\n\n");
        }

        let order = [
            ChangeType::Added,
            ChangeType::Changed,
            ChangeType::Deprecated,
            ChangeType::Removed,
            ChangeType::Fixed,
            ChangeType::Security,
        ];

        let by_type = self.entries_by_type();
        for change_type in order {
            if let Some(entries) = by_type.get(&change_type) {
                output.push_str(&format!("### {}\n\n", change_type.as_str()));
                for entry in entries {
                    let mut line = format!("- {}", entry.description);
                    if let Some(pr) = entry.pr_number {
                        line.push_str(&format!(" (#{pr})"));
                    }
                    if entry.breaking {
                        line.push_str(" **BREAKING**");
                    }
                    output.push_str(&format!("{line}\n"));
                }
                output.push('\n');
            }
        }

        output
    }
}

/// Full changelog document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changelog {
    pub title: String,
    pub description: Option<String>,
    pub releases: Vec<ChangelogRelease>,
    pub unreleased: Option<ChangelogRelease>,
}

impl Default for Changelog {
    fn default() -> Self {
        Self {
            title: "Changelog".to_string(),
            description: Some(
                "All notable changes to this project will be documented in this file."
                    .to_string(),
            ),
            releases: vec![],
            unreleased: None,
        }
    }
}

impl Changelog {
    /// Format as Keep a Changelog markdown
    pub fn to_markdown(&self) -> String {
        let mut output = format!("# {}\n\n", self.title);

        if let Some(desc) = &self.description {
            output.push_str(&format!("{desc}\n\n"));
        }

        output.push_str(
            "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n",
        );
        output.push_str(
            "and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n",
        );

        if let Some(unreleased) = &self.unreleased {
            output.push_str("## [Unreleased]\n\n");
            for entry in &unreleased.entries {
                output.push_str(&format!("- {}\n", entry.description));
            }
            output.push('\n');
        }

        for release in &self.releases {
            output.push_str(&release.to_markdown());
        }

        output
    }
}

// ==================== ADR Types ====================

/// ADR status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
    Rejected,
}

impl AdrStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::Accepted => "Accepted",
            Self::Deprecated => "Deprecated",
            Self::Superseded => "Superseded",
            Self::Rejected => "Rejected",
        }
    }
}

impl std::fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for AdrStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "deprecated" => Ok(Self::Deprecated),
            "superseded" => Ok(Self::Superseded),
            "rejected" => Ok(Self::Rejected),
            _ => Err(format!("Unknown ADR status: {}", s)),
        }
    }
}

/// Architecture Decision Record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adr {
    pub number: i32,
    pub title: String,
    pub status: AdrStatus,
    pub date: DateTime<Utc>,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<AdrConsequence>,
    pub related_adrs: Vec<i32>,
    pub superseded_by: Option<i32>,
    pub tags: Vec<String>,
}

/// ADR consequence (positive or negative)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdrConsequence {
    pub positive: bool,
    pub description: String,
}

impl Adr {
    /// Create a new ADR with the given number and title
    pub fn new(number: i32, title: String) -> Self {
        Self {
            number,
            title,
            status: AdrStatus::Proposed,
            date: Utc::now(),
            context: String::new(),
            decision: String::new(),
            consequences: vec![],
            related_adrs: vec![],
            superseded_by: None,
            tags: vec![],
        }
    }

    /// Format ADR number with leading zeros
    pub fn formatted_number(&self) -> String {
        format!("{:04}", self.number)
    }

    /// Format as markdown
    pub fn to_markdown(&self) -> String {
        let mut output = format!(
            "# ADR-{}: {}\n\n",
            self.formatted_number(),
            self.title
        );

        output.push_str(&format!("## Status\n\n{}\n\n", self.status.as_str()));

        if let Some(superseded_by) = self.superseded_by {
            output.push_str(&format!(
                "Superseded by [ADR-{:04}](./adr-{:04}.md)\n\n",
                superseded_by, superseded_by
            ));
        }

        output.push_str(&format!(
            "## Date\n\n{}\n\n",
            self.date.format("%Y-%m-%d")
        ));

        output.push_str(&format!("## Context\n\n{}\n\n", self.context));

        output.push_str(&format!("## Decision\n\n{}\n\n", self.decision));

        output.push_str("## Consequences\n\n");

        let positives: Vec<_> = self
            .consequences
            .iter()
            .filter(|c| c.positive)
            .collect();
        let negatives: Vec<_> = self
            .consequences
            .iter()
            .filter(|c| !c.positive)
            .collect();

        if !positives.is_empty() {
            output.push_str("### Positive\n\n");
            for c in positives {
                output.push_str(&format!("- {}\n", c.description));
            }
            output.push('\n');
        }

        if !negatives.is_empty() {
            output.push_str("### Negative\n\n");
            for c in negatives {
                output.push_str(&format!("- {}\n", c.description));
            }
            output.push('\n');
        }

        if !self.related_adrs.is_empty() {
            output.push_str("## Related ADRs\n\n");
            for related in &self.related_adrs {
                output.push_str(&format!(
                    "- [ADR-{:04}](./adr-{:04}.md)\n",
                    related, related
                ));
            }
            output.push('\n');
        }

        output
    }
}

// ==================== Documentation Validation ====================

/// Documentation validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocValidationResult {
    pub total_items: usize,
    pub documented_items: usize,
    pub coverage_percentage: f64,
    pub issues: Vec<DocValidationIssue>,
}

impl DocValidationResult {
    pub fn new() -> Self {
        Self {
            total_items: 0,
            documented_items: 0,
            coverage_percentage: 0.0,
            issues: vec![],
        }
    }

    /// Calculate coverage percentage from total and documented items
    pub fn calculate_coverage(&mut self) {
        if self.total_items > 0 {
            self.coverage_percentage =
                (self.documented_items as f64 / self.total_items as f64) * 100.0;
        }
    }

    /// Add an issue to the validation result
    pub fn add_issue(&mut self, issue: DocValidationIssue) {
        self.issues.push(issue);
    }

    /// Check if validation passed (no issues)
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    /// Get issues grouped by type
    pub fn issues_by_type(&self) -> HashMap<DocIssueType, Vec<&DocValidationIssue>> {
        let mut grouped: HashMap<DocIssueType, Vec<&DocValidationIssue>> = HashMap::new();
        for issue in &self.issues {
            grouped.entry(issue.issue_type).or_default().push(issue);
        }
        grouped
    }

    /// Get count of each issue type
    pub fn issue_counts(&self) -> HashMap<DocIssueType, usize> {
        let mut counts = HashMap::new();
        for issue in &self.issues {
            *counts.entry(issue.issue_type).or_insert(0) += 1;
        }
        counts
    }

    /// Generate a summary report
    pub fn to_summary(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "Documentation Coverage: {:.1}% ({}/{})\n",
            self.coverage_percentage, self.documented_items, self.total_items
        ));

        if !self.issues.is_empty() {
            output.push_str(&format!("\nIssues found: {}\n", self.issues.len()));
            let counts = self.issue_counts();
            for (issue_type, count) in counts {
                output.push_str(&format!("  - {}: {}\n", issue_type.as_str(), count));
            }
        } else {
            output.push_str("\nNo issues found.\n");
        }

        output
    }
}

impl Default for DocValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A documentation validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocValidationIssue {
    pub file_path: String,
    pub line_number: Option<usize>,
    pub item_name: String,
    pub item_type: DocItemType,
    pub issue_type: DocIssueType,
    pub message: String,
}

/// Type of documentable item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocItemType {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Constant,
    Type,
}

impl DocItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Module => "module",
            Self::Constant => "constant",
            Self::Type => "type",
        }
    }
}

/// Type of documentation issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocIssueType {
    MissingDoc,
    IncompleteDoc,
    OutdatedDoc,
    InvalidFormat,
}

impl DocIssueType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingDoc => "missing_doc",
            Self::IncompleteDoc => "incomplete_doc",
            Self::OutdatedDoc => "outdated_doc",
            Self::InvalidFormat => "invalid_format",
        }
    }
}

// ==================== README Generation ====================

/// README section type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadmeSection {
    Title,
    Description,
    Badges,
    Installation,
    Usage,
    Configuration,
    Api,
    Examples,
    Contributing,
    License,
    Acknowledgments,
    Custom,
}

impl ReadmeSection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Title => "Title",
            Self::Description => "Description",
            Self::Badges => "Badges",
            Self::Installation => "Installation",
            Self::Usage => "Usage",
            Self::Configuration => "Configuration",
            Self::Api => "API",
            Self::Examples => "Examples",
            Self::Contributing => "Contributing",
            Self::License => "License",
            Self::Acknowledgments => "Acknowledgments",
            Self::Custom => "Custom",
        }
    }

    pub fn default_heading(&self) -> &'static str {
        match self {
            Self::Title => "",
            Self::Description => "",
            Self::Badges => "",
            Self::Installation => "## Installation",
            Self::Usage => "## Usage",
            Self::Configuration => "## Configuration",
            Self::Api => "## API",
            Self::Examples => "## Examples",
            Self::Contributing => "## Contributing",
            Self::License => "## License",
            Self::Acknowledgments => "## Acknowledgments",
            Self::Custom => "",
        }
    }
}

/// Generated README content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadmeContent {
    pub sections: Vec<ReadmeSectionContent>,
}

/// A single README section with content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadmeSectionContent {
    pub section_type: ReadmeSection,
    pub heading: Option<String>,
    pub content: String,
}

impl Default for ReadmeContent {
    fn default() -> Self {
        Self { sections: vec![] }
    }
}

impl ReadmeContent {
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();
        for section in &self.sections {
            if let Some(heading) = &section.heading {
                if !heading.is_empty() {
                    output.push_str(heading);
                    output.push_str("\n\n");
                }
            }
            output.push_str(&section.content);
            output.push_str("\n\n");
        }
        output.trim_end().to_string() + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_change_type_from_commit_type() {
        assert_eq!(
            ChangeType::from_commit_type("feat"),
            Some(ChangeType::Added)
        );
        assert_eq!(ChangeType::from_commit_type("fix"), Some(ChangeType::Fixed));
        assert_eq!(
            ChangeType::from_commit_type("refactor"),
            Some(ChangeType::Changed)
        );
        assert_eq!(ChangeType::from_commit_type("unknown"), None);
    }

    #[test]
    fn test_changelog_release_to_markdown() {
        let release = ChangelogRelease {
            version: "1.0.0".to_string(),
            date: chrono::DateTime::parse_from_rfc3339("2024-01-15T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            entries: vec![
                ChangelogEntry {
                    change_type: ChangeType::Added,
                    description: "New feature".to_string(),
                    commit_hash: None,
                    pr_number: Some(123),
                    issue_number: None,
                    author: None,
                    scope: None,
                    breaking: false,
                },
                ChangelogEntry {
                    change_type: ChangeType::Fixed,
                    description: "Bug fix".to_string(),
                    commit_hash: None,
                    pr_number: None,
                    issue_number: None,
                    author: None,
                    scope: None,
                    breaking: false,
                },
            ],
            yanked: false,
        };

        let md = release.to_markdown();
        assert!(md.contains("## [1.0.0] - 2024-01-15"));
        assert!(md.contains("### Added"));
        assert!(md.contains("- New feature (#123)"));
        assert!(md.contains("### Fixed"));
        assert!(md.contains("- Bug fix"));
    }

    #[test]
    fn test_adr_to_markdown() {
        let adr = Adr {
            number: 1,
            title: "Use SQLite for Storage".to_string(),
            status: AdrStatus::Accepted,
            date: chrono::DateTime::parse_from_rfc3339("2024-01-15T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            context: "We need a database for agent state.".to_string(),
            decision: "We will use SQLite.".to_string(),
            consequences: vec![
                AdrConsequence {
                    positive: true,
                    description: "Simple deployment".to_string(),
                },
                AdrConsequence {
                    positive: false,
                    description: "Single-node limitation".to_string(),
                },
            ],
            related_adrs: vec![],
            superseded_by: None,
            tags: vec!["database".to_string()],
        };

        let md = adr.to_markdown();
        assert!(md.contains("# ADR-0001: Use SQLite for Storage"));
        assert!(md.contains("## Status\n\nAccepted"));
        assert!(md.contains("### Positive"));
        assert!(md.contains("- Simple deployment"));
        assert!(md.contains("### Negative"));
        assert!(md.contains("- Single-node limitation"));
    }

    #[test]
    fn test_adr_status_from_str() {
        assert_eq!(
            AdrStatus::from_str("accepted").unwrap(),
            AdrStatus::Accepted
        );
        assert_eq!(
            AdrStatus::from_str("Proposed").unwrap(),
            AdrStatus::Proposed
        );
        assert!(AdrStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_api_documentation_yaml_generation() {
        let mut api_doc = ApiDocumentation::new("Orchestrate API", "1.0.0", Some("Agent orchestration system API"));
        api_doc.add_server("http://localhost:3000", Some("Development server"));

        let endpoint = ApiEndpoint::new("GET", "/api/agents")
            .with_summary("List all agents")
            .with_description("Returns a list of all agents in the system")
            .with_tag("agents")
            .with_query_param("status", false, Some("Filter by agent status"));
        api_doc.add_endpoint(endpoint);

        let endpoint2 = ApiEndpoint::new("GET", "/api/agents/{id}")
            .with_summary("Get agent by ID")
            .with_tag("agents")
            .with_path_param("id", Some("Agent ID"));
        api_doc.add_endpoint(endpoint2);

        let yaml = api_doc.to_openapi_yaml();

        assert!(yaml.contains("openapi: '3.0.0'"));
        assert!(yaml.contains("title: 'Orchestrate API'"));
        assert!(yaml.contains("version: '1.0.0'"));
        assert!(yaml.contains("'/api/agents':"));
        assert!(yaml.contains("summary: 'List all agents'"));
        assert!(yaml.contains("- 'agents'"));
        assert!(yaml.contains("'/api/agents/{id}':"));
    }

    #[test]
    fn test_api_documentation_json_generation() {
        let mut api_doc = ApiDocumentation::new("Test API", "1.0.0", None);
        api_doc.add_server("http://localhost:8080", None);

        let endpoint = ApiEndpoint::new("POST", "/api/items")
            .with_summary("Create item")
            .with_tag("items");
        api_doc.add_endpoint(endpoint);

        let json = api_doc.to_openapi_json();

        assert_eq!(json["openapi"], "3.0.0");
        assert_eq!(json["info"]["title"], "Test API");
        assert!(json["paths"]["/api/items"]["post"].is_object());
    }

    #[test]
    fn test_api_endpoint_builder() {
        let endpoint = ApiEndpoint::new("DELETE", "/api/items/{id}")
            .with_summary("Delete item")
            .with_description("Permanently removes an item")
            .with_tag("items")
            .with_tag("destructive")
            .with_path_param("id", Some("Item identifier"));

        assert_eq!(endpoint.method, "DELETE");
        assert_eq!(endpoint.path, "/api/items/{id}");
        assert_eq!(endpoint.summary, Some("Delete item".to_string()));
        assert_eq!(endpoint.tags.len(), 2);
        assert_eq!(endpoint.parameters.len(), 1);
        assert_eq!(endpoint.parameters[0].name, "id");
        assert_eq!(endpoint.parameters[0].location, ParameterLocation::Path);
    }
}
