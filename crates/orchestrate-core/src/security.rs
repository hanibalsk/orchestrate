//! Security Scanner Module
//!
//! This module provides security scanning and remediation capabilities including:
//! - Dependency vulnerability scanning
//! - Static Application Security Testing (SAST)
//! - Secret detection
//! - License compliance
//! - Container image scanning
//! - Security fix automation
//! - SARIF report generation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Vulnerability severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Unknown,
    Low,
    Medium,
    High,
    Critical,
}

impl FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unknown" => Ok(Self::Unknown),
            "low" => Ok(Self::Low),
            "medium" | "moderate" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("Unknown severity: {}", s)),
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "UNKNOWN"),
            Self::Low => write!(f, "LOW"),
            Self::Medium => write!(f, "MEDIUM"),
            Self::High => write!(f, "HIGH"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Type of security scan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScanType {
    Dependencies,
    Code,
    Secrets,
    Licenses,
    Container,
    Full,
}

impl FromStr for ScanType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dependencies" | "deps" => Ok(Self::Dependencies),
            "code" | "sast" => Ok(Self::Code),
            "secrets" => Ok(Self::Secrets),
            "licenses" | "license" => Ok(Self::Licenses),
            "container" | "docker" => Ok(Self::Container),
            "full" | "all" => Ok(Self::Full),
            _ => Err(format!("Unknown scan type: {}", s)),
        }
    }
}

/// Scan status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A security scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScan {
    pub id: String,
    pub scan_types: Vec<ScanType>,
    pub status: ScanStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<f64>,
    pub vulnerabilities: Vec<Vulnerability>,
    pub secrets: Vec<DetectedSecret>,
    pub license_issues: Vec<LicenseIssue>,
    pub summary: ScanSummary,
    pub triggered_by: String,
    pub commit_sha: Option<String>,
    pub branch: Option<String>,
}

impl SecurityScan {
    pub fn new(scan_types: Vec<ScanType>, triggered_by: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            scan_types,
            status: ScanStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            duration_seconds: None,
            vulnerabilities: Vec::new(),
            secrets: Vec::new(),
            license_issues: Vec::new(),
            summary: ScanSummary::default(),
            triggered_by: triggered_by.into(),
            commit_sha: None,
            branch: None,
        }
    }

    pub fn start(&mut self) {
        self.status = ScanStatus::Running;
        self.started_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = ScanStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.duration_seconds = Some(
            (self.completed_at.unwrap() - self.started_at).num_milliseconds() as f64 / 1000.0,
        );
        self.update_summary();
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = ScanStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.summary.error = Some(error.into());
    }

    pub fn add_vulnerability(&mut self, vuln: Vulnerability) {
        self.vulnerabilities.push(vuln);
    }

    pub fn add_secret(&mut self, secret: DetectedSecret) {
        self.secrets.push(secret);
    }

    pub fn add_license_issue(&mut self, issue: LicenseIssue) {
        self.license_issues.push(issue);
    }

    fn update_summary(&mut self) {
        self.summary.total_vulnerabilities = self.vulnerabilities.len();
        self.summary.critical_count = self.vulnerabilities.iter().filter(|v| v.severity == Severity::Critical).count();
        self.summary.high_count = self.vulnerabilities.iter().filter(|v| v.severity == Severity::High).count();
        self.summary.medium_count = self.vulnerabilities.iter().filter(|v| v.severity == Severity::Medium).count();
        self.summary.low_count = self.vulnerabilities.iter().filter(|v| v.severity == Severity::Low).count();
        self.summary.secrets_count = self.secrets.len();
        self.summary.license_issues_count = self.license_issues.len();
        self.summary.auto_fixable_count = self.vulnerabilities.iter().filter(|v| v.auto_fixable).count();
    }

    pub fn has_blocking_issues(&self, policy: &SecurityPolicy) -> bool {
        for vuln in &self.vulnerabilities {
            if policy.should_block(&vuln.severity) {
                return true;
            }
        }
        !self.secrets.is_empty() && policy.block_on_secrets
    }
}

/// Scan summary statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_vulnerabilities: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub secrets_count: usize,
    pub license_issues_count: usize,
    pub auto_fixable_count: usize,
    pub error: Option<String>,
}

/// A detected vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub cve_id: Option<String>,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub vulnerability_type: VulnerabilityType,
    pub package_name: Option<String>,
    pub installed_version: Option<String>,
    pub fixed_version: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub auto_fixable: bool,
    pub fix_command: Option<String>,
    pub references: Vec<String>,
    pub cvss_score: Option<f64>,
    pub discovered_at: DateTime<Utc>,
}

impl Vulnerability {
    pub fn dependency(
        package_name: impl Into<String>,
        installed_version: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            cve_id: None,
            title: String::new(),
            description: String::new(),
            severity,
            vulnerability_type: VulnerabilityType::DependencyVulnerability,
            package_name: Some(package_name.into()),
            installed_version: Some(installed_version.into()),
            fixed_version: None,
            file_path: None,
            line_number: None,
            auto_fixable: false,
            fix_command: None,
            references: Vec::new(),
            cvss_score: None,
            discovered_at: Utc::now(),
        }
    }

    pub fn code(
        title: impl Into<String>,
        file_path: impl Into<String>,
        line_number: u32,
        severity: Severity,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            cve_id: None,
            title: title.into(),
            description: String::new(),
            severity,
            vulnerability_type: VulnerabilityType::CodeVulnerability,
            package_name: None,
            installed_version: None,
            fixed_version: None,
            file_path: Some(file_path.into()),
            line_number: Some(line_number),
            auto_fixable: false,
            fix_command: None,
            references: Vec::new(),
            cvss_score: None,
            discovered_at: Utc::now(),
        }
    }

    pub fn with_cve(mut self, cve: impl Into<String>) -> Self {
        self.cve_id = Some(cve.into());
        self
    }

    pub fn with_fix(mut self, fixed_version: impl Into<String>) -> Self {
        self.fixed_version = Some(fixed_version.into());
        self.auto_fixable = true;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

/// Type of vulnerability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VulnerabilityType {
    DependencyVulnerability,
    CodeVulnerability,
    ConfigurationVulnerability,
    ContainerVulnerability,
    SqlInjection,
    Xss,
    CommandInjection,
    PathTraversal,
    InsecureDeserialization,
    WeakCryptography,
    HardcodedSecret,
    Other(String),
}

impl std::fmt::Display for VulnerabilityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependencyVulnerability => write!(f, "Dependency Vulnerability"),
            Self::CodeVulnerability => write!(f, "Code Vulnerability"),
            Self::ConfigurationVulnerability => write!(f, "Configuration Issue"),
            Self::ContainerVulnerability => write!(f, "Container Vulnerability"),
            Self::SqlInjection => write!(f, "SQL Injection"),
            Self::Xss => write!(f, "Cross-Site Scripting (XSS)"),
            Self::CommandInjection => write!(f, "Command Injection"),
            Self::PathTraversal => write!(f, "Path Traversal"),
            Self::InsecureDeserialization => write!(f, "Insecure Deserialization"),
            Self::WeakCryptography => write!(f, "Weak Cryptography"),
            Self::HardcodedSecret => write!(f, "Hardcoded Secret"),
            Self::Other(s) => write!(f, "{}", s),
        }
    }
}

/// A detected secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedSecret {
    pub id: String,
    pub secret_type: SecretType,
    pub file_path: String,
    pub line_number: u32,
    pub match_text: String,
    pub is_in_history: bool,
    pub commit_sha: Option<String>,
    pub author: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub verified: Option<bool>,
    pub rotation_needed: bool,
}

impl DetectedSecret {
    pub fn new(
        secret_type: SecretType,
        file_path: impl Into<String>,
        line_number: u32,
        match_text: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            secret_type,
            file_path: file_path.into(),
            line_number,
            match_text: match_text.into(),
            is_in_history: false,
            commit_sha: None,
            author: None,
            detected_at: Utc::now(),
            verified: None,
            rotation_needed: true,
        }
    }

    pub fn in_history(mut self, commit_sha: impl Into<String>) -> Self {
        self.is_in_history = true;
        self.commit_sha = Some(commit_sha.into());
        self
    }
}

/// Type of secret detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SecretType {
    AwsAccessKey,
    AwsSecretKey,
    GitHubToken,
    SlackToken,
    DatabaseConnectionString,
    PrivateKey,
    SshKey,
    JwtSecret,
    ApiKey,
    Password,
    GenericSecret,
}

impl std::fmt::Display for SecretType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AwsAccessKey => write!(f, "AWS Access Key"),
            Self::AwsSecretKey => write!(f, "AWS Secret Key"),
            Self::GitHubToken => write!(f, "GitHub Token"),
            Self::SlackToken => write!(f, "Slack Token"),
            Self::DatabaseConnectionString => write!(f, "Database Connection String"),
            Self::PrivateKey => write!(f, "Private Key"),
            Self::SshKey => write!(f, "SSH Key"),
            Self::JwtSecret => write!(f, "JWT Secret"),
            Self::ApiKey => write!(f, "API Key"),
            Self::Password => write!(f, "Password"),
            Self::GenericSecret => write!(f, "Generic Secret"),
        }
    }
}

/// A license compliance issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseIssue {
    pub id: String,
    pub package_name: String,
    pub license: String,
    pub issue_type: LicenseIssueType,
    pub severity: Severity,
    pub description: String,
}

impl LicenseIssue {
    pub fn new(
        package_name: impl Into<String>,
        license: impl Into<String>,
        issue_type: LicenseIssueType,
    ) -> Self {
        let severity = match &issue_type {
            LicenseIssueType::Denied => Severity::High,
            LicenseIssueType::ReviewRequired => Severity::Medium,
            LicenseIssueType::Unknown => Severity::Low,
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            package_name: package_name.into(),
            license: license.into(),
            issue_type,
            severity,
            description: String::new(),
        }
    }
}

/// Type of license issue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LicenseIssueType {
    Denied,
    ReviewRequired,
    Unknown,
}

/// Security policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: String,
    pub name: String,
    pub block_on_critical: bool,
    pub block_on_high: bool,
    pub block_on_high_age_days: Option<u32>,
    pub block_on_secrets: bool,
    pub allowed_licenses: Vec<String>,
    pub denied_licenses: Vec<String>,
    pub review_required_licenses: Vec<String>,
    pub allow_exceptions: bool,
    pub max_exception_days: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SecurityPolicy {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            block_on_critical: true,
            block_on_high: true,
            block_on_high_age_days: Some(7),
            block_on_secrets: true,
            allowed_licenses: vec![
                "MIT".to_string(),
                "Apache-2.0".to_string(),
                "BSD-2-Clause".to_string(),
                "BSD-3-Clause".to_string(),
                "ISC".to_string(),
            ],
            denied_licenses: vec![
                "GPL-2.0".to_string(),
                "GPL-3.0".to_string(),
                "AGPL-3.0".to_string(),
            ],
            review_required_licenses: vec![
                "LGPL-2.1".to_string(),
                "MPL-2.0".to_string(),
            ],
            allow_exceptions: true,
            max_exception_days: Some(30),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn should_block(&self, severity: &Severity) -> bool {
        match severity {
            Severity::Critical => self.block_on_critical,
            Severity::High => self.block_on_high,
            _ => false,
        }
    }

    pub fn check_license(&self, license: &str) -> LicenseCheckResult {
        if self.denied_licenses.iter().any(|l| l == license) {
            LicenseCheckResult::Denied
        } else if self.review_required_licenses.iter().any(|l| l == license) {
            LicenseCheckResult::ReviewRequired
        } else if self.allowed_licenses.iter().any(|l| l == license) {
            LicenseCheckResult::Allowed
        } else {
            LicenseCheckResult::Unknown
        }
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::new("default")
    }
}

/// License check result
#[derive(Debug, Clone, PartialEq)]
pub enum LicenseCheckResult {
    Allowed,
    Denied,
    ReviewRequired,
    Unknown,
}

/// Security exception for approved vulnerabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityException {
    pub id: String,
    pub vulnerability_id: String,
    pub reason: String,
    pub approved_by: String,
    pub approved_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_active: bool,
}

impl SecurityException {
    pub fn new(
        vulnerability_id: impl Into<String>,
        reason: impl Into<String>,
        approved_by: impl Into<String>,
        duration_days: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            vulnerability_id: vulnerability_id.into(),
            reason: reason.into(),
            approved_by: approved_by.into(),
            approved_at: now,
            expires_at: now + chrono::Duration::days(duration_days as i64),
            is_active: true,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Security fix request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFix {
    pub id: String,
    pub vulnerability_ids: Vec<String>,
    pub fix_type: FixType,
    pub status: FixStatus,
    pub pr_number: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_by: String,
    pub changes: Vec<FixChange>,
}

impl SecurityFix {
    pub fn new(vulnerability_ids: Vec<String>, fix_type: FixType, created_by: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            vulnerability_ids,
            fix_type,
            status: FixStatus::Pending,
            pr_number: None,
            created_at: Utc::now(),
            completed_at: None,
            created_by: created_by.into(),
            changes: Vec::new(),
        }
    }

    pub fn complete(&mut self, pr_number: i32) {
        self.status = FixStatus::Completed;
        self.pr_number = Some(pr_number);
        self.completed_at = Some(Utc::now());
    }
}

/// Type of fix
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FixType {
    DependencyUpgrade,
    CodePatch,
    ConfigurationChange,
    SecretRotation,
}

impl FromStr for FixType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dependency_upgrade" | "dependency" => Ok(Self::DependencyUpgrade),
            "code_patch" | "code" => Ok(Self::CodePatch),
            "configuration_change" | "configuration" | "config" => Ok(Self::ConfigurationChange),
            "secret_rotation" | "secret" => Ok(Self::SecretRotation),
            _ => Err(format!("Unknown fix type: {}", s)),
        }
    }
}

impl std::fmt::Display for FixType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependencyUpgrade => write!(f, "dependency_upgrade"),
            Self::CodePatch => write!(f, "code_patch"),
            Self::ConfigurationChange => write!(f, "configuration_change"),
            Self::SecretRotation => write!(f, "secret_rotation"),
        }
    }
}

/// Fix status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FixStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// A change made by a fix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixChange {
    pub file_path: String,
    pub description: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// SARIF report structure (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifReport {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

impl SarifReport {
    pub fn new() -> Self {
        Self {
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json".to_string(),
            version: "2.1.0".to_string(),
            runs: Vec::new(),
        }
    }

    pub fn add_run(&mut self, run: SarifRun) {
        self.runs.push(run);
    }

    pub fn from_scan(scan: &SecurityScan, tool_name: &str) -> Self {
        let mut report = Self::new();
        let mut run = SarifRun::new(tool_name);

        for vuln in &scan.vulnerabilities {
            let result = SarifResult {
                rule_id: vuln.cve_id.clone().unwrap_or_else(|| vuln.id.clone()),
                level: match vuln.severity {
                    Severity::Critical | Severity::High => "error".to_string(),
                    Severity::Medium => "warning".to_string(),
                    _ => "note".to_string(),
                },
                message: SarifMessage { text: vuln.description.clone() },
                locations: vuln.file_path.as_ref().map(|path| {
                    vec![SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation {
                                uri: path.clone(),
                            },
                            region: vuln.line_number.map(|line| SarifRegion {
                                start_line: line,
                            }),
                        },
                    }]
                }).unwrap_or_default(),
            };
            run.results.push(result);
        }

        report.add_run(run);
        report
    }
}

impl Default for SarifReport {
    fn default() -> Self {
        Self::new()
    }
}

/// SARIF run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

impl SarifRun {
    pub fn new(name: &str) -> Self {
        Self {
            tool: SarifTool {
                driver: SarifDriver {
                    name: name.to_string(),
                    version: "1.0.0".to_string(),
                },
            },
            results: Vec::new(),
        }
    }
}

/// SARIF tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

/// SARIF driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
}

/// SARIF result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
}

/// SARIF message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifMessage {
    pub text: String,
}

/// SARIF location
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifLocation {
    pub physical_location: SarifPhysicalLocation,
}

/// SARIF physical location
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifPhysicalLocation {
    pub artifact_location: SarifArtifactLocation,
    pub region: Option<SarifRegion>,
}

/// SARIF artifact location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

/// SARIF region
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifRegion {
    pub start_line: u32,
}

/// Package manager type
#[derive(Debug, Clone, PartialEq)]
pub enum PackageManager {
    Cargo,
    Npm,
    Pip,
}

impl PackageManager {
    /// Detect package manager from project structure
    pub fn detect(project_path: &str) -> Option<Self> {
        let path = std::path::Path::new(project_path);

        if path.join("Cargo.toml").exists() {
            Some(PackageManager::Cargo)
        } else if path.join("package.json").exists() {
            Some(PackageManager::Npm)
        } else if path.join("requirements.txt").exists() || path.join("setup.py").exists() {
            Some(PackageManager::Pip)
        } else {
            None
        }
    }

    /// Get the audit command for this package manager
    pub fn audit_command(&self) -> &str {
        match self {
            PackageManager::Cargo => "cargo audit --json",
            PackageManager::Npm => "npm audit --json",
            PackageManager::Pip => "pip-audit --format json",
        }
    }
}

/// Dependency scanner for detecting vulnerable dependencies
pub struct DependencyScanner;

impl DependencyScanner {
    /// Parse cargo audit JSON output
    pub fn parse_cargo_audit(output: &str) -> Vec<Vulnerability> {
        let mut vulnerabilities = Vec::new();

        if let Ok(data) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(vulns) = data["vulnerabilities"]["list"].as_array() {
                for vuln in vulns {
                    let package = vuln["package"]["name"].as_str().unwrap_or("unknown");
                    let version = vuln["package"]["version"].as_str().unwrap_or("unknown");
                    let cve_id = vuln["advisory"]["id"].as_str().map(String::from);
                    let title = vuln["advisory"]["title"].as_str().unwrap_or("").to_string();
                    let description = vuln["advisory"]["description"].as_str().unwrap_or("").to_string();
                    let cvss = vuln["advisory"]["cvss"].as_f64();

                    // Determine severity from CVSS score
                    let severity = if let Some(score) = cvss {
                        if score >= 9.0 { Severity::Critical }
                        else if score >= 7.0 { Severity::High }
                        else if score >= 4.0 { Severity::Medium }
                        else { Severity::Low }
                    } else {
                        Severity::Unknown
                    };

                    // Get fixed versions
                    let mut fixed_version = None;
                    if let Some(versions) = vuln["versions"]["patched"].as_array() {
                        if let Some(first_fix) = versions.first() {
                            fixed_version = first_fix.as_str().map(String::from);
                        }
                    }

                    let mut vulnerability = Vulnerability::dependency(package, version, severity)
                        .with_description(description);

                    if let Some(cve) = cve_id {
                        vulnerability = vulnerability.with_cve(cve);
                    }

                    if let Some(fix) = fixed_version {
                        vulnerability = vulnerability.with_fix(fix);
                    }

                    vulnerability.title = title;
                    vulnerability.cvss_score = cvss;

                    vulnerabilities.push(vulnerability);
                }
            }
        }

        vulnerabilities
    }

    /// Generate fix recommendation for a dependency vulnerability
    pub fn generate_fix_recommendation(vuln: &Vulnerability, pkg_manager: &PackageManager) -> Option<String> {
        if !vuln.auto_fixable {
            return None;
        }

        let package = vuln.package_name.as_ref()?;
        let fixed_version = vuln.fixed_version.as_ref()?;

        Some(match pkg_manager {
            PackageManager::Cargo => format!("cargo update {}", package),
            PackageManager::Npm => format!("npm update {} || npm install {}@{}", package, package, fixed_version),
            PackageManager::Pip => format!("pip install --upgrade {}=={}", package, fixed_version),
        })
    }
}

/// SAST (Static Application Security Testing) scanner
pub struct SastScanner;

impl SastScanner {
    /// Detect SQL injection vulnerabilities
    pub fn detect_sql_injection(file_path: &str, content: &str) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        let patterns = vec![
            (r#"\.execute\([^)]*\+[^)]*\)"#, "String concatenation in SQL execute"),
            (r#"\.query\([^)]*format[^)]*\)"#, "String formatting in SQL query"),
            (r#"SELECT[^;]*\+[^;]*FROM"#, "SQL injection via string concatenation"),
            (r#"format!\([^)]*SELECT"#, "SQL injection via format macro"),
        ];

        for (line_num, line) in content.lines().enumerate() {
            for (pattern, description) in &patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(line) {
                        let vuln = Vulnerability::code(
                            "Potential SQL Injection",
                            file_path,
                            (line_num + 1) as u32,
                            Severity::High,
                        )
                        .with_description(*description);

                        let mut v = vuln;
                        v.vulnerability_type = VulnerabilityType::SqlInjection;
                        vulns.push(v);
                    }
                }
            }
        }
        vulns
    }

    /// Detect XSS vulnerabilities
    pub fn detect_xss(file_path: &str, content: &str) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        let patterns = vec![
            (r#"innerHTML\s*=.*"#, "Direct innerHTML assignment"),
            (r#"dangerouslySetInnerHTML"#, "React dangerouslySetInnerHTML usage"),
            (r#"document\.write\("#, "document.write with user input"),
        ];

        for (line_num, line) in content.lines().enumerate() {
            for (pattern, description) in &patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(line) {
                        let vuln = Vulnerability::code(
                            "Potential Cross-Site Scripting (XSS)",
                            file_path,
                            (line_num + 1) as u32,
                            Severity::High,
                        )
                        .with_description(*description);

                        let mut v = vuln;
                        v.vulnerability_type = VulnerabilityType::Xss;
                        vulns.push(v);
                    }
                }
            }
        }
        vulns
    }

    /// Detect command injection vulnerabilities
    pub fn detect_command_injection(file_path: &str, content: &str) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        let patterns = vec![
            (r#"Command::new[^;]*\+[^;]*"#, "Command execution with concatenation"),
            (r#"exec\([^)]*format"#, "exec with string formatting"),
            (r#"system\([^)]*\+"#, "system call with concatenation"),
        ];

        for (line_num, line) in content.lines().enumerate() {
            for (pattern, description) in &patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(line) {
                        let vuln = Vulnerability::code(
                            "Potential Command Injection",
                            file_path,
                            (line_num + 1) as u32,
                            Severity::Critical,
                        )
                        .with_description(*description);

                        let mut v = vuln;
                        v.vulnerability_type = VulnerabilityType::CommandInjection;
                        vulns.push(v);
                    }
                }
            }
        }
        vulns
    }

    /// Detect hardcoded credentials
    pub fn detect_hardcoded_credentials(file_path: &str, content: &str) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        let patterns = vec![
            (r#"password\s*=\s*["'][^"']{3,}["']"#, "Hardcoded password"),
            (r#"api[_-]?key\s*=\s*["'][^"']{10,}["']"#, "Hardcoded API key"),
            (r#"secret\s*=\s*["'][^"']{8,}["']"#, "Hardcoded secret"),
        ];

        for (line_num, line) in content.lines().enumerate() {
            for (pattern, description) in &patterns {
                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(line) {
                        let vuln = Vulnerability::code(
                            "Hardcoded Credential Detected",
                            file_path,
                            (line_num + 1) as u32,
                            Severity::Critical,
                        )
                        .with_description(*description);

                        let mut v = vuln;
                        v.vulnerability_type = VulnerabilityType::HardcodedSecret;
                        vulns.push(v);
                    }
                }
            }
        }
        vulns
    }

    /// Run all SAST checks on a file
    pub fn scan_file(file_path: &str, content: &str) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        vulns.extend(Self::detect_sql_injection(file_path, content));
        vulns.extend(Self::detect_xss(file_path, content));
        vulns.extend(Self::detect_command_injection(file_path, content));
        vulns.extend(Self::detect_hardcoded_credentials(file_path, content));
        vulns
    }
}

/// Secret detector for finding exposed credentials
pub struct SecretDetector;

impl SecretDetector {
    /// Calculate Shannon entropy of a string
    pub fn calculate_entropy(s: &str) -> f64 {
        let mut freq: HashMap<char, usize> = HashMap::new();
        for c in s.chars() {
            *freq.entry(c).or_insert(0) += 1;
        }

        let len = s.len() as f64;
        let mut entropy = 0.0;

        for count in freq.values() {
            let p = (*count as f64) / len;
            entropy -= p * p.log2();
        }

        entropy
    }

    /// Detect AWS access keys
    pub fn detect_aws_keys(file_path: &str, content: &str) -> Vec<DetectedSecret> {
        let mut secrets = Vec::new();

        // AWS Access Key pattern
        if let Ok(re) = regex::Regex::new(r"AKIA[0-9A-Z]{16}") {
            for (line_num, line) in content.lines().enumerate() {
                if let Some(mat) = re.find(line) {
                    secrets.push(DetectedSecret::new(
                        SecretType::AwsAccessKey,
                        file_path,
                        (line_num + 1) as u32,
                        format!("{}***", &mat.as_str()[..8]),
                    ));
                }
            }
        }

        secrets
    }

    /// Detect GitHub tokens
    pub fn detect_github_tokens(file_path: &str, content: &str) -> Vec<DetectedSecret> {
        let mut secrets = Vec::new();

        // GitHub tokens are ghp_ followed by 36 alphanumeric characters
        if let Ok(re) = regex::Regex::new(r"ghp_[a-zA-Z0-9]{36,}") {
            for (line_num, line) in content.lines().enumerate() {
                if let Some(mat) = re.find(line) {
                    secrets.push(DetectedSecret::new(
                        SecretType::GitHubToken,
                        file_path,
                        (line_num + 1) as u32,
                        format!("{}***", &mat.as_str()[..8]),
                    ));
                }
            }
        }

        secrets
    }

    /// Detect Slack tokens
    pub fn detect_slack_tokens(file_path: &str, content: &str) -> Vec<DetectedSecret> {
        let mut secrets = Vec::new();

        if let Ok(re) = regex::Regex::new(r"xox[baprs]-[0-9a-zA-Z-]+") {
            for (line_num, line) in content.lines().enumerate() {
                if let Some(mat) = re.find(line) {
                    secrets.push(DetectedSecret::new(
                        SecretType::SlackToken,
                        file_path,
                        (line_num + 1) as u32,
                        format!("{}***", &mat.as_str()[..8]),
                    ));
                }
            }
        }

        secrets
    }

    /// Detect generic high-entropy secrets
    pub fn detect_high_entropy_strings(file_path: &str, content: &str) -> Vec<DetectedSecret> {
        let mut secrets = Vec::new();

        // Look for assignment patterns with high entropy values
        if let Ok(re) = regex::Regex::new(r#"(?:key|secret|token|password)\s*[=:]\s*["']([^"']{20,})["']"#) {
            for (line_num, line) in content.lines().enumerate() {
                if let Some(caps) = re.captures(line) {
                    if let Some(value) = caps.get(1) {
                        let entropy = Self::calculate_entropy(value.as_str());
                        if entropy > 4.5 {
                            secrets.push(DetectedSecret::new(
                                SecretType::GenericSecret,
                                file_path,
                                (line_num + 1) as u32,
                                format!("{}***", &value.as_str()[..std::cmp::min(8, value.as_str().len())]),
                            ));
                        }
                    }
                }
            }
        }

        secrets
    }

    /// Scan file for all types of secrets
    pub fn scan_file(file_path: &str, content: &str) -> Vec<DetectedSecret> {
        let mut secrets = Vec::new();
        secrets.extend(Self::detect_aws_keys(file_path, content));
        secrets.extend(Self::detect_github_tokens(file_path, content));
        secrets.extend(Self::detect_slack_tokens(file_path, content));
        secrets.extend(Self::detect_high_entropy_strings(file_path, content));
        secrets
    }

    /// Generate rotation recommendation for a secret
    pub fn generate_rotation_recommendation(secret: &DetectedSecret) -> String {
        match secret.secret_type {
            SecretType::AwsAccessKey | SecretType::AwsSecretKey => {
                "1. Login to AWS IAM Console\n2. Deactivate the exposed access key\n3. Create a new access key\n4. Update applications to use the new key\n5. Delete the old access key\n6. Consider using AWS Secrets Manager".to_string()
            }
            SecretType::GitHubToken => {
                "1. Go to GitHub Settings > Developer settings > Personal access tokens\n2. Delete the exposed token\n3. Generate a new token with minimal required scopes\n4. Update applications to use the new token\n5. Consider using GitHub Apps for automation".to_string()
            }
            SecretType::SlackToken => {
                "1. Go to Slack App settings\n2. Regenerate the OAuth token or Bot token\n3. Update applications with the new token\n4. Revoke the old token if possible".to_string()
            }
            _ => {
                "1. Identify the service this secret belongs to\n2. Rotate the credential through the service provider\n3. Update all references to use the new credential\n4. Consider using a secret management solution (HashiCorp Vault, etc.)".to_string()
            }
        }
    }
}

/// License compliance scanner
pub struct LicenseScanner;

impl LicenseScanner {
    /// Check if a license is in the denied list
    pub fn is_denied_license(license: &str) -> bool {
        matches!(license, "GPL-2.0" | "GPL-3.0" | "AGPL-3.0")
    }

    /// Check if a license requires review
    pub fn requires_review(license: &str) -> bool {
        matches!(license, "LGPL-2.1" | "LGPL-3.0" | "MPL-2.0")
    }

    /// Check if a license is allowed
    pub fn is_allowed_license(license: &str) -> bool {
        matches!(license, "MIT" | "Apache-2.0" | "BSD-2-Clause" | "BSD-3-Clause" | "ISC")
    }

    /// Scan a package and determine license compliance
    pub fn check_license(package_name: &str, license: &str, policy: &SecurityPolicy) -> Option<LicenseIssue> {
        match policy.check_license(license) {
            LicenseCheckResult::Denied => {
                Some(LicenseIssue::new(package_name, license, LicenseIssueType::Denied))
            }
            LicenseCheckResult::ReviewRequired => {
                Some(LicenseIssue::new(package_name, license, LicenseIssueType::ReviewRequired))
            }
            LicenseCheckResult::Unknown => {
                Some(LicenseIssue::new(package_name, license, LicenseIssueType::Unknown))
            }
            LicenseCheckResult::Allowed => None,
        }
    }
}

/// Container image scanner
pub struct ContainerScanner;

impl ContainerScanner {
    /// Check if container is running as root
    pub fn detect_root_user(image_metadata: &serde_json::Value) -> Option<Vulnerability> {
        if let Some(user) = image_metadata["Config"]["User"].as_str() {
            if user.is_empty() || user == "root" || user == "0" {
                return Some(
                    Vulnerability::code(
                        "Container running as root",
                        "Dockerfile",
                        0,
                        Severity::Medium,
                    )
                    .with_description("Container is configured to run as root user, which poses security risks")
                );
            }
        }
        None
    }

    /// Detect exposed sensitive ports
    pub fn detect_sensitive_ports(image_metadata: &serde_json::Value) -> Vec<Vulnerability> {
        let mut vulns = Vec::new();
        let sensitive_ports = vec!["22", "3389", "5432", "3306", "27017"];

        if let Some(ports) = image_metadata["Config"]["ExposedPorts"].as_object() {
            for port in ports.keys() {
                let port_num = port.split('/').next().unwrap_or("");
                if sensitive_ports.contains(&port_num) {
                    vulns.push(
                        Vulnerability::code(
                            format!("Sensitive port {} exposed", port_num),
                            "Dockerfile",
                            0,
                            Severity::High,
                        )
                        .with_description(format!("Port {} is typically sensitive and should not be exposed", port_num))
                    );
                }
            }
        }

        vulns
    }

    /// Recommend base image improvements
    pub fn recommend_base_image(current_image: &str) -> String {
        if current_image.contains(":latest") {
            return "Use specific version tags instead of 'latest' for reproducible builds".to_string();
        }

        if !current_image.contains("alpine") && !current_image.contains("distroless") {
            return "Consider using Alpine or distroless base images to reduce attack surface".to_string();
        }

        "Base image configuration looks good".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Unknown);
    }

    #[test]
    fn test_scan_creation_and_completion() {
        let mut scan = SecurityScan::new(
            vec![ScanType::Dependencies, ScanType::Secrets],
            "user@example.com",
        );

        assert_eq!(scan.status, ScanStatus::Pending);

        scan.start();
        assert_eq!(scan.status, ScanStatus::Running);

        scan.add_vulnerability(
            Vulnerability::dependency("lodash", "4.17.20", Severity::Critical)
                .with_cve("CVE-2021-23337")
                .with_fix("4.17.21"),
        );

        scan.add_secret(DetectedSecret::new(
            SecretType::AwsAccessKey,
            "config/secrets.yaml",
            15,
            "AKIA***",
        ));

        scan.complete();
        assert_eq!(scan.status, ScanStatus::Completed);
        assert_eq!(scan.summary.critical_count, 1);
        assert_eq!(scan.summary.secrets_count, 1);
        assert_eq!(scan.summary.auto_fixable_count, 1);
    }

    #[test]
    fn test_vulnerability_builder() {
        let vuln = Vulnerability::dependency("axios", "0.21.0", Severity::High)
            .with_cve("CVE-2021-3749")
            .with_fix("0.21.2")
            .with_description("ReDoS vulnerability");

        assert_eq!(vuln.package_name, Some("axios".to_string()));
        assert_eq!(vuln.cve_id, Some("CVE-2021-3749".to_string()));
        assert!(vuln.auto_fixable);
    }

    #[test]
    fn test_code_vulnerability() {
        let vuln = Vulnerability::code(
            "SQL Injection",
            "src/api/users.rs",
            42,
            Severity::Critical,
        );

        assert_eq!(vuln.vulnerability_type, VulnerabilityType::CodeVulnerability);
        assert_eq!(vuln.file_path, Some("src/api/users.rs".to_string()));
        assert_eq!(vuln.line_number, Some(42));
    }

    #[test]
    fn test_secret_detection() {
        let secret = DetectedSecret::new(
            SecretType::GitHubToken,
            ".env",
            5,
            "ghp_***",
        )
        .in_history("abc123");

        assert!(secret.is_in_history);
        assert_eq!(secret.commit_sha, Some("abc123".to_string()));
        assert!(secret.rotation_needed);
    }

    #[test]
    fn test_security_policy() {
        let policy = SecurityPolicy::default();

        assert!(policy.should_block(&Severity::Critical));
        assert!(policy.should_block(&Severity::High));
        assert!(!policy.should_block(&Severity::Medium));

        assert_eq!(policy.check_license("MIT"), LicenseCheckResult::Allowed);
        assert_eq!(policy.check_license("GPL-3.0"), LicenseCheckResult::Denied);
        assert_eq!(policy.check_license("LGPL-2.1"), LicenseCheckResult::ReviewRequired);
        assert_eq!(policy.check_license("CustomLicense"), LicenseCheckResult::Unknown);
    }

    #[test]
    fn test_security_exception() {
        let exception = SecurityException::new(
            "vuln-123",
            "False positive in test code",
            "security-team",
            30,
        );

        assert!(exception.is_active);
        assert!(!exception.is_expired());
    }

    #[test]
    fn test_security_fix() {
        let mut fix = SecurityFix::new(
            vec!["vuln-1".to_string(), "vuln-2".to_string()],
            FixType::DependencyUpgrade,
            "bot",
        );

        assert_eq!(fix.status, FixStatus::Pending);

        fix.complete(123);
        assert_eq!(fix.status, FixStatus::Completed);
        assert_eq!(fix.pr_number, Some(123));
    }

    #[test]
    fn test_license_issue() {
        let issue = LicenseIssue::new("my-package", "GPL-3.0", LicenseIssueType::Denied);

        assert_eq!(issue.severity, Severity::High);
        assert_eq!(issue.issue_type, LicenseIssueType::Denied);
    }

    #[test]
    fn test_sarif_report_generation() {
        let mut scan = SecurityScan::new(vec![ScanType::Code], "test");
        scan.add_vulnerability(
            Vulnerability::code("XSS", "src/app.js", 10, Severity::High)
                .with_description("Unescaped user input"),
        );
        scan.complete();

        let report = SarifReport::from_scan(&scan, "orchestrate-security");

        assert_eq!(report.version, "2.1.0");
        assert_eq!(report.runs.len(), 1);
        assert_eq!(report.runs[0].results.len(), 1);
        assert_eq!(report.runs[0].results[0].level, "error");
    }

    #[test]
    fn test_scan_blocking() {
        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        scan.add_vulnerability(
            Vulnerability::dependency("vulnerable-pkg", "1.0.0", Severity::Critical),
        );
        scan.complete();

        let policy = SecurityPolicy::default();
        assert!(scan.has_blocking_issues(&policy));

        let mut safe_scan = SecurityScan::new(vec![ScanType::Full], "test");
        safe_scan.add_vulnerability(
            Vulnerability::dependency("minor-issue", "1.0.0", Severity::Low),
        );
        safe_scan.complete();
        assert!(!safe_scan.has_blocking_issues(&policy));
    }

    // ==================== Package Manager Tests ====================

    #[test]
    fn test_package_manager_detection() {
        // Note: These tests would normally check actual file system,
        // but we're testing the logic here
        assert_eq!(PackageManager::Cargo.audit_command(), "cargo audit --json");
        assert_eq!(PackageManager::Npm.audit_command(), "npm audit --json");
        assert_eq!(PackageManager::Pip.audit_command(), "pip-audit --format json");
    }

    // ==================== Dependency Scanner Tests ====================

    #[test]
    fn test_parse_cargo_audit() {
        let json = r#"{
            "vulnerabilities": {
                "list": [
                    {
                        "package": {
                            "name": "time",
                            "version": "0.1.44"
                        },
                        "advisory": {
                            "id": "RUSTSEC-2020-0071",
                            "title": "Potential segfault in time-0.1",
                            "description": "Unix-like operating systems may segfault due to dereferencing a dangling pointer",
                            "cvss": 7.5
                        },
                        "versions": {
                            "patched": [">=0.2.23"]
                        }
                    }
                ]
            }
        }"#;

        let vulns = DependencyScanner::parse_cargo_audit(json);
        assert_eq!(vulns.len(), 1);

        let vuln = &vulns[0];
        assert_eq!(vuln.package_name, Some("time".to_string()));
        assert_eq!(vuln.installed_version, Some("0.1.44".to_string()));
        assert_eq!(vuln.cve_id, Some("RUSTSEC-2020-0071".to_string()));
        assert_eq!(vuln.severity, Severity::High);
        assert!(vuln.auto_fixable);
        assert_eq!(vuln.fixed_version, Some(">=0.2.23".to_string()));
    }

    #[test]
    fn test_generate_fix_recommendation() {
        let vuln = Vulnerability::dependency("axios", "0.21.0", Severity::High)
            .with_fix("0.21.2");

        let cargo_fix = DependencyScanner::generate_fix_recommendation(&vuln, &PackageManager::Cargo);
        assert_eq!(cargo_fix, Some("cargo update axios".to_string()));

        let npm_fix = DependencyScanner::generate_fix_recommendation(&vuln, &PackageManager::Npm);
        assert!(npm_fix.is_some());
        let npm_fix_str = npm_fix.unwrap();
        assert!(npm_fix_str.contains("npm"));
        assert!(npm_fix_str.contains("0.21.2"));
    }

    // ==================== SAST Scanner Tests ====================

    #[test]
    fn test_detect_sql_injection() {
        let code = r#"
            fn vulnerable() {
                let query = "SELECT * FROM users WHERE id = " + user_id;
                db.execute("SELECT * " + "FROM users");
            }
        "#;

        let vulns = SastScanner::detect_sql_injection("test.rs", code);
        assert!(vulns.len() > 0);
        assert_eq!(vulns[0].vulnerability_type, VulnerabilityType::SqlInjection);
        assert_eq!(vulns[0].severity, Severity::High);
    }

    #[test]
    fn test_detect_xss() {
        let code = r#"
            element.innerHTML = userInput;
            <div dangerouslySetInnerHTML={{__html: data}} />
        "#;

        let vulns = SastScanner::detect_xss("app.js", code);
        assert!(vulns.len() >= 1);
        assert_eq!(vulns[0].vulnerability_type, VulnerabilityType::Xss);
    }

    #[test]
    fn test_detect_command_injection() {
        let code = r#"
            let cmd = Command::new("bash").arg("-c").arg("ls " + user_input);
        "#;

        let vulns = SastScanner::detect_command_injection("main.rs", code);
        assert!(vulns.len() > 0);
        assert_eq!(vulns[0].vulnerability_type, VulnerabilityType::CommandInjection);
        assert_eq!(vulns[0].severity, Severity::Critical);
    }

    #[test]
    fn test_detect_hardcoded_credentials() {
        let code = r#"
            let password = "super_secret_123";
            let api_key = "sk_live_1234567890abcdef";
        "#;

        let vulns = SastScanner::detect_hardcoded_credentials("config.rs", code);
        assert!(vulns.len() >= 1);
        assert_eq!(vulns[0].vulnerability_type, VulnerabilityType::HardcodedSecret);
        assert_eq!(vulns[0].severity, Severity::Critical);
    }

    #[test]
    fn test_sast_scan_file() {
        let code = r#"
            let password = "hardcoded123";
            element.innerHTML = data;
        "#;

        let vulns = SastScanner::scan_file("test.js", code);
        assert!(vulns.len() >= 2); // At least password and XSS
    }

    // ==================== Secret Detector Tests ====================

    #[test]
    fn test_calculate_entropy() {
        // Low entropy (repeated characters)
        let low = SecretDetector::calculate_entropy("aaaaaaaaaa");
        assert!(low < 1.0);

        // High entropy (random-looking string)
        let high = SecretDetector::calculate_entropy("aB3$xK9!mP2@nQ8#");
        assert!(high > 3.0);
    }

    #[test]
    fn test_detect_aws_keys() {
        let content = "AWS_ACCESS_KEY=AKIAIOSFODNN7EXAMPLE";
        let secrets = SecretDetector::detect_aws_keys("config.txt", content);

        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0].secret_type, SecretType::AwsAccessKey);
        assert!(secrets[0].match_text.contains("***"));
    }

    #[test]
    fn test_detect_github_tokens() {
        // GitHub personal access tokens are ghp_ followed by 36 alphanumeric chars
        let content = "GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let secrets = SecretDetector::detect_github_tokens(".env", content);

        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0].secret_type, SecretType::GitHubToken);
    }

    #[test]
    fn test_detect_slack_tokens() {
        // Use a fake token that matches pattern but won't trigger GitHub secret scanning
        let content = "SLACK_TOKEN=xoxb-FAKE0FAKE0FAKE-FAKE0FAKE0FAKE-abcdefghijklmnopqrstuv";
        let secrets = SecretDetector::detect_slack_tokens("config.yaml", content);

        assert_eq!(secrets.len(), 1);
        assert_eq!(secrets[0].secret_type, SecretType::SlackToken);
    }

    #[test]
    fn test_detect_high_entropy_strings() {
        let content = r#"secret = "aB3$xK9!mP2@nQ8#wR7%vT6^uY5&sX4*""#;
        let secrets = SecretDetector::detect_high_entropy_strings("app.conf", content);

        assert!(secrets.len() > 0);
        assert_eq!(secrets[0].secret_type, SecretType::GenericSecret);
    }

    #[test]
    fn test_secret_scan_file() {
        let content = r#"
            AWS_ACCESS_KEY=AKIAIOSFODNN7EXAMPLE
            GITHUB_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyz
        "#;

        let secrets = SecretDetector::scan_file("secrets.env", content);
        assert!(secrets.len() >= 2);
    }

    #[test]
    fn test_generate_rotation_recommendation() {
        let aws_secret = DetectedSecret::new(
            SecretType::AwsAccessKey,
            ".env",
            1,
            "AKIA***",
        );
        let recommendation = SecretDetector::generate_rotation_recommendation(&aws_secret);
        assert!(recommendation.contains("AWS IAM"));
        assert!(recommendation.contains("Deactivate"));

        let github_secret = DetectedSecret::new(
            SecretType::GitHubToken,
            ".env",
            2,
            "ghp_***",
        );
        let recommendation = SecretDetector::generate_rotation_recommendation(&github_secret);
        assert!(recommendation.contains("GitHub"));
        assert!(recommendation.contains("Personal access tokens"));
    }

    // ==================== License Scanner Tests ====================

    #[test]
    fn test_is_denied_license() {
        assert!(LicenseScanner::is_denied_license("GPL-2.0"));
        assert!(LicenseScanner::is_denied_license("GPL-3.0"));
        assert!(LicenseScanner::is_denied_license("AGPL-3.0"));
        assert!(!LicenseScanner::is_denied_license("MIT"));
    }

    #[test]
    fn test_requires_review() {
        assert!(LicenseScanner::requires_review("LGPL-2.1"));
        assert!(LicenseScanner::requires_review("MPL-2.0"));
        assert!(!LicenseScanner::requires_review("MIT"));
    }

    #[test]
    fn test_is_allowed_license() {
        assert!(LicenseScanner::is_allowed_license("MIT"));
        assert!(LicenseScanner::is_allowed_license("Apache-2.0"));
        assert!(LicenseScanner::is_allowed_license("BSD-3-Clause"));
        assert!(!LicenseScanner::is_allowed_license("GPL-3.0"));
    }

    #[test]
    fn test_check_license() {
        let policy = SecurityPolicy::default();

        // Denied license
        let issue = LicenseScanner::check_license("my-gpl-dep", "GPL-3.0", &policy);
        assert!(issue.is_some());
        assert_eq!(issue.unwrap().issue_type, LicenseIssueType::Denied);

        // Review required
        let issue = LicenseScanner::check_license("my-lgpl-dep", "LGPL-2.1", &policy);
        assert!(issue.is_some());
        assert_eq!(issue.unwrap().issue_type, LicenseIssueType::ReviewRequired);

        // Allowed license
        let issue = LicenseScanner::check_license("my-mit-dep", "MIT", &policy);
        assert!(issue.is_none());

        // Unknown license
        let issue = LicenseScanner::check_license("my-custom-dep", "CustomLicense", &policy);
        assert!(issue.is_some());
        assert_eq!(issue.unwrap().issue_type, LicenseIssueType::Unknown);
    }

    // ==================== Container Scanner Tests ====================

    #[test]
    fn test_detect_root_user() {
        // Running as root
        let metadata_root = serde_json::json!({
            "Config": {
                "User": ""
            }
        });
        let vuln = ContainerScanner::detect_root_user(&metadata_root);
        assert!(vuln.is_some());
        assert_eq!(vuln.unwrap().severity, Severity::Medium);

        // Running as non-root user
        let metadata_user = serde_json::json!({
            "Config": {
                "User": "appuser"
            }
        });
        let vuln = ContainerScanner::detect_root_user(&metadata_user);
        assert!(vuln.is_none());
    }

    #[test]
    fn test_detect_sensitive_ports() {
        let metadata = serde_json::json!({
            "Config": {
                "ExposedPorts": {
                    "22/tcp": {},
                    "5432/tcp": {},
                    "8080/tcp": {}
                }
            }
        });

        let vulns = ContainerScanner::detect_sensitive_ports(&metadata);
        assert!(vulns.len() >= 2); // SSH and PostgreSQL ports
        assert!(vulns.iter().any(|v| v.title.contains("22")));
        assert!(vulns.iter().any(|v| v.title.contains("5432")));
    }

    #[test]
    fn test_recommend_base_image() {
        // Using 'latest' tag
        let rec = ContainerScanner::recommend_base_image("ubuntu:latest");
        assert!(rec.contains("specific version"));

        // Using large base image
        let rec = ContainerScanner::recommend_base_image("ubuntu:20.04");
        assert!(rec.contains("alpine") || rec.contains("distroless"));

        // Good base image
        let rec = ContainerScanner::recommend_base_image("alpine:3.14");
        assert!(rec.contains("looks good"));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_full_security_scan_workflow() {
        let mut scan = SecurityScan::new(
            vec![ScanType::Dependencies, ScanType::Code, ScanType::Secrets],
            "security-bot",
        );

        scan.start();

        // Add dependency vulnerability
        scan.add_vulnerability(
            Vulnerability::dependency("lodash", "4.17.20", Severity::Critical)
                .with_cve("CVE-2021-23337")
                .with_fix("4.17.21"),
        );

        // Add code vulnerability
        scan.add_vulnerability(
            Vulnerability::code("SQL Injection", "api.rs", 42, Severity::High)
                .with_description("Unsafe SQL query construction"),
        );

        // Add detected secret
        scan.add_secret(DetectedSecret::new(
            SecretType::AwsAccessKey,
            ".env",
            5,
            "AKIA***",
        ));

        scan.complete();

        assert_eq!(scan.status, ScanStatus::Completed);
        assert_eq!(scan.summary.total_vulnerabilities, 2);
        assert_eq!(scan.summary.critical_count, 1);
        assert_eq!(scan.summary.high_count, 1);
        assert_eq!(scan.summary.secrets_count, 1);
        assert_eq!(scan.summary.auto_fixable_count, 1);
    }
}
