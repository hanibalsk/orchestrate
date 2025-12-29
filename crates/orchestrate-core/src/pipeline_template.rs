//! Built-in pipeline templates
//!
//! This module provides pre-configured pipeline templates for common workflows.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A pipeline template with pre-configured YAML definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTemplate {
    /// Template name
    pub name: String,
    /// Template description
    pub description: String,
    /// YAML pipeline definition
    pub yaml: String,
}

/// Get all built-in pipeline templates
pub fn get_templates() -> HashMap<String, PipelineTemplate> {
    let templates = HashMap::new();

    templates
        .into_iter()
        .chain(std::iter::once((
            "ci".to_string(),
            PipelineTemplate {
                name: "ci".to_string(),
                description: "Continuous integration pipeline with lint, test, and build stages".to_string(),
                yaml: include_str!("../../../examples/pipelines/ci-pipeline.yaml").to_string(),
            },
        )))
        .chain(std::iter::once((
            "cd".to_string(),
            PipelineTemplate {
                name: "cd".to_string(),
                description: "Continuous deployment pipeline with staging and production deployments".to_string(),
                yaml: include_str!("../../../examples/pipelines/cd-pipeline.yaml").to_string(),
            },
        )))
        .chain(std::iter::once((
            "release".to_string(),
            PipelineTemplate {
                name: "release".to_string(),
                description: "Release pipeline with version bump, changelog generation, and artifact publishing".to_string(),
                yaml: include_str!("../../../examples/pipelines/release-pipeline.yaml").to_string(),
            },
        )))
        .chain(std::iter::once((
            "security".to_string(),
            PipelineTemplate {
                name: "security".to_string(),
                description: "Security pipeline with vulnerability scanning, reporting, and automated fixes".to_string(),
                yaml: SECURITY_PIPELINE_YAML.to_string(),
            },
        )))
        .collect()
}

const SECURITY_PIPELINE_YAML: &str = r#"name: security-pipeline
description: Security scanning and remediation pipeline
version: 1

triggers:
  - event: schedule
    cron: "0 2 * * *"  # Daily at 2 AM
  - event: pull_request.opened
    branches: [main, develop]

stages:
  - name: dependency-scan
    agent: dependency-scanner
    task: Scan dependencies for known vulnerabilities
    timeout: 10m
    on_failure: continue

  - name: code-scan
    agent: code-scanner
    task: Run static analysis security testing (SAST)
    timeout: 15m
    parallel_with: dependency-scan
    on_failure: continue

  - name: secrets-scan
    agent: secrets-scanner
    task: Scan for hardcoded secrets and credentials
    timeout: 5m
    parallel_with: dependency-scan
    on_failure: halt

  - name: generate-report
    agent: security-reporter
    task: Generate comprehensive security report
    timeout: 5m
    depends_on: [dependency-scan, code-scan, secrets-scan]
    on_failure: halt

  - name: auto-fix
    agent: security-fixer
    task: Attempt automated fixes for identified vulnerabilities
    timeout: 20m
    depends_on: [generate-report]
    on_failure: continue
    when:
      labels: ["auto-fix-security"]

  - name: notify
    agent: notifier
    task: Send security report to team channels
    timeout: 5m
    depends_on: [generate-report]
    on_failure: continue
"#;

/// Get a specific template by name
pub fn get_template(name: &str) -> Option<PipelineTemplate> {
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

        // Should have exactly 4 templates
        assert_eq!(templates.len(), 4);

        // Verify all required templates exist
        assert!(templates.contains_key("ci"));
        assert!(templates.contains_key("cd"));
        assert!(templates.contains_key("release"));
        assert!(templates.contains_key("security"));
    }

    #[test]
    fn test_ci_pipeline_template() {
        let template = get_template("ci").expect("ci template should exist");

        assert_eq!(template.name, "ci");
        assert!(!template.description.is_empty());

        // Verify YAML contains expected elements
        assert!(template.yaml.contains("name: ci-pipeline"));
        assert!(template.yaml.contains("lint"));
        assert!(template.yaml.contains("test"));
        assert!(template.yaml.contains("build"));
        assert!(template.yaml.contains("on_failure: halt"));
    }

    #[test]
    fn test_cd_pipeline_template() {
        let template = get_template("cd").expect("cd template should exist");

        assert_eq!(template.name, "cd");
        assert!(!template.description.is_empty());

        // Verify YAML contains expected elements
        assert!(template.yaml.contains("name: cd-pipeline"));
        assert!(template.yaml.contains("deploy-staging"));
        assert!(template.yaml.contains("smoke-test"));
        assert!(template.yaml.contains("deploy-prod"));
        assert!(template.yaml.contains("requires_approval: true"));
    }

    #[test]
    fn test_release_pipeline_template() {
        let template = get_template("release").expect("release template should exist");

        assert_eq!(template.name, "release");
        assert!(!template.description.is_empty());

        // Verify YAML contains expected elements
        assert!(template.yaml.contains("name: release-pipeline"));
        assert!(template.yaml.contains("version"));
        assert!(template.yaml.contains("changelog"));
        assert!(template.yaml.contains("release"));
    }

    #[test]
    fn test_security_pipeline_template() {
        let template = get_template("security").expect("security template should exist");

        assert_eq!(template.name, "security");
        assert!(!template.description.is_empty());

        // Verify YAML contains expected elements
        assert!(template.yaml.contains("name: security-pipeline"));
        assert!(template.yaml.contains("scan"));
        assert!(template.yaml.contains("report"));
        assert!(template.yaml.contains("fix"));
    }

    #[test]
    fn test_get_template_returns_none_for_invalid() {
        let template = get_template("non-existent-template");
        assert!(template.is_none());
    }

    #[test]
    fn test_list_template_names() {
        let names = list_template_names();

        assert_eq!(names.len(), 4);
        assert!(names.contains(&"ci".to_string()));
        assert!(names.contains(&"cd".to_string()));
        assert!(names.contains(&"release".to_string()));
        assert!(names.contains(&"security".to_string()));

        // Verify names are sorted
        let sorted: Vec<_> = names.iter().cloned().collect();
        let mut expected = sorted.clone();
        expected.sort();
        assert_eq!(sorted, expected);
    }

    #[test]
    fn test_templates_have_valid_yaml() {
        let templates = get_templates();

        for (name, template) in templates.iter() {
            // Verify YAML is not empty
            assert!(!template.yaml.is_empty(), "Template {} has empty YAML", name);

            // Verify YAML contains basic required fields
            assert!(template.yaml.contains("name:"), "Template {} missing 'name' field", name);
            assert!(template.yaml.contains("description:"), "Template {} missing 'description' field", name);
            assert!(template.yaml.contains("stages:"), "Template {} missing 'stages' field", name);
        }
    }

    #[test]
    fn test_templates_have_consistent_naming() {
        let templates = get_templates();

        for (key, template) in templates.iter() {
            // Template key should match template.name
            assert_eq!(&template.name, key, "Template key '{}' doesn't match template.name '{}'", key, template.name);
        }
    }
}
