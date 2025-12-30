//! Security Fix Agent
//!
//! Automatically fixes security vulnerabilities by:
//! - Upgrading vulnerable dependencies
//! - Applying code patches for common vulnerabilities
//! - Creating PRs with security fixes
//! - Categorizing fixes (auto-fix vs manual review)

use crate::security::{FixChange, FixType, SecurityFix, Severity, Vulnerability};
use std::collections::HashMap;

/// Security fix agent that can automatically remediate vulnerabilities
pub struct SecurityFixAgent {
    /// Maximum number of vulnerabilities to fix in a single PR
    max_fixes_per_pr: usize,
    /// Whether to create separate PRs for different fix types
    separate_prs_by_type: bool,
}

impl SecurityFixAgent {
    /// Create a new security fix agent
    pub fn new() -> Self {
        Self {
            max_fixes_per_pr: 10,
            separate_prs_by_type: true,
        }
    }

    /// Configure maximum fixes per PR
    pub fn with_max_fixes_per_pr(mut self, max: usize) -> Self {
        self.max_fixes_per_pr = max;
        self
    }

    /// Configure whether to separate PRs by fix type
    pub fn with_separate_prs(mut self, separate: bool) -> Self {
        self.separate_prs_by_type = separate;
        self
    }

    /// Categorize a vulnerability as auto-fixable or requiring manual review
    pub fn categorize_fix(&self, vuln: &Vulnerability) -> FixCategory {
        // If the vulnerability already has a fixed_version, it's auto-fixable
        if vuln.auto_fixable && vuln.fixed_version.is_some() {
            return FixCategory::AutoFix(self.determine_fix_type(vuln));
        }

        // Check severity and type for categorization
        match (&vuln.severity, &vuln.vulnerability_type) {
            // Critical/High severity code vulnerabilities need manual review
            (Severity::Critical | Severity::High, crate::security::VulnerabilityType::CodeVulnerability) => {
                FixCategory::ManualReview("High severity code vulnerability requires manual review".to_string())
            }
            // Secrets always need manual review (requires rotation)
            (_, crate::security::VulnerabilityType::HardcodedSecret) => {
                FixCategory::ManualReview("Secret rotation requires manual intervention".to_string())
            }
            // Dependency vulnerabilities with known fixes are auto-fixable
            (_, crate::security::VulnerabilityType::DependencyVulnerability) if vuln.fixed_version.is_some() => {
                FixCategory::AutoFix(FixType::DependencyUpgrade)
            }
            // Configuration issues can often be auto-fixed
            (_, crate::security::VulnerabilityType::ConfigurationVulnerability) => {
                FixCategory::AutoFix(FixType::ConfigurationChange)
            }
            // Everything else requires manual review
            _ => FixCategory::ManualReview("Requires security expert review".to_string()),
        }
    }

    /// Determine the fix type for a vulnerability
    fn determine_fix_type(&self, vuln: &Vulnerability) -> FixType {
        match &vuln.vulnerability_type {
            crate::security::VulnerabilityType::DependencyVulnerability => FixType::DependencyUpgrade,
            crate::security::VulnerabilityType::HardcodedSecret => FixType::SecretRotation,
            crate::security::VulnerabilityType::ConfigurationVulnerability => FixType::ConfigurationChange,
            _ => FixType::CodePatch,
        }
    }

    /// Group vulnerabilities by fix type for batching
    pub fn group_by_fix_type(&self, vulnerabilities: &[Vulnerability]) -> HashMap<FixType, Vec<String>> {
        let mut grouped: HashMap<FixType, Vec<String>> = HashMap::new();

        for vuln in vulnerabilities {
            if let FixCategory::AutoFix(fix_type) = self.categorize_fix(vuln) {
                grouped.entry(fix_type).or_default().push(vuln.id.clone());
            }
        }

        grouped
    }

    /// Create security fixes for auto-fixable vulnerabilities
    pub fn create_fixes(&self, vulnerabilities: &[Vulnerability], created_by: &str) -> Vec<SecurityFix> {
        let mut fixes = Vec::new();

        if self.separate_prs_by_type {
            // Group by fix type
            let grouped = self.group_by_fix_type(vulnerabilities);

            for (fix_type, vuln_ids) in grouped {
                // Split into batches if too many
                for chunk in vuln_ids.chunks(self.max_fixes_per_pr) {
                    fixes.push(SecurityFix::new(
                        chunk.to_vec(),
                        fix_type.clone(),
                        created_by,
                    ));
                }
            }
        } else {
            // Create a single fix for all auto-fixable vulnerabilities
            let auto_fixable: Vec<String> = vulnerabilities
                .iter()
                .filter(|v| matches!(self.categorize_fix(v), FixCategory::AutoFix(_)))
                .map(|v| v.id.clone())
                .collect();

            if !auto_fixable.is_empty() {
                for chunk in auto_fixable.chunks(self.max_fixes_per_pr) {
                    // Use DependencyUpgrade as default for mixed fixes
                    fixes.push(SecurityFix::new(
                        chunk.to_vec(),
                        FixType::DependencyUpgrade,
                        created_by,
                    ));
                }
            }
        }

        fixes
    }

    /// Generate fix changes for a dependency upgrade
    pub fn generate_dependency_upgrade_changes(
        &self,
        vulnerabilities: &[Vulnerability],
    ) -> Vec<FixChange> {
        let mut changes = Vec::new();

        for vuln in vulnerabilities {
            if let Some(package) = &vuln.package_name {
                if let (Some(installed), Some(fixed)) = (&vuln.installed_version, &vuln.fixed_version) {
                    changes.push(FixChange {
                        file_path: "Cargo.toml".to_string(), // or package.json, etc.
                        description: format!(
                            "Upgrade {} from {} to {} (fixes {})",
                            package,
                            installed,
                            fixed,
                            vuln.cve_id.as_ref().unwrap_or(&vuln.id)
                        ),
                        before: Some(format!("{} = \"{}\"", package, installed)),
                        after: Some(format!("{} = \"{}\"", package, fixed)),
                    });
                }
            }
        }

        changes
    }

    /// Generate fix command for a vulnerability
    pub fn generate_fix_command(&self, vuln: &Vulnerability) -> Option<String> {
        // If fix_command is already set, use it
        if let Some(cmd) = &vuln.fix_command {
            return Some(cmd.clone());
        }

        // Generate command based on fix type
        match self.categorize_fix(vuln) {
            FixCategory::AutoFix(FixType::DependencyUpgrade) => {
                if let (Some(package), Some(version)) = (&vuln.package_name, &vuln.fixed_version) {
                    // Rust: cargo update
                    Some(format!("cargo update -p {} --precise {}", package, version))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Filter vulnerabilities that can be auto-fixed
    pub fn filter_auto_fixable<'a>(&self, vulnerabilities: &'a [Vulnerability]) -> Vec<&'a Vulnerability> {
        vulnerabilities
            .iter()
            .filter(|v| matches!(self.categorize_fix(v), FixCategory::AutoFix(_)))
            .collect()
    }

    /// Filter vulnerabilities that require manual review
    pub fn filter_manual_review<'a>(&self, vulnerabilities: &'a [Vulnerability]) -> Vec<&'a Vulnerability> {
        vulnerabilities
            .iter()
            .filter(|v| matches!(self.categorize_fix(v), FixCategory::ManualReview(_)))
            .collect()
    }
}

impl Default for SecurityFixAgent {
    fn default() -> Self {
        Self::new()
    }
}

/// Fix categorization result
#[derive(Debug, Clone, PartialEq)]
pub enum FixCategory {
    /// Can be automatically fixed
    AutoFix(FixType),
    /// Requires manual review with reason
    ManualReview(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::VulnerabilityType;

    #[test]
    fn test_categorize_dependency_with_fix() {
        let agent = SecurityFixAgent::new();
        let vuln = Vulnerability::dependency("lodash", "4.17.20", Severity::Critical)
            .with_cve("CVE-2021-23337")
            .with_fix("4.17.21");

        let category = agent.categorize_fix(&vuln);
        assert_eq!(category, FixCategory::AutoFix(FixType::DependencyUpgrade));
    }

    #[test]
    fn test_categorize_dependency_without_fix() {
        let agent = SecurityFixAgent::new();
        let mut vuln = Vulnerability::dependency("old-pkg", "1.0.0", Severity::High);
        vuln.auto_fixable = false;

        let category = agent.categorize_fix(&vuln);
        assert!(matches!(category, FixCategory::ManualReview(_)));
    }

    #[test]
    fn test_categorize_critical_code_vulnerability() {
        let agent = SecurityFixAgent::new();
        let vuln = Vulnerability::code("SQL Injection", "src/db.rs", 42, Severity::Critical);

        let category = agent.categorize_fix(&vuln);
        assert!(matches!(category, FixCategory::ManualReview(_)));
    }

    #[test]
    fn test_categorize_hardcoded_secret() {
        let agent = SecurityFixAgent::new();
        let mut vuln = Vulnerability::code("Hardcoded API Key", "src/config.rs", 10, Severity::High);
        vuln.vulnerability_type = VulnerabilityType::HardcodedSecret;

        let category = agent.categorize_fix(&vuln);
        assert!(matches!(category, FixCategory::ManualReview(_)));
    }

    #[test]
    fn test_group_by_fix_type() {
        let agent = SecurityFixAgent::new();
        let vulns = vec![
            Vulnerability::dependency("pkg1", "1.0.0", Severity::High).with_fix("1.0.1"),
            Vulnerability::dependency("pkg2", "2.0.0", Severity::Medium).with_fix("2.0.1"),
            Vulnerability::code("XSS", "src/app.rs", 10, Severity::High),
        ];

        let grouped = agent.group_by_fix_type(&vulns);
        assert_eq!(grouped.len(), 1); // Only dependency upgrades
        assert_eq!(grouped.get(&FixType::DependencyUpgrade).unwrap().len(), 2);
    }

    #[test]
    fn test_create_fixes_separate_prs() {
        let agent = SecurityFixAgent::new().with_separate_prs(true);
        let vulns = vec![
            Vulnerability::dependency("pkg1", "1.0.0", Severity::High).with_fix("1.0.1"),
            Vulnerability::dependency("pkg2", "2.0.0", Severity::Medium).with_fix("2.0.1"),
        ];

        let fixes = agent.create_fixes(&vulns, "security-bot");
        assert_eq!(fixes.len(), 1); // One PR for dependency upgrades
        assert_eq!(fixes[0].vulnerability_ids.len(), 2);
        assert_eq!(fixes[0].fix_type, FixType::DependencyUpgrade);
    }

    #[test]
    fn test_create_fixes_max_per_pr() {
        let agent = SecurityFixAgent::new()
            .with_max_fixes_per_pr(2)
            .with_separate_prs(true);

        let vulns = vec![
            Vulnerability::dependency("pkg1", "1.0.0", Severity::High).with_fix("1.0.1"),
            Vulnerability::dependency("pkg2", "2.0.0", Severity::Medium).with_fix("2.0.1"),
            Vulnerability::dependency("pkg3", "3.0.0", Severity::Low).with_fix("3.0.1"),
        ];

        let fixes = agent.create_fixes(&vulns, "security-bot");
        assert_eq!(fixes.len(), 2); // Split into 2 PRs (2 + 1)
    }

    #[test]
    fn test_generate_dependency_upgrade_changes() {
        let agent = SecurityFixAgent::new();
        let vulns = vec![
            Vulnerability::dependency("axios", "0.21.0", Severity::High)
                .with_cve("CVE-2021-3749")
                .with_fix("0.21.2"),
        ];

        let changes = agent.generate_dependency_upgrade_changes(&vulns);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].description.contains("axios"));
        assert!(changes[0].description.contains("0.21.0"));
        assert!(changes[0].description.contains("0.21.2"));
    }

    #[test]
    fn test_generate_fix_command() {
        let agent = SecurityFixAgent::new();
        let vuln = Vulnerability::dependency("serde", "1.0.0", Severity::Medium)
            .with_fix("1.0.1");

        let command = agent.generate_fix_command(&vuln);
        assert!(command.is_some());
        assert!(command.unwrap().contains("cargo update"));
    }

    #[test]
    fn test_filter_auto_fixable() {
        let agent = SecurityFixAgent::new();
        let vulns = vec![
            Vulnerability::dependency("pkg1", "1.0.0", Severity::High).with_fix("1.0.1"),
            Vulnerability::code("SQL Injection", "src/db.rs", 42, Severity::Critical),
            Vulnerability::dependency("pkg2", "2.0.0", Severity::Medium).with_fix("2.0.1"),
        ];

        let auto_fixable = agent.filter_auto_fixable(&vulns);
        assert_eq!(auto_fixable.len(), 2);
    }

    #[test]
    fn test_filter_manual_review() {
        let agent = SecurityFixAgent::new();
        let vulns = vec![
            Vulnerability::dependency("pkg1", "1.0.0", Severity::High).with_fix("1.0.1"),
            Vulnerability::code("SQL Injection", "src/db.rs", 42, Severity::Critical),
            Vulnerability::dependency("pkg2", "2.0.0", Severity::Medium).with_fix("2.0.1"),
        ];

        let manual_review = agent.filter_manual_review(&vulns);
        assert_eq!(manual_review.len(), 1);
    }
}
