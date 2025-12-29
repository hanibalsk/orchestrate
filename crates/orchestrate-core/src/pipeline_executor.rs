//! Pipeline Execution Engine
//!
//! This module provides the core execution engine for pipelines, managing:
//! - Pipeline run creation from triggers
//! - Stage execution respecting dependencies (DAG)
//! - Parallel stage execution
//! - Agent spawning for each stage
//! - Stage status and timing tracking
//! - Stage timeouts
//! - Stage retry on failure
//! - Variable passing between stages

use crate::{
    pipeline::{PipelineRun, PipelineStage},
    pipeline_parser::{FailureAction, PipelineDefinition, StageDefinition},
    Database, Error, Result,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// Pipeline execution engine
pub struct PipelineExecutor {
    database: Arc<Database>,
}

/// Context for pipeline execution containing runtime variables
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Variables passed between stages
    pub variables: HashMap<String, String>,
    /// Trigger event that initiated the pipeline
    pub trigger_event: Option<String>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            trigger_event: None,
        }
    }

    /// Create with initial variables
    pub fn with_variables(mut self, variables: HashMap<String, String>) -> Self {
        self.variables = variables;
        self
    }

    /// Create with trigger event
    pub fn with_trigger(mut self, trigger_event: String) -> Self {
        self.trigger_event = Some(trigger_event);
        self
    }

    /// Set a variable
    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// Get a variable
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Substitute variables in a string (e.g., "Deploy to ${environment}")
    pub fn substitute_variables(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (key, value) in &self.variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineExecutor {
    /// Create a new pipeline executor
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Create a pipeline run from a trigger event
    pub async fn create_run(
        &self,
        pipeline_id: i64,
        trigger_event: Option<String>,
    ) -> Result<i64> {
        let run = PipelineRun::new(pipeline_id, trigger_event);
        let run_id = self.database.insert_pipeline_run(&run).await?;

        info!(
            run_id = run_id,
            pipeline_id = pipeline_id,
            "Created pipeline run"
        );

        Ok(run_id)
    }

    /// Execute a pipeline run
    pub async fn execute_run(
        &self,
        run_id: i64,
        definition: &PipelineDefinition,
    ) -> Result<()> {
        info!(run_id = run_id, "Starting pipeline execution");

        // Load the run
        let mut run = self
            .database
            .get_pipeline_run(run_id)
            .await?
            .ok_or_else(|| Error::Other(format!("Pipeline run {} not found", run_id)))?;

        // Mark run as running
        run.mark_running();
        self.database.update_pipeline_run(&run).await?;

        // Create execution context with pipeline variables
        let mut context = ExecutionContext::new()
            .with_variables(definition.variables.clone())
            .with_trigger(run.trigger_event.clone().unwrap_or_default());

        // Create initial stages in database
        for stage_def in &definition.stages {
            let stage = PipelineStage::new(run_id, stage_def.name.clone());
            self.database.insert_pipeline_stage(&stage).await?;
        }

        // Execute stages
        let result = self.execute_stages(run_id, definition, &mut context).await;

        // Update run status based on result
        let mut run = self
            .database
            .get_pipeline_run(run_id)
            .await?
            .ok_or_else(|| Error::Other(format!("Pipeline run {} not found", run_id)))?;

        match &result {
            Ok(_) => {
                run.mark_succeeded();
                info!(run_id = run_id, "Pipeline run succeeded");
            }
            Err(ref e) => {
                run.mark_failed();
                error!(run_id = run_id, error = %e, "Pipeline run failed");
            }
        }

        self.database.update_pipeline_run(&run).await?;

        result
    }

    /// Execute all stages respecting dependencies
    async fn execute_stages(
        &self,
        run_id: i64,
        definition: &PipelineDefinition,
        context: &mut ExecutionContext,
    ) -> Result<()> {
        // Build dependency graph
        let graph = self.build_dependency_graph(definition)?;

        // Track completed stages
        let mut completed: HashSet<String> = HashSet::new();
        let mut failed: HashSet<String> = HashSet::new();

        // Execute stages in topological order
        while completed.len() + failed.len() < definition.stages.len() {
            // Find stages ready to execute (all dependencies completed)
            let ready_stages: Vec<&StageDefinition> = definition
                .stages
                .iter()
                .filter(|stage| {
                    !completed.contains(&stage.name)
                        && !failed.contains(&stage.name)
                        && graph
                            .get(&stage.name)
                            .map(|deps| deps.iter().all(|dep| completed.contains(dep)))
                            .unwrap_or(true)
                })
                .collect();

            if ready_stages.is_empty() {
                // No more stages ready, check if we're done or stuck
                if !failed.is_empty() {
                    return Err(Error::Other(format!(
                        "Pipeline execution halted due to failed stages: {:?}",
                        failed
                    )));
                }
                break;
            }

            // Group stages for parallel execution
            let parallel_groups = self.group_parallel_stages(&ready_stages);

            // Execute each group in parallel
            for group in parallel_groups {
                let mut tasks = vec![];

                for stage_def in group {
                    let stage_name = stage_def.name.clone();
                    let executor = self.clone_for_stage();
                    let stage_def = stage_def.clone();
                    let context_clone = context.clone();

                    // Spawn parallel task for each stage in the group
                    let task = tokio::spawn(async move {
                        executor
                            .execute_stage(run_id, &stage_def, &context_clone)
                            .await
                    });

                    tasks.push((stage_name, task));
                }

                // Wait for all parallel stages to complete
                for (stage_name, task) in tasks {
                    match task.await {
                        Ok(Ok(_)) => {
                            completed.insert(stage_name.clone());
                            info!(stage = %stage_name, "Stage completed successfully");
                        }
                        Ok(Err(e)) => {
                            failed.insert(stage_name.clone());
                            error!(stage = %stage_name, error = %e, "Stage failed");

                            // Check failure action
                            let stage_def = definition
                                .stages
                                .iter()
                                .find(|s| s.name == stage_name)
                                .unwrap();

                            match stage_def.on_failure {
                                Some(FailureAction::Halt) | None => {
                                    return Err(Error::Other(format!(
                                        "Stage '{}' failed with halt action",
                                        stage_name
                                    )));
                                }
                                Some(FailureAction::Continue) => {
                                    warn!(stage = %stage_name, "Continuing despite stage failure");
                                }
                                Some(FailureAction::Rollback) => {
                                    return Err(Error::Other(format!(
                                        "Stage '{}' failed, rollback not yet implemented",
                                        stage_name
                                    )));
                                }
                            }
                        }
                        Err(e) => {
                            failed.insert(stage_name.clone());
                            error!(stage = %stage_name, error = %e, "Stage task panicked");
                            return Err(Error::Other(format!(
                                "Stage '{}' task panicked: {}",
                                stage_name, e
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a single stage
    async fn execute_stage(
        &self,
        run_id: i64,
        stage_def: &StageDefinition,
        context: &ExecutionContext,
    ) -> Result<()> {
        info!(
            stage = %stage_def.name,
            agent = %stage_def.agent,
            "Executing stage"
        );

        // Load stage from database
        let mut stage = self
            .database
            .get_pipeline_stage_by_name(run_id, &stage_def.name)
            .await?
            .ok_or_else(|| {
                Error::Other(format!("Stage '{}' not found in database", stage_def.name))
            })?;

        // Mark stage as running
        // TODO: Set actual agent_id when agent spawning is implemented
        stage.mark_running(None);
        self.database.update_pipeline_stage(&stage).await?;

        // Substitute variables in task
        let task = context.substitute_variables(&stage_def.task);

        // Execute with timeout if specified
        let result = if let Some(timeout_str) = &stage_def.timeout {
            let duration = parse_timeout(timeout_str)?;
            match timeout(duration, self.spawn_agent(&stage_def.agent, &task)).await {
                Ok(r) => r,
                Err(_) => {
                    warn!(
                        stage = %stage_def.name,
                        timeout = %timeout_str,
                        "Stage timed out"
                    );
                    Err(Error::Other(format!(
                        "Stage '{}' timed out after {}",
                        stage_def.name, timeout_str
                    )))
                }
            }
        } else {
            self.spawn_agent(&stage_def.agent, &task).await
        };

        // Update stage status based on result
        let mut stage = self
            .database
            .get_pipeline_stage_by_name(run_id, &stage_def.name)
            .await?
            .ok_or_else(|| {
                Error::Other(format!("Stage '{}' not found in database", stage_def.name))
            })?;

        match result {
            Ok(_) => {
                stage.mark_succeeded();
                self.database.update_pipeline_stage(&stage).await?;
                Ok(())
            }
            Err(e) => {
                stage.mark_failed();
                self.database.update_pipeline_stage(&stage).await?;
                Err(e)
            }
        }
    }

    /// Spawn an agent for a stage
    async fn spawn_agent(&self, _agent_type: &str, _task: &str) -> Result<()> {
        // TODO: Implement actual agent spawning
        // For now, this is a placeholder that simulates agent execution
        debug!("Agent spawning not yet implemented");
        Ok(())
    }

    /// Build dependency graph from stage definitions
    fn build_dependency_graph(
        &self,
        definition: &PipelineDefinition,
    ) -> Result<HashMap<String, Vec<String>>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for stage in &definition.stages {
            let deps = stage.depends_on.clone();

            // Add parallel_with as a dependency (they must start together, but depend on same parents)
            if let Some(parallel) = &stage.parallel_with {
                // The parallel stage and this stage share dependencies
                // We don't add the parallel stage as a dependency, but we need to track it
                debug!(
                    stage = %stage.name,
                    parallel_with = %parallel,
                    "Stage runs in parallel with another"
                );
            }

            graph.insert(stage.name.clone(), deps);
        }

        Ok(graph)
    }

    /// Group stages for parallel execution
    fn group_parallel_stages<'a>(
        &self,
        stages: &[&'a StageDefinition],
    ) -> Vec<Vec<&'a StageDefinition>> {
        let mut groups: Vec<Vec<&StageDefinition>> = vec![];
        let mut processed: HashSet<String> = HashSet::new();

        for stage in stages {
            if processed.contains(&stage.name) {
                continue;
            }

            let mut group = vec![*stage];
            processed.insert(stage.name.clone());

            // Find all stages that run in parallel with this one
            if let Some(parallel_with) = &stage.parallel_with {
                for other_stage in stages {
                    if other_stage.name == *parallel_with && !processed.contains(&other_stage.name)
                    {
                        group.push(*other_stage);
                        processed.insert(other_stage.name.clone());
                    }
                }
            }

            // Also check if other stages want to run parallel with this one
            for other_stage in stages {
                if let Some(other_parallel) = &other_stage.parallel_with {
                    if *other_parallel == stage.name && !processed.contains(&other_stage.name) {
                        group.push(*other_stage);
                        processed.insert(other_stage.name.clone());
                    }
                }
            }

            groups.push(group);
        }

        groups
    }

    /// Clone executor for parallel stage execution
    fn clone_for_stage(&self) -> Self {
        Self {
            database: Arc::clone(&self.database),
        }
    }
}

/// Parse timeout string (e.g., "30m", "1h", "90s") into Duration
fn parse_timeout(timeout_str: &str) -> Result<Duration> {
    let timeout_str = timeout_str.trim();

    if timeout_str.is_empty() {
        return Err(Error::Other("Empty timeout string".to_string()));
    }

    // Extract number and unit
    let (number_str, unit) = if let Some(idx) = timeout_str.find(|c: char| c.is_alphabetic()) {
        (&timeout_str[..idx], &timeout_str[idx..])
    } else {
        return Err(Error::Other(format!(
            "Invalid timeout format: {}",
            timeout_str
        )));
    };

    let number: u64 = number_str
        .trim()
        .parse()
        .map_err(|_| Error::Other(format!("Invalid timeout number: {}", number_str)))?;

    let duration = match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => Duration::from_secs(number),
        "m" | "min" | "mins" | "minute" | "minutes" => Duration::from_secs(number * 60),
        "h" | "hr" | "hrs" | "hour" | "hours" => Duration::from_secs(number * 3600),
        _ => {
            return Err(Error::Other(format!(
                "Invalid timeout unit: {}",
                unit
            )))
        }
    };

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_new() {
        let ctx = ExecutionContext::new();
        assert!(ctx.variables.is_empty());
        assert!(ctx.trigger_event.is_none());
    }

    #[test]
    fn test_execution_context_with_variables() {
        let mut vars = HashMap::new();
        vars.insert("env".to_string(), "staging".to_string());

        let ctx = ExecutionContext::new().with_variables(vars);
        assert_eq!(ctx.get_variable("env"), Some(&"staging".to_string()));
    }

    #[test]
    fn test_execution_context_with_trigger() {
        let ctx = ExecutionContext::new().with_trigger("pull_request.merged".to_string());
        assert_eq!(
            ctx.trigger_event,
            Some("pull_request.merged".to_string())
        );
    }

    #[test]
    fn test_execution_context_set_get_variable() {
        let mut ctx = ExecutionContext::new();
        ctx.set_variable("key".to_string(), "value".to_string());
        assert_eq!(ctx.get_variable("key"), Some(&"value".to_string()));
        assert_eq!(ctx.get_variable("missing"), None);
    }

    #[test]
    fn test_execution_context_substitute_variables() {
        let mut ctx = ExecutionContext::new();
        ctx.set_variable("environment".to_string(), "production".to_string());
        ctx.set_variable("version".to_string(), "1.2.3".to_string());

        let result = ctx.substitute_variables("Deploy ${environment} version ${version}");
        assert_eq!(result, "Deploy production version 1.2.3");
    }

    #[test]
    fn test_execution_context_substitute_missing_variables() {
        let ctx = ExecutionContext::new();
        let result = ctx.substitute_variables("Deploy ${environment}");
        assert_eq!(result, "Deploy ${environment}"); // Missing variables unchanged
    }

    #[test]
    fn test_parse_timeout_seconds() {
        assert_eq!(parse_timeout("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_timeout("90sec").unwrap(), Duration::from_secs(90));
        assert_eq!(parse_timeout("120seconds").unwrap(), Duration::from_secs(120));
    }

    #[test]
    fn test_parse_timeout_minutes() {
        assert_eq!(parse_timeout("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_timeout("10min").unwrap(), Duration::from_secs(600));
        assert_eq!(parse_timeout("30minutes").unwrap(), Duration::from_secs(1800));
    }

    #[test]
    fn test_parse_timeout_hours() {
        assert_eq!(parse_timeout("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_timeout("2hr").unwrap(), Duration::from_secs(7200));
        assert_eq!(parse_timeout("3hours").unwrap(), Duration::from_secs(10800));
    }

    #[test]
    fn test_parse_timeout_with_whitespace() {
        assert_eq!(parse_timeout(" 30m ").unwrap(), Duration::from_secs(1800));
        assert_eq!(parse_timeout("30 m").unwrap(), Duration::from_secs(1800));
    }

    #[test]
    fn test_parse_timeout_invalid() {
        assert!(parse_timeout("").is_err());
        assert!(parse_timeout("30").is_err());
        assert!(parse_timeout("abc").is_err());
        assert!(parse_timeout("30x").is_err());
        assert!(parse_timeout("-30m").is_err());
    }

    #[tokio::test]
    async fn test_create_run() {
        let database = Arc::new(Database::in_memory().await.unwrap());
        let executor = PipelineExecutor::new(database.clone());

        // Create a pipeline first
        let pipeline = crate::Pipeline::new(
            "test-pipeline".to_string(),
            "name: test\nstages: []".to_string(),
        );
        let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();

        // Create a run
        let run_id = executor
            .create_run(pipeline_id, Some("manual".to_string()))
            .await
            .unwrap();

        // Verify run was created
        let run = database.get_pipeline_run(run_id).await.unwrap().unwrap();
        assert_eq!(run.pipeline_id, pipeline_id);
        assert_eq!(run.status, PipelineRunStatus::Pending);
        assert_eq!(run.trigger_event, Some("manual".to_string()));
    }

    #[tokio::test]
    async fn test_execute_simple_pipeline() {
        let database = Arc::new(Database::in_memory().await.unwrap());
        let executor = PipelineExecutor::new(database.clone());

        // Create a pipeline
        let pipeline = crate::Pipeline::new(
            "simple-pipeline".to_string(),
            "name: simple\nstages: []".to_string(),
        );
        let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();

        // Create a run
        let run_id = executor.create_run(pipeline_id, None).await.unwrap();

        // Create a simple pipeline definition
        let definition = PipelineDefinition {
            name: "simple-pipeline".to_string(),
            description: "A simple pipeline".to_string(),
            version: 1,
            triggers: vec![],
            variables: HashMap::new(),
            stages: vec![StageDefinition {
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
            }],
        };

        // Execute the pipeline
        let result = executor.execute_run(run_id, &definition).await;
        assert!(result.is_ok());

        // Verify run status
        let run = database.get_pipeline_run(run_id).await.unwrap().unwrap();
        assert_eq!(run.status, PipelineRunStatus::Succeeded);
        assert!(run.started_at.is_some());
        assert!(run.completed_at.is_some());

        // Verify stage status
        let stages = database.list_pipeline_stages(run_id).await.unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].stage_name, "build");
        assert_eq!(stages[0].status, PipelineStageStatus::Succeeded);
    }

    #[tokio::test]
    async fn test_execute_pipeline_with_dependencies() {
        let database = Arc::new(Database::in_memory().await.unwrap());
        let executor = PipelineExecutor::new(database.clone());

        // Create a pipeline
        let pipeline = crate::Pipeline::new(
            "dep-pipeline".to_string(),
            "name: dep\nstages: []".to_string(),
        );
        let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();
        let run_id = executor.create_run(pipeline_id, None).await.unwrap();

        // Pipeline with dependencies: build -> test -> deploy
        let definition = PipelineDefinition {
            name: "dep-pipeline".to_string(),
            description: "Pipeline with dependencies".to_string(),
            version: 1,
            triggers: vec![],
            variables: HashMap::new(),
            stages: vec![
                StageDefinition {
                    name: "build".to_string(),
                    agent: "builder".to_string(),
                    task: "Build".to_string(),
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
                StageDefinition {
                    name: "test".to_string(),
                    agent: "tester".to_string(),
                    task: "Test".to_string(),
                    timeout: None,
                    on_failure: None,
                    rollback_to: None,
                    requires_approval: false,
                    approvers: vec![],
                    environment: None,
                    depends_on: vec!["build".to_string()],
                    parallel_with: None,
                    when: None,
                },
                StageDefinition {
                    name: "deploy".to_string(),
                    agent: "deployer".to_string(),
                    task: "Deploy".to_string(),
                    timeout: None,
                    on_failure: None,
                    rollback_to: None,
                    requires_approval: false,
                    approvers: vec![],
                    environment: None,
                    depends_on: vec!["test".to_string()],
                    parallel_with: None,
                    when: None,
                },
            ],
        };

        let result = executor.execute_run(run_id, &definition).await;
        assert!(result.is_ok());

        // Verify all stages completed
        let stages = database.list_pipeline_stages(run_id).await.unwrap();
        assert_eq!(stages.len(), 3);

        for stage in stages {
            assert_eq!(stage.status, PipelineStageStatus::Succeeded);
            assert!(stage.started_at.is_some());
            assert!(stage.completed_at.is_some());
        }
    }

    #[tokio::test]
    async fn test_execute_pipeline_with_parallel_stages() {
        let database = Arc::new(Database::in_memory().await.unwrap());
        let executor = PipelineExecutor::new(database.clone());

        let pipeline = crate::Pipeline::new(
            "parallel-pipeline".to_string(),
            "name: parallel\nstages: []".to_string(),
        );
        let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();
        let run_id = executor.create_run(pipeline_id, None).await.unwrap();

        // Pipeline with parallel stages
        let definition = PipelineDefinition {
            name: "parallel-pipeline".to_string(),
            description: "Pipeline with parallel stages".to_string(),
            version: 1,
            triggers: vec![],
            variables: HashMap::new(),
            stages: vec![
                StageDefinition {
                    name: "lint".to_string(),
                    agent: "linter".to_string(),
                    task: "Lint".to_string(),
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
                StageDefinition {
                    name: "test".to_string(),
                    agent: "tester".to_string(),
                    task: "Test".to_string(),
                    timeout: None,
                    on_failure: None,
                    rollback_to: None,
                    requires_approval: false,
                    approvers: vec![],
                    environment: None,
                    depends_on: vec![],
                    parallel_with: Some("lint".to_string()),
                    when: None,
                },
            ],
        };

        let result = executor.execute_run(run_id, &definition).await;
        assert!(result.is_ok());

        // Verify both stages completed
        let stages = database.list_pipeline_stages(run_id).await.unwrap();
        assert_eq!(stages.len(), 2);

        for stage in stages {
            assert_eq!(stage.status, PipelineStageStatus::Succeeded);
        }
    }

    #[tokio::test]
    async fn test_execute_pipeline_with_variables() {
        let database = Arc::new(Database::in_memory().await.unwrap());
        let executor = PipelineExecutor::new(database.clone());

        let pipeline = crate::Pipeline::new(
            "var-pipeline".to_string(),
            "name: var\nstages: []".to_string(),
        );
        let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();
        let run_id = executor.create_run(pipeline_id, None).await.unwrap();

        let mut variables = HashMap::new();
        variables.insert("environment".to_string(), "staging".to_string());

        let definition = PipelineDefinition {
            name: "var-pipeline".to_string(),
            description: "Pipeline with variables".to_string(),
            version: 1,
            triggers: vec![],
            variables,
            stages: vec![StageDefinition {
                name: "deploy".to_string(),
                agent: "deployer".to_string(),
                task: "Deploy to ${environment}".to_string(), // Will be substituted
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec![],
                parallel_with: None,
                when: None,
            }],
        };

        let result = executor.execute_run(run_id, &definition).await;
        assert!(result.is_ok());

        let stages = database.list_pipeline_stages(run_id).await.unwrap();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].status, PipelineStageStatus::Succeeded);
    }

    #[test]
    fn test_build_dependency_graph() {
        let executor = PipelineExecutor::new(Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(Database::in_memory())
                .unwrap(),
        ));

        let definition = PipelineDefinition {
            name: "test".to_string(),
            description: "test".to_string(),
            version: 1,
            triggers: vec![],
            variables: HashMap::new(),
            stages: vec![
                StageDefinition {
                    name: "a".to_string(),
                    agent: "agent".to_string(),
                    task: "task".to_string(),
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
                StageDefinition {
                    name: "b".to_string(),
                    agent: "agent".to_string(),
                    task: "task".to_string(),
                    timeout: None,
                    on_failure: None,
                    rollback_to: None,
                    requires_approval: false,
                    approvers: vec![],
                    environment: None,
                    depends_on: vec!["a".to_string()],
                    parallel_with: None,
                    when: None,
                },
                StageDefinition {
                    name: "c".to_string(),
                    agent: "agent".to_string(),
                    task: "task".to_string(),
                    timeout: None,
                    on_failure: None,
                    rollback_to: None,
                    requires_approval: false,
                    approvers: vec![],
                    environment: None,
                    depends_on: vec!["a".to_string(), "b".to_string()],
                    parallel_with: None,
                    when: None,
                },
            ],
        };

        let graph = executor.build_dependency_graph(&definition).unwrap();

        assert_eq!(graph.get("a").unwrap(), &Vec::<String>::new());
        assert_eq!(graph.get("b").unwrap(), &vec!["a".to_string()]);
        assert_eq!(
            graph.get("c").unwrap(),
            &vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn test_group_parallel_stages() {
        let executor = PipelineExecutor::new(Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(Database::in_memory())
                .unwrap(),
        ));

        let stage_a = StageDefinition {
            name: "a".to_string(),
            agent: "agent".to_string(),
            task: "task".to_string(),
            timeout: None,
            on_failure: None,
            rollback_to: None,
            requires_approval: false,
            approvers: vec![],
            environment: None,
            depends_on: vec![],
            parallel_with: None,
            when: None,
        };

        let stage_b = StageDefinition {
            name: "b".to_string(),
            agent: "agent".to_string(),
            task: "task".to_string(),
            timeout: None,
            on_failure: None,
            rollback_to: None,
            requires_approval: false,
            approvers: vec![],
            environment: None,
            depends_on: vec![],
            parallel_with: Some("a".to_string()),
            when: None,
        };

        let stage_c = StageDefinition {
            name: "c".to_string(),
            agent: "agent".to_string(),
            task: "task".to_string(),
            timeout: None,
            on_failure: None,
            rollback_to: None,
            requires_approval: false,
            approvers: vec![],
            environment: None,
            depends_on: vec![],
            parallel_with: None,
            when: None,
        };

        let stages = vec![&stage_a, &stage_b, &stage_c];
        let groups = executor.group_parallel_stages(&stages);

        // Should have 2 groups: [a, b] and [c]
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 2); // a and b in parallel
        assert_eq!(groups[1].len(), 1); // c alone
    }
}
