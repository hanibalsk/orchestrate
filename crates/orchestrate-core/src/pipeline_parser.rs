//! Pipeline YAML parser
//!
//! This module provides parsing and validation for pipeline definitions
//! from YAML format.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::{Error, Result};

/// Pipeline definition parsed from YAML
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineDefinition {
    /// Pipeline name
    pub name: String,
    /// Pipeline description
    pub description: String,
    /// Pipeline version
    #[serde(default = "default_version")]
    pub version: u32,
    /// Trigger definitions
    #[serde(default)]
    pub triggers: Vec<TriggerDefinition>,
    /// Pipeline variables
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// Stage definitions
    pub stages: Vec<StageDefinition>,
}

fn default_version() -> u32 {
    1
}

/// Trigger definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerDefinition {
    /// Event type (e.g., "pull_request.merged")
    pub event: String,
    /// Branches to trigger on
    #[serde(default)]
    pub branches: Vec<String>,
}

/// Stage definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageDefinition {
    /// Stage name
    pub name: String,
    /// Agent to execute this stage
    pub agent: String,
    /// Task description
    pub task: String,
    /// Timeout duration (e.g., "30m", "1h")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    /// Action on failure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<FailureAction>,
    /// Rollback target stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollback_to: Option<String>,
    /// Requires human approval
    #[serde(default)]
    pub requires_approval: bool,
    /// List of approvers
    #[serde(default)]
    pub approvers: Vec<String>,
    /// Environment for this stage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    /// Dependencies (stages that must complete first)
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Stage to run in parallel with
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_with: Option<String>,
    /// Conditional execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<StageCondition>,
}

/// Action to take on stage failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailureAction {
    /// Halt pipeline execution
    Halt,
    /// Continue to next stage
    Continue,
    /// Rollback to a previous stage
    Rollback,
}

/// Stage execution condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageCondition {
    /// Branch conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<Vec<String>>,
    /// Path conditions (glob patterns)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
    /// Label conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// Variable conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<HashMap<String, String>>,
    /// OR condition (alternative conditions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub or: Option<Box<StageCondition>>,
}

impl PipelineDefinition {
    /// Parse pipeline from YAML string
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let definition: PipelineDefinition = serde_yaml::from_str(yaml)
            .map_err(|e| Error::Other(format!("Failed to parse pipeline YAML: {}", e)))?;

        // Validate the pipeline structure
        definition.validate()?;

        Ok(definition)
    }

    /// Parse pipeline from YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Other(format!("Failed to read pipeline file: {}", e)))?;
        Self::from_yaml_str(&content)
    }

    /// Validate pipeline structure
    pub fn validate(&self) -> Result<()> {
        // Validate name is not empty
        if self.name.is_empty() {
            return Err(Error::Other("Pipeline name cannot be empty".to_string()));
        }

        // Validate stages exist
        if self.stages.is_empty() {
            return Err(Error::Other("Pipeline must have at least one stage".to_string()));
        }

        // Collect stage names for dependency validation
        let stage_names: HashSet<_> = self.stages.iter().map(|s| s.name.as_str()).collect();

        // Validate each stage
        for stage in &self.stages {
            self.validate_stage(stage, &stage_names)?;
        }

        // Validate no circular dependencies
        self.validate_no_cycles()?;

        Ok(())
    }

    /// Validate a single stage
    fn validate_stage(
        &self,
        stage: &StageDefinition,
        all_stage_names: &HashSet<&str>,
    ) -> Result<()> {
        // Validate stage name is not empty
        if stage.name.is_empty() {
            return Err(Error::Other("Stage name cannot be empty".to_string()));
        }

        // Validate agent is not empty
        if stage.agent.is_empty() {
            return Err(Error::Other(format!(
                "Stage '{}' must specify an agent",
                stage.name
            )));
        }

        // Validate task is not empty
        if stage.task.is_empty() {
            return Err(Error::Other(format!(
                "Stage '{}' must specify a task",
                stage.name
            )));
        }

        // Validate dependencies exist
        for dep in &stage.depends_on {
            if !all_stage_names.contains(dep.as_str()) {
                return Err(Error::Other(format!(
                    "Stage '{}' depends on non-existent stage '{}'",
                    stage.name, dep
                )));
            }
        }

        // Validate parallel_with exists
        if let Some(parallel) = &stage.parallel_with {
            if !all_stage_names.contains(parallel.as_str()) {
                return Err(Error::Other(format!(
                    "Stage '{}' parallel_with non-existent stage '{}'",
                    stage.name, parallel
                )));
            }
        }

        // Validate rollback_to exists and on_failure is rollback
        if let Some(rollback_to) = &stage.rollback_to {
            if stage.on_failure != Some(FailureAction::Rollback) {
                return Err(Error::Other(format!(
                    "Stage '{}' has rollback_to but on_failure is not 'rollback'",
                    stage.name
                )));
            }
            if !all_stage_names.contains(rollback_to.as_str()) {
                return Err(Error::Other(format!(
                    "Stage '{}' rollback_to non-existent stage '{}'",
                    stage.name, rollback_to
                )));
            }
            // Prevent rollback to itself
            if rollback_to == &stage.name {
                return Err(Error::Other(format!(
                    "Stage '{}' cannot rollback to itself - this would create a rollback loop",
                    stage.name
                )));
            }
        }

        // Validate approvers when requires_approval is true
        if stage.requires_approval && stage.approvers.is_empty() {
            return Err(Error::Other(format!(
                "Stage '{}' requires approval but has no approvers",
                stage.name
            )));
        }

        Ok(())
    }

    /// Validate no circular dependencies
    fn validate_no_cycles(&self) -> Result<()> {
        // Build adjacency list
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
        for stage in &self.stages {
            let deps: Vec<&str> = stage.depends_on.iter().map(|s| s.as_str()).collect();
            graph.insert(&stage.name, deps);
        }

        // DFS to detect cycles
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for stage in &self.stages {
            if !visited.contains(stage.name.as_str()) {
                if self.has_cycle(&graph, stage.name.as_str(), &mut visited, &mut rec_stack) {
                    return Err(Error::Other(format!(
                        "Circular dependency detected involving stage '{}'",
                        stage.name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Check for cycle in dependency graph using DFS
    fn has_cycle<'a>(
        &self,
        graph: &HashMap<&'a str, Vec<&'a str>>,
        node: &'a str,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = graph.get(node) {
            for &neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle(graph, neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Convert pipeline definition to YAML string
    pub fn to_yaml_string(&self) -> Result<String> {
        serde_yaml::to_string(self)
            .map_err(|e| Error::Other(format!("Failed to serialize pipeline: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_pipeline() {
        let yaml = r#"
name: test-pipeline
description: A test pipeline
version: 1
stages:
  - name: build
    agent: builder
    task: Build the project
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();
        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.description, "A test pipeline");
        assert_eq!(pipeline.version, 1);
        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].name, "build");
        assert_eq!(pipeline.stages[0].agent, "builder");
        assert_eq!(pipeline.stages[0].task, "Build the project");
    }

    #[test]
    fn test_parse_pipeline_with_triggers() {
        let yaml = r#"
name: ci-pipeline
description: CI pipeline
triggers:
  - event: pull_request.opened
    branches: [main, develop]
  - event: push
    branches: [main]
stages:
  - name: test
    agent: tester
    task: Run tests
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();
        assert_eq!(pipeline.triggers.len(), 2);
        assert_eq!(pipeline.triggers[0].event, "pull_request.opened");
        assert_eq!(pipeline.triggers[0].branches, vec!["main", "develop"]);
        assert_eq!(pipeline.triggers[1].event, "push");
        assert_eq!(pipeline.triggers[1].branches, vec!["main"]);
    }

    #[test]
    fn test_parse_pipeline_with_variables() {
        let yaml = r#"
name: deploy-pipeline
description: Deployment pipeline
variables:
  environment: staging
  region: us-west-2
stages:
  - name: deploy
    agent: deployer
    task: Deploy to ${environment}
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();
        assert_eq!(pipeline.variables.len(), 2);
        assert_eq!(pipeline.variables.get("environment"), Some(&"staging".to_string()));
        assert_eq!(pipeline.variables.get("region"), Some(&"us-west-2".to_string()));
    }

    #[test]
    fn test_parse_stage_with_failure_actions() {
        let yaml = r#"
name: test-pipeline
description: Test pipeline
stages:
  - name: validate
    agent: validator
    task: Validate
    on_failure: halt
  - name: build
    agent: builder
    task: Build
    on_failure: continue
  - name: deploy
    agent: deployer
    task: Deploy
    on_failure: rollback
    rollback_to: build
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();
        assert_eq!(pipeline.stages[0].on_failure, Some(FailureAction::Halt));
        assert_eq!(pipeline.stages[1].on_failure, Some(FailureAction::Continue));
        assert_eq!(pipeline.stages[2].on_failure, Some(FailureAction::Rollback));
        assert_eq!(pipeline.stages[2].rollback_to, Some("build".to_string()));
    }

    #[test]
    fn test_parse_stage_with_approval() {
        let yaml = r#"
name: prod-pipeline
description: Production pipeline
stages:
  - name: deploy
    agent: deployer
    task: Deploy to production
    requires_approval: true
    approvers: [team-lead, devops]
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();
        assert!(pipeline.stages[0].requires_approval);
        assert_eq!(pipeline.stages[0].approvers, vec!["team-lead", "devops"]);
    }

    #[test]
    fn test_parse_stage_with_dependencies() {
        let yaml = r#"
name: complex-pipeline
description: Complex pipeline
stages:
  - name: build
    agent: builder
    task: Build
  - name: test
    agent: tester
    task: Test
    depends_on: [build]
  - name: deploy
    agent: deployer
    task: Deploy
    depends_on: [test]
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();
        assert_eq!(pipeline.stages[1].depends_on, vec!["build"]);
        assert_eq!(pipeline.stages[2].depends_on, vec!["test"]);
    }

    #[test]
    fn test_parse_stage_with_parallel() {
        let yaml = r#"
name: parallel-pipeline
description: Pipeline with parallel stages
stages:
  - name: validate
    agent: validator
    task: Validate
  - name: security-scan
    agent: security-scanner
    task: Security scan
    parallel_with: validate
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();
        assert_eq!(pipeline.stages[1].parallel_with, Some("validate".to_string()));
    }

    #[test]
    fn test_parse_stage_with_conditions() {
        let yaml = r#"
name: conditional-pipeline
description: Pipeline with conditions
stages:
  - name: deploy-docs
    agent: doc-deployer
    task: Deploy documentation
    when:
      paths: ["docs/**", "README.md"]
  - name: full-test
    agent: tester
    task: Run full tests
    when:
      labels: ["needs-full-test"]
      or:
        paths: ["src/core/**"]
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

        let when1 = pipeline.stages[0].when.as_ref().unwrap();
        assert_eq!(when1.paths, Some(vec!["docs/**".to_string(), "README.md".to_string()]));

        let when2 = pipeline.stages[1].when.as_ref().unwrap();
        assert_eq!(when2.labels, Some(vec!["needs-full-test".to_string()]));
        assert!(when2.or.is_some());
    }

    #[test]
    fn test_parse_complete_example_pipeline() {
        let yaml = r#"
name: feature-complete
description: Full deployment pipeline after feature merge
version: 1

triggers:
  - event: pull_request.merged
    branches: [main]

variables:
  environment: staging

stages:
  - name: validate
    agent: regression-tester
    task: "Run full test suite"
    timeout: 30m
    on_failure: halt

  - name: security-scan
    agent: security-scanner
    task: "Run security analysis"
    parallel_with: validate
    on_failure: halt

  - name: deploy-staging
    agent: deployer
    task: "Deploy to ${environment}"
    environment: staging
    depends_on: [validate, security-scan]

  - name: smoke-test
    agent: smoke-tester
    task: "Run smoke tests on staging"
    depends_on: [deploy-staging]
    on_failure: rollback
    rollback_to: deploy-staging

  - name: deploy-prod
    agent: deployer
    task: "Deploy to production"
    environment: production
    requires_approval: true
    approvers: [team-lead, devops]
    depends_on: [smoke-test]
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

        assert_eq!(pipeline.name, "feature-complete");
        assert_eq!(pipeline.description, "Full deployment pipeline after feature merge");
        assert_eq!(pipeline.version, 1);
        assert_eq!(pipeline.triggers.len(), 1);
        assert_eq!(pipeline.variables.len(), 1);
        assert_eq!(pipeline.stages.len(), 5);

        // Validate stage
        assert_eq!(pipeline.stages[0].name, "validate");
        assert_eq!(pipeline.stages[0].timeout, Some("30m".to_string()));
        assert_eq!(pipeline.stages[0].on_failure, Some(FailureAction::Halt));

        // Security scan (parallel)
        assert_eq!(pipeline.stages[1].parallel_with, Some("validate".to_string()));

        // Deploy staging (multiple dependencies)
        assert_eq!(pipeline.stages[2].depends_on, vec!["validate", "security-scan"]);
        assert_eq!(pipeline.stages[2].environment, Some("staging".to_string()));

        // Smoke test (with rollback)
        assert_eq!(pipeline.stages[3].on_failure, Some(FailureAction::Rollback));
        assert_eq!(pipeline.stages[3].rollback_to, Some("deploy-staging".to_string()));

        // Deploy prod (with approval)
        assert!(pipeline.stages[4].requires_approval);
        assert_eq!(pipeline.stages[4].approvers, vec!["team-lead", "devops"]);
    }

    #[test]
    fn test_validation_empty_name() {
        let yaml = r#"
name: ""
description: Test
stages:
  - name: test
    agent: tester
    task: Test
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name cannot be empty"));
    }

    #[test]
    fn test_validation_no_stages() {
        let yaml = r#"
name: test-pipeline
description: Test
stages: []
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one stage"));
    }

    #[test]
    fn test_validation_empty_stage_name() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: ""
    agent: tester
    task: Test
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Stage name cannot be empty"));
    }

    #[test]
    fn test_validation_empty_agent() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: test
    agent: ""
    task: Test
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must specify an agent"));
    }

    #[test]
    fn test_validation_empty_task() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: test
    agent: tester
    task: ""
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must specify a task"));
    }

    #[test]
    fn test_validation_invalid_dependency() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: test
    agent: tester
    task: Test
    depends_on: [nonexistent]
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-existent stage"));
    }

    #[test]
    fn test_validation_invalid_parallel_with() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: test
    agent: tester
    task: Test
    parallel_with: nonexistent
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("non-existent stage"));
    }

    #[test]
    fn test_validation_rollback_without_failure_action() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: build
    agent: builder
    task: Build
  - name: deploy
    agent: deployer
    task: Deploy
    rollback_to: build
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("on_failure is not 'rollback'"));
    }

    #[test]
    fn test_validation_invalid_rollback_target() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: deploy
    agent: deployer
    task: Deploy
    on_failure: rollback
    rollback_to: nonexistent
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rollback_to non-existent stage"));
    }

    #[test]
    fn test_validation_rollback_to_self() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: deploy
    agent: deployer
    task: Deploy
    on_failure: rollback
    rollback_to: deploy
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rollback to itself"));
    }

    #[test]
    fn test_validation_approval_without_approvers() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: deploy
    agent: deployer
    task: Deploy
    requires_approval: true
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires approval but has no approvers"));
    }

    #[test]
    fn test_validation_circular_dependency() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: stage1
    agent: agent1
    task: Task 1
    depends_on: [stage2]
  - name: stage2
    agent: agent2
    task: Task 2
    depends_on: [stage1]
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
    }

    #[test]
    fn test_validation_complex_circular_dependency() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: stage1
    agent: agent1
    task: Task 1
    depends_on: [stage3]
  - name: stage2
    agent: agent2
    task: Task 2
    depends_on: [stage1]
  - name: stage3
    agent: agent3
    task: Task 3
    depends_on: [stage2]
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
    }

    #[test]
    fn test_to_yaml_string() {
        let pipeline = PipelineDefinition {
            name: "test-pipeline".to_string(),
            description: "A test pipeline".to_string(),
            version: 1,
            triggers: vec![],
            variables: HashMap::new(),
            stages: vec![
                StageDefinition {
                    name: "build".to_string(),
                    agent: "builder".to_string(),
                    task: "Build the project".to_string(),
                    timeout: None,
                    on_failure: None,
                    rollback_to: None,
                    requires_approval: false,
                    approvers: vec![],
                    environment: None,
                    depends_on: vec![],
                    parallel_with: None,
                    when: None,
                },
            ],
        };

        let yaml = pipeline.to_yaml_string().unwrap();
        assert!(yaml.contains("name: test-pipeline"));
        assert!(yaml.contains("description: A test pipeline"));
        assert!(yaml.contains("- name: build"));
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original_yaml = r#"
name: test-pipeline
description: A test pipeline
version: 1
stages:
  - name: build
    agent: builder
    task: Build the project
"#;

        let pipeline = PipelineDefinition::from_yaml_str(original_yaml).unwrap();
        let serialized = pipeline.to_yaml_string().unwrap();
        let deserialized = PipelineDefinition::from_yaml_str(&serialized).unwrap();

        assert_eq!(pipeline, deserialized);
    }

    #[test]
    fn test_default_version() {
        let yaml = r#"
name: test-pipeline
description: Test
stages:
  - name: build
    agent: builder
    task: Build
"#;

        let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();
        assert_eq!(pipeline.version, 1);
    }

    #[test]
    fn test_parse_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml = r#"
name: file-pipeline
description: Pipeline from file
stages:
  - name: test
    agent: tester
    task: Run tests
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let pipeline = PipelineDefinition::from_yaml_file(temp_file.path()).unwrap();
        assert_eq!(pipeline.name, "file-pipeline");
    }

    #[test]
    fn test_parse_from_nonexistent_file() {
        let result = PipelineDefinition::from_yaml_file("/nonexistent/path.yaml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read pipeline file"));
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let yaml = r#"
name: test
description: Test
stages:
  - name: test
    agent: tester
    this is not valid yaml: [
"#;

        let result = PipelineDefinition::from_yaml_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse pipeline YAML"));
    }
}
