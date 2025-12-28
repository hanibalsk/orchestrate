//! Database layer for SQLite

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

use crate::{Agent, AgentState, AgentType, Epic, EpicStatus, Message, MessageRole, PrStatus, PullRequest, MergeStrategy, Result};
use crate::network::{AgentId, StepOutput, StepOutputType};
use crate::instruction::{
    CustomInstruction, InstructionEffectiveness, InstructionScope, InstructionSource,
    LearningPattern, PatternStatus, PatternType,
};

/// Database configuration
pub struct DatabaseConfig {
    /// Maximum number of connections
    pub max_connections: u32,
    /// Connection acquire timeout
    pub acquire_timeout: Duration,
    /// Idle connection timeout
    pub idle_timeout: Duration,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_connections: 5,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
        }
    }
}

/// Database connection and operations
#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection with default config
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        Self::with_config(path, DatabaseConfig::default()).await
    }

    /// Create a new database connection with custom config
    pub async fn with_config(path: impl AsRef<Path>, config: DatabaseConfig) -> Result<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Use WAL mode for better concurrent access
        let url = format!("sqlite:{}?mode=rwc", path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(config.acquire_timeout)
            .idle_timeout(config.idle_timeout)
            .connect(&url)
            .await?;

        // Enable WAL mode, foreign keys, and set busy timeout
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys=ON")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout=5000")
            .execute(&pool)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Create an in-memory database (for testing)
    pub async fn in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(include_str!("../../../migrations/001_initial.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::query(include_str!("../../../migrations/002_agent_network.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::query(include_str!("../../../migrations/003_step_outputs.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::query(include_str!("../../../migrations/004_custom_instructions.sql"))
            .execute(&self.pool)
            .await?;
        // Token tracking migration - uses ALTER TABLE which may fail if columns exist
        // This is safe because SQLite ALTER TABLE ADD COLUMN is idempotent for this use case
        let _ = sqlx::query(include_str!("../../../migrations/005_token_tracking.sql"))
            .execute(&self.pool)
            .await;
        Ok(())
    }

    /// Begin a transaction
    pub async fn begin(&self) -> Result<sqlx::Transaction<'_, sqlx::Sqlite>> {
        Ok(self.pool.begin().await?)
    }

    // ==================== Agent Operations ====================

    /// Insert a new agent
    pub async fn insert_agent(&self, agent: &Agent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agents (id, agent_type, state, task, context, session_id, parent_agent_id, worktree_id, error_message, created_at, updated_at, completed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(agent.id.to_string())
        .bind(agent.agent_type.as_str())
        .bind(agent.state.as_str())
        .bind(&agent.task)
        .bind(serde_json::to_string(&agent.context)?)
        .bind(&agent.session_id)
        .bind(agent.parent_agent_id.map(|id| id.to_string()))
        .bind(&agent.worktree_id)
        .bind(&agent.error_message)
        .bind(agent.created_at.to_rfc3339())
        .bind(agent.updated_at.to_rfc3339())
        .bind(agent.completed_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get an agent by ID
    pub async fn get_agent(&self, id: Uuid) -> Result<Option<Agent>> {
        let row = sqlx::query_as::<_, AgentRow>(
            "SELECT * FROM agents WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Update an agent
    pub async fn update_agent(&self, agent: &Agent) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE agents SET
                state = ?, task = ?, context = ?, session_id = ?, worktree_id = ?,
                error_message = ?, updated_at = ?, completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(agent.state.as_str())
        .bind(&agent.task)
        .bind(serde_json::to_string(&agent.context)?)
        .bind(&agent.session_id)
        .bind(&agent.worktree_id)
        .bind(&agent.error_message)
        .bind(agent.updated_at.to_rfc3339())
        .bind(agent.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(agent.id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update an agent with optimistic locking (returns true if updated)
    pub async fn update_agent_with_version(&self, agent: &Agent, expected_updated_at: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE agents SET
                state = ?, task = ?, context = ?, session_id = ?, worktree_id = ?,
                error_message = ?, updated_at = ?, completed_at = ?
            WHERE id = ? AND updated_at = ?
            "#,
        )
        .bind(agent.state.as_str())
        .bind(&agent.task)
        .bind(serde_json::to_string(&agent.context)?)
        .bind(&agent.session_id)
        .bind(&agent.worktree_id)
        .bind(&agent.error_message)
        .bind(agent.updated_at.to_rfc3339())
        .bind(agent.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(agent.id.to_string())
        .bind(expected_updated_at)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List agents by state
    pub async fn list_agents_by_state(&self, state: AgentState) -> Result<Vec<Agent>> {
        let rows = sqlx::query_as::<_, AgentRow>(
            "SELECT * FROM agents WHERE state = ? ORDER BY created_at DESC",
        )
        .bind(state.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List all agents
    pub async fn list_agents(&self) -> Result<Vec<Agent>> {
        let rows = sqlx::query_as::<_, AgentRow>(
            "SELECT * FROM agents ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Worktree Operations ====================

    /// Get worktree path by ID
    pub async fn get_worktree_path(&self, worktree_id: &str) -> Result<Option<String>> {
        let row = sqlx::query_scalar::<_, String>(
            "SELECT path FROM worktrees WHERE id = ?",
        )
        .bind(worktree_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    // ==================== Message Operations ====================

    /// Insert a message
    pub async fn insert_message(&self, message: &Message) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO agent_messages (agent_id, role, content, tool_calls, tool_results, input_tokens, output_tokens, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.agent_id.to_string())
        .bind(message.role.as_str())
        .bind(&message.content)
        .bind(message.tool_calls.as_ref().map(|tc| serde_json::to_string(tc).ok()).flatten())
        .bind(message.tool_results.as_ref().map(|tr| serde_json::to_string(tr).ok()).flatten())
        .bind(message.input_tokens)
        .bind(message.output_tokens)
        .bind(message.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get messages for an agent
    pub async fn get_messages(&self, agent_id: Uuid) -> Result<Vec<Message>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM agent_messages WHERE agent_id = ? ORDER BY created_at ASC",
        )
        .bind(agent_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Get messages for an agent with pagination
    pub async fn get_messages_paginated(
        &self,
        agent_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM agent_messages WHERE agent_id = ? ORDER BY created_at ASC LIMIT ? OFFSET ?",
        )
        .bind(agent_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Count messages for an agent
    pub async fn count_messages(&self, agent_id: Uuid) -> Result<i64> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_messages WHERE agent_id = ?",
        )
        .bind(agent_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get agent message statistics (token usage, tool calls)
    pub async fn get_agent_stats(&self, agent_id: Uuid) -> Result<AgentStats> {
        let row = sqlx::query_as::<_, AgentStatsRow>(
            r#"
            SELECT
                COUNT(*) as message_count,
                SUM(input_tokens) as total_input_tokens,
                SUM(output_tokens) as total_output_tokens,
                COUNT(CASE WHEN tool_calls IS NOT NULL AND tool_calls != '[]' THEN 1 END) as tool_call_count,
                COUNT(CASE WHEN tool_results LIKE '%"is_error":true%' THEN 1 END) as error_count,
                MIN(created_at) as first_message_at,
                MAX(created_at) as last_message_at
            FROM agent_messages
            WHERE agent_id = ?
            "#,
        )
        .bind(agent_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    /// List agents with pagination and optional filters
    pub async fn list_agents_paginated(
        &self,
        limit: i64,
        offset: i64,
        state_filter: Option<AgentState>,
        agent_type_filter: Option<AgentType>,
    ) -> Result<Vec<Agent>> {
        let mut query = String::from("SELECT * FROM agents WHERE 1=1");

        if state_filter.is_some() {
            query.push_str(" AND state = ?");
        }
        if agent_type_filter.is_some() {
            query.push_str(" AND agent_type = ?");
        }
        query.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, AgentRow>(&query);

        if let Some(state) = state_filter {
            q = q.bind(state.as_str());
        }
        if let Some(agent_type) = agent_type_filter {
            q = q.bind(agent_type.as_str());
        }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Count total agents
    pub async fn count_agents(&self) -> Result<i64> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agents",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get recent tool errors for an agent
    pub async fn get_tool_errors(&self, agent_id: Uuid, limit: i64) -> Result<Vec<Message>> {
        let rows = sqlx::query_as::<_, MessageRow>(
            r#"
            SELECT * FROM agent_messages
            WHERE agent_id = ?
            AND tool_results LIKE '%"is_error":true%'
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(agent_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Session Operations ====================

    /// Create a new session for an agent
    pub async fn create_session(&self, session: &crate::Session) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, agent_id, parent_id, api_session_id, total_tokens, is_forked, forked_at, created_at, closed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(session.agent_id.to_string())
        .bind(&session.parent_id)
        .bind(&session.api_session_id)
        .bind(session.total_tokens)
        .bind(session.is_forked)
        .bind(session.forked_at.map(|dt| dt.to_rfc3339()))
        .bind(session.created_at.to_rfc3339())
        .bind(session.closed_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Result<Option<crate::Session>> {
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT * FROM sessions WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get open session for an agent
    pub async fn get_open_session_for_agent(&self, agent_id: Uuid) -> Result<Option<crate::Session>> {
        let row = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT * FROM sessions
            WHERE agent_id = ? AND closed_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(agent_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Update session tokens
    pub async fn update_session_tokens(&self, session_id: &str, total_tokens: i64) -> Result<()> {
        sqlx::query(
            "UPDATE sessions SET total_tokens = ? WHERE id = ?",
        )
        .bind(total_tokens)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Set API session ID for a session
    pub async fn set_api_session_id(&self, session_id: &str, api_session_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE sessions SET api_session_id = ? WHERE id = ?",
        )
        .bind(api_session_id)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE sessions SET closed_at = ? WHERE id = ?",
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get session stats (total tokens used)
    pub async fn get_session_token_total(&self, agent_id: Uuid) -> Result<i64> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(SUM(total_tokens), 0) FROM sessions
            WHERE agent_id = ?
            "#,
        )
        .bind(agent_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    // ==================== PR Operations ====================

    /// Insert a PR
    pub async fn insert_pr(&self, pr: &PullRequest) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO pr_queue (epic_id, worktree_id, branch_name, title, body, pr_number, status, merge_strategy, agent_id, error_message, created_at, updated_at, merged_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&pr.epic_id)
        .bind(&pr.worktree_id)
        .bind(&pr.branch_name)
        .bind(&pr.title)
        .bind(&pr.body)
        .bind(pr.pr_number)
        .bind(pr.status.as_str())
        .bind(pr.merge_strategy.as_str())
        .bind(pr.agent_id.map(|id| id.to_string()))
        .bind(&pr.error_message)
        .bind(pr.created_at.to_rfc3339())
        .bind(pr.updated_at.to_rfc3339())
        .bind(pr.merged_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get pending PRs
    pub async fn get_pending_prs(&self) -> Result<Vec<PullRequest>> {
        let rows = sqlx::query_as::<_, PrRow>(
            r#"
            SELECT * FROM pr_queue
            WHERE status NOT IN ('merged', 'failed', 'closed')
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Update PR status
    pub async fn update_pr_status(&self, id: i64, status: PrStatus) -> Result<()> {
        let merged_at = if status == PrStatus::Merged {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE pr_queue SET status = ?, updated_at = ?, merged_at = COALESCE(?, merged_at)
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(merged_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ==================== Epic Operations ====================

    /// Upsert an epic
    pub async fn upsert_epic(&self, epic: &Epic) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO epics (id, title, source_file, pattern, status, current_phase, agent_id, pr_id, error_message, created_at, updated_at, completed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                source_file = excluded.source_file,
                status = excluded.status,
                current_phase = excluded.current_phase,
                agent_id = excluded.agent_id,
                pr_id = excluded.pr_id,
                error_message = excluded.error_message,
                updated_at = excluded.updated_at,
                completed_at = excluded.completed_at
            "#,
        )
        .bind(&epic.id)
        .bind(&epic.title)
        .bind(&epic.source_file)
        .bind(&epic.pattern)
        .bind(epic.status.as_str())
        .bind(epic.current_phase.map(|p| p.to_string()))
        .bind(epic.agent_id.map(|id| id.to_string()))
        .bind(epic.pr_id)
        .bind(&epic.error_message)
        .bind(epic.created_at.to_rfc3339())
        .bind(epic.updated_at.to_rfc3339())
        .bind(epic.completed_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get pending epics
    pub async fn get_pending_epics(&self) -> Result<Vec<Epic>> {
        let rows = sqlx::query_as::<_, EpicRow>(
            "SELECT * FROM epics WHERE status = 'pending' ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Step Output Operations ====================

    /// Insert a step output
    #[tracing::instrument(skip(self, output), level = "debug", fields(agent_id = %output.agent_id, skill = %output.skill_name))]
    pub async fn insert_step_output(&self, output: &StepOutput) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO step_outputs (agent_id, skill_name, output_type, data, consumed, consumed_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(output.agent_id.0.to_string())
        .bind(&output.skill_name)
        .bind(output.output_type.as_str())
        .bind(serde_json::to_string(&output.data)?)
        .bind(output.consumed)
        .bind(output.consumed_by.map(|id| id.0.to_string()))
        .bind(output.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get step outputs for an agent
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_step_outputs(&self, agent_id: AgentId) -> Result<Vec<StepOutput>> {
        let rows = sqlx::query_as::<_, StepOutputRow>(
            "SELECT * FROM step_outputs WHERE agent_id = ? ORDER BY created_at DESC",
        )
        .bind(agent_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Get unconsumed step outputs from specific agents (dependencies)
    #[tracing::instrument(skip(self), level = "debug", fields(count = dependency_agent_ids.len()))]
    pub async fn get_dependency_outputs(&self, dependency_agent_ids: &[AgentId]) -> Result<Vec<StepOutput>> {
        if dependency_agent_ids.is_empty() {
            return Ok(vec![]);
        }

        // Limit batch size to prevent DoS
        const MAX_BATCH_SIZE: usize = 1000;
        if dependency_agent_ids.len() > MAX_BATCH_SIZE {
            return Err(crate::Error::Other(format!(
                "Batch size {} exceeds maximum of {}",
                dependency_agent_ids.len(),
                MAX_BATCH_SIZE
            )));
        }

        // Build placeholders for IN clause
        let placeholders: Vec<String> = dependency_agent_ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT * FROM step_outputs WHERE agent_id IN ({}) AND consumed = 0 ORDER BY created_at ASC",
            placeholders.join(", ")
        );

        let mut query_builder = sqlx::query_as::<_, StepOutputRow>(&query);
        for id in dependency_agent_ids {
            query_builder = query_builder.bind(id.0.to_string());
        }

        let rows = query_builder.fetch_all(&self.pool).await?;
        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Get the latest step output from an agent
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_latest_step_output(&self, agent_id: AgentId) -> Result<Option<StepOutput>> {
        let row = sqlx::query_as::<_, StepOutputRow>(
            "SELECT * FROM step_outputs WHERE agent_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(agent_id.0.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get step outputs by skill name
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_step_outputs_by_skill(&self, agent_id: AgentId, skill_name: &str) -> Result<Vec<StepOutput>> {
        let rows = sqlx::query_as::<_, StepOutputRow>(
            "SELECT * FROM step_outputs WHERE agent_id = ? AND skill_name = ? ORDER BY created_at DESC",
        )
        .bind(agent_id.0.to_string())
        .bind(skill_name)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Mark step outputs as consumed by an agent
    /// Returns the number of outputs that were actually consumed (not already consumed)
    #[tracing::instrument(skip(self), level = "debug", fields(count = output_ids.len()))]
    pub async fn mark_outputs_consumed(&self, output_ids: &[i64], consumed_by: AgentId) -> Result<u64> {
        if output_ids.is_empty() {
            return Ok(0);
        }

        // Limit batch size to prevent DoS
        const MAX_BATCH_SIZE: usize = 1000;
        if output_ids.len() > MAX_BATCH_SIZE {
            return Err(crate::Error::Other(format!(
                "Batch size {} exceeds maximum of {}",
                output_ids.len(),
                MAX_BATCH_SIZE
            )));
        }

        let placeholders: Vec<String> = output_ids.iter().map(|_| "?".to_string()).collect();
        // Add AND consumed = 0 to prevent race condition - only consume if not already consumed
        let query = format!(
            "UPDATE step_outputs SET consumed = 1, consumed_by = ?, consumed_at = ? WHERE id IN ({}) AND consumed = 0",
            placeholders.join(", ")
        );

        let mut query_builder = sqlx::query(&query)
            .bind(consumed_by.0.to_string())
            .bind(chrono::Utc::now().to_rfc3339());

        for id in output_ids {
            query_builder = query_builder.bind(id);
        }

        let result = query_builder.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Get unconsumed step output count for an agent's dependencies
    pub async fn count_unconsumed_outputs(&self, dependency_agent_ids: &[AgentId]) -> Result<i64> {
        if dependency_agent_ids.is_empty() {
            return Ok(0);
        }

        // Limit batch size to prevent DoS
        const MAX_BATCH_SIZE: usize = 1000;
        if dependency_agent_ids.len() > MAX_BATCH_SIZE {
            return Err(crate::Error::Other(format!(
                "Batch size {} exceeds maximum of {}",
                dependency_agent_ids.len(),
                MAX_BATCH_SIZE
            )));
        }

        let placeholders: Vec<String> = dependency_agent_ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "SELECT COUNT(*) as count FROM step_outputs WHERE agent_id IN ({}) AND consumed = 0",
            placeholders.join(", ")
        );

        let mut query_builder = sqlx::query_scalar::<_, i64>(&query);
        for id in dependency_agent_ids {
            query_builder = query_builder.bind(id.0.to_string());
        }

        let count = query_builder.fetch_one(&self.pool).await?;
        Ok(count)
    }

    // ==================== Custom Instruction Operations ====================

    /// Insert a new custom instruction
    #[tracing::instrument(skip(self, instruction), level = "debug", fields(name = %instruction.name))]
    pub async fn insert_instruction(&self, instruction: &CustomInstruction) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO custom_instructions (name, content, scope, agent_type, priority, enabled, source, confidence, tags, created_at, updated_at, created_by)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&instruction.name)
        .bind(&instruction.content)
        .bind(instruction.scope.as_str())
        .bind(instruction.agent_type.map(|t| t.as_str()))
        .bind(instruction.priority)
        .bind(instruction.enabled)
        .bind(instruction.source.as_str())
        .bind(instruction.confidence)
        .bind(serde_json::to_string(&instruction.tags)?)
        .bind(instruction.created_at.to_rfc3339())
        .bind(instruction.updated_at.to_rfc3339())
        .bind(&instruction.created_by)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();

        // Initialize effectiveness metrics
        sqlx::query(
            "INSERT INTO instruction_effectiveness (instruction_id) VALUES (?)"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get instruction by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instruction(&self, id: i64) -> Result<Option<CustomInstruction>> {
        let row = sqlx::query_as::<_, InstructionRow>(
            "SELECT * FROM custom_instructions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get instruction by name
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instruction_by_name(&self, name: &str) -> Result<Option<CustomInstruction>> {
        let row = sqlx::query_as::<_, InstructionRow>(
            "SELECT * FROM custom_instructions WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get all enabled instructions for an agent type (includes global)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instructions_for_agent(&self, agent_type: AgentType) -> Result<Vec<CustomInstruction>> {
        let rows = sqlx::query_as::<_, InstructionRow>(
            r#"
            SELECT * FROM custom_instructions
            WHERE enabled = 1
            AND (scope = 'global' OR (scope = 'agent_type' AND agent_type = ?))
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .bind(agent_type.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List all instructions with optional filters
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_instructions(
        &self,
        enabled_only: bool,
        scope: Option<InstructionScope>,
        source: Option<InstructionSource>,
    ) -> Result<Vec<CustomInstruction>> {
        let mut query = String::from("SELECT * FROM custom_instructions WHERE 1=1");

        if enabled_only {
            query.push_str(" AND enabled = 1");
        }
        if let Some(s) = &scope {
            query.push_str(&format!(" AND scope = '{}'", s.as_str()));
        }
        if let Some(s) = &source {
            query.push_str(&format!(" AND source = '{}'", s.as_str()));
        }
        query.push_str(" ORDER BY priority DESC, created_at ASC");

        let rows = sqlx::query_as::<_, InstructionRow>(&query)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Update an instruction
    #[tracing::instrument(skip(self, instruction), level = "debug", fields(id = instruction.id))]
    pub async fn update_instruction(&self, instruction: &CustomInstruction) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE custom_instructions SET
                name = ?, content = ?, scope = ?, agent_type = ?,
                priority = ?, enabled = ?, source = ?, confidence = ?,
                tags = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&instruction.name)
        .bind(&instruction.content)
        .bind(instruction.scope.as_str())
        .bind(instruction.agent_type.map(|t| t.as_str()))
        .bind(instruction.priority)
        .bind(instruction.enabled)
        .bind(instruction.source.as_str())
        .bind(instruction.confidence)
        .bind(serde_json::to_string(&instruction.tags)?)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(instruction.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Enable/disable an instruction
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn set_instruction_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        sqlx::query(
            "UPDATE custom_instructions SET enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(enabled)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete an instruction
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn delete_instruction(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM custom_instructions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Record instruction usage
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn record_instruction_usage(
        &self,
        instruction_id: i64,
        agent_id: Uuid,
        session_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO instruction_usage (instruction_id, agent_id, session_id)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(instruction_id)
        .bind(agent_id.to_string())
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        // Update usage count
        sqlx::query(
            r#"
            UPDATE instruction_effectiveness
            SET usage_count = usage_count + 1, updated_at = ?
            WHERE instruction_id = ?
            "#,
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(instruction_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record instruction outcome (success/failure)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn record_instruction_outcome(
        &self,
        instruction_id: i64,
        success: bool,
        completion_time_secs: Option<f64>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        if success {
            sqlx::query(
                r#"
                UPDATE instruction_effectiveness SET
                    success_count = success_count + 1,
                    last_success_at = ?,
                    updated_at = ?
                WHERE instruction_id = ?
                "#,
            )
            .bind(&now)
            .bind(&now)
            .bind(instruction_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE instruction_effectiveness SET
                    failure_count = failure_count + 1,
                    last_failure_at = ?,
                    updated_at = ?
                WHERE instruction_id = ?
                "#,
            )
            .bind(&now)
            .bind(&now)
            .bind(instruction_id)
            .execute(&self.pool)
            .await?;
        }

        // Update average completion time if provided
        if let Some(time) = completion_time_secs {
            sqlx::query(
                r#"
                UPDATE instruction_effectiveness SET
                    avg_completion_time = COALESCE(
                        (avg_completion_time * (usage_count - 1) + ?) / usage_count,
                        ?
                    )
                WHERE instruction_id = ?
                "#,
            )
            .bind(time)
            .bind(time)
            .bind(instruction_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Apply penalty to an instruction
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn apply_penalty(&self, id: i64, amount: f64, reason: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE instruction_effectiveness
            SET penalty_score = MIN(penalty_score + ?, 2.0),
                last_penalty_at = ?,
                updated_at = ?
            WHERE instruction_id = ?
            "#,
        )
        .bind(amount)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        tracing::warn!(instruction_id = id, penalty = amount, reason, "Penalty applied");
        Ok(())
    }

    /// Decay penalty on success
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn decay_penalty(&self, id: i64, amount: f64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE instruction_effectiveness
            SET penalty_score = MAX(penalty_score - ?, 0.0),
                updated_at = ?
            WHERE instruction_id = ?
            "#,
        )
        .bind(amount)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get instruction effectiveness
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instruction_effectiveness(&self, instruction_id: i64) -> Result<Option<InstructionEffectiveness>> {
        let row = sqlx::query_as::<_, EffectivenessRow>(
            "SELECT * FROM instruction_effectiveness WHERE instruction_id = ?",
        )
        .bind(instruction_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get instructions with high penalty scores
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_high_penalty_instructions(&self, threshold: f64) -> Result<Vec<i64>> {
        let ids = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT instruction_id FROM instruction_effectiveness
            WHERE penalty_score >= ?
            "#,
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;

        Ok(ids)
    }

    /// Auto-disable instructions with high penalty
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn auto_disable_penalized(&self, threshold: f64) -> Result<Vec<i64>> {
        let disabled = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT instruction_id FROM instruction_effectiveness
            WHERE penalty_score >= ?
            AND instruction_id IN (
                SELECT id FROM custom_instructions WHERE enabled = 1
            )
            "#,
        )
        .bind(threshold)
        .fetch_all(&self.pool)
        .await?;

        for id in &disabled {
            self.set_instruction_enabled(*id, false).await?;
            tracing::info!(instruction_id = id, "Auto-disabled due to high penalty");
        }

        Ok(disabled)
    }

    /// Delete ineffective learned instructions
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn delete_ineffective_instructions(&self) -> Result<Vec<String>> {
        let to_delete = sqlx::query_as::<_, (i64, String)>(
            r#"
            SELECT ci.id, ci.name FROM custom_instructions ci
            JOIN instruction_effectiveness ie ON ci.id = ie.instruction_id
            WHERE ie.penalty_score >= 1.0
            AND ie.usage_count >= 10
            AND (CAST(ie.success_count AS REAL) / NULLIF(ie.usage_count, 0)) < 0.3
            AND ci.source = 'learned'
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let names: Vec<String> = to_delete.iter().map(|(_, n)| n.clone()).collect();

        for (id, name) in to_delete {
            self.delete_instruction(id).await?;
            tracing::info!(instruction_id = id, name = %name, "Deleted ineffective instruction");
        }

        Ok(names)
    }

    /// Reset penalty score for an instruction
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn reset_penalty(&self, id: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE instruction_effectiveness
            SET penalty_score = 0.0, last_penalty_at = NULL, updated_at = ?
            WHERE instruction_id = ?
            "#,
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ==================== Learning Pattern Operations ====================

    /// Upsert a learning pattern (increment count if exists)
    #[tracing::instrument(skip(self, pattern), level = "debug", fields(signature = %pattern.pattern_signature))]
    pub async fn upsert_learning_pattern(&self, pattern: &LearningPattern) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        // Try to find existing pattern by signature
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM learning_patterns WHERE pattern_signature = ?"
        )
        .bind(&pattern.pattern_signature)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(id) = existing {
            // Update existing pattern
            sqlx::query(
                r#"
                UPDATE learning_patterns SET
                    occurrence_count = occurrence_count + 1,
                    last_seen_at = ?,
                    pattern_data = ?
                WHERE id = ?
                "#,
            )
            .bind(&now)
            .bind(serde_json::to_string(&pattern.pattern_data)?)
            .bind(id)
            .execute(&self.pool)
            .await?;
            Ok(id)
        } else {
            // Insert new pattern
            let result = sqlx::query(
                r#"
                INSERT INTO learning_patterns (pattern_type, agent_type, pattern_signature, pattern_data, first_seen_at, last_seen_at, status)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(pattern.pattern_type.as_str())
            .bind(pattern.agent_type.map(|t| t.as_str()))
            .bind(&pattern.pattern_signature)
            .bind(serde_json::to_string(&pattern.pattern_data)?)
            .bind(&now)
            .bind(&now)
            .bind(pattern.status.as_str())
            .execute(&self.pool)
            .await?;
            Ok(result.last_insert_rowid())
        }
    }

    /// Get pattern by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_pattern(&self, id: i64) -> Result<Option<LearningPattern>> {
        let row = sqlx::query_as::<_, PatternRow>(
            "SELECT * FROM learning_patterns WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get patterns ready for instruction generation
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_patterns_for_review(&self, min_occurrences: i64) -> Result<Vec<LearningPattern>> {
        let rows = sqlx::query_as::<_, PatternRow>(
            r#"
            SELECT * FROM learning_patterns
            WHERE status = 'observed'
            AND occurrence_count >= ?
            ORDER BY occurrence_count DESC
            "#,
        )
        .bind(min_occurrences)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List all patterns with optional status filter
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_patterns(&self, status: Option<PatternStatus>) -> Result<Vec<LearningPattern>> {
        let rows = if let Some(s) = status {
            sqlx::query_as::<_, PatternRow>(
                "SELECT * FROM learning_patterns WHERE status = ? ORDER BY occurrence_count DESC",
            )
            .bind(s.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, PatternRow>(
                "SELECT * FROM learning_patterns ORDER BY occurrence_count DESC",
            )
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Update pattern status
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn update_pattern_status(
        &self,
        id: i64,
        status: PatternStatus,
        instruction_id: Option<i64>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE learning_patterns SET status = ?, instruction_id = ? WHERE id = ?",
        )
        .bind(status.as_str())
        .bind(instruction_id)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ==================== Token Tracking Operations ====================

    /// Record token usage for a session turn
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn record_session_tokens(
        &self,
        session_id: &str,
        agent_id: Uuid,
        turn_number: i32,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_write_tokens: i64,
        context_window_used: i64,
        messages_included: i32,
        messages_summarized: i32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO session_token_stats (
                session_id, agent_id, turn_number,
                input_tokens, output_tokens,
                cache_read_tokens, cache_write_tokens,
                context_window_used, messages_included, messages_summarized
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(session_id)
        .bind(agent_id.to_string())
        .bind(turn_number)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cache_read_tokens)
        .bind(cache_write_tokens)
        .bind(context_window_used)
        .bind(messages_included)
        .bind(messages_summarized)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update daily token usage aggregation
    /// Uses INSERT ... ON CONFLICT to avoid race conditions
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn update_daily_token_usage(
        &self,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_write_tokens: i64,
    ) -> Result<()> {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Calculate estimated cost based on model pricing (USD per 1M tokens)
        let estimated_cost = Self::calculate_token_cost(
            model,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
        );

        // Use upsert to avoid race conditions
        sqlx::query(
            r#"
            INSERT INTO daily_token_usage (
                date, model,
                total_input_tokens, total_output_tokens,
                total_cache_read_tokens, total_cache_write_tokens,
                request_count, agent_count, estimated_cost_usd,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, 1, 1, ?, ?, ?)
            ON CONFLICT(date, model) DO UPDATE SET
                total_input_tokens = total_input_tokens + excluded.total_input_tokens,
                total_output_tokens = total_output_tokens + excluded.total_output_tokens,
                total_cache_read_tokens = total_cache_read_tokens + excluded.total_cache_read_tokens,
                total_cache_write_tokens = total_cache_write_tokens + excluded.total_cache_write_tokens,
                request_count = request_count + 1,
                estimated_cost_usd = COALESCE(estimated_cost_usd, 0) + excluded.estimated_cost_usd,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&date)
        .bind(model)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cache_read_tokens)
        .bind(cache_write_tokens)
        .bind(estimated_cost)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Calculate token cost in USD based on model pricing
    /// Prices as of Dec 2024 (per 1M tokens)
    fn calculate_token_cost(
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_write_tokens: i64,
    ) -> f64 {
        // Pricing per 1M tokens (USD)
        let (input_price, output_price, cache_read_price, cache_write_price) = if model.contains("opus") {
            // Claude Opus 4
            (15.0, 75.0, 1.5, 18.75)  // cache read 90% off, cache write 25% premium
        } else if model.contains("haiku") {
            // Claude Haiku 3
            (0.25, 1.25, 0.025, 0.3125)
        } else {
            // Claude Sonnet 4 (default)
            (3.0, 15.0, 0.3, 3.75)
        };

        // Calculate regular input tokens (excluding cached)
        let regular_input = input_tokens - cache_read_tokens - cache_write_tokens;

        // Cost calculation
        let input_cost = (regular_input as f64 / 1_000_000.0) * input_price;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;
        let cache_read_cost = (cache_read_tokens as f64 / 1_000_000.0) * cache_read_price;
        let cache_write_cost = (cache_write_tokens as f64 / 1_000_000.0) * cache_write_price;

        input_cost + output_cost + cache_read_cost + cache_write_cost
    }

    /// Update instruction effectiveness with token data
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn update_instruction_tokens(
        &self,
        instruction_id: i64,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_write_tokens: i64,
    ) -> Result<()> {
        // First update the totals, then calculate avg in a separate step
        // This ensures correct avg_tokens_per_run calculation
        sqlx::query(
            r#"
            UPDATE instruction_effectiveness SET
                total_input_tokens = COALESCE(total_input_tokens, 0) + ?,
                total_output_tokens = COALESCE(total_output_tokens, 0) + ?,
                total_cache_read_tokens = COALESCE(total_cache_read_tokens, 0) + ?,
                total_cache_write_tokens = COALESCE(total_cache_write_tokens, 0) + ?,
                avg_tokens_per_run = CAST(
                    (COALESCE(total_input_tokens, 0) + ? + COALESCE(total_output_tokens, 0) + ?)
                    AS REAL
                ) / CAST(MAX(COALESCE(usage_count, 1), 1) AS REAL),
                updated_at = ?
            WHERE instruction_id = ?
            "#,
        )
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cache_read_tokens)
        .bind(cache_write_tokens)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(instruction_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get token usage stats for an agent's session
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_session_token_stats(&self, session_id: &str) -> Result<TokenStats> {
        let row = sqlx::query_as::<_, TokenStatsRow>(
            r#"
            SELECT
                COUNT(*) as turn_count,
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(cache_read_tokens), 0) as total_cache_read_tokens,
                COALESCE(SUM(cache_write_tokens), 0) as total_cache_write_tokens,
                CAST(COALESCE(AVG(context_window_used), 0) AS REAL) as avg_context_used,
                CAST(COALESCE(AVG(messages_included), 0) AS REAL) as avg_messages_included,
                COALESCE(SUM(messages_summarized), 0) as total_messages_summarized
            FROM session_token_stats
            WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    /// Get daily token usage for cost analysis
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_daily_token_usage(&self, days: i32) -> Result<Vec<DailyTokenUsage>> {
        let rows = sqlx::query_as::<_, DailyTokenUsageRow>(
            r#"
            SELECT * FROM daily_token_usage
            WHERE date >= date('now', '-' || ? || ' days')
            ORDER BY date DESC, model
            "#,
        )
        .bind(days)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Get token stats for a specific agent
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_agent_token_stats(&self, agent_id: Uuid) -> Result<TokenStats> {
        let row = sqlx::query_as::<_, TokenStatsRow>(
            r#"
            SELECT
                COUNT(*) as turn_count,
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(cache_read_tokens), 0) as total_cache_read_tokens,
                COALESCE(SUM(cache_write_tokens), 0) as total_cache_write_tokens,
                CAST(COALESCE(AVG(context_window_used), 0) AS REAL) as avg_context_used,
                CAST(COALESCE(AVG(messages_included), 0) AS REAL) as avg_messages_included,
                COALESCE(SUM(messages_summarized), 0) as total_messages_summarized
            FROM session_token_stats
            WHERE agent_id = ?
            "#,
        )
        .bind(agent_id.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }
}

/// Token usage statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenStats {
    pub turn_count: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_write_tokens: i64,
    pub avg_context_used: f64,
    pub avg_messages_included: f64,
    pub total_messages_summarized: i64,
    pub cache_hit_rate: f64,
}

#[derive(sqlx::FromRow)]
struct TokenStatsRow {
    turn_count: i64,
    total_input_tokens: i64,
    total_output_tokens: i64,
    total_cache_read_tokens: i64,
    total_cache_write_tokens: i64,
    avg_context_used: f64,
    avg_messages_included: f64,
    total_messages_summarized: i64,
}

impl From<TokenStatsRow> for TokenStats {
    fn from(row: TokenStatsRow) -> Self {
        let total_input = row.total_input_tokens;
        let cache_read = row.total_cache_read_tokens;
        let cache_hit_rate = if total_input > 0 {
            (cache_read as f64 / total_input as f64) * 100.0
        } else {
            0.0
        };

        Self {
            turn_count: row.turn_count,
            total_input_tokens: row.total_input_tokens,
            total_output_tokens: row.total_output_tokens,
            total_cache_read_tokens: row.total_cache_read_tokens,
            total_cache_write_tokens: row.total_cache_write_tokens,
            avg_context_used: row.avg_context_used,
            avg_messages_included: row.avg_messages_included,
            total_messages_summarized: row.total_messages_summarized,
            cache_hit_rate,
        }
    }
}

/// Daily token usage for cost tracking
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyTokenUsage {
    pub date: String,
    pub model: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_write_tokens: i64,
    pub request_count: i64,
    pub agent_count: i64,
    pub estimated_cost_usd: Option<f64>,
}

#[derive(sqlx::FromRow)]
struct DailyTokenUsageRow {
    date: String,
    model: String,
    total_input_tokens: i64,
    total_output_tokens: i64,
    total_cache_read_tokens: i64,
    total_cache_write_tokens: i64,
    request_count: i64,
    agent_count: i64,
    estimated_cost_usd: Option<f64>,
}

impl From<DailyTokenUsageRow> for DailyTokenUsage {
    fn from(row: DailyTokenUsageRow) -> Self {
        Self {
            date: row.date,
            model: row.model,
            total_input_tokens: row.total_input_tokens,
            total_output_tokens: row.total_output_tokens,
            total_cache_read_tokens: row.total_cache_read_tokens,
            total_cache_write_tokens: row.total_cache_write_tokens,
            request_count: row.request_count,
            agent_count: row.agent_count,
            estimated_cost_usd: row.estimated_cost_usd,
        }
    }
}

// ==================== Row Types for SQLx ====================

#[derive(sqlx::FromRow)]
struct AgentRow {
    id: String,
    agent_type: String,
    state: String,
    task: String,
    context: String,
    session_id: Option<String>,
    parent_agent_id: Option<String>,
    worktree_id: Option<String>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
}

impl TryFrom<AgentRow> for Agent {
    type Error = crate::Error;

    fn try_from(row: AgentRow) -> Result<Self> {
        Ok(Agent {
            id: Uuid::parse_str(&row.id).map_err(|e| crate::Error::Other(e.to_string()))?,
            agent_type: AgentType::from_str(&row.agent_type)?,
            state: AgentState::from_str(&row.state)?,
            task: row.task,
            context: serde_json::from_str(&row.context)?,
            session_id: row.session_id,
            parent_agent_id: row.parent_agent_id.map(|s| Uuid::parse_str(&s)).transpose().map_err(|e| crate::Error::Other(e.to_string()))?,
            worktree_id: row.worktree_id,
            error_message: row.error_message,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at).map_err(|e| crate::Error::Other(e.to_string()))?.into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at).map_err(|e| crate::Error::Other(e.to_string()))?.into(),
            completed_at: row.completed_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s)).transpose().map_err(|e| crate::Error::Other(e.to_string()))?.map(Into::into),
        })
    }
}

/// Statistics for an agent's message history
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentStats {
    pub message_count: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub tool_call_count: i64,
    pub error_count: i64,
    pub first_message_at: Option<String>,
    pub last_message_at: Option<String>,
}

#[derive(sqlx::FromRow)]
struct AgentStatsRow {
    message_count: i64,
    total_input_tokens: Option<i64>,
    total_output_tokens: Option<i64>,
    tool_call_count: i64,
    error_count: i64,
    first_message_at: Option<String>,
    last_message_at: Option<String>,
}

impl From<AgentStatsRow> for AgentStats {
    fn from(row: AgentStatsRow) -> Self {
        let input = row.total_input_tokens.unwrap_or(0);
        let output = row.total_output_tokens.unwrap_or(0);
        Self {
            message_count: row.message_count,
            total_input_tokens: input,
            total_output_tokens: output,
            total_tokens: input + output,
            tool_call_count: row.tool_call_count,
            error_count: row.error_count,
            first_message_at: row.first_message_at,
            last_message_at: row.last_message_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: String,
    agent_id: String,
    parent_id: Option<String>,
    api_session_id: Option<String>,
    total_tokens: i64,
    is_forked: bool,
    forked_at: Option<String>,
    created_at: String,
    closed_at: Option<String>,
}

impl TryFrom<SessionRow> for crate::Session {
    type Error = crate::Error;

    fn try_from(row: SessionRow) -> Result<Self> {
        Ok(crate::Session {
            id: row.id,
            agent_id: Uuid::parse_str(&row.agent_id).map_err(|e| crate::Error::Other(e.to_string()))?,
            parent_id: row.parent_id,
            api_session_id: row.api_session_id,
            total_tokens: row.total_tokens,
            is_forked: row.is_forked,
            forked_at: row.forked_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s)).transpose().map_err(|e| crate::Error::Other(e.to_string()))?.map(Into::into),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at).map_err(|e| crate::Error::Other(e.to_string()))?.into(),
            closed_at: row.closed_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s)).transpose().map_err(|e| crate::Error::Other(e.to_string()))?.map(Into::into),
        })
    }
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: i64,
    agent_id: String,
    role: String,
    content: String,
    tool_calls: Option<String>,
    tool_results: Option<String>,
    input_tokens: i32,
    output_tokens: i32,
    created_at: String,
}

impl TryFrom<MessageRow> for Message {
    type Error = crate::Error;

    fn try_from(row: MessageRow) -> Result<Self> {
        Ok(Message {
            id: row.id,
            agent_id: Uuid::parse_str(&row.agent_id).map_err(|e| crate::Error::Other(e.to_string()))?,
            role: MessageRole::from_str(&row.role)?,
            content: row.content,
            tool_calls: row.tool_calls.map(|s| serde_json::from_str(&s)).transpose()?,
            tool_results: row.tool_results.map(|s| serde_json::from_str(&s)).transpose()?,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at).map_err(|e| crate::Error::Other(e.to_string()))?.into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct PrRow {
    id: i64,
    epic_id: Option<String>,
    worktree_id: Option<String>,
    branch_name: String,
    title: Option<String>,
    body: Option<String>,
    pr_number: Option<i32>,
    status: String,
    merge_strategy: String,
    agent_id: Option<String>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
    merged_at: Option<String>,
}

impl TryFrom<PrRow> for PullRequest {
    type Error = crate::Error;

    fn try_from(row: PrRow) -> Result<Self> {
        Ok(PullRequest {
            id: row.id,
            epic_id: row.epic_id,
            worktree_id: row.worktree_id,
            branch_name: row.branch_name,
            title: row.title,
            body: row.body,
            pr_number: row.pr_number,
            status: PrStatus::from_str(&row.status)?,
            merge_strategy: MergeStrategy::from_str(&row.merge_strategy)?,
            agent_id: row.agent_id.map(|s| Uuid::parse_str(&s)).transpose().map_err(|e| crate::Error::Other(e.to_string()))?,
            error_message: row.error_message,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at).map_err(|e| crate::Error::Other(e.to_string()))?.into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at).map_err(|e| crate::Error::Other(e.to_string()))?.into(),
            merged_at: row.merged_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s)).transpose().map_err(|e| crate::Error::Other(e.to_string()))?.map(Into::into),
        })
    }
}

#[derive(sqlx::FromRow)]
struct EpicRow {
    id: String,
    title: String,
    source_file: Option<String>,
    pattern: Option<String>,
    status: String,
    current_phase: Option<String>,
    agent_id: Option<String>,
    pr_id: Option<i64>,
    error_message: Option<String>,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
}

impl TryFrom<EpicRow> for Epic {
    type Error = crate::Error;

    fn try_from(row: EpicRow) -> Result<Self> {
        Ok(Epic {
            id: row.id,
            title: row.title,
            source_file: row.source_file,
            pattern: row.pattern,
            status: EpicStatus::from_str(&row.status)?,
            current_phase: row.current_phase.map(|p| serde_json::from_str(&format!("\"{}\"", p))).transpose()?,
            agent_id: row.agent_id.map(|s| Uuid::parse_str(&s)).transpose().map_err(|e| crate::Error::Other(e.to_string()))?,
            pr_id: row.pr_id,
            error_message: row.error_message,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at).map_err(|e| crate::Error::Other(e.to_string()))?.into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at).map_err(|e| crate::Error::Other(e.to_string()))?.into(),
            completed_at: row.completed_at.map(|s| chrono::DateTime::parse_from_rfc3339(&s)).transpose().map_err(|e| crate::Error::Other(e.to_string()))?.map(Into::into),
        })
    }
}

#[derive(sqlx::FromRow)]
struct StepOutputRow {
    id: i64,
    agent_id: String,
    skill_name: String,
    output_type: String,
    data: String,
    consumed: bool,
    consumed_by: Option<String>,
    consumed_at: Option<String>,
    created_at: String,
}

impl TryFrom<StepOutputRow> for StepOutput {
    type Error = crate::Error;

    fn try_from(row: StepOutputRow) -> Result<Self> {
        use std::str::FromStr;

        let agent_uuid = Uuid::parse_str(&row.agent_id)
            .map_err(|e| crate::Error::Other(e.to_string()))?;

        let consumed_by = row.consumed_by
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|e| crate::Error::Other(e.to_string()))?
            .map(AgentId::from_uuid);

        let consumed_at = row.consumed_at
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| crate::Error::Other(e.to_string()))?
            .map(Into::into);

        let output_type = StepOutputType::from_str(&row.output_type)?;

        Ok(StepOutput {
            id: Some(row.id),
            agent_id: AgentId::from_uuid(agent_uuid),
            skill_name: row.skill_name,
            output_type,
            data: serde_json::from_str(&row.data)?,
            consumed: row.consumed,
            consumed_by,
            consumed_at,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct InstructionRow {
    id: i64,
    name: String,
    content: String,
    scope: String,
    agent_type: Option<String>,
    priority: i32,
    enabled: bool,
    source: String,
    confidence: f64,
    tags: Option<String>,
    created_at: String,
    updated_at: String,
    created_by: Option<String>,
}

impl TryFrom<InstructionRow> for CustomInstruction {
    type Error = crate::Error;

    fn try_from(row: InstructionRow) -> Result<Self> {
        let agent_type = row.agent_type
            .map(|s| AgentType::from_str(&s))
            .transpose()?;

        let tags: Vec<String> = row.tags
            .map(|s| serde_json::from_str(&s))
            .transpose()?
            .unwrap_or_default();

        Ok(CustomInstruction {
            id: row.id,
            name: row.name,
            content: row.content,
            scope: InstructionScope::from_str(&row.scope)?,
            agent_type,
            priority: row.priority,
            enabled: row.enabled,
            source: InstructionSource::from_str(&row.source)?,
            confidence: row.confidence,
            tags,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            created_by: row.created_by,
        })
    }
}

#[derive(sqlx::FromRow)]
struct EffectivenessRow {
    instruction_id: i64,
    usage_count: i64,
    success_count: i64,
    failure_count: i64,
    penalty_score: f64,
    avg_completion_time: Option<f64>,
    last_success_at: Option<String>,
    last_failure_at: Option<String>,
    last_penalty_at: Option<String>,
    updated_at: String,
}

impl TryFrom<EffectivenessRow> for InstructionEffectiveness {
    type Error = crate::Error;

    fn try_from(row: EffectivenessRow) -> Result<Self> {
        let last_success_at = row.last_success_at
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| crate::Error::Other(e.to_string()))?
            .map(Into::into);

        let last_failure_at = row.last_failure_at
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| crate::Error::Other(e.to_string()))?
            .map(Into::into);

        let last_penalty_at = row.last_penalty_at
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| crate::Error::Other(e.to_string()))?
            .map(Into::into);

        let success_rate = if row.usage_count > 0 {
            row.success_count as f64 / row.usage_count as f64
        } else {
            0.0
        };

        Ok(InstructionEffectiveness {
            instruction_id: row.instruction_id,
            usage_count: row.usage_count,
            success_count: row.success_count,
            failure_count: row.failure_count,
            penalty_score: row.penalty_score,
            avg_completion_time: row.avg_completion_time,
            success_rate,
            last_success_at,
            last_failure_at,
            last_penalty_at,
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct PatternRow {
    id: i64,
    pattern_type: String,
    agent_type: Option<String>,
    pattern_signature: String,
    pattern_data: String,
    occurrence_count: i64,
    first_seen_at: String,
    last_seen_at: String,
    instruction_id: Option<i64>,
    status: String,
}

impl TryFrom<PatternRow> for LearningPattern {
    type Error = crate::Error;

    fn try_from(row: PatternRow) -> Result<Self> {
        let agent_type = row.agent_type
            .map(|s| AgentType::from_str(&s))
            .transpose()?;

        Ok(LearningPattern {
            id: row.id,
            pattern_type: PatternType::from_str(&row.pattern_type)?,
            agent_type,
            pattern_signature: row.pattern_signature,
            pattern_data: serde_json::from_str(&row.pattern_data)?,
            occurrence_count: row.occurrence_count,
            first_seen_at: chrono::DateTime::parse_from_rfc3339(&row.first_seen_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            last_seen_at: chrono::DateTime::parse_from_rfc3339(&row.last_seen_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            instruction_id: row.instruction_id,
            status: PatternStatus::from_str(&row.status)?,
        })
    }
}
