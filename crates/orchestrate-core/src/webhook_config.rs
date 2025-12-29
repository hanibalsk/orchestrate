//! Webhook configuration
//!
//! This module provides configuration for webhook event handling, including
//! event filtering, agent type mapping, and behavior customization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::{AgentType, Error, Result};

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookConfig {
    /// Webhook secret for signature verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,

    /// Event-specific configurations
    #[serde(default)]
    pub events: HashMap<String, EventConfig>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            secret: None,
            events: HashMap::new(),
        }
    }
}

impl WebhookConfig {
    /// Load configuration from a YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Other(format!("Failed to read config file: {}", e)))?;
        Self::from_yaml_str(&content)
    }

    /// Parse configuration from a YAML string
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        // Substitute environment variables
        let yaml = substitute_env_vars(yaml);

        let config: WebhookConfigFile = serde_yaml::from_str(&yaml)
            .map_err(|e| Error::Other(format!("Failed to parse YAML config: {}", e)))?;

        Ok(config.webhooks.unwrap_or_default())
    }

    /// Check if an event should be handled
    pub fn should_handle_event(&self, event_type: &str) -> bool {
        self.events.contains_key(event_type)
    }

    /// Get the agent type for a specific event
    pub fn get_agent_type(&self, event_type: &str) -> Option<AgentType> {
        self.events.get(event_type).and_then(|e| e.agent.clone())
    }

    /// Get the filter for a specific event
    pub fn get_filter(&self, event_type: &str) -> Option<&EventFilter> {
        self.events.get(event_type).and_then(|e| e.filter.as_ref())
    }
}

/// Wrapper for the YAML file structure (supports top-level "webhooks" key)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WebhookConfigFile {
    webhooks: Option<WebhookConfig>,
}

/// Configuration for a specific event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventConfig {
    /// Agent type to spawn for this event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentType>,

    /// Event filter rules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<EventFilter>,
}

/// Event filtering rules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EventFilter {
    /// Filter by base branch (for PR events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<Vec<String>>,

    /// Skip events from forks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_forks: Option<bool>,

    /// Filter by CI conclusion (for check events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conclusion: Option<Vec<String>>,

    /// Filter by issue/PR labels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,

    /// Filter by author username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Vec<String>>,

    /// Filter by changed file paths (regex patterns)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
}

impl EventFilter {
    /// Check if the filter allows processing based on branch
    pub fn allows_branch(&self, branch: &str) -> bool {
        if let Some(allowed_branches) = &self.base_branch {
            allowed_branches.contains(&branch.to_string())
        } else {
            true // No filter = allow all
        }
    }

    /// Check if the filter allows processing based on fork status
    pub fn allows_fork(&self, is_fork: bool) -> bool {
        if let Some(skip_forks) = self.skip_forks {
            if skip_forks && is_fork {
                return false;
            }
        }
        true
    }

    /// Check if the filter allows processing based on conclusion
    pub fn allows_conclusion(&self, conclusion: &str) -> bool {
        if let Some(allowed_conclusions) = &self.conclusion {
            allowed_conclusions.contains(&conclusion.to_string())
        } else {
            true
        }
    }

    /// Check if the filter allows processing based on labels
    pub fn allows_labels(&self, labels: &[String]) -> bool {
        if let Some(required_labels) = &self.labels {
            // At least one required label must be present
            required_labels.iter().any(|req| labels.contains(req))
        } else {
            true
        }
    }

    /// Check if the filter allows processing based on author
    pub fn allows_author(&self, author: &str) -> bool {
        if let Some(allowed_authors) = &self.author {
            allowed_authors.contains(&author.to_string())
        } else {
            true
        }
    }

    /// Check if the filter allows processing based on changed paths
    pub fn allows_paths(&self, paths: &[String]) -> bool {
        if let Some(path_patterns) = &self.paths {
            // At least one path must match one of the patterns
            paths.iter().any(|path| {
                path_patterns.iter().any(|pattern| {
                    // Simple glob-like matching (could use regex crate for more complex patterns)
                    path.contains(pattern)
                })
            })
        } else {
            true
        }
    }
}

/// Substitute environment variables in the YAML string
/// Supports ${VAR_NAME} syntax
fn substitute_env_vars(yaml: &str) -> String {
    let mut result = yaml.to_string();

    // Match ${VAR_NAME} patterns
    let re = regex::Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();

    for cap in re.captures_iter(yaml) {
        let var_name = &cap[1];
        if let Ok(var_value) = std::env::var(var_name) {
            let placeholder = format!("${{{}}}", var_name);
            result = result.replace(&placeholder, &var_value);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_default() {
        let config = WebhookConfig::default();
        assert!(config.secret.is_none());
        assert!(config.events.is_empty());
    }

    #[test]
    fn test_webhook_config_from_yaml_basic() {
        let yaml = r#"
webhooks:
  secret: my-secret-key
  events:
    pull_request.opened:
      agent: pr_shepherd
    check_run.completed:
      agent: issue_fixer
"#;

        let config = WebhookConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.secret, Some("my-secret-key".to_string()));
        assert_eq!(config.events.len(), 2);

        let pr_config = config.events.get("pull_request.opened").unwrap();
        assert_eq!(pr_config.agent, Some(AgentType::PrShepherd));

        let check_config = config.events.get("check_run.completed").unwrap();
        assert_eq!(check_config.agent, Some(AgentType::IssueFixer));
    }

    #[test]
    fn test_webhook_config_with_filters() {
        let yaml = r#"
webhooks:
  events:
    pull_request.opened:
      agent: pr_shepherd
      filter:
        base_branch: [main, develop]
        skip_forks: true
    check_run.completed:
      agent: issue_fixer
      filter:
        conclusion: [failure, timed_out]
"#;

        let config = WebhookConfig::from_yaml_str(yaml).unwrap();

        let pr_config = config.events.get("pull_request.opened").unwrap();
        assert!(pr_config.filter.is_some());
        let pr_filter = pr_config.filter.as_ref().unwrap();
        assert_eq!(pr_filter.base_branch, Some(vec!["main".to_string(), "develop".to_string()]));
        assert_eq!(pr_filter.skip_forks, Some(true));

        let check_config = config.events.get("check_run.completed").unwrap();
        let check_filter = check_config.filter.as_ref().unwrap();
        assert_eq!(check_filter.conclusion, Some(vec!["failure".to_string(), "timed_out".to_string()]));
    }

    #[test]
    fn test_webhook_config_env_var_substitution() {
        std::env::set_var("TEST_WEBHOOK_SECRET", "secret-from-env");

        let yaml = r#"
webhooks:
  secret: ${TEST_WEBHOOK_SECRET}
  events: {}
"#;

        let config = WebhookConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.secret, Some("secret-from-env".to_string()));

        std::env::remove_var("TEST_WEBHOOK_SECRET");
    }

    #[test]
    fn test_webhook_config_missing_env_var() {
        // Ensure the var doesn't exist
        std::env::remove_var("NONEXISTENT_VAR");

        let yaml = r#"
webhooks:
  secret: ${NONEXISTENT_VAR}
  events: {}
"#;

        let config = WebhookConfig::from_yaml_str(yaml).unwrap();
        // Should keep the placeholder if var doesn't exist
        assert_eq!(config.secret, Some("${NONEXISTENT_VAR}".to_string()));
    }

    #[test]
    fn test_should_handle_event() {
        let yaml = r#"
webhooks:
  events:
    pull_request.opened:
      agent: pr_shepherd
"#;

        let config = WebhookConfig::from_yaml_str(yaml).unwrap();
        assert!(config.should_handle_event("pull_request.opened"));
        assert!(!config.should_handle_event("push"));
    }

    #[test]
    fn test_get_agent_type() {
        let yaml = r#"
webhooks:
  events:
    pull_request.opened:
      agent: pr_shepherd
    check_run.completed:
      agent: issue_fixer
"#;

        let config = WebhookConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(
            config.get_agent_type("pull_request.opened"),
            Some(AgentType::PrShepherd)
        );
        assert_eq!(
            config.get_agent_type("check_run.completed"),
            Some(AgentType::IssueFixer)
        );
        assert_eq!(config.get_agent_type("unknown"), None);
    }

    #[test]
    fn test_event_filter_allows_branch() {
        let filter = EventFilter {
            base_branch: Some(vec!["main".to_string(), "develop".to_string()]),
            ..Default::default()
        };

        assert!(filter.allows_branch("main"));
        assert!(filter.allows_branch("develop"));
        assert!(!filter.allows_branch("feature/test"));
    }

    #[test]
    fn test_event_filter_allows_branch_no_filter() {
        let filter = EventFilter::default();
        assert!(filter.allows_branch("any-branch"));
    }

    #[test]
    fn test_event_filter_allows_fork() {
        let filter = EventFilter {
            skip_forks: Some(true),
            ..Default::default()
        };

        assert!(filter.allows_fork(false));
        assert!(!filter.allows_fork(true));
    }

    #[test]
    fn test_event_filter_allows_fork_no_filter() {
        let filter = EventFilter::default();
        assert!(filter.allows_fork(true));
        assert!(filter.allows_fork(false));
    }

    #[test]
    fn test_event_filter_allows_conclusion() {
        let filter = EventFilter {
            conclusion: Some(vec!["failure".to_string(), "timed_out".to_string()]),
            ..Default::default()
        };

        assert!(filter.allows_conclusion("failure"));
        assert!(filter.allows_conclusion("timed_out"));
        assert!(!filter.allows_conclusion("success"));
    }

    #[test]
    fn test_event_filter_allows_labels() {
        let filter = EventFilter {
            labels: Some(vec!["bug".to_string(), "enhancement".to_string()]),
            ..Default::default()
        };

        assert!(filter.allows_labels(&["bug".to_string()]));
        assert!(filter.allows_labels(&["enhancement".to_string(), "priority:high".to_string()]));
        assert!(!filter.allows_labels(&["documentation".to_string()]));
    }

    #[test]
    fn test_event_filter_allows_labels_no_filter() {
        let filter = EventFilter::default();
        assert!(filter.allows_labels(&[]));
        assert!(filter.allows_labels(&["any".to_string()]));
    }

    #[test]
    fn test_event_filter_allows_author() {
        let filter = EventFilter {
            author: Some(vec!["user1".to_string(), "user2".to_string()]),
            ..Default::default()
        };

        assert!(filter.allows_author("user1"));
        assert!(filter.allows_author("user2"));
        assert!(!filter.allows_author("user3"));
    }

    #[test]
    fn test_event_filter_allows_paths() {
        let filter = EventFilter {
            paths: Some(vec!["src/".to_string(), "lib/".to_string()]),
            ..Default::default()
        };

        assert!(filter.allows_paths(&["src/main.rs".to_string()]));
        assert!(filter.allows_paths(&["lib/utils.rs".to_string()]));
        assert!(filter.allows_paths(&["README.md".to_string(), "src/lib.rs".to_string()]));
        assert!(!filter.allows_paths(&["README.md".to_string()]));
    }

    #[test]
    fn test_event_filter_allows_paths_no_filter() {
        let filter = EventFilter::default();
        assert!(filter.allows_paths(&[]));
        assert!(filter.allows_paths(&["any/path".to_string()]));
    }

    #[test]
    fn test_complex_filter_all_labels() {
        let yaml = r#"
webhooks:
  events:
    issues.opened:
      agent: issue_triager
      filter:
        labels: [bug, security]
        author: [security-team, admin]
"#;

        let config = WebhookConfig::from_yaml_str(yaml).unwrap();
        let filter = config.get_filter("issues.opened").unwrap();

        // Must have at least one required label
        assert!(filter.allows_labels(&["bug".to_string()]));
        assert!(filter.allows_labels(&["security".to_string(), "priority:high".to_string()]));
        assert!(!filter.allows_labels(&["enhancement".to_string()]));

        // Must be from allowed author
        assert!(filter.allows_author("admin"));
        assert!(!filter.allows_author("random-user"));
    }

    #[test]
    fn test_substitute_env_vars() {
        std::env::set_var("TEST_VAR_1", "value1");
        std::env::set_var("TEST_VAR_2", "value2");

        let input = "secret: ${TEST_VAR_1}, token: ${TEST_VAR_2}";
        let result = substitute_env_vars(input);
        assert_eq!(result, "secret: value1, token: value2");

        std::env::remove_var("TEST_VAR_1");
        std::env::remove_var("TEST_VAR_2");
    }

    #[test]
    fn test_substitute_env_vars_partial() {
        std::env::set_var("EXISTS", "found");
        std::env::remove_var("NOT_EXISTS");

        let input = "a: ${EXISTS}, b: ${NOT_EXISTS}";
        let result = substitute_env_vars(input);
        assert_eq!(result, "a: found, b: ${NOT_EXISTS}");

        std::env::remove_var("EXISTS");
    }

    #[test]
    fn test_webhook_config_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml = r#"
webhooks:
  secret: test-secret
  events:
    pull_request.opened:
      agent: pr_shepherd
      filter:
        base_branch: [main]
        skip_forks: true
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config = WebhookConfig::from_yaml_file(temp_file.path()).unwrap();
        assert_eq!(config.secret, Some("test-secret".to_string()));
        assert_eq!(config.events.len(), 1);

        let pr_config = config.events.get("pull_request.opened").unwrap();
        assert_eq!(pr_config.agent, Some(AgentType::PrShepherd));

        let filter = pr_config.filter.as_ref().unwrap();
        assert_eq!(filter.base_branch, Some(vec!["main".to_string()]));
        assert_eq!(filter.skip_forks, Some(true));
    }

    #[test]
    fn test_webhook_config_missing_file() {
        let result = WebhookConfig::from_yaml_file("/nonexistent/path/config.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_config_invalid_yaml() {
        let yaml = r#"
webhooks:
  events:
    pull_request.opened:
      agent: invalid_agent_type_that_does_not_exist
"#;

        let result = WebhookConfig::from_yaml_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_config_empty() {
        let yaml = r#"
webhooks:
  events: {}
"#;

        let config = WebhookConfig::from_yaml_str(yaml).unwrap();
        assert!(config.secret.is_none());
        assert!(config.events.is_empty());
    }
}
