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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FixType {
    DependencyUpgrade,
    CodePatch,
    ConfigurationChange,
    SecretRotation,
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
}
