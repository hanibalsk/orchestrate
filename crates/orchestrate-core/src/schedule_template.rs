//! Built-in schedule templates
//!
//! This module provides pre-configured schedule templates for common tasks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A schedule template with pre-configured settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTemplate {
    /// Template name
    pub name: String,
    /// Cron expression
    pub cron: String,
    /// Agent type to execute
    pub agent: String,
    /// Task description
    pub task: String,
    /// Template description
    pub description: String,
}

/// Get all built-in schedule templates
pub fn get_templates() -> HashMap<String, ScheduleTemplate> {
    let mut templates = HashMap::new();

    templates.insert(
        "security-scan".to_string(),
        ScheduleTemplate {
            name: "security-scan".to_string(),
            cron: "0 2 * * *".to_string(),
            agent: "security-scanner".to_string(),
            task: "Run full security scan and report vulnerabilities".to_string(),
            description: "Daily security scan at 2 AM to detect vulnerabilities and security issues".to_string(),
        },
    );

    templates.insert(
        "dependency-check".to_string(),
        ScheduleTemplate {
            name: "dependency-check".to_string(),
            cron: "0 3 * * *".to_string(),
            agent: "dependency-checker".to_string(),
            task: "Check for outdated dependencies and security advisories".to_string(),
            description: "Daily dependency check at 3 AM to find outdated packages and security advisories".to_string(),
        },
    );

    templates.insert(
        "code-quality".to_string(),
        ScheduleTemplate {
            name: "code-quality".to_string(),
            cron: "0 9 * * 1".to_string(),
            agent: "code-quality-checker".to_string(),
            task: "Run code quality analysis and generate report".to_string(),
            description: "Weekly code quality report every Monday at 9 AM".to_string(),
        },
    );

    templates.insert(
        "documentation-check".to_string(),
        ScheduleTemplate {
            name: "documentation-check".to_string(),
            cron: "0 10 * * 1".to_string(),
            agent: "documentation-checker".to_string(),
            task: "Check documentation freshness and completeness".to_string(),
            description: "Weekly documentation freshness check every Monday at 10 AM".to_string(),
        },
    );

    templates.insert(
        "database-backup".to_string(),
        ScheduleTemplate {
            name: "database-backup".to_string(),
            cron: "0 1 * * *".to_string(),
            agent: "backup-controller".to_string(),
            task: "Perform database backup and verify integrity".to_string(),
            description: "Daily database backup at 1 AM with integrity verification".to_string(),
        },
    );

    templates
}

/// Get a specific template by name
pub fn get_template(name: &str) -> Option<ScheduleTemplate> {
    get_templates().get(name).cloned()
}

/// List all available template names
pub fn list_template_names() -> Vec<String> {
    let mut names: Vec<String> = get_templates().keys().cloned().collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_templates_returns_all_templates() {
        let templates = get_templates();

        // Should have exactly 5 templates
        assert_eq!(templates.len(), 5);

        // Verify all required templates exist
        assert!(templates.contains_key("security-scan"));
        assert!(templates.contains_key("dependency-check"));
        assert!(templates.contains_key("code-quality"));
        assert!(templates.contains_key("documentation-check"));
        assert!(templates.contains_key("database-backup"));
    }

    #[test]
    fn test_security_scan_template() {
        let template = get_template("security-scan").expect("security-scan template should exist");

        assert_eq!(template.name, "security-scan");
        assert_eq!(template.cron, "0 2 * * *"); // Daily at 2 AM
        assert_eq!(template.agent, "security-scanner");
        assert_eq!(template.task, "Run full security scan and report vulnerabilities");
        assert!(!template.description.is_empty());
    }

    #[test]
    fn test_dependency_check_template() {
        let template = get_template("dependency-check").expect("dependency-check template should exist");

        assert_eq!(template.name, "dependency-check");
        assert_eq!(template.cron, "0 3 * * *"); // Daily at 3 AM
        assert_eq!(template.agent, "dependency-checker");
        assert_eq!(template.task, "Check for outdated dependencies and security advisories");
        assert!(!template.description.is_empty());
    }

    #[test]
    fn test_code_quality_template() {
        let template = get_template("code-quality").expect("code-quality template should exist");

        assert_eq!(template.name, "code-quality");
        assert_eq!(template.cron, "0 9 * * 1"); // Weekly Monday at 9 AM
        assert_eq!(template.agent, "code-quality-checker");
        assert_eq!(template.task, "Run code quality analysis and generate report");
        assert!(!template.description.is_empty());
    }

    #[test]
    fn test_documentation_check_template() {
        let template = get_template("documentation-check").expect("documentation-check template should exist");

        assert_eq!(template.name, "documentation-check");
        assert_eq!(template.cron, "0 10 * * 1"); // Weekly Monday at 10 AM
        assert_eq!(template.agent, "documentation-checker");
        assert_eq!(template.task, "Check documentation freshness and completeness");
        assert!(!template.description.is_empty());
    }

    #[test]
    fn test_database_backup_template() {
        let template = get_template("database-backup").expect("database-backup template should exist");

        assert_eq!(template.name, "database-backup");
        assert_eq!(template.cron, "0 1 * * *"); // Daily at 1 AM
        assert_eq!(template.agent, "backup-controller");
        assert_eq!(template.task, "Perform database backup and verify integrity");
        assert!(!template.description.is_empty());
    }

    #[test]
    fn test_get_template_returns_none_for_invalid() {
        let template = get_template("non-existent-template");
        assert!(template.is_none());
    }

    #[test]
    fn test_list_template_names() {
        let names = list_template_names();

        assert_eq!(names.len(), 5);
        assert!(names.contains(&"security-scan".to_string()));
        assert!(names.contains(&"dependency-check".to_string()));
        assert!(names.contains(&"code-quality".to_string()));
        assert!(names.contains(&"documentation-check".to_string()));
        assert!(names.contains(&"database-backup".to_string()));
    }

    #[test]
    fn test_templates_have_valid_cron_expressions() {
        let templates = get_templates();

        for (name, template) in templates.iter() {
            // Just verify the cron string is not empty
            // Actual validation happens when creating schedules
            assert!(!template.cron.is_empty(), "Template {} has empty cron", name);
        }
    }
}
