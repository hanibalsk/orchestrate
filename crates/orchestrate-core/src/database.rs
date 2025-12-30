//! Database layer for SQLite

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

use crate::approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
use crate::experiment::{
    Experiment, ExperimentMetric, ExperimentStatus, ExperimentType, ExperimentVariant,
    VariantResults,
};
use crate::model_selection::{
    ModelPerformance, ModelSelectionConfig, ModelSelectionRule, OptimizationGoal, TaskComplexity,
};
use crate::feedback::{Feedback, FeedbackRating, FeedbackSource, FeedbackStats};
use crate::instruction::{
    CustomInstruction, InstructionEffectiveness, InstructionScope, InstructionSource,
    LearningPattern, PatternStatus, PatternType, SuccessPattern, SuccessPatternType,
};
use crate::network::{AgentId, StepOutput, StepOutputType};
use crate::schedule::{Schedule, ScheduleRun, ScheduleRunStatus};
use crate::webhook::{WebhookEvent, WebhookEventStatus};
use crate::{
    Agent, AgentState, AgentType, Epic, EpicStatus, MergeStrategy, Message, MessageRole, PrStatus,
    PullRequest, Result, Story, StoryStatus,
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
    #[cfg(test)]
    pub(crate) pool: SqlitePool,
    #[cfg(not(test))]
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
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;
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
        sqlx::query(include_str!(
            "../../../migrations/004_custom_instructions.sql"
        ))
        .execute(&self.pool)
        .await?;
        // Token tracking migration - uses ALTER TABLE which may fail if columns exist
        // This is safe because SQLite ALTER TABLE ADD COLUMN is idempotent for this use case
        let _ = sqlx::query(include_str!("../../../migrations/005_token_tracking.sql"))
            .execute(&self.pool)
            .await;
        // Webhook events migration
        sqlx::query(include_str!("../../../migrations/006_webhook_events.sql"))
            .execute(&self.pool)
            .await?;
        // Schedules migration
        sqlx::query(include_str!("../../../migrations/007_schedules.sql"))
            .execute(&self.pool)
            .await?;
        // Pipelines migration
        sqlx::query(include_str!("../../../migrations/008_pipelines.sql"))
            .execute(&self.pool)
            .await?;
        sqlx::query(include_str!("../../../migrations/009_approvals.sql"))
            .execute(&self.pool)
            .await?;
        // Rollback events migration
        sqlx::query(include_str!("../../../migrations/010_rollback_events.sql"))
            .execute(&self.pool)
            .await?;
        // Success patterns migration
        sqlx::query(include_str!("../../../migrations/011_success_patterns.sql"))
            .execute(&self.pool)
            .await?;
        // Feedback migration
        sqlx::query(include_str!("../../../migrations/012_feedback.sql"))
            .execute(&self.pool)
            .await?;
        // Experiments migration
        sqlx::query(include_str!("../../../migrations/013_experiments.sql"))
            .execute(&self.pool)
            .await?;
        // Model selection migration
        sqlx::query(include_str!("../../../migrations/014_model_selection.sql"))
            .execute(&self.pool)
            .await?;
        // Prompt optimization migration
        sqlx::query(include_str!("../../../migrations/015_prompt_optimization.sql"))
            .execute(&self.pool)
            .await?;
        // Incidents migration
        sqlx::query(include_str!("../../../migrations/016_incidents.sql"))
            .execute(&self.pool)
            .await?;
        // Autonomous sessions migration (Epic 016)
        sqlx::query(include_str!(
            "../../../migrations/020_autonomous_sessions.sql"
        ))
        .execute(&self.pool)
        .await?;
        // Agent continuations migration (Epic 016 - Story 3)
        sqlx::query(include_str!(
            "../../../migrations/021_agent_continuations.sql"
        ))
        .execute(&self.pool)
        .await?;
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
        let row = sqlx::query_as::<_, AgentRow>("SELECT * FROM agents WHERE id = ?")
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
    pub async fn update_agent_with_version(
        &self,
        agent: &Agent,
        expected_updated_at: &str,
    ) -> Result<bool> {
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
        let rows = sqlx::query_as::<_, AgentRow>("SELECT * FROM agents ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Worktree Operations ====================

    /// Get worktree path by ID
    pub async fn get_worktree_path(&self, worktree_id: &str) -> Result<Option<String>> {
        let row = sqlx::query_scalar::<_, String>("SELECT path FROM worktrees WHERE id = ?")
            .bind(worktree_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row)
    }

    /// Insert a new worktree
    pub async fn insert_worktree(&self, worktree: &crate::Worktree) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO worktrees (id, name, path, branch_name, base_branch, status, agent_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                branch_name = excluded.branch_name,
                status = excluded.status,
                agent_id = excluded.agent_id
            "#,
        )
        .bind(&worktree.id)
        .bind(&worktree.name)
        .bind(&worktree.path)
        .bind(&worktree.branch_name)
        .bind(&worktree.base_branch)
        .bind(format!("{:?}", worktree.status).to_lowercase())
        .bind(worktree.agent_id.map(|id| id.to_string()))
        .bind(worktree.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
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
        let result =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_messages WHERE agent_id = ?")
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
        let result = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agents")
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
        let row = sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get open session for an agent
    pub async fn get_open_session_for_agent(
        &self,
        agent_id: Uuid,
    ) -> Result<Option<crate::Session>> {
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
        sqlx::query("UPDATE sessions SET total_tokens = ? WHERE id = ?")
            .bind(total_tokens)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Set API session ID for a session
    pub async fn set_api_session_id(&self, session_id: &str, api_session_id: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET api_session_id = ? WHERE id = ?")
            .bind(api_session_id)
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET closed_at = ? WHERE id = ?")
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

    // ==================== Story Operations ====================

    /// Upsert a story
    pub async fn upsert_story(&self, story: &Story) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO stories (id, epic_id, title, description, acceptance_criteria, status, agent_id, created_at, updated_at, completed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                description = excluded.description,
                acceptance_criteria = excluded.acceptance_criteria,
                status = excluded.status,
                agent_id = excluded.agent_id,
                updated_at = excluded.updated_at,
                completed_at = excluded.completed_at
            "#,
        )
        .bind(&story.id)
        .bind(&story.epic_id)
        .bind(&story.title)
        .bind(&story.description)
        .bind(story.acceptance_criteria.as_ref().map(|c| serde_json::to_string(c).ok()).flatten())
        .bind(story.status.as_str())
        .bind(story.agent_id.map(|id| id.to_string()))
        .bind(story.created_at.to_rfc3339())
        .bind(story.updated_at.to_rfc3339())
        .bind(story.completed_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a story by ID
    pub async fn get_story(&self, id: &str) -> Result<Option<Story>> {
        let row = sqlx::query_as::<_, StoryRow>("SELECT * FROM stories WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get all stories for an epic
    pub async fn get_stories_for_epic(&self, epic_id: &str) -> Result<Vec<Story>> {
        let rows = sqlx::query_as::<_, StoryRow>(
            "SELECT * FROM stories WHERE epic_id = ? ORDER BY created_at ASC",
        )
        .bind(epic_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Get pending stories (optionally for a specific epic)
    pub async fn get_pending_stories(&self, epic_id: Option<&str>) -> Result<Vec<Story>> {
        let rows = if let Some(eid) = epic_id {
            sqlx::query_as::<_, StoryRow>(
                "SELECT * FROM stories WHERE epic_id = ? AND status = 'pending' ORDER BY created_at ASC",
            )
            .bind(eid)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, StoryRow>(
                "SELECT * FROM stories WHERE status = 'pending' ORDER BY created_at ASC",
            )
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Update story status
    pub async fn update_story_status(
        &self,
        id: &str,
        status: StoryStatus,
        agent_id: Option<Uuid>,
    ) -> Result<bool> {
        let now = chrono::Utc::now();
        let completed_at = if status == StoryStatus::Completed {
            Some(now.to_rfc3339())
        } else {
            None
        };

        let result = sqlx::query(
            r#"
            UPDATE stories SET
                status = ?,
                agent_id = ?,
                updated_at = ?,
                completed_at = COALESCE(?, completed_at)
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(agent_id.map(|id| id.to_string()))
        .bind(now.to_rfc3339())
        .bind(completed_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List all stories with optional status filter
    pub async fn list_stories(&self, status: Option<StoryStatus>) -> Result<Vec<Story>> {
        let rows = if let Some(s) = status {
            sqlx::query_as::<_, StoryRow>(
                "SELECT * FROM stories WHERE status = ? ORDER BY created_at DESC",
            )
            .bind(s.as_str())
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, StoryRow>("SELECT * FROM stories ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?
        };

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Delete a story
    pub async fn delete_story(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM stories WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
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
    pub async fn get_dependency_outputs(
        &self,
        dependency_agent_ids: &[AgentId],
    ) -> Result<Vec<StepOutput>> {
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
        let placeholders: Vec<String> = dependency_agent_ids
            .iter()
            .map(|_| "?".to_string())
            .collect();
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
    pub async fn get_step_outputs_by_skill(
        &self,
        agent_id: AgentId,
        skill_name: &str,
    ) -> Result<Vec<StepOutput>> {
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
    pub async fn mark_outputs_consumed(
        &self,
        output_ids: &[i64],
        consumed_by: AgentId,
    ) -> Result<u64> {
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

        let placeholders: Vec<String> = dependency_agent_ids
            .iter()
            .map(|_| "?".to_string())
            .collect();
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
        sqlx::query("INSERT INTO instruction_effectiveness (instruction_id) VALUES (?)")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(id)
    }

    /// Get instruction by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instruction(&self, id: i64) -> Result<Option<CustomInstruction>> {
        let row =
            sqlx::query_as::<_, InstructionRow>("SELECT * FROM custom_instructions WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get instruction by name
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instruction_by_name(&self, name: &str) -> Result<Option<CustomInstruction>> {
        let row =
            sqlx::query_as::<_, InstructionRow>("SELECT * FROM custom_instructions WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get all enabled instructions for an agent type (includes global)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instructions_for_agent(
        &self,
        agent_type: AgentType,
    ) -> Result<Vec<CustomInstruction>> {
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
        sqlx::query("UPDATE custom_instructions SET enabled = ?, updated_at = ? WHERE id = ?")
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

        tracing::warn!(
            instruction_id = id,
            penalty = amount,
            reason,
            "Penalty applied"
        );
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
    pub async fn get_instruction_effectiveness(
        &self,
        instruction_id: i64,
    ) -> Result<Option<InstructionEffectiveness>> {
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
            "SELECT id FROM learning_patterns WHERE pattern_signature = ?",
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
        let row = sqlx::query_as::<_, PatternRow>("SELECT * FROM learning_patterns WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get patterns ready for instruction generation
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_patterns_for_review(
        &self,
        min_occurrences: i64,
    ) -> Result<Vec<LearningPattern>> {
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
    pub async fn list_patterns(
        &self,
        status: Option<PatternStatus>,
    ) -> Result<Vec<LearningPattern>> {
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
        sqlx::query("UPDATE learning_patterns SET status = ?, instruction_id = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(instruction_id)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ==================== Success Pattern Operations ====================

    /// Upsert a success pattern (insert or update occurrence count and averages)
    #[tracing::instrument(skip(self, pattern), level = "debug")]
    pub async fn upsert_success_pattern(&self, pattern: &SuccessPattern) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let pattern_type_str = pattern.pattern_type.as_str();
        let agent_type_str = pattern.agent_type.as_ref().map(|a| a.as_str());
        let pattern_data_str = pattern.pattern_data.to_string();

        // Try to find existing pattern by signature
        let existing: Option<SuccessPatternRow> = sqlx::query_as(
            "SELECT * FROM success_patterns WHERE pattern_signature = ?",
        )
        .bind(&pattern.pattern_signature)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(existing) = existing {
            // Update with weighted averages
            let new_count = existing.occurrence_count + 1;

            // Calculate weighted average for completion time
            let new_avg_time = match (existing.avg_completion_time_ms, pattern.avg_completion_time_ms)
            {
                (Some(old), Some(new)) => {
                    Some((old * existing.occurrence_count + new) / new_count)
                }
                (Some(old), None) => Some(old),
                (None, Some(new)) => Some(new),
                (None, None) => None,
            };

            // Calculate weighted average for token usage
            let new_avg_tokens = match (existing.avg_token_usage, pattern.avg_token_usage) {
                (Some(old), Some(new)) => {
                    Some((old * existing.occurrence_count + new) / new_count)
                }
                (Some(old), None) => Some(old),
                (None, Some(new)) => Some(new),
                (None, None) => None,
            };

            sqlx::query(
                r#"
                UPDATE success_patterns
                SET occurrence_count = ?,
                    avg_completion_time_ms = ?,
                    avg_token_usage = ?,
                    last_seen_at = ?
                WHERE id = ?
                "#,
            )
            .bind(new_count)
            .bind(new_avg_time)
            .bind(new_avg_tokens)
            .bind(&now)
            .bind(existing.id)
            .execute(&self.pool)
            .await?;

            Ok(existing.id)
        } else {
            // Insert new pattern
            let result = sqlx::query(
                r#"
                INSERT INTO success_patterns (
                    pattern_type, agent_type, task_type, pattern_signature,
                    pattern_data, occurrence_count, avg_completion_time_ms,
                    avg_token_usage, success_rate, first_seen_at, last_seen_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(pattern_type_str)
            .bind(agent_type_str)
            .bind(&pattern.task_type)
            .bind(&pattern.pattern_signature)
            .bind(&pattern_data_str)
            .bind(pattern.occurrence_count)
            .bind(pattern.avg_completion_time_ms)
            .bind(pattern.avg_token_usage)
            .bind(pattern.success_rate)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;

            Ok(result.last_insert_rowid())
        }
    }

    /// Get a success pattern by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_success_pattern(&self, id: i64) -> Result<Option<SuccessPattern>> {
        let row: Option<SuccessPatternRow> =
            sqlx::query_as("SELECT * FROM success_patterns WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// List success patterns with optional type filter
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_success_patterns(
        &self,
        pattern_type: Option<SuccessPatternType>,
        limit: i64,
    ) -> Result<Vec<SuccessPattern>> {
        let rows: Vec<SuccessPatternRow> = match pattern_type {
            Some(pt) => {
                sqlx::query_as(
                    "SELECT * FROM success_patterns WHERE pattern_type = ? ORDER BY occurrence_count DESC LIMIT ?",
                )
                .bind(pt.as_str())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as(
                    "SELECT * FROM success_patterns ORDER BY occurrence_count DESC LIMIT ?",
                )
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Get success patterns for a specific agent type
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_success_patterns_for_agent(
        &self,
        agent_type: AgentType,
        limit: i64,
    ) -> Result<Vec<SuccessPattern>> {
        let rows: Vec<SuccessPatternRow> = sqlx::query_as(
            r#"
            SELECT * FROM success_patterns
            WHERE agent_type = ? OR agent_type IS NULL
            ORDER BY occurrence_count DESC
            LIMIT ?
            "#,
        )
        .bind(agent_type.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Get success patterns by task type
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_success_patterns_by_task(
        &self,
        task_type: &str,
        limit: i64,
    ) -> Result<Vec<SuccessPattern>> {
        let rows: Vec<SuccessPatternRow> = sqlx::query_as(
            r#"
            SELECT * FROM success_patterns
            WHERE task_type = ? OR task_type IS NULL
            ORDER BY occurrence_count DESC
            LIMIT ?
            "#,
        )
        .bind(task_type)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Cleanup old success patterns that haven't been seen recently
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn cleanup_old_success_patterns(&self, days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        let result = sqlx::query(
            "DELETE FROM success_patterns WHERE last_seen_at < ? AND occurrence_count < 5",
        )
        .bind(&cutoff_str)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }

    // ==================== Feedback Operations ====================

    /// Insert a new feedback record
    #[tracing::instrument(skip(self, feedback), level = "debug")]
    pub async fn insert_feedback(&self, feedback: &Feedback) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO feedback (
                agent_id, message_id, rating, comment, source, created_by, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(feedback.agent_id.to_string())
        .bind(feedback.message_id)
        .bind(feedback.rating.as_str())
        .bind(&feedback.comment)
        .bind(feedback.source.as_str())
        .bind(&feedback.created_by)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get feedback by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_feedback(&self, id: i64) -> Result<Option<Feedback>> {
        let row: Option<FeedbackRow> = sqlx::query_as("SELECT * FROM feedback WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// List feedback for an agent
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_feedback_for_agent(
        &self,
        agent_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Feedback>> {
        let rows: Vec<FeedbackRow> = sqlx::query_as(
            "SELECT * FROM feedback WHERE agent_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(agent_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List all feedback with optional filters
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_feedback(
        &self,
        rating: Option<FeedbackRating>,
        source: Option<FeedbackSource>,
        limit: i64,
    ) -> Result<Vec<Feedback>> {
        let rows: Vec<FeedbackRow> = match (rating, source) {
            (Some(r), Some(s)) => {
                sqlx::query_as(
                    "SELECT * FROM feedback WHERE rating = ? AND source = ? ORDER BY created_at DESC LIMIT ?",
                )
                .bind(r.as_str())
                .bind(s.as_str())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(r), None) => {
                sqlx::query_as(
                    "SELECT * FROM feedback WHERE rating = ? ORDER BY created_at DESC LIMIT ?",
                )
                .bind(r.as_str())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(s)) => {
                sqlx::query_as(
                    "SELECT * FROM feedback WHERE source = ? ORDER BY created_at DESC LIMIT ?",
                )
                .bind(s.as_str())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as("SELECT * FROM feedback ORDER BY created_at DESC LIMIT ?")
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Get feedback statistics for an agent
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_feedback_stats_for_agent(&self, agent_id: Uuid) -> Result<FeedbackStats> {
        let row: Option<FeedbackStatsRow> = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN rating = 'positive' THEN 1 ELSE 0 END) as positive,
                SUM(CASE WHEN rating = 'negative' THEN 1 ELSE 0 END) as negative,
                SUM(CASE WHEN rating = 'neutral' THEN 1 ELSE 0 END) as neutral
            FROM feedback
            WHERE agent_id = ?
            "#,
        )
        .bind(agent_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|r| FeedbackStats::from_counts(r.positive, r.negative, r.neutral))
            .unwrap_or_else(FeedbackStats::empty))
    }

    /// Get overall feedback statistics
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_feedback_stats(&self) -> Result<FeedbackStats> {
        let row: Option<FeedbackStatsRow> = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN rating = 'positive' THEN 1 ELSE 0 END) as positive,
                SUM(CASE WHEN rating = 'negative' THEN 1 ELSE 0 END) as negative,
                SUM(CASE WHEN rating = 'neutral' THEN 1 ELSE 0 END) as neutral
            FROM feedback
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|r| FeedbackStats::from_counts(r.positive, r.negative, r.neutral))
            .unwrap_or_else(FeedbackStats::empty))
    }

    /// Get feedback statistics grouped by agent type
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_feedback_stats_by_agent_type(
        &self,
    ) -> Result<Vec<(AgentType, FeedbackStats)>> {
        let rows: Vec<FeedbackStatsByTypeRow> = sqlx::query_as(
            r#"
            SELECT
                a.agent_type,
                COUNT(*) as total,
                SUM(CASE WHEN f.rating = 'positive' THEN 1 ELSE 0 END) as positive,
                SUM(CASE WHEN f.rating = 'negative' THEN 1 ELSE 0 END) as negative,
                SUM(CASE WHEN f.rating = 'neutral' THEN 1 ELSE 0 END) as neutral
            FROM feedback f
            JOIN agents a ON f.agent_id = a.id
            GROUP BY a.agent_type
            ORDER BY total DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                let agent_type = AgentType::from_str(&r.agent_type)?;
                let stats = FeedbackStats::from_counts(r.positive, r.negative, r.neutral);
                Ok((agent_type, stats))
            })
            .collect()
    }

    /// Delete feedback by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn delete_feedback(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM feedback WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Cleanup old feedback records
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn cleanup_old_feedback(&self, days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        let result = sqlx::query("DELETE FROM feedback WHERE created_at < ?")
            .bind(&cutoff_str)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as usize)
    }

    // ==================== Effectiveness Analysis Operations ====================

    /// Get recent success rate for an instruction (last N days)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_recent_instruction_success_rate(
        &self,
        instruction_id: i64,
        days: i64,
    ) -> Result<Option<f64>> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.to_rfc3339();

        // Check success_patterns for recent outcomes with this instruction
        let row = sqlx::query_as::<_, (i64, i64)>(
            r#"
            SELECT
                COUNT(CASE WHEN sp.outcome = 'success' THEN 1 END) as successes,
                COUNT(*) as total
            FROM success_patterns sp
            WHERE sp.instruction_ids LIKE '%' || ? || '%'
            AND sp.detected_at > ?
            "#,
        )
        .bind(instruction_id)
        .bind(&cutoff_str)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(successes, total)| {
            if total > 0 {
                successes as f64 / total as f64
            } else {
                // Fall back to overall effectiveness
                0.5 // Default neutral if no recent data
            }
        }))
    }

    /// Get feedback score for an instruction (-1.0 to 1.0)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instruction_feedback_score(&self, instruction_id: i64) -> Result<f64> {
        // Get feedback for agents that used this instruction
        let row = sqlx::query_as::<_, (i64, i64, i64)>(
            r#"
            SELECT
                COUNT(CASE WHEN f.rating = 'positive' THEN 1 END) as positive,
                COUNT(CASE WHEN f.rating = 'negative' THEN 1 END) as negative,
                COUNT(CASE WHEN f.rating = 'neutral' THEN 1 END) as neutral
            FROM feedback f
            WHERE f.agent_id IN (
                SELECT DISTINCT a.id FROM agents a
                JOIN sessions s ON a.session_id = s.id
                WHERE s.instructions_used LIKE '%' || ? || '%'
            )
            "#,
        )
        .bind(instruction_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map_or(0.0, |(pos, neg, neu)| {
            let total = pos + neg + neu;
            if total > 0 {
                // Score from -1 (all negative) to +1 (all positive)
                (pos as f64 - neg as f64) / total as f64
            } else {
                0.0
            }
        }))
    }

    /// Get comprehensive effectiveness analysis for an instruction
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_instruction_effectiveness_analysis(
        &self,
        instruction_id: i64,
    ) -> Result<Option<EffectivenessAnalysisRow>> {
        let row = sqlx::query_as::<_, EffectivenessAnalysisRow>(
            r#"
            SELECT
                ci.id as instruction_id,
                ci.name,
                ci.source as instruction_source,
                ci.enabled,
                COALESCE(ie.usage_count, 0) as usage_count,
                COALESCE(ie.success_count, 0) as success_count,
                COALESCE(ie.failure_count, 0) as failure_count,
                COALESCE(ie.penalty_score, 0.0) as penalty_score,
                COALESCE(ie.avg_completion_time, 0.0) as avg_completion_time,
                CASE WHEN COALESCE(ie.usage_count, 0) > 0
                    THEN CAST(ie.success_count AS REAL) / ie.usage_count
                    ELSE 0.0
                END as success_rate,
                ie.last_success_at,
                ie.last_failure_at,
                ie.updated_at
            FROM custom_instructions ci
            LEFT JOIN instruction_effectiveness ie ON ci.id = ie.instruction_id
            WHERE ci.id = ?
            "#,
        )
        .bind(instruction_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// List all instructions with their effectiveness analysis
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_instruction_effectiveness(
        &self,
        include_disabled: bool,
        min_usage: i64,
    ) -> Result<Vec<EffectivenessAnalysisRow>> {
        let query = if include_disabled {
            r#"
            SELECT
                ci.id as instruction_id,
                ci.name,
                ci.source as instruction_source,
                ci.enabled,
                COALESCE(ie.usage_count, 0) as usage_count,
                COALESCE(ie.success_count, 0) as success_count,
                COALESCE(ie.failure_count, 0) as failure_count,
                COALESCE(ie.penalty_score, 0.0) as penalty_score,
                COALESCE(ie.avg_completion_time, 0.0) as avg_completion_time,
                CASE WHEN COALESCE(ie.usage_count, 0) > 0
                    THEN CAST(ie.success_count AS REAL) / ie.usage_count
                    ELSE 0.0
                END as success_rate,
                ie.last_success_at,
                ie.last_failure_at,
                ie.updated_at
            FROM custom_instructions ci
            LEFT JOIN instruction_effectiveness ie ON ci.id = ie.instruction_id
            WHERE COALESCE(ie.usage_count, 0) >= ?
            ORDER BY success_rate ASC, penalty_score DESC
            "#
        } else {
            r#"
            SELECT
                ci.id as instruction_id,
                ci.name,
                ci.source as instruction_source,
                ci.enabled,
                COALESCE(ie.usage_count, 0) as usage_count,
                COALESCE(ie.success_count, 0) as success_count,
                COALESCE(ie.failure_count, 0) as failure_count,
                COALESCE(ie.penalty_score, 0.0) as penalty_score,
                COALESCE(ie.avg_completion_time, 0.0) as avg_completion_time,
                CASE WHEN COALESCE(ie.usage_count, 0) > 0
                    THEN CAST(ie.success_count AS REAL) / ie.usage_count
                    ELSE 0.0
                END as success_rate,
                ie.last_success_at,
                ie.last_failure_at,
                ie.updated_at
            FROM custom_instructions ci
            LEFT JOIN instruction_effectiveness ie ON ci.id = ie.instruction_id
            WHERE ci.enabled = 1 AND COALESCE(ie.usage_count, 0) >= ?
            ORDER BY success_rate ASC, penalty_score DESC
            "#
        };

        let rows = sqlx::query_as::<_, EffectivenessAnalysisRow>(query)
            .bind(min_usage)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows)
    }

    /// List ineffective instructions (low success rate + high penalty)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_ineffective_instructions(
        &self,
        success_rate_threshold: f64,
        min_usage: i64,
    ) -> Result<Vec<EffectivenessAnalysisRow>> {
        let rows = sqlx::query_as::<_, EffectivenessAnalysisRow>(
            r#"
            SELECT
                ci.id as instruction_id,
                ci.name,
                ci.source as instruction_source,
                ci.enabled,
                COALESCE(ie.usage_count, 0) as usage_count,
                COALESCE(ie.success_count, 0) as success_count,
                COALESCE(ie.failure_count, 0) as failure_count,
                COALESCE(ie.penalty_score, 0.0) as penalty_score,
                COALESCE(ie.avg_completion_time, 0.0) as avg_completion_time,
                CASE WHEN COALESCE(ie.usage_count, 0) > 0
                    THEN CAST(ie.success_count AS REAL) / ie.usage_count
                    ELSE 0.0
                END as success_rate,
                ie.last_success_at,
                ie.last_failure_at,
                ie.updated_at
            FROM custom_instructions ci
            LEFT JOIN instruction_effectiveness ie ON ci.id = ie.instruction_id
            WHERE COALESCE(ie.usage_count, 0) >= ?
            AND (
                CASE WHEN COALESCE(ie.usage_count, 0) > 0
                    THEN CAST(ie.success_count AS REAL) / ie.usage_count
                    ELSE 0.0
                END
            ) < ?
            ORDER BY success_rate ASC, penalty_score DESC
            "#,
        )
        .bind(min_usage)
        .bind(success_rate_threshold)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get effectiveness summary stats
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_effectiveness_summary(&self) -> Result<EffectivenessSummary> {
        let row = sqlx::query_as::<_, EffectivenessSummaryRow>(
            r#"
            SELECT
                COUNT(*) as total_instructions,
                COUNT(CASE WHEN ci.enabled = 1 THEN 1 END) as enabled_count,
                COUNT(CASE WHEN ie.usage_count > 0 THEN 1 END) as used_count,
                COALESCE(AVG(CASE WHEN ie.usage_count > 0
                    THEN CAST(ie.success_count AS REAL) / ie.usage_count
                    ELSE NULL END), 0) as avg_success_rate,
                COALESCE(AVG(ie.penalty_score), 0) as avg_penalty_score,
                COALESCE(SUM(ie.usage_count), 0) as total_usage,
                COUNT(CASE WHEN ie.usage_count >= 5 AND
                    CAST(ie.success_count AS REAL) / NULLIF(ie.usage_count, 0) < 0.5 THEN 1 END) as ineffective_count
            FROM custom_instructions ci
            LEFT JOIN instruction_effectiveness ie ON ci.id = ie.instruction_id
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(EffectivenessSummary {
            total_instructions: row.total_instructions,
            enabled_count: row.enabled_count,
            used_count: row.used_count,
            avg_success_rate: row.avg_success_rate,
            avg_penalty_score: row.avg_penalty_score,
            total_usage: row.total_usage,
            ineffective_count: row.ineffective_count,
        })
    }

    // ==================== Experiment Operations ====================

    /// Create a new experiment
    #[tracing::instrument(skip(self, experiment), level = "debug", fields(name = %experiment.name))]
    pub async fn create_experiment(&self, experiment: &Experiment) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO experiments (name, description, hypothesis, experiment_type, metric, agent_type, status, min_samples, confidence_level, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&experiment.name)
        .bind(&experiment.description)
        .bind(&experiment.hypothesis)
        .bind(experiment.experiment_type.as_str())
        .bind(experiment.metric.as_str())
        .bind(&experiment.agent_type)
        .bind(experiment.status.as_str())
        .bind(experiment.min_samples)
        .bind(experiment.confidence_level)
        .bind(experiment.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get an experiment by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_experiment(&self, id: i64) -> Result<Option<Experiment>> {
        let row = sqlx::query_as::<_, ExperimentRow>("SELECT * FROM experiments WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get an experiment by name
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_experiment_by_name(&self, name: &str) -> Result<Option<Experiment>> {
        let row = sqlx::query_as::<_, ExperimentRow>("SELECT * FROM experiments WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// List experiments with optional status filter
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_experiments(
        &self,
        status: Option<ExperimentStatus>,
        limit: i64,
    ) -> Result<Vec<Experiment>> {
        let rows = if let Some(status) = status {
            sqlx::query_as::<_, ExperimentRow>(
                "SELECT * FROM experiments WHERE status = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(status.as_str())
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ExperimentRow>(
                "SELECT * FROM experiments ORDER BY created_at DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Update experiment status
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn update_experiment_status(
        &self,
        id: i64,
        status: ExperimentStatus,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        let (started_update, completed_update) = match status {
            ExperimentStatus::Running => (", started_at = ?", ""),
            ExperimentStatus::Completed | ExperimentStatus::Cancelled => ("", ", completed_at = ?"),
            _ => ("", ""),
        };

        let query = format!(
            "UPDATE experiments SET status = ?{}{} WHERE id = ?",
            started_update, completed_update
        );

        let mut q = sqlx::query(&query).bind(status.as_str());

        if !started_update.is_empty() || !completed_update.is_empty() {
            q = q.bind(&now);
        }

        q.bind(id).execute(&self.pool).await?;

        Ok(())
    }

    /// Set experiment winner
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn set_experiment_winner(&self, experiment_id: i64, variant_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE experiments SET winner_variant_id = ?, status = 'completed', completed_at = ? WHERE id = ?",
        )
        .bind(variant_id)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(experiment_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create an experiment variant
    #[tracing::instrument(skip(self, variant), level = "debug", fields(name = %variant.name))]
    pub async fn create_experiment_variant(&self, variant: &ExperimentVariant) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO experiment_variants (experiment_id, name, description, is_control, weight, config, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(variant.experiment_id)
        .bind(&variant.name)
        .bind(&variant.description)
        .bind(variant.is_control)
        .bind(variant.weight)
        .bind(variant.config.to_string())
        .bind(variant.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get variants for an experiment
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_experiment_variants(&self, experiment_id: i64) -> Result<Vec<ExperimentVariant>> {
        let rows = sqlx::query_as::<_, ExperimentVariantRow>(
            "SELECT * FROM experiment_variants WHERE experiment_id = ? ORDER BY is_control DESC, id",
        )
        .bind(experiment_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Assign an agent to a variant (random weighted selection)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn assign_agent_to_experiment(
        &self,
        experiment_id: i64,
        agent_id: Uuid,
    ) -> Result<Option<ExperimentVariant>> {
        // Check if already assigned
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT variant_id FROM experiment_assignments WHERE experiment_id = ? AND agent_id = ?",
        )
        .bind(experiment_id)
        .bind(agent_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(variant_id) = existing {
            // Return existing assignment
            let row = sqlx::query_as::<_, ExperimentVariantRow>(
                "SELECT * FROM experiment_variants WHERE id = ?",
            )
            .bind(variant_id)
            .fetch_optional(&self.pool)
            .await?;

            return row.map(|r| r.try_into()).transpose();
        }

        // Get variants and their weights
        let variants = self.get_experiment_variants(experiment_id).await?;
        if variants.is_empty() {
            return Ok(None);
        }

        // Random weighted selection
        let total_weight: i32 = variants.iter().map(|v| v.weight).sum();
        let mut random_value = (rand::random::<f64>() * total_weight as f64) as i32;

        let mut selected_variant = &variants[0];
        for variant in &variants {
            random_value -= variant.weight;
            if random_value <= 0 {
                selected_variant = variant;
                break;
            }
        }

        // Create assignment
        sqlx::query(
            "INSERT INTO experiment_assignments (experiment_id, variant_id, agent_id, assigned_at) VALUES (?, ?, ?, ?)",
        )
        .bind(experiment_id)
        .bind(selected_variant.id)
        .bind(agent_id.to_string())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(Some(selected_variant.clone()))
    }

    /// Record an observation for an experiment assignment
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn record_experiment_observation(
        &self,
        experiment_id: i64,
        agent_id: Uuid,
        metric_name: &str,
        metric_value: f64,
    ) -> Result<()> {
        // Get assignment ID
        let assignment_id = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM experiment_assignments WHERE experiment_id = ? AND agent_id = ?",
        )
        .bind(experiment_id)
        .bind(agent_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let Some(assignment_id) = assignment_id else {
            return Err(crate::Error::Other(format!(
                "No experiment assignment found for agent {}",
                agent_id
            )));
        };

        sqlx::query(
            "INSERT INTO experiment_observations (assignment_id, metric_name, metric_value, recorded_at) VALUES (?, ?, ?, ?)",
        )
        .bind(assignment_id)
        .bind(metric_name)
        .bind(metric_value)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get aggregated results for an experiment
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_experiment_results(&self, experiment_id: i64) -> Result<Vec<VariantResults>> {
        let rows = sqlx::query_as::<_, VariantResultsRow>(
            r#"
            SELECT
                v.id as variant_id,
                v.name as variant_name,
                v.is_control,
                COUNT(o.id) as sample_count,
                COALESCE(AVG(o.metric_value), 0) as mean,
                COALESCE(
                    SQRT(AVG(o.metric_value * o.metric_value) - AVG(o.metric_value) * AVG(o.metric_value)),
                    0
                ) as std_dev,
                COALESCE(MIN(o.metric_value), 0) as min_value,
                COALESCE(MAX(o.metric_value), 0) as max_value,
                SUM(CASE WHEN o.metric_value >= 1.0 THEN 1 ELSE 0 END) as success_count
            FROM experiment_variants v
            LEFT JOIN experiment_assignments a ON v.id = a.variant_id
            LEFT JOIN experiment_observations o ON a.id = o.assignment_id
            WHERE v.experiment_id = ?
            GROUP BY v.id, v.name, v.is_control
            ORDER BY v.is_control DESC, v.id
            "#,
        )
        .bind(experiment_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get count of running experiments for an agent type
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_running_experiments_for_agent_type(
        &self,
        agent_type: &str,
    ) -> Result<Vec<Experiment>> {
        let rows = sqlx::query_as::<_, ExperimentRow>(
            r#"
            SELECT * FROM experiments
            WHERE status = 'running'
            AND (agent_type = ? OR agent_type IS NULL)
            ORDER BY created_at DESC
            "#,
        )
        .bind(agent_type)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Delete an experiment and all related data
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn delete_experiment(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM experiments WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Model Selection Operations ====================

    /// Record or update model performance for a task type
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn record_model_performance(
        &self,
        model: &str,
        task_type: &str,
        agent_type: Option<&str>,
        success: bool,
        tokens: i64,
        cost: f64,
        duration_secs: f64,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let (success_inc, failure_inc) = if success { (1, 0) } else { (0, 1) };

        sqlx::query(
            r#"
            INSERT INTO model_performance (model, task_type, agent_type, success_count, failure_count, total_tokens, total_cost, total_duration_secs, sample_count, last_used_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)
            ON CONFLICT(model, task_type, agent_type) DO UPDATE SET
                success_count = success_count + ?,
                failure_count = failure_count + ?,
                total_tokens = total_tokens + ?,
                total_cost = total_cost + ?,
                total_duration_secs = total_duration_secs + ?,
                sample_count = sample_count + 1,
                last_used_at = ?,
                updated_at = ?
            "#,
        )
        .bind(model)
        .bind(task_type)
        .bind(agent_type)
        .bind(success_inc)
        .bind(failure_inc)
        .bind(tokens)
        .bind(cost)
        .bind(duration_secs)
        .bind(&now)
        .bind(&now)
        .bind(success_inc)
        .bind(failure_inc)
        .bind(tokens)
        .bind(cost)
        .bind(duration_secs)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get model performance statistics
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_model_performance(
        &self,
        task_type: Option<&str>,
        agent_type: Option<&str>,
    ) -> Result<Vec<ModelPerformance>> {
        let rows = if let Some(task) = task_type {
            if let Some(agent) = agent_type {
                sqlx::query_as::<_, ModelPerformanceRow>(
                    "SELECT * FROM model_performance WHERE task_type = ? AND (agent_type = ? OR agent_type IS NULL) ORDER BY sample_count DESC",
                )
                .bind(task)
                .bind(agent)
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as::<_, ModelPerformanceRow>(
                    "SELECT * FROM model_performance WHERE task_type = ? ORDER BY sample_count DESC",
                )
                .bind(task)
                .fetch_all(&self.pool)
                .await?
            }
        } else {
            sqlx::query_as::<_, ModelPerformanceRow>(
                "SELECT * FROM model_performance ORDER BY sample_count DESC",
            )
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// Get the best performing model for a task type
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_best_model_for_task(
        &self,
        task_type: &str,
        agent_type: Option<&str>,
        optimization_goal: OptimizationGoal,
        min_samples: i64,
    ) -> Result<Option<ModelPerformance>> {
        let performances = self.get_model_performance(Some(task_type), agent_type).await?;

        let filtered: Vec<_> = performances
            .into_iter()
            .filter(|p| p.sample_count >= min_samples && p.success_rate >= 0.5)
            .collect();

        if filtered.is_empty() {
            return Ok(None);
        }

        let best = match optimization_goal {
            OptimizationGoal::Cost => filtered.into_iter().max_by(|a, b| {
                a.cost_score()
                    .partial_cmp(&b.cost_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            OptimizationGoal::Quality => filtered.into_iter().max_by(|a, b| {
                a.quality_score()
                    .partial_cmp(&b.quality_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            OptimizationGoal::Balanced => filtered.into_iter().max_by(|a, b| {
                a.balanced_score()
                    .partial_cmp(&b.balanced_score())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        };

        Ok(best)
    }

    /// Create a model selection rule
    #[tracing::instrument(skip(self, rule), level = "debug", fields(name = %rule.name))]
    pub async fn create_model_selection_rule(&self, rule: &ModelSelectionRule) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO model_selection_rules (name, task_type, agent_type, complexity, preferred_model, fallback_model, max_cost, min_success_rate, priority, enabled, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&rule.name)
        .bind(&rule.task_type)
        .bind(&rule.agent_type)
        .bind(rule.complexity.map(|c| c.as_str()))
        .bind(&rule.preferred_model)
        .bind(&rule.fallback_model)
        .bind(rule.max_cost)
        .bind(rule.min_success_rate)
        .bind(rule.priority)
        .bind(rule.enabled)
        .bind(rule.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// List model selection rules
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_model_selection_rules(&self, enabled_only: bool) -> Result<Vec<ModelSelectionRule>> {
        let rows = if enabled_only {
            sqlx::query_as::<_, ModelSelectionRuleRow>(
                "SELECT * FROM model_selection_rules WHERE enabled = 1 ORDER BY priority DESC, id",
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ModelSelectionRuleRow>(
                "SELECT * FROM model_selection_rules ORDER BY priority DESC, id",
            )
            .fetch_all(&self.pool)
            .await?
        };

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Find matching model selection rule for a task
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn find_matching_rule(
        &self,
        task_type: &str,
        agent_type: Option<&str>,
        complexity: Option<TaskComplexity>,
    ) -> Result<Option<ModelSelectionRule>> {
        let rules = self.list_model_selection_rules(true).await?;

        // Find best matching rule (highest priority first)
        for rule in rules {
            // Check task_type match
            if let Some(ref rule_task) = rule.task_type {
                if rule_task != task_type {
                    continue;
                }
            }

            // Check agent_type match
            if let Some(ref rule_agent) = rule.agent_type {
                if agent_type.map_or(true, |a| a != rule_agent) {
                    continue;
                }
            }

            // Check complexity match
            if let Some(rule_complexity) = rule.complexity {
                if complexity.map_or(true, |c| c != rule_complexity) {
                    continue;
                }
            }

            return Ok(Some(rule));
        }

        Ok(None)
    }

    /// Get model selection config
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_model_selection_config(&self) -> Result<ModelSelectionConfig> {
        let row = sqlx::query_as::<_, ModelSelectionConfigRow>(
            "SELECT * FROM model_selection_config WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into()).unwrap_or_default())
    }

    /// Update model selection config
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn update_model_selection_config(&self, config: &ModelSelectionConfig) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO model_selection_config (id, optimization_goal, max_cost_per_task, min_success_rate, min_samples_for_auto, enabled, updated_at)
            VALUES (1, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                optimization_goal = ?,
                max_cost_per_task = ?,
                min_success_rate = ?,
                min_samples_for_auto = ?,
                enabled = ?,
                updated_at = ?
            "#,
        )
        .bind(config.optimization_goal.as_str())
        .bind(config.max_cost_per_task)
        .bind(config.min_success_rate)
        .bind(config.min_samples_for_auto)
        .bind(config.enabled)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(config.optimization_goal.as_str())
        .bind(config.max_cost_per_task)
        .bind(config.min_success_rate)
        .bind(config.min_samples_for_auto)
        .bind(config.enabled)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a model selection rule
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn delete_model_selection_rule(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM model_selection_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
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
        let (input_price, output_price, cache_read_price, cache_write_price) =
            if model.contains("opus") {
                // Claude Opus 4
                (15.0, 75.0, 1.5, 18.75) // cache read 90% off, cache write 25% premium
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

    // ==================== Schedule Operations ====================

    /// Insert a new schedule
    #[tracing::instrument(skip(self, schedule), level = "debug", fields(name = %schedule.name))]
    pub async fn insert_schedule(&self, schedule: &Schedule) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO schedules (name, cron_expression, agent_type, task, enabled, last_run, next_run, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&schedule.name)
        .bind(&schedule.cron_expression)
        .bind(&schedule.agent_type)
        .bind(&schedule.task)
        .bind(schedule.enabled)
        .bind(schedule.last_run.map(|dt| dt.to_rfc3339()))
        .bind(schedule.next_run.map(|dt| dt.to_rfc3339()))
        .bind(schedule.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get a schedule by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_schedule(&self, id: i64) -> Result<Option<Schedule>> {
        let row = sqlx::query_as::<_, ScheduleRow>("SELECT * FROM schedules WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get a schedule by name
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_schedule_by_name(&self, name: &str) -> Result<Option<Schedule>> {
        let row = sqlx::query_as::<_, ScheduleRow>("SELECT * FROM schedules WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// List all schedules with optional enabled filter
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn list_schedules(&self, enabled_only: bool) -> Result<Vec<Schedule>> {
        let rows = if enabled_only {
            sqlx::query_as::<_, ScheduleRow>(
                "SELECT * FROM schedules WHERE enabled = 1 ORDER BY created_at DESC",
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ScheduleRow>("SELECT * FROM schedules ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?
        };

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Update a schedule
    #[tracing::instrument(skip(self, schedule), level = "debug", fields(id = schedule.id))]
    pub async fn update_schedule(&self, schedule: &Schedule) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE schedules SET
                name = ?, cron_expression = ?, agent_type = ?, task = ?,
                enabled = ?, last_run = ?, next_run = ?
            WHERE id = ?
            "#,
        )
        .bind(&schedule.name)
        .bind(&schedule.cron_expression)
        .bind(&schedule.agent_type)
        .bind(&schedule.task)
        .bind(schedule.enabled)
        .bind(schedule.last_run.map(|dt| dt.to_rfc3339()))
        .bind(schedule.next_run.map(|dt| dt.to_rfc3339()))
        .bind(schedule.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a schedule
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn delete_schedule(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM schedules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get schedules that are due for execution
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_due_schedules(&self) -> Result<Vec<Schedule>> {
        let now = chrono::Utc::now().to_rfc3339();
        let rows = sqlx::query_as::<_, ScheduleRow>(
            "SELECT * FROM schedules WHERE enabled = 1 AND next_run <= ? ORDER BY next_run ASC",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Try to acquire a lock for schedule execution
    ///
    /// Returns true if lock was acquired, false if schedule is already locked.
    /// Locks expire after 5 minutes to prevent deadlocks.
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn try_lock_schedule(&self, schedule_id: i64) -> Result<bool> {
        let now = chrono::Utc::now();
        let lock_expiry = now - chrono::Duration::minutes(5);

        // Try to acquire lock using UPDATE with WHERE clause
        // This will only update if the schedule is not locked or lock has expired
        let result = sqlx::query(
            r#"
            UPDATE schedules
            SET locked_at = ?
            WHERE id = ?
            AND (locked_at IS NULL OR locked_at < ?)
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(schedule_id)
        .bind(lock_expiry.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Release a schedule lock
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn unlock_schedule(&self, schedule_id: i64) -> Result<()> {
        sqlx::query("UPDATE schedules SET locked_at = NULL WHERE id = ?")
            .bind(schedule_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Insert a schedule run record
    #[tracing::instrument(skip(self, run), level = "debug", fields(schedule_id = run.schedule_id))]
    pub async fn insert_schedule_run(&self, run: &ScheduleRun) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO schedule_runs (schedule_id, agent_id, started_at, completed_at, status, error_message)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(run.schedule_id)
        .bind(&run.agent_id)
        .bind(run.started_at.to_rfc3339())
        .bind(run.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(run.status.as_str())
        .bind(&run.error_message)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Update a schedule run
    #[tracing::instrument(skip(self, run), level = "debug", fields(id = run.id))]
    pub async fn update_schedule_run(&self, run: &ScheduleRun) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE schedule_runs SET
                agent_id = ?, completed_at = ?, status = ?, error_message = ?
            WHERE id = ?
            "#,
        )
        .bind(&run.agent_id)
        .bind(run.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(run.status.as_str())
        .bind(&run.error_message)
        .bind(run.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get schedule runs for a schedule
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_schedule_runs(&self, schedule_id: i64, limit: i64) -> Result<Vec<ScheduleRun>> {
        let rows = sqlx::query_as::<_, ScheduleRunRow>(
            "SELECT * FROM schedule_runs WHERE schedule_id = ? ORDER BY started_at DESC LIMIT ?",
        )
        .bind(schedule_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Webhook Event Operations ====================

    /// Insert a new webhook event (idempotent by delivery_id)
    #[tracing::instrument(skip(self, event), level = "debug", fields(delivery_id = %event.delivery_id))]
    pub async fn insert_webhook_event(&self, event: &WebhookEvent) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO webhook_events (
                delivery_id, event_type, payload, status, retry_count, max_retries,
                error_message, next_retry_at, received_at, processed_at, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(delivery_id) DO NOTHING
            "#,
        )
        .bind(&event.delivery_id)
        .bind(&event.event_type)
        .bind(&event.payload)
        .bind(event.status.as_str())
        .bind(event.retry_count)
        .bind(event.max_retries)
        .bind(&event.error_message)
        .bind(event.next_retry_at.map(|dt| dt.to_rfc3339()))
        .bind(event.received_at.to_rfc3339())
        .bind(event.processed_at.map(|dt| dt.to_rfc3339()))
        .bind(event.created_at.to_rfc3339())
        .bind(event.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        // If insert was ignored due to conflict, fetch the existing ID
        if result.rows_affected() == 0 {
            let id = sqlx::query_scalar::<_, i64>(
                "SELECT id FROM webhook_events WHERE delivery_id = ?",
            )
            .bind(&event.delivery_id)
            .fetch_one(&self.pool)
            .await?;
            Ok(id)
        } else {
            Ok(result.last_insert_rowid())
        }
    }

    /// Get webhook event by ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_webhook_event(&self, id: i64) -> Result<Option<WebhookEvent>> {
        let row = sqlx::query_as::<_, WebhookEventRow>(
            "SELECT * FROM webhook_events WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get webhook event by delivery ID
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_webhook_event_by_delivery_id(
        &self,
        delivery_id: &str,
    ) -> Result<Option<WebhookEvent>> {
        let row = sqlx::query_as::<_, WebhookEventRow>(
            "SELECT * FROM webhook_events WHERE delivery_id = ?",
        )
        .bind(delivery_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get pending webhook events ready for processing
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_pending_webhook_events(&self, limit: i64) -> Result<Vec<WebhookEvent>> {
        let now = chrono::Utc::now().to_rfc3339();
        let rows = sqlx::query_as::<_, WebhookEventRow>(
            r#"
            SELECT * FROM webhook_events
            WHERE status = 'pending'
            AND (next_retry_at IS NULL OR next_retry_at <= ?)
            ORDER BY received_at ASC
            LIMIT ?
            "#,
        )
        .bind(&now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Update webhook event status and metadata
    #[tracing::instrument(skip(self, event), level = "debug", fields(id = event.id))]
    pub async fn update_webhook_event(&self, event: &WebhookEvent) -> Result<()> {
        let id = event.id.ok_or_else(|| {
            crate::Error::Other("Cannot update webhook event without ID".to_string())
        })?;

        sqlx::query(
            r#"
            UPDATE webhook_events SET
                status = ?,
                retry_count = ?,
                error_message = ?,
                next_retry_at = ?,
                processed_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(event.status.as_str())
        .bind(event.retry_count)
        .bind(&event.error_message)
        .bind(event.next_retry_at.map(|dt| dt.to_rfc3339()))
        .bind(event.processed_at.map(|dt| dt.to_rfc3339()))
        .bind(event.updated_at.to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get webhook events by status
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_webhook_events_by_status(
        &self,
        status: WebhookEventStatus,
        limit: i64,
    ) -> Result<Vec<WebhookEvent>> {
        let rows = sqlx::query_as::<_, WebhookEventRow>(
            r#"
            SELECT * FROM webhook_events
            WHERE status = ?
            ORDER BY received_at DESC
            LIMIT ?
            "#,
        )
        .bind(status.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Count webhook events by status
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn count_webhook_events_by_status(
        &self,
        status: WebhookEventStatus,
    ) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM webhook_events WHERE status = ?",
        )
        .bind(status.as_str())
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Delete old completed webhook events (for cleanup)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn delete_old_webhook_events(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM webhook_events
            WHERE status IN ('completed', 'dead_letter')
            AND received_at < datetime('now', '-' || ? || ' days')
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get recent webhook events (all statuses)
    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn get_recent_webhook_events(&self, limit: i64) -> Result<Vec<WebhookEvent>> {
        let rows = sqlx::query_as::<_, WebhookEventRow>(
            r#"
            SELECT * FROM webhook_events
            ORDER BY received_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Pipeline Operations ====================

    /// Insert a new pipeline
    pub async fn insert_pipeline(&self, pipeline: &crate::Pipeline) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO pipelines (name, definition, enabled, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&pipeline.name)
        .bind(&pipeline.definition)
        .bind(pipeline.enabled as i32)
        .bind(pipeline.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get pipeline by ID
    pub async fn get_pipeline(&self, id: i64) -> Result<Option<crate::Pipeline>> {
        let row = sqlx::query_as::<_, PipelineRow>("SELECT * FROM pipelines WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get pipeline by name
    pub async fn get_pipeline_by_name(&self, name: &str) -> Result<Option<crate::Pipeline>> {
        let row = sqlx::query_as::<_, PipelineRow>("SELECT * FROM pipelines WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Update pipeline
    pub async fn update_pipeline(&self, pipeline: &crate::Pipeline) -> Result<()> {
        let id = pipeline
            .id
            .ok_or_else(|| crate::Error::Other("Cannot update pipeline without ID".to_string()))?;

        sqlx::query(
            r#"
            UPDATE pipelines SET
                name = ?,
                definition = ?,
                enabled = ?
            WHERE id = ?
            "#,
        )
        .bind(&pipeline.name)
        .bind(&pipeline.definition)
        .bind(pipeline.enabled as i32)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List all pipelines
    pub async fn list_pipelines(&self) -> Result<Vec<crate::Pipeline>> {
        let rows = sqlx::query_as::<_, PipelineRow>("SELECT * FROM pipelines ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List enabled pipelines
    pub async fn list_enabled_pipelines(&self) -> Result<Vec<crate::Pipeline>> {
        let rows = sqlx::query_as::<_, PipelineRow>(
            "SELECT * FROM pipelines WHERE enabled = 1 ORDER BY name ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Delete pipeline
    pub async fn delete_pipeline(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM pipelines WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ==================== Pipeline Run Operations ====================

    /// Insert a new pipeline run
    pub async fn insert_pipeline_run(&self, run: &crate::PipelineRun) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO pipeline_runs (
                pipeline_id, status, trigger_event, started_at, completed_at, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(run.pipeline_id)
        .bind(run.status.as_str())
        .bind(&run.trigger_event)
        .bind(run.started_at.map(|dt| dt.to_rfc3339()))
        .bind(run.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(run.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get pipeline run by ID
    pub async fn get_pipeline_run(&self, id: i64) -> Result<Option<crate::PipelineRun>> {
        let row = sqlx::query_as::<_, PipelineRunRow>("SELECT * FROM pipeline_runs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Update pipeline run
    pub async fn update_pipeline_run(&self, run: &crate::PipelineRun) -> Result<()> {
        let id = run
            .id
            .ok_or_else(|| crate::Error::Other("Cannot update pipeline run without ID".to_string()))?;

        sqlx::query(
            r#"
            UPDATE pipeline_runs SET
                status = ?,
                started_at = ?,
                completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(run.status.as_str())
        .bind(run.started_at.map(|dt| dt.to_rfc3339()))
        .bind(run.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List pipeline runs for a pipeline
    pub async fn list_pipeline_runs(&self, pipeline_id: i64) -> Result<Vec<crate::PipelineRun>> {
        let rows = sqlx::query_as::<_, PipelineRunRow>(
            "SELECT * FROM pipeline_runs WHERE pipeline_id = ? ORDER BY created_at DESC",
        )
        .bind(pipeline_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List pipeline runs by status
    pub async fn list_pipeline_runs_by_status(
        &self,
        status: crate::PipelineRunStatus,
    ) -> Result<Vec<crate::PipelineRun>> {
        let rows = sqlx::query_as::<_, PipelineRunRow>(
            "SELECT * FROM pipeline_runs WHERE status = ? ORDER BY created_at DESC",
        )
        .bind(status.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Pipeline Stage Operations ====================

    /// Insert a new pipeline stage
    pub async fn insert_pipeline_stage(&self, stage: &crate::PipelineStage) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO pipeline_stages (
                run_id, stage_name, status, agent_id, started_at, completed_at, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(stage.run_id)
        .bind(&stage.stage_name)
        .bind(stage.status.as_str())
        .bind(&stage.agent_id)
        .bind(stage.started_at.map(|dt| dt.to_rfc3339()))
        .bind(stage.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(stage.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get pipeline stage by ID
    pub async fn get_pipeline_stage(&self, id: i64) -> Result<Option<crate::PipelineStage>> {
        let row =
            sqlx::query_as::<_, PipelineStageRow>("SELECT * FROM pipeline_stages WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get pipeline stage by name within a run
    pub async fn get_pipeline_stage_by_name(
        &self,
        run_id: i64,
        stage_name: &str,
    ) -> Result<Option<crate::PipelineStage>> {
        let row = sqlx::query_as::<_, PipelineStageRow>(
            "SELECT * FROM pipeline_stages WHERE run_id = ? AND stage_name = ?",
        )
        .bind(run_id)
        .bind(stage_name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Update pipeline stage
    pub async fn update_pipeline_stage(&self, stage: &crate::PipelineStage) -> Result<()> {
        let id = stage.id.ok_or_else(|| {
            crate::Error::Other("Cannot update pipeline stage without ID".to_string())
        })?;

        sqlx::query(
            r#"
            UPDATE pipeline_stages SET
                status = ?,
                agent_id = ?,
                started_at = ?,
                completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(stage.status.as_str())
        .bind(&stage.agent_id)
        .bind(stage.started_at.map(|dt| dt.to_rfc3339()))
        .bind(stage.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List pipeline stages for a run
    pub async fn list_pipeline_stages(&self, run_id: i64) -> Result<Vec<crate::PipelineStage>> {
        let rows = sqlx::query_as::<_, PipelineStageRow>(
            "SELECT * FROM pipeline_stages WHERE run_id = ? ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List pipeline stages by status within a run
    pub async fn list_pipeline_stages_by_status(
        &self,
        run_id: i64,
        status: crate::PipelineStageStatus,
    ) -> Result<Vec<crate::PipelineStage>> {
        let rows = sqlx::query_as::<_, PipelineStageRow>(
            "SELECT * FROM pipeline_stages WHERE run_id = ? AND status = ? ORDER BY created_at ASC",
        )
        .bind(run_id)
        .bind(status.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // Rollback event operations

    /// Insert a rollback event
    pub async fn insert_rollback_event(&self, event: &crate::RollbackEvent) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO rollback_events
                (run_id, failed_stage_name, rollback_to_stage, trigger_type, status,
                 error_message, started_at, completed_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.run_id)
        .bind(&event.failed_stage_name)
        .bind(&event.rollback_to_stage)
        .bind(event.trigger_type.as_str())
        .bind(event.status.as_str())
        .bind(&event.error_message)
        .bind(event.started_at.map(|t| t.to_rfc3339()))
        .bind(event.completed_at.map(|t| t.to_rfc3339()))
        .bind(event.created_at.map(|t| t.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get a rollback event by ID
    pub async fn get_rollback_event(&self, id: i64) -> Result<Option<crate::RollbackEvent>> {
        let row = sqlx::query_as::<_, RollbackEventRow>(
            "SELECT * FROM rollback_events WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Update a rollback event
    pub async fn update_rollback_event(&self, event: &crate::RollbackEvent) -> Result<()> {
        let id = event.id.ok_or_else(|| {
            crate::Error::Other("Cannot update rollback event without an ID".to_string())
        })?;

        sqlx::query(
            r#"
            UPDATE rollback_events
            SET status = ?, error_message = ?, started_at = ?, completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(event.status.as_str())
        .bind(&event.error_message)
        .bind(event.started_at.map(|t| t.to_rfc3339()))
        .bind(event.completed_at.map(|t| t.to_rfc3339()))
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List all rollback events for a pipeline run
    pub async fn list_rollback_events(&self, run_id: i64) -> Result<Vec<crate::RollbackEvent>> {
        let rows = sqlx::query_as::<_, RollbackEventRow>(
            "SELECT * FROM rollback_events WHERE run_id = ? ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Count rollback events for a specific stage in a run (for loop prevention)
    pub async fn count_rollback_events_for_stage(
        &self,
        run_id: i64,
        stage_name: &str,
    ) -> Result<i64> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM rollback_events WHERE run_id = ? AND rollback_to_stage = ?",
        )
        .bind(run_id)
        .bind(stage_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
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
            parent_agent_id: row
                .parent_agent_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            worktree_id: row.worktree_id,
            error_message: row.error_message,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            completed_at: row
                .completed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
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
            agent_id: Uuid::parse_str(&row.agent_id)
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            parent_id: row.parent_id,
            api_session_id: row.api_session_id,
            total_tokens: row.total_tokens,
            is_forked: row.is_forked,
            forked_at: row
                .forked_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            closed_at: row
                .closed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
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
            agent_id: Uuid::parse_str(&row.agent_id)
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            role: MessageRole::from_str(&row.role)?,
            content: row.content,
            tool_calls: row
                .tool_calls
                .map(|s| serde_json::from_str(&s))
                .transpose()?,
            tool_results: row
                .tool_results
                .map(|s| serde_json::from_str(&s))
                .transpose()?,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
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
            agent_id: row
                .agent_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            error_message: row.error_message,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            merged_at: row
                .merged_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
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
            current_phase: row
                .current_phase
                .map(|p| serde_json::from_str(&format!("\"{}\"", p)))
                .transpose()?,
            agent_id: row
                .agent_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            pr_id: row.pr_id,
            error_message: row.error_message,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            completed_at: row
                .completed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
        })
    }
}

#[derive(sqlx::FromRow)]
struct StoryRow {
    id: String,
    epic_id: String,
    title: String,
    description: Option<String>,
    acceptance_criteria: Option<String>,
    status: String,
    agent_id: Option<String>,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
}

impl TryFrom<StoryRow> for Story {
    type Error = crate::Error;

    fn try_from(row: StoryRow) -> Result<Self> {
        Ok(Story {
            id: row.id,
            epic_id: row.epic_id,
            title: row.title,
            description: row.description,
            acceptance_criteria: row
                .acceptance_criteria
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            status: StoryStatus::from_str(&row.status)?,
            agent_id: row
                .agent_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            completed_at: row
                .completed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
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

        let agent_uuid =
            Uuid::parse_str(&row.agent_id).map_err(|e| crate::Error::Other(e.to_string()))?;

        let consumed_by = row
            .consumed_by
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|e| crate::Error::Other(e.to_string()))?
            .map(AgentId::from_uuid);

        let consumed_at = row
            .consumed_at
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
        let agent_type = row
            .agent_type
            .map(|s| AgentType::from_str(&s))
            .transpose()?;

        let tags: Vec<String> = row
            .tags
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
        let last_success_at = row
            .last_success_at
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| crate::Error::Other(e.to_string()))?
            .map(Into::into);

        let last_failure_at = row
            .last_failure_at
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| crate::Error::Other(e.to_string()))?
            .map(Into::into);

        let last_penalty_at = row
            .last_penalty_at
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
        let agent_type = row
            .agent_type
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

#[derive(sqlx::FromRow)]
struct ScheduleRow {
    id: i64,
    name: String,
    cron_expression: String,
    agent_type: String,
    task: String,
    enabled: bool,
    last_run: Option<String>,
    next_run: Option<String>,
    created_at: String,
}

impl TryFrom<ScheduleRow> for Schedule {
    type Error = crate::Error;

    fn try_from(row: ScheduleRow) -> Result<Self> {
        Ok(Schedule {
            id: row.id,
            name: row.name,
            cron_expression: row.cron_expression,
            agent_type: row.agent_type,
            task: row.task,
            enabled: row.enabled,
            last_run: row
                .last_run
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            next_run: row
                .next_run
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct WebhookEventRow {
    id: i64,
    delivery_id: String,
    event_type: String,
    payload: String,
    status: String,
    retry_count: i32,
    max_retries: i32,
    error_message: Option<String>,
    next_retry_at: Option<String>,
    received_at: String,
    processed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<WebhookEventRow> for WebhookEvent {
    type Error = crate::Error;

    fn try_from(row: WebhookEventRow) -> Result<Self> {
        use std::str::FromStr;

        Ok(WebhookEvent {
            id: Some(row.id),
            delivery_id: row.delivery_id,
            event_type: row.event_type,
            payload: row.payload,
            status: WebhookEventStatus::from_str(&row.status)?,
            retry_count: row.retry_count,
            max_retries: row.max_retries,
            error_message: row.error_message,
            next_retry_at: row
                .next_retry_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            received_at: chrono::DateTime::parse_from_rfc3339(&row.received_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            processed_at: row
                .processed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct ScheduleRunRow {
    id: i64,
    schedule_id: i64,
    agent_id: Option<String>,
    started_at: String,
    completed_at: Option<String>,
    status: String,
    error_message: Option<String>,
}

impl TryFrom<ScheduleRunRow> for ScheduleRun {
    type Error = crate::Error;

    fn try_from(row: ScheduleRunRow) -> Result<Self> {
        use std::str::FromStr;

        Ok(ScheduleRun {
            id: row.id,
            schedule_id: row.schedule_id,
            agent_id: row.agent_id,
            started_at: chrono::DateTime::parse_from_rfc3339(&row.started_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            completed_at: row
                .completed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            status: ScheduleRunStatus::from_str(&row.status)?,
            error_message: row.error_message,
        })
    }
}

// ==================== Pipeline Row Structs ====================

#[derive(sqlx::FromRow)]
struct PipelineRow {
    id: i64,
    name: String,
    definition: String,
    enabled: i32,
    created_at: String,
}

impl TryFrom<PipelineRow> for crate::Pipeline {
    type Error = crate::Error;

    fn try_from(row: PipelineRow) -> Result<Self> {
        Ok(crate::Pipeline {
            id: Some(row.id),
            name: row.name,
            definition: row.definition,
            enabled: row.enabled != 0,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct PipelineRunRow {
    id: i64,
    pipeline_id: i64,
    status: String,
    trigger_event: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
}

impl TryFrom<PipelineRunRow> for crate::PipelineRun {
    type Error = crate::Error;

    fn try_from(row: PipelineRunRow) -> Result<Self> {
        use std::str::FromStr;

        Ok(crate::PipelineRun {
            id: Some(row.id),
            pipeline_id: row.pipeline_id,
            status: crate::PipelineRunStatus::from_str(&row.status)?,
            trigger_event: row.trigger_event,
            started_at: row
                .started_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            completed_at: row
                .completed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct PipelineStageRow {
    id: i64,
    run_id: i64,
    stage_name: String,
    status: String,
    agent_id: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
}

impl TryFrom<PipelineStageRow> for crate::PipelineStage {
    type Error = crate::Error;

    fn try_from(row: PipelineStageRow) -> Result<Self> {
        use std::str::FromStr;

        Ok(crate::PipelineStage {
            id: Some(row.id),
            run_id: row.run_id,
            stage_name: row.stage_name,
            status: crate::PipelineStageStatus::from_str(&row.status)?,
            agent_id: row.agent_id,
            started_at: row
                .started_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            completed_at: row
                .completed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct RollbackEventRow {
    id: i64,
    run_id: i64,
    failed_stage_name: String,
    rollback_to_stage: String,
    trigger_type: String,
    status: String,
    error_message: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: Option<String>,
}

impl TryFrom<RollbackEventRow> for crate::RollbackEvent {
    type Error = crate::Error;

    fn try_from(row: RollbackEventRow) -> Result<Self> {
        use std::str::FromStr;

        Ok(crate::RollbackEvent {
            id: Some(row.id),
            run_id: row.run_id,
            failed_stage_name: row.failed_stage_name,
            rollback_to_stage: row.rollback_to_stage,
            trigger_type: crate::RollbackTriggerType::from_str(&row.trigger_type)?,
            status: crate::RollbackStatus::from_str(&row.status)?,
            error_message: row.error_message,
            started_at: row
                .started_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            completed_at: row
                .completed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            created_at: row
                .created_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
        })
    }
}

impl Database {
    // Approval Operations

    /// Create a new approval request
    pub async fn create_approval_request(
        &self,
        request: ApprovalRequest,
    ) -> Result<ApprovalRequest> {
        let mut request = request;
        let result = sqlx::query(
            r#"
            INSERT INTO approval_requests
            (stage_id, run_id, status, required_approvers, required_count,
             approval_count, rejection_count, timeout_seconds, timeout_action,
             timeout_at, resolved_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(request.stage_id)
        .bind(request.run_id)
        .bind(request.status.as_str())
        .bind(&request.required_approvers)
        .bind(request.required_count)
        .bind(request.approval_count)
        .bind(request.rejection_count)
        .bind(request.timeout_seconds)
        .bind(&request.timeout_action)
        .bind(request.timeout_at.map(|t| t.to_rfc3339()))
        .bind(request.resolved_at.map(|t| t.to_rfc3339()))
        .bind(request.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        request.id = Some(result.last_insert_rowid());
        Ok(request)
    }

    /// Get an approval request by ID
    pub async fn get_approval_request(&self, id: i64) -> Result<Option<ApprovalRequest>> {
        let row = sqlx::query_as::<_, ApprovalRequestRow>(
            r#"
            SELECT id, stage_id, run_id, status, required_approvers, required_count,
                   approval_count, rejection_count, timeout_seconds, timeout_action,
                   timeout_at, resolved_at, created_at
            FROM approval_requests
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get approval request by stage ID
    pub async fn get_approval_request_by_stage(
        &self,
        stage_id: i64,
    ) -> Result<Option<ApprovalRequest>> {
        let row = sqlx::query_as::<_, ApprovalRequestRow>(
            r#"
            SELECT id, stage_id, run_id, status, required_approvers, required_count,
                   approval_count, rejection_count, timeout_seconds, timeout_action,
                   timeout_at, resolved_at, created_at
            FROM approval_requests
            WHERE stage_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(stage_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Update an approval request
    pub async fn update_approval_request(&self, request: &ApprovalRequest) -> Result<()> {
        let id = request
            .id
            .ok_or_else(|| crate::Error::Other("Approval request ID is required".to_string()))?;

        sqlx::query(
            r#"
            UPDATE approval_requests
            SET status = ?, approval_count = ?, rejection_count = ?,
                required_approvers = ?, resolved_at = ?
            WHERE id = ?
            "#,
        )
        .bind(request.status.as_str())
        .bind(request.approval_count)
        .bind(request.rejection_count)
        .bind(&request.required_approvers)
        .bind(request.resolved_at.map(|t| t.to_rfc3339()))
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List all pending approval requests
    pub async fn list_pending_approvals(&self) -> Result<Vec<ApprovalRequest>> {
        let rows = sqlx::query_as::<_, ApprovalRequestRow>(
            r#"
            SELECT id, stage_id, run_id, status, required_approvers, required_count,
                   approval_count, rejection_count, timeout_seconds, timeout_action,
                   timeout_at, resolved_at, created_at
            FROM approval_requests
            WHERE status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// List approval requests that have timed out
    pub async fn list_timed_out_approvals(&self) -> Result<Vec<ApprovalRequest>> {
        let now = chrono::Utc::now().to_rfc3339();
        let rows = sqlx::query_as::<_, ApprovalRequestRow>(
            r#"
            SELECT id, stage_id, run_id, status, required_approvers, required_count,
                   approval_count, rejection_count, timeout_seconds, timeout_action,
                   timeout_at, resolved_at, created_at
            FROM approval_requests
            WHERE status = 'pending'
              AND timeout_at IS NOT NULL
              AND timeout_at < ?
            ORDER BY timeout_at ASC
            "#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Create an approval decision
    pub async fn create_approval_decision(
        &self,
        decision: ApprovalDecision,
    ) -> Result<ApprovalDecision> {
        let mut decision = decision;
        let result = sqlx::query(
            r#"
            INSERT INTO approval_decisions
            (approval_id, approver, decision, comment, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(decision.approval_id)
        .bind(&decision.approver)
        .bind(decision.decision as i32)
        .bind(&decision.comment)
        .bind(decision.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        decision.id = Some(result.last_insert_rowid());
        Ok(decision)
    }

    /// Get all decisions for an approval request
    pub async fn get_approval_decisions(&self, approval_id: i64) -> Result<Vec<ApprovalDecision>> {
        let rows = sqlx::query_as::<_, ApprovalDecisionRow>(
            r#"
            SELECT id, approval_id, approver, decision, comment, created_at
            FROM approval_decisions
            WHERE approval_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(approval_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }
}

// Database row types for approval

#[derive(sqlx::FromRow)]
struct ApprovalRequestRow {
    id: i64,
    stage_id: i64,
    run_id: i64,
    status: String,
    required_approvers: String,
    required_count: i32,
    approval_count: i32,
    rejection_count: i32,
    timeout_seconds: Option<i64>,
    timeout_action: Option<String>,
    timeout_at: Option<String>,
    resolved_at: Option<String>,
    created_at: String,
}

impl TryFrom<ApprovalRequestRow> for ApprovalRequest {
    type Error = crate::Error;

    fn try_from(row: ApprovalRequestRow) -> Result<Self> {
        use std::str::FromStr;

        Ok(Self {
            id: Some(row.id),
            stage_id: row.stage_id,
            run_id: row.run_id,
            status: ApprovalStatus::from_str(&row.status)?,
            required_approvers: row.required_approvers,
            required_count: row.required_count,
            approval_count: row.approval_count,
            rejection_count: row.rejection_count,
            timeout_seconds: row.timeout_seconds,
            timeout_action: row.timeout_action,
            timeout_at: row
                .timeout_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            resolved_at: row
                .resolved_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct ApprovalDecisionRow {
    id: i64,
    approval_id: i64,
    approver: String,
    decision: i32,
    comment: Option<String>,
    created_at: String,
}

impl TryFrom<ApprovalDecisionRow> for ApprovalDecision {
    type Error = crate::Error;

    fn try_from(row: ApprovalDecisionRow) -> Result<Self> {
        Ok(Self {
            id: Some(row.id),
            approval_id: row.approval_id,
            approver: row.approver,
            decision: row.decision != 0,
            comment: row.comment,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

// ==================== Success Pattern Row Struct ====================

#[derive(sqlx::FromRow)]
struct SuccessPatternRow {
    id: i64,
    pattern_type: String,
    agent_type: Option<String>,
    task_type: Option<String>,
    pattern_signature: String,
    pattern_data: String,
    occurrence_count: i64,
    avg_completion_time_ms: Option<i64>,
    avg_token_usage: Option<i64>,
    success_rate: f64,
    first_seen_at: String,
    last_seen_at: String,
}

impl TryFrom<SuccessPatternRow> for SuccessPattern {
    type Error = crate::Error;

    fn try_from(row: SuccessPatternRow) -> Result<Self> {
        use std::str::FromStr;

        Ok(SuccessPattern {
            id: row.id,
            pattern_type: SuccessPatternType::from_str(&row.pattern_type)?,
            agent_type: row
                .agent_type
                .map(|s| AgentType::from_str(&s))
                .transpose()?,
            task_type: row.task_type,
            pattern_signature: row.pattern_signature,
            pattern_data: serde_json::from_str(&row.pattern_data)
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            occurrence_count: row.occurrence_count,
            avg_completion_time_ms: row.avg_completion_time_ms,
            avg_token_usage: row.avg_token_usage,
            success_rate: row.success_rate,
            first_seen_at: chrono::DateTime::parse_from_rfc3339(&row.first_seen_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            last_seen_at: chrono::DateTime::parse_from_rfc3339(&row.last_seen_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

// ==================== Feedback Row Structs ====================

#[derive(sqlx::FromRow)]
struct FeedbackRow {
    id: i64,
    agent_id: String,
    message_id: Option<i64>,
    rating: String,
    comment: Option<String>,
    source: String,
    created_by: String,
    created_at: String,
}

impl TryFrom<FeedbackRow> for Feedback {
    type Error = crate::Error;

    fn try_from(row: FeedbackRow) -> Result<Self> {
        use std::str::FromStr;

        Ok(Feedback {
            id: row.id,
            agent_id: Uuid::parse_str(&row.agent_id)
                .map_err(|e| crate::Error::Other(e.to_string()))?,
            message_id: row.message_id,
            rating: FeedbackRating::from_str(&row.rating)?,
            comment: row.comment,
            source: FeedbackSource::from_str(&row.source)?,
            created_by: row.created_by,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct FeedbackStatsRow {
    #[allow(dead_code)]
    total: i64,
    positive: i64,
    negative: i64,
    neutral: i64,
}

#[derive(sqlx::FromRow)]
struct FeedbackStatsByTypeRow {
    agent_type: String,
    #[allow(dead_code)]
    total: i64,
    positive: i64,
    negative: i64,
    neutral: i64,
}

// ==================== Effectiveness Analysis Row Structs ====================

/// Row for effectiveness analysis queries
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct EffectivenessAnalysisRow {
    pub instruction_id: i64,
    pub name: String,
    pub instruction_source: String,
    pub enabled: bool,
    pub usage_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub penalty_score: f64,
    pub avg_completion_time: f64,
    pub success_rate: f64,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub updated_at: Option<String>,
}

impl EffectivenessAnalysisRow {
    /// Get the effectiveness level as a string
    pub fn effectiveness_level(&self) -> &'static str {
        if self.usage_count < 5 {
            "insufficient_data"
        } else if self.success_rate >= 0.8 {
            "high"
        } else if self.success_rate >= 0.5 {
            "medium"
        } else if self.success_rate >= 0.3 {
            "low"
        } else {
            "very_low"
        }
    }

    /// Check if this instruction is considered ineffective
    pub fn is_ineffective(&self) -> bool {
        self.usage_count >= 5 && self.success_rate < 0.5
    }
}

#[derive(sqlx::FromRow)]
struct EffectivenessSummaryRow {
    total_instructions: i64,
    enabled_count: i64,
    used_count: i64,
    avg_success_rate: f64,
    avg_penalty_score: f64,
    total_usage: i64,
    ineffective_count: i64,
}

/// Summary of instruction effectiveness across the system
#[derive(Debug, Clone, serde::Serialize)]
pub struct EffectivenessSummary {
    pub total_instructions: i64,
    pub enabled_count: i64,
    pub used_count: i64,
    pub avg_success_rate: f64,
    pub avg_penalty_score: f64,
    pub total_usage: i64,
    pub ineffective_count: i64,
}

// ==================== Experiment Row Structs ====================

#[derive(sqlx::FromRow)]
struct ExperimentRow {
    id: i64,
    name: String,
    description: Option<String>,
    hypothesis: Option<String>,
    experiment_type: String,
    metric: String,
    agent_type: Option<String>,
    status: String,
    min_samples: i64,
    confidence_level: f64,
    created_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    winner_variant_id: Option<i64>,
}

impl TryFrom<ExperimentRow> for Experiment {
    type Error = crate::Error;

    fn try_from(row: ExperimentRow) -> Result<Self> {
        use std::str::FromStr;
        Ok(Experiment {
            id: row.id,
            name: row.name,
            description: row.description,
            hypothesis: row.hypothesis,
            experiment_type: ExperimentType::from_str(&row.experiment_type)?,
            metric: ExperimentMetric::from_str(&row.metric)?,
            agent_type: row.agent_type,
            status: ExperimentStatus::from_str(&row.status)?,
            min_samples: row.min_samples,
            confidence_level: row.confidence_level,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            started_at: row
                .started_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            completed_at: row
                .completed_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(Into::into),
            winner_variant_id: row.winner_variant_id,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ExperimentVariantRow {
    id: i64,
    experiment_id: i64,
    name: String,
    description: Option<String>,
    is_control: bool,
    weight: i32,
    config: String,
    created_at: String,
}

impl TryFrom<ExperimentVariantRow> for ExperimentVariant {
    type Error = crate::Error;

    fn try_from(row: ExperimentVariantRow) -> Result<Self> {
        Ok(ExperimentVariant {
            id: row.id,
            experiment_id: row.experiment_id,
            name: row.name,
            description: row.description,
            is_control: row.is_control,
            weight: row.weight,
            config: serde_json::from_str(&row.config)
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct VariantResultsRow {
    variant_id: i64,
    variant_name: String,
    is_control: bool,
    sample_count: i64,
    mean: f64,
    std_dev: f64,
    min_value: f64,
    max_value: f64,
    success_count: Option<i64>,
}

impl From<VariantResultsRow> for VariantResults {
    fn from(row: VariantResultsRow) -> Self {
        let success_rate = if row.sample_count > 0 {
            row.success_count.map(|c| c as f64 / row.sample_count as f64)
        } else {
            None
        };

        VariantResults {
            variant_id: row.variant_id,
            variant_name: row.variant_name,
            is_control: row.is_control,
            sample_count: row.sample_count,
            mean: row.mean,
            std_dev: row.std_dev,
            min_value: row.min_value,
            max_value: row.max_value,
            success_count: row.success_count,
            success_rate,
        }
    }
}

// ==================== Model Selection Row Structs ====================

#[derive(sqlx::FromRow)]
struct ModelPerformanceRow {
    #[allow(dead_code)]
    id: i64,
    model: String,
    task_type: String,
    agent_type: Option<String>,
    success_count: i64,
    failure_count: i64,
    total_tokens: i64,
    total_cost: f64,
    total_duration_secs: f64,
    sample_count: i64,
    last_used_at: Option<String>,
    #[allow(dead_code)]
    updated_at: String,
}

impl From<ModelPerformanceRow> for ModelPerformance {
    fn from(row: ModelPerformanceRow) -> Self {
        let success_rate = if row.sample_count > 0 {
            row.success_count as f64 / row.sample_count as f64
        } else {
            0.0
        };

        let avg_tokens = if row.sample_count > 0 {
            row.total_tokens as f64 / row.sample_count as f64
        } else {
            0.0
        };

        let avg_cost = if row.sample_count > 0 {
            row.total_cost / row.sample_count as f64
        } else {
            0.0
        };

        let avg_duration_secs = if row.sample_count > 0 {
            row.total_duration_secs / row.sample_count as f64
        } else {
            0.0
        };

        ModelPerformance {
            model: row.model,
            task_type: row.task_type,
            agent_type: row.agent_type,
            success_count: row.success_count,
            failure_count: row.failure_count,
            success_rate,
            avg_tokens,
            avg_cost,
            avg_duration_secs,
            sample_count: row.sample_count,
            last_used_at: row
                .last_used_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(Into::into),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ModelSelectionRuleRow {
    id: i64,
    name: String,
    task_type: Option<String>,
    agent_type: Option<String>,
    complexity: Option<String>,
    preferred_model: String,
    fallback_model: Option<String>,
    max_cost: Option<f64>,
    min_success_rate: Option<f64>,
    priority: i32,
    enabled: bool,
    created_at: String,
}

impl TryFrom<ModelSelectionRuleRow> for ModelSelectionRule {
    type Error = crate::Error;

    fn try_from(row: ModelSelectionRuleRow) -> Result<Self> {
        use std::str::FromStr;

        let complexity = row
            .complexity
            .map(|s| TaskComplexity::from_str(&s))
            .transpose()?;

        Ok(ModelSelectionRule {
            id: row.id,
            name: row.name,
            task_type: row.task_type,
            agent_type: row.agent_type,
            complexity,
            preferred_model: row.preferred_model,
            fallback_model: row.fallback_model,
            max_cost: row.max_cost,
            min_success_rate: row.min_success_rate,
            priority: row.priority,
            enabled: row.enabled,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct ModelSelectionConfigRow {
    #[allow(dead_code)]
    id: i64,
    optimization_goal: String,
    max_cost_per_task: Option<f64>,
    min_success_rate: f64,
    min_samples_for_auto: i64,
    enabled: bool,
    #[allow(dead_code)]
    updated_at: String,
}

impl From<ModelSelectionConfigRow> for ModelSelectionConfig {
    fn from(row: ModelSelectionConfigRow) -> Self {
        use std::str::FromStr;

        ModelSelectionConfig {
            optimization_goal: OptimizationGoal::from_str(&row.optimization_goal)
                .unwrap_or(OptimizationGoal::Balanced),
            max_cost_per_task: row.max_cost_per_task,
            min_success_rate: row.min_success_rate,
            min_samples_for_auto: row.min_samples_for_auto,
            enabled: row.enabled,
        }
    }
}

impl Database {
    // ==================== Incident Operations ====================

    /// Create a new incident
    pub async fn create_incident(&self, incident: &crate::incident::Incident) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO incidents (
                id, title, description, severity, status, detected_at,
                acknowledged_at, resolved_at, affected_services,
                related_incidents, tags, metadata
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&incident.id)
        .bind(&incident.title)
        .bind(&incident.description)
        .bind(incident.severity.as_str())
        .bind(incident.status.as_str())
        .bind(incident.detected_at.to_rfc3339())
        .bind(incident.acknowledged_at.as_ref().map(|t| t.to_rfc3339()))
        .bind(incident.resolved_at.as_ref().map(|t| t.to_rfc3339()))
        .bind(serde_json::to_string(&incident.affected_services)?)
        .bind(serde_json::to_string(&incident.related_incidents)?)
        .bind(serde_json::to_string(&incident.tags)?)
        .bind(serde_json::to_string(&incident.metadata)?)
        .execute(&self.pool)
        .await?;

        // Insert timeline events
        for event in &incident.timeline {
            self.add_timeline_event(&incident.id, event).await?;
        }

        Ok(())
    }

    /// Update an incident
    pub async fn update_incident(&self, incident: &crate::incident::Incident) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE incidents SET
                title = ?, description = ?, severity = ?, status = ?,
                acknowledged_at = ?, resolved_at = ?,
                affected_services = ?, related_incidents = ?,
                tags = ?, metadata = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(&incident.title)
        .bind(&incident.description)
        .bind(incident.severity.as_str())
        .bind(incident.status.as_str())
        .bind(incident.acknowledged_at.as_ref().map(|t| t.to_rfc3339()))
        .bind(incident.resolved_at.as_ref().map(|t| t.to_rfc3339()))
        .bind(serde_json::to_string(&incident.affected_services)?)
        .bind(serde_json::to_string(&incident.related_incidents)?)
        .bind(serde_json::to_string(&incident.tags)?)
        .bind(serde_json::to_string(&incident.metadata)?)
        .bind(&incident.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get an incident by ID
    pub async fn get_incident(&self, id: &str) -> Result<Option<crate::incident::Incident>> {
        let row = sqlx::query_as::<_, IncidentRow>(
            r#"
            SELECT id, title, description, severity, status, detected_at,
                   acknowledged_at, resolved_at, affected_services,
                   related_incidents, tags, metadata
            FROM incidents
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let timeline = self.get_timeline_events(id).await?;
            Ok(Some(row.into_incident(timeline)?))
        } else {
            Ok(None)
        }
    }

    /// List incidents with optional filters
    pub async fn list_incidents(
        &self,
        status: Option<&str>,
        severity: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<crate::incident::Incident>> {
        let mut query = String::from(
            r#"
            SELECT id, title, description, severity, status, detected_at,
                   acknowledged_at, resolved_at, affected_services,
                   related_incidents, tags, metadata
            FROM incidents
            WHERE 1=1
            "#,
        );

        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        if severity.is_some() {
            query.push_str(" AND severity = ?");
        }

        query.push_str(" ORDER BY detected_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut q = sqlx::query_as::<_, IncidentRow>(&query);
        if let Some(s) = status {
            q = q.bind(s);
        }
        if let Some(sev) = severity {
            q = q.bind(sev);
        }

        let rows = q.fetch_all(&self.pool).await?;

        let mut incidents = Vec::new();
        for row in rows {
            let timeline = self.get_timeline_events(&row.id).await?;
            incidents.push(row.into_incident(timeline)?);
        }

        Ok(incidents)
    }

    /// Add timeline event to incident
    pub async fn add_timeline_event(
        &self,
        incident_id: &str,
        event: &crate::incident::TimelineEvent,
    ) -> Result<()> {
        let event_type_str = match event.event_type {
            crate::incident::TimelineEventType::Detected => "detected",
            crate::incident::TimelineEventType::Acknowledged => "acknowledged",
            crate::incident::TimelineEventType::InvestigationStarted => "investigation_started",
            crate::incident::TimelineEventType::RootCauseIdentified => "root_cause_identified",
            crate::incident::TimelineEventType::MitigationStarted => "mitigation_started",
            crate::incident::TimelineEventType::PlaybookExecuted => "playbook_executed",
            crate::incident::TimelineEventType::Escalated => "escalated",
            crate::incident::TimelineEventType::Resolved => "resolved",
            crate::incident::TimelineEventType::PostMortemCreated => "post_mortem_created",
            crate::incident::TimelineEventType::Comment => "comment",
        };

        sqlx::query(
            r#"
            INSERT INTO incident_timeline (
                incident_id, timestamp, event_type, description, actor, metadata
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(incident_id)
        .bind(event.timestamp.to_rfc3339())
        .bind(event_type_str)
        .bind(&event.description)
        .bind(&event.actor)
        .bind(serde_json::to_string(&event.metadata)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get timeline events for an incident
    pub async fn get_timeline_events(
        &self,
        incident_id: &str,
    ) -> Result<Vec<crate::incident::TimelineEvent>> {
        let rows = sqlx::query_as::<_, TimelineEventRow>(
            r#"
            SELECT timestamp, event_type, description, actor, metadata
            FROM incident_timeline
            WHERE incident_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(incident_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Root Cause Analysis ====================

    /// Save root cause analysis
    pub async fn save_root_cause_analysis(
        &self,
        rca: &crate::incident::RootCauseAnalysis,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO root_cause_analyses (
                incident_id, primary_cause, evidence, contributing_factors,
                hypotheses, related_events, analyzed_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&rca.incident_id)
        .bind(&rca.primary_cause)
        .bind(serde_json::to_string(&rca.evidence)?)
        .bind(serde_json::to_string(&rca.contributing_factors)?)
        .bind(serde_json::to_string(&rca.hypotheses)?)
        .bind(serde_json::to_string(&rca.related_events)?)
        .bind(rca.analyzed_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get root cause analysis for an incident
    pub async fn get_root_cause_analysis(
        &self,
        incident_id: &str,
    ) -> Result<Option<crate::incident::RootCauseAnalysis>> {
        let row = sqlx::query_as::<_, RootCauseAnalysisRow>(
            r#"
            SELECT incident_id, primary_cause, evidence, contributing_factors,
                   hypotheses, related_events, analyzed_at
            FROM root_cause_analyses
            WHERE incident_id = ?
            "#,
        )
        .bind(incident_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    // ==================== Playbook Operations ====================

    /// Create a playbook
    pub async fn create_playbook(&self, playbook: &crate::incident::Playbook) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO playbooks (id, name, description, triggers, actions)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&playbook.id)
        .bind(&playbook.name)
        .bind(&playbook.description)
        .bind(serde_json::to_string(&playbook.triggers)?)
        .bind(serde_json::to_string(&playbook.actions)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update a playbook
    pub async fn update_playbook(&self, playbook: &crate::incident::Playbook) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE playbooks SET
                name = ?, description = ?, triggers = ?, actions = ?,
                updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(&playbook.name)
        .bind(&playbook.description)
        .bind(serde_json::to_string(&playbook.triggers)?)
        .bind(serde_json::to_string(&playbook.actions)?)
        .bind(&playbook.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a playbook by ID
    pub async fn get_playbook(&self, id: &str) -> Result<Option<crate::incident::Playbook>> {
        let row = sqlx::query_as::<_, PlaybookRow>(
            r#"
            SELECT id, name, description, triggers, actions, created_at, updated_at
            FROM playbooks
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// Get a playbook by name
    pub async fn get_playbook_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::incident::Playbook>> {
        let row = sqlx::query_as::<_, PlaybookRow>(
            r#"
            SELECT id, name, description, triggers, actions, created_at, updated_at
            FROM playbooks
            WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// List all playbooks
    pub async fn list_playbooks(&self) -> Result<Vec<crate::incident::Playbook>> {
        let rows = sqlx::query_as::<_, PlaybookRow>(
            r#"
            SELECT id, name, description, triggers, actions, created_at, updated_at
            FROM playbooks
            ORDER BY name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    /// Delete a playbook
    pub async fn delete_playbook(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM playbooks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ==================== Playbook Execution ====================

    /// Create playbook execution
    pub async fn create_playbook_execution(
        &self,
        execution: &crate::incident::PlaybookExecution,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO playbook_executions (
                id, playbook_id, incident_id, status, started_at,
                completed_at, action_results, triggered_by
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&execution.id)
        .bind(&execution.playbook_id)
        .bind(&execution.incident_id)
        .bind(execution.status.as_str())
        .bind(execution.started_at.to_rfc3339())
        .bind(execution.completed_at.as_ref().map(|t| t.to_rfc3339()))
        .bind(serde_json::to_string(&execution.action_results)?)
        .bind(&execution.triggered_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update playbook execution
    pub async fn update_playbook_execution(
        &self,
        execution: &crate::incident::PlaybookExecution,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE playbook_executions SET
                status = ?, completed_at = ?, action_results = ?
            WHERE id = ?
            "#,
        )
        .bind(execution.status.as_str())
        .bind(execution.completed_at.as_ref().map(|t| t.to_rfc3339()))
        .bind(serde_json::to_string(&execution.action_results)?)
        .bind(&execution.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get playbook execution by ID
    pub async fn get_playbook_execution(
        &self,
        id: &str,
    ) -> Result<Option<crate::incident::PlaybookExecution>> {
        let row = sqlx::query_as::<_, PlaybookExecutionRow>(
            r#"
            SELECT id, playbook_id, incident_id, status, started_at,
                   completed_at, action_results, triggered_by
            FROM playbook_executions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    /// List executions for a playbook
    pub async fn list_playbook_executions(
        &self,
        playbook_id: &str,
        limit: Option<i64>,
    ) -> Result<Vec<crate::incident::PlaybookExecution>> {
        let mut query = String::from(
            r#"
            SELECT id, playbook_id, incident_id, status, started_at,
                   completed_at, action_results, triggered_by
            FROM playbook_executions
            WHERE playbook_id = ?
            ORDER BY started_at DESC
            "#,
        );

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let rows = sqlx::query_as::<_, PlaybookExecutionRow>(&query)
            .bind(playbook_id)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Post-Mortem Operations ====================

    /// Save post-mortem
    pub async fn save_post_mortem(&self, pm: &crate::incident::PostMortem) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO post_mortems (
                incident_id, title, summary, impact, root_cause,
                contributing_factors, resolution, action_items,
                lessons_learned, authors
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&pm.incident_id)
        .bind(&pm.title)
        .bind(&pm.summary)
        .bind(serde_json::to_string(&pm.impact)?)
        .bind(&pm.root_cause)
        .bind(serde_json::to_string(&pm.contributing_factors)?)
        .bind(&pm.resolution)
        .bind(serde_json::to_string(&pm.action_items)?)
        .bind(serde_json::to_string(&pm.lessons_learned)?)
        .bind(serde_json::to_string(&pm.authors)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get post-mortem for an incident
    pub async fn get_post_mortem(
        &self,
        incident_id: &str,
    ) -> Result<Option<crate::incident::PostMortem>> {
        let row = sqlx::query_as::<_, PostMortemRow>(
            r#"
            SELECT incident_id, title, summary, impact, root_cause,
                   contributing_factors, resolution, action_items,
                   lessons_learned, authors, created_at
            FROM post_mortems
            WHERE incident_id = ?
            "#,
        )
        .bind(incident_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            // Get timeline from incident
            let timeline = self.get_timeline_events(incident_id).await?;
            Ok(Some(row.into_post_mortem(timeline)?))
        } else {
            Ok(None)
        }
    }

    // ==================== Anomaly Metrics ====================

    /// Record anomaly metric
    pub async fn record_anomaly_metric(
        &self,
        metric: &crate::incident::AnomalyMetric,
        incident_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO anomaly_metrics (
                name, current_value, baseline_value, threshold,
                deviation_percent, is_anomaly, incident_id, timestamp, metadata
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&metric.name)
        .bind(metric.current_value)
        .bind(metric.baseline_value)
        .bind(metric.threshold)
        .bind(metric.deviation_percent)
        .bind(metric.is_anomaly)
        .bind(incident_id)
        .bind(metric.timestamp.to_rfc3339())
        .bind("{}")
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get recent anomalies
    pub async fn get_recent_anomalies(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::incident::AnomalyMetric>> {
        let rows = sqlx::query_as::<_, AnomalyMetricRow>(
            r#"
            SELECT name, current_value, baseline_value, threshold,
                   deviation_percent, is_anomaly, timestamp
            FROM anomaly_metrics
            WHERE is_anomaly = 1
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }
}

// ==================== Database Row Types ====================

#[derive(sqlx::FromRow)]
struct IncidentRow {
    id: String,
    title: String,
    description: String,
    severity: String,
    status: String,
    detected_at: String,
    acknowledged_at: Option<String>,
    resolved_at: Option<String>,
    affected_services: String,
    related_incidents: String,
    tags: String,
    metadata: String,
}

impl IncidentRow {
    fn into_incident(
        self,
        timeline: Vec<crate::incident::TimelineEvent>,
    ) -> Result<crate::incident::Incident> {
        use std::str::FromStr;

        Ok(crate::incident::Incident {
            id: self.id,
            title: self.title,
            description: self.description,
            severity: crate::incident::IncidentSeverity::from_str(&self.severity)
                .map_err(|e| crate::Error::Other(e))?,
            status: crate::incident::IncidentStatus::from_str(&self.status)
                .map_err(|e| crate::Error::Other(e))?,
            detected_at: chrono::DateTime::parse_from_rfc3339(&self.detected_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            acknowledged_at: self
                .acknowledged_at
                .as_ref()
                .map(|s| chrono::DateTime::parse_from_rfc3339(s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(|dt| dt.into()),
            resolved_at: self
                .resolved_at
                .as_ref()
                .map(|s| chrono::DateTime::parse_from_rfc3339(s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(|dt| dt.into()),
            timeline,
            affected_services: serde_json::from_str(&self.affected_services)?,
            related_incidents: serde_json::from_str(&self.related_incidents)?,
            tags: serde_json::from_str(&self.tags)?,
            metadata: serde_json::from_str(&self.metadata)?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct TimelineEventRow {
    timestamp: String,
    event_type: String,
    description: String,
    actor: Option<String>,
    metadata: String,
}

impl TryFrom<TimelineEventRow> for crate::incident::TimelineEvent {
    type Error = crate::Error;

    fn try_from(row: TimelineEventRow) -> Result<Self> {
        let event_type = match row.event_type.as_str() {
            "detected" => crate::incident::TimelineEventType::Detected,
            "acknowledged" => crate::incident::TimelineEventType::Acknowledged,
            "investigation_started" | "investigationstarted" => {
                crate::incident::TimelineEventType::InvestigationStarted
            }
            "root_cause_identified" | "rootcauseidentified" => {
                crate::incident::TimelineEventType::RootCauseIdentified
            }
            "mitigation_started" | "mitigationstarted" => {
                crate::incident::TimelineEventType::MitigationStarted
            }
            "playbook_executed" | "playbookexecuted" => {
                crate::incident::TimelineEventType::PlaybookExecuted
            }
            "escalated" => crate::incident::TimelineEventType::Escalated,
            "resolved" => crate::incident::TimelineEventType::Resolved,
            "post_mortem_created" | "postmortemcreated" => {
                crate::incident::TimelineEventType::PostMortemCreated
            }
            "comment" => crate::incident::TimelineEventType::Comment,
            _ => {
                return Err(crate::Error::Other(format!(
                    "Unknown event type: {}",
                    row.event_type
                )))
            }
        };

        Ok(crate::incident::TimelineEvent {
            timestamp: chrono::DateTime::parse_from_rfc3339(&row.timestamp)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            event_type,
            description: row.description,
            actor: row.actor,
            metadata: serde_json::from_str(&row.metadata)?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct RootCauseAnalysisRow {
    incident_id: String,
    primary_cause: String,
    evidence: String,
    contributing_factors: String,
    hypotheses: String,
    related_events: String,
    analyzed_at: String,
}

impl TryFrom<RootCauseAnalysisRow> for crate::incident::RootCauseAnalysis {
    type Error = crate::Error;

    fn try_from(row: RootCauseAnalysisRow) -> Result<Self> {
        Ok(crate::incident::RootCauseAnalysis {
            incident_id: row.incident_id,
            primary_cause: row.primary_cause,
            evidence: serde_json::from_str(&row.evidence)?,
            contributing_factors: serde_json::from_str(&row.contributing_factors)?,
            hypotheses: serde_json::from_str(&row.hypotheses)?,
            related_events: serde_json::from_str(&row.related_events)?,
            analyzed_at: chrono::DateTime::parse_from_rfc3339(&row.analyzed_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct PlaybookRow {
    id: String,
    name: String,
    description: String,
    triggers: String,
    actions: String,
    created_at: String,
    updated_at: String,
}

impl TryFrom<PlaybookRow> for crate::incident::Playbook {
    type Error = crate::Error;

    fn try_from(row: PlaybookRow) -> Result<Self> {
        Ok(crate::incident::Playbook {
            id: row.id,
            name: row.name,
            description: row.description,
            triggers: serde_json::from_str(&row.triggers)?,
            actions: serde_json::from_str(&row.actions)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            updated_at: chrono::DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

#[derive(sqlx::FromRow)]
struct PlaybookExecutionRow {
    id: String,
    playbook_id: String,
    incident_id: Option<String>,
    status: String,
    started_at: String,
    completed_at: Option<String>,
    action_results: String,
    triggered_by: Option<String>,
}

impl TryFrom<PlaybookExecutionRow> for crate::incident::PlaybookExecution {
    type Error = crate::Error;

    fn try_from(row: PlaybookExecutionRow) -> Result<Self> {

        let status = match row.status.as_str() {
            "running" => crate::incident::PlaybookExecutionStatus::Running,
            "waiting_approval" => crate::incident::PlaybookExecutionStatus::WaitingApproval,
            "completed" => crate::incident::PlaybookExecutionStatus::Completed,
            "failed" => crate::incident::PlaybookExecutionStatus::Failed,
            "cancelled" => crate::incident::PlaybookExecutionStatus::Cancelled,
            _ => {
                return Err(crate::Error::Other(format!(
                    "Unknown execution status: {}",
                    row.status
                )))
            }
        };

        Ok(crate::incident::PlaybookExecution {
            id: row.id,
            playbook_id: row.playbook_id,
            incident_id: row.incident_id,
            status,
            started_at: chrono::DateTime::parse_from_rfc3339(&row.started_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            completed_at: row
                .completed_at
                .as_ref()
                .map(|s| chrono::DateTime::parse_from_rfc3339(s))
                .transpose()
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .map(|dt| dt.into()),
            action_results: serde_json::from_str(&row.action_results)?,
            triggered_by: row.triggered_by,
        })
    }
}

#[derive(sqlx::FromRow)]
struct PostMortemRow {
    incident_id: String,
    title: String,
    summary: String,
    impact: String,
    root_cause: String,
    contributing_factors: String,
    resolution: String,
    action_items: String,
    lessons_learned: String,
    authors: String,
    created_at: String,
}

impl PostMortemRow {
    fn into_post_mortem(
        self,
        timeline: Vec<crate::incident::TimelineEvent>,
    ) -> Result<crate::incident::PostMortem> {
        Ok(crate::incident::PostMortem {
            incident_id: self.incident_id,
            title: self.title,
            summary: self.summary,
            impact: serde_json::from_str(&self.impact)?,
            timeline,
            root_cause: self.root_cause,
            contributing_factors: serde_json::from_str(&self.contributing_factors)?,
            resolution: self.resolution,
            action_items: serde_json::from_str(&self.action_items)?,
            lessons_learned: serde_json::from_str(&self.lessons_learned)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            authors: serde_json::from_str(&self.authors)?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AnomalyMetricRow {
    name: String,
    current_value: f64,
    baseline_value: f64,
    threshold: f64,
    deviation_percent: f64,
    is_anomaly: bool,
    timestamp: String,
}

impl TryFrom<AnomalyMetricRow> for crate::incident::AnomalyMetric {
    type Error = crate::Error;

    fn try_from(row: AnomalyMetricRow) -> Result<Self> {
        Ok(crate::incident::AnomalyMetric {
            name: row.name,
            current_value: row.current_value,
            baseline_value: row.baseline_value,
            threshold: row.threshold,
            deviation_percent: row.deviation_percent,
            is_anomaly: row.is_anomaly,
            timestamp: chrono::DateTime::parse_from_rfc3339(&row.timestamp)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
        })
    }
}

// ==================== Monitoring & Metrics Database Methods ====================

/// Agent count by state and type
#[derive(Debug, Clone, Default)]
pub struct AgentCountsByStateAndType {
    pub by_state: std::collections::HashMap<String, i64>,
    pub by_type: std::collections::HashMap<String, i64>,
}

/// Token usage by model
#[derive(Debug, Clone)]
pub struct TokenUsageByModel {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

/// PR cycle time stats
#[derive(Debug, Clone)]
pub struct PrCycleTime {
    pub pr_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub merged_at: Option<chrono::DateTime<chrono::Utc>>,
    pub cycle_time_hours: Option<f64>,
}

/// Story completion rate
#[derive(Debug, Clone)]
pub struct StoryCompletionRate {
    pub epic_id: String,
    pub total_stories: i64,
    pub completed_stories: i64,
    pub completion_rate: f64,
}

/// Agent success rate
#[derive(Debug, Clone)]
pub struct AgentSuccessRate {
    pub agent_type: String,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub success_rate: f64,
}

impl Database {
    // ==================== Agent Metrics ====================

    /// Get agent counts grouped by state and type
    pub async fn get_agent_counts_by_state_and_type(&self) -> Result<AgentCountsByStateAndType> {
        let mut result = AgentCountsByStateAndType::default();

        // Count by state
        let state_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT state, COUNT(*) as count FROM agents GROUP BY state"
        )
        .fetch_all(&self.pool)
        .await?;

        for (state, count) in state_rows {
            result.by_state.insert(state, count);
        }

        // Count by type
        let type_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT agent_type, COUNT(*) as count FROM agents GROUP BY agent_type"
        )
        .fetch_all(&self.pool)
        .await?;

        for (agent_type, count) in type_rows {
            result.by_type.insert(agent_type, count);
        }

        Ok(result)
    }

    /// Get agent success rates
    pub async fn get_agent_success_rates(&self) -> Result<Vec<AgentSuccessRate>> {
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                agent_type,
                COUNT(*) as total_runs,
                SUM(CASE WHEN state = 'completed' THEN 1 ELSE 0 END) as successful_runs
            FROM agents
            GROUP BY agent_type
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(agent_type, total_runs, successful_runs)| {
            AgentSuccessRate {
                agent_type,
                total_runs,
                successful_runs,
                success_rate: if total_runs > 0 { successful_runs as f64 / total_runs as f64 } else { 0.0 },
            }
        }).collect())
    }

    // ==================== Token Usage ====================

    /// Get token usage grouped by model
    pub async fn get_token_usage_by_model(&self) -> Result<Vec<TokenUsageByModel>> {
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                model,
                SUM(input_tokens) as input_tokens,
                SUM(output_tokens) as output_tokens
            FROM daily_token_usage
            GROUP BY model
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(model, input_tokens, output_tokens)| {
            TokenUsageByModel {
                model,
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
            }
        }).collect())
    }

    // ==================== Webhook Metrics ====================

    /// Get count of pending webhook events
    pub async fn get_pending_webhook_events_count(&self) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM webhook_events WHERE status = 'pending'"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    // ==================== PR Metrics ====================

    /// Get PR cycle times
    pub async fn get_pr_cycle_times(&self) -> Result<Vec<PrCycleTime>> {
        let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT id, created_at, merged_at
            FROM pull_requests
            ORDER BY created_at DESC
            LIMIT 100
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for (pr_id, created_at_str, merged_at_str) in rows {
            let created_at: chrono::DateTime<chrono::Utc> = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into();

            let merged_at: Option<chrono::DateTime<chrono::Utc>> = if let Some(m) = merged_at_str {
                Some(chrono::DateTime::parse_from_rfc3339(&m)
                    .map_err(|e| crate::Error::Other(e.to_string()))?
                    .into())
            } else {
                None
            };

            let cycle_time_hours = merged_at.map(|m| {
                (m - created_at).num_hours() as f64
            });

            result.push(PrCycleTime {
                pr_id,
                created_at,
                merged_at,
                cycle_time_hours,
            });
        }

        Ok(result)
    }

    // ==================== Story Metrics ====================

    /// Get story completion rates by epic
    pub async fn get_story_completion_rates(&self) -> Result<Vec<StoryCompletionRate>> {
        let rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                epic_id,
                COUNT(*) as total_stories,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed_stories
            FROM stories
            GROUP BY epic_id
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(epic_id, total_stories, completed_stories)| {
            StoryCompletionRate {
                epic_id,
                total_stories,
                completed_stories,
                completion_rate: if total_stories > 0 { completed_stories as f64 / total_stories as f64 } else { 0.0 },
            }
        }).collect())
    }

    // ==================== Alerting ====================

    /// Create a new alert rule
    pub async fn create_alert_rule(&self, rule: &crate::monitoring::AlertRule) -> Result<i64> {
        let id = sqlx::query_scalar(
            r#"
            INSERT INTO alert_rules (name, condition, severity, channels, enabled, evaluation_interval_seconds, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            RETURNING id
            "#
        )
        .bind(&rule.name)
        .bind(&rule.condition)
        .bind(format!("{:?}", rule.severity).to_lowercase())
        .bind(serde_json::to_string(&rule.channels).unwrap_or_default())
        .bind(rule.enabled)
        .bind(60i64)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get an alert by ID
    pub async fn get_alert(&self, id: i64) -> Result<Option<crate::monitoring::Alert>> {
        let row: Option<AlertRowSimple> = sqlx::query_as(
            r#"
            SELECT a.id, a.rule_id, r.name as rule_name, a.status, r.severity,
                   a.triggered_at, a.acknowledged_at, a.acknowledged_by, a.resolved_at
            FROM alerts a
            JOIN alert_rules r ON a.rule_id = r.id
            WHERE a.id = ?
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    /// Update an alert
    pub async fn update_alert(&self, alert: &crate::monitoring::Alert) -> Result<()> {
        let id: i64 = alert.id.parse().map_err(|_| crate::Error::Other("Invalid alert ID".into()))?;

        sqlx::query(
            r#"
            UPDATE alerts
            SET status = ?, acknowledged_at = ?, acknowledged_by = ?, resolved_at = ?
            WHERE id = ?
            "#
        )
        .bind(format!("{:?}", alert.status).to_lowercase())
        .bind(alert.acknowledged_at.map(|dt| dt.to_rfc3339()))
        .bind(&alert.acknowledged_by)
        .bind(alert.resolved_at.map(|dt| dt.to_rfc3339()))
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List alerts by status
    pub async fn list_alerts_by_status(
        &self,
        status: Option<&str>,
        severity: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::monitoring::Alert>> {
        let mut query = String::from(
            r#"
            SELECT a.id, a.rule_id, r.name as rule_name, a.status, r.severity,
                   a.triggered_at, a.acknowledged_at, a.acknowledged_by, a.resolved_at
            FROM alerts a
            JOIN alert_rules r ON a.rule_id = r.id
            WHERE 1=1
            "#
        );

        if status.is_some() {
            query.push_str(" AND a.status = ?");
        }
        if severity.is_some() {
            query.push_str(" AND r.severity = ?");
        }
        query.push_str(" ORDER BY a.triggered_at DESC LIMIT ? OFFSET ?");

        let mut q = sqlx::query_as::<_, AlertRowSimple>(&query);

        if let Some(s) = status {
            q = q.bind(s);
        }
        if let Some(sev) = severity {
            q = q.bind(sev);
        }
        q = q.bind(limit).bind(offset);

        let rows: Vec<AlertRowSimple> = q.fetch_all(&self.pool).await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    // ==================== Audit Log ====================

    /// Insert an audit entry
    pub async fn insert_audit_entry(&self, entry: &crate::monitoring::AuditEntry) -> Result<i64> {
        let details_json = if entry.details.is_empty() {
            None
        } else {
            serde_json::to_string(&entry.details).ok()
        };

        let id = sqlx::query_scalar(
            r#"
            INSERT INTO audit_log (actor, actor_type, action, resource_type, resource_id, details, success, error_message, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&entry.actor)
        .bind(format!("{:?}", entry.actor_type).to_lowercase())
        .bind(format!("{:?}", entry.action).to_lowercase())
        .bind(&entry.resource_type)
        .bind(&entry.resource_id)
        .bind(details_json)
        .bind(entry.success)
        .bind(&entry.error_message)
        .bind(entry.timestamp.to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Query audit log (stub - returns empty list)
    pub async fn query_audit_log(&self, _query: &crate::audit::AuditQuery) -> Result<Vec<crate::monitoring::AuditEntry>> {
        // Audit log feature not fully implemented - return empty list
        Ok(Vec::new())
    }

    /// Get audit stats (stub)
    pub async fn get_audit_stats(&self) -> Result<crate::audit::AuditStats> {
        // Audit log feature not fully implemented - return empty stats
        Ok(crate::audit::AuditStats {
            total_entries: 0,
            entries_by_action: std::collections::HashMap::new(),
            entries_by_actor_type: std::collections::HashMap::new(),
            success_count: 0,
            failure_count: 0,
            first_entry_at: None,
            last_entry_at: None,
        })
    }

    /// Count audit entries (stub)
    pub async fn count_audit_entries(&self, _query: &crate::audit::AuditQuery) -> Result<i64> {
        // Audit log feature not fully implemented
        Ok(0)
    }

    /// Export audit log entries in various formats
    pub async fn export_audit_log(
        &self,
        query: &crate::audit::AuditQuery,
        format: crate::audit::ExportFormat,
    ) -> Result<String> {
        let entries = self.query_audit_log(query).await?;

        match format {
            crate::audit::ExportFormat::Json => {
                serde_json::to_string_pretty(&entries)
                    .map_err(|e| crate::Error::Other(e.to_string()))
            }
            crate::audit::ExportFormat::JsonLines => {
                let lines: Vec<String> = entries
                    .iter()
                    .filter_map(|e| serde_json::to_string(e).ok())
                    .collect();
                Ok(lines.join("\n"))
            }
            crate::audit::ExportFormat::Csv => {
                let mut csv = String::from("id,timestamp,actor,actor_type,action,resource_type,resource_id,success,error_message\n");
                for entry in entries {
                    let action_str = format!("{:?}", entry.action).to_lowercase().replace("_", ".");
                    csv.push_str(&format!(
                        "{},{},{},{:?},{},{},{},{},{}\n",
                        entry.id,
                        entry.timestamp.to_rfc3339(),
                        entry.actor,
                        entry.actor_type,
                        action_str,
                        entry.resource_type,
                        entry.resource_id,
                        entry.success,
                        entry.error_message.as_deref().unwrap_or("")
                    ));
                }
                Ok(csv)
            }
        }
    }

    /// Apply retention policy to audit logs
    pub async fn apply_retention_policy(
        &self,
        policy: &crate::audit::RetentionPolicy,
    ) -> Result<i64> {
        let cutoff = policy.cutoff_date().to_rfc3339();

        let result = sqlx::query("DELETE FROM audit_log WHERE created_at < ?")
            .bind(&cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as i64)
    }

    // ==================== Repository Methods (Stubs) ====================

    /// Insert a repository (stub)
    pub async fn insert_repository(&self, _repo: &crate::multi_repo::Repository) -> Result<i64> {
        Err(crate::Error::Other("Repository operations not yet implemented".to_string()))
    }

    /// List repositories (stub)
    pub async fn list_repositories(&self) -> Result<Vec<crate::multi_repo::Repository>> {
        Ok(Vec::new())
    }

    /// Delete a repository (stub)
    pub async fn delete_repository(&self, _id: &str) -> Result<()> {
        Err(crate::Error::Other("Repository operations not yet implemented".to_string()))
    }

    /// Get repository by name (stub)
    pub async fn get_repository_by_name(&self, _name: &str) -> Result<Option<crate::multi_repo::Repository>> {
        Ok(None)
    }

    /// Update a repository (stub)
    pub async fn update_repository(&self, _repo: &crate::multi_repo::Repository) -> Result<()> {
        Err(crate::Error::Other("Repository operations not yet implemented".to_string()))
    }

    /// Get dependency graph (stub)
    pub async fn get_dependency_graph(&self) -> Result<crate::multi_repo::RepoDependencyGraph> {
        Ok(crate::multi_repo::RepoDependencyGraph::new())
    }
}

// Helper structs for database rows

#[derive(sqlx::FromRow)]
struct AlertRowSimple {
    id: i64,
    rule_id: i64,
    rule_name: String,
    status: String,
    severity: String,
    triggered_at: String,
    acknowledged_at: Option<String>,
    acknowledged_by: Option<String>,
    resolved_at: Option<String>,
}

impl TryFrom<AlertRowSimple> for crate::monitoring::Alert {
    type Error = crate::Error;

    fn try_from(row: AlertRowSimple) -> Result<Self> {
        let status = match row.status.as_str() {
            "pending" => crate::monitoring::AlertStatus::Pending,
            "firing" => crate::monitoring::AlertStatus::Firing,
            "acknowledged" => crate::monitoring::AlertStatus::Acknowledged,
            "resolved" => crate::monitoring::AlertStatus::Resolved,
            "silenced" => crate::monitoring::AlertStatus::Silenced,
            _ => crate::monitoring::AlertStatus::Pending,
        };

        let severity = match row.severity.as_str() {
            "info" => crate::monitoring::AlertSeverity::Info,
            "warning" => crate::monitoring::AlertSeverity::Warning,
            "critical" => crate::monitoring::AlertSeverity::Critical,
            _ => crate::monitoring::AlertSeverity::Info,
        };

        Ok(crate::monitoring::Alert {
            id: row.id.to_string(),
            rule_id: row.rule_id.to_string(),
            rule_name: row.rule_name,
            status,
            severity,
            message: String::new(),
            current_value: None,
            threshold: None,
            labels: std::collections::HashMap::new(),
            triggered_at: chrono::DateTime::parse_from_rfc3339(&row.triggered_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            acknowledged_at: row.acknowledged_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.into()),
            acknowledged_by: row.acknowledged_by,
            resolved_at: row.resolved_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.into()),
            silenced_until: None,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AuditEntryRow {
    id: i64,
    actor: String,
    actor_type: String,
    action: String,
    resource_type: Option<String>,
    resource_id: Option<String>,
    details: Option<String>,
    success: bool,
    error_message: Option<String>,
    created_at: String,
}

impl TryFrom<AuditEntryRow> for crate::monitoring::AuditEntry {
    type Error = crate::Error;

    fn try_from(row: AuditEntryRow) -> Result<Self> {
        let actor_type = match row.actor_type.as_str() {
            "user" => crate::monitoring::ActorType::User,
            "system" => crate::monitoring::ActorType::System,
            "agent" => crate::monitoring::ActorType::Agent,
            "api_key" | "apikey" => crate::monitoring::ActorType::ApiKey,
            "webhook" => crate::monitoring::ActorType::Webhook,
            _ => crate::monitoring::ActorType::System,
        };

        let action = match row.action.as_str() {
            "agent_spawned" | "agentspawned" => crate::monitoring::AuditAction::AgentSpawned,
            "agent_terminated" | "agentterminated" => crate::monitoring::AuditAction::AgentTerminated,
            "configuration_changed" | "configurationchanged" => crate::monitoring::AuditAction::ConfigurationChanged,
            "approval_granted" | "approvalgranted" => crate::monitoring::AuditAction::ApprovalGranted,
            "approval_denied" | "approvaldenied" => crate::monitoring::AuditAction::ApprovalDenied,
            "deployment_triggered" | "deploymenttriggered" => crate::monitoring::AuditAction::DeploymentTriggered,
            "deployment_rolled_back" | "deploymentrolledback" => crate::monitoring::AuditAction::DeploymentRolledBack,
            "alert_acknowledged" | "alertacknowledged" => crate::monitoring::AuditAction::AlertAcknowledged,
            "alert_silenced" | "alertsilenced" => crate::monitoring::AuditAction::AlertSilenced,
            "user_login" | "userlogin" => crate::monitoring::AuditAction::UserLogin,
            _ => crate::monitoring::AuditAction::ConfigurationChanged,
        };

        let details: std::collections::HashMap<String, serde_json::Value> = row.details
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(crate::monitoring::AuditEntry {
            id: row.id.to_string(),
            actor: row.actor,
            actor_type,
            action,
            resource_type: row.resource_type.unwrap_or_default(),
            resource_id: row.resource_id.unwrap_or_default(),
            details,
            success: row.success,
            error_message: row.error_message,
            timestamp: chrono::DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| crate::Error::Other(e.to_string()))?
                .into(),
            ip_address: None,
            user_agent: None,
        })
    }
}

// ==================== Autonomous Session Row ====================

#[derive(Debug, sqlx::FromRow)]
struct AutonomousSessionRow {
    id: String,
    state: String,
    started_at: String,
    updated_at: String,
    completed_at: Option<String>,
    current_epic_id: Option<String>,
    current_story_id: Option<String>,
    current_agent_id: Option<String>,
    config: String,
    work_queue: String,
    completed_items: String,
    metrics: String,
    error_message: Option<String>,
    blocked_reason: Option<String>,
    pause_reason: Option<String>,
    created_at: String,
}

/// Parse datetime from either RFC3339 or SQLite format
fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    // Try RFC3339 first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.into());
    }
    // Try SQLite format (YYYY-MM-DD HH:MM:SS)
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            dt,
            chrono::Utc,
        ));
    }
    Err(crate::Error::Other(format!(
        "Failed to parse datetime: {}",
        s
    )))
}

impl AutonomousSessionRow {
    fn into_session(self) -> Result<crate::autonomous_session::AutonomousSession> {
        Ok(crate::autonomous_session::AutonomousSession {
            id: self.id,
            state: crate::autonomous_session::AutonomousSessionState::from_str(&self.state)?,
            started_at: parse_datetime(&self.started_at)?,
            updated_at: parse_datetime(&self.updated_at)?,
            completed_at: self.completed_at.map(|s| parse_datetime(&s)).transpose()?,
            current_epic_id: self.current_epic_id,
            current_story_id: self.current_story_id,
            current_agent_id: self.current_agent_id,
            config: serde_json::from_str(&self.config)?,
            work_queue: serde_json::from_str(&self.work_queue)?,
            completed_items: serde_json::from_str(&self.completed_items)?,
            metrics: serde_json::from_str(&self.metrics)?,
            error_message: self.error_message,
            blocked_reason: self.blocked_reason,
            pause_reason: self.pause_reason,
            created_at: parse_datetime(&self.created_at)?,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct SessionHistoryRow {
    id: i64,
    session_id: String,
    from_state: String,
    to_state: String,
    reason: Option<String>,
    transitioned_at: String,
    metadata: String,
    created_at: String,
}

impl SessionHistoryRow {
    fn into_history(self) -> Result<crate::autonomous_session::SessionStateHistory> {
        Ok(crate::autonomous_session::SessionStateHistory {
            id: self.id,
            session_id: self.session_id,
            from_state: crate::autonomous_session::AutonomousSessionState::from_str(
                &self.from_state,
            )?,
            to_state: crate::autonomous_session::AutonomousSessionState::from_str(&self.to_state)?,
            reason: self.reason,
            transitioned_at: parse_datetime(&self.transitioned_at)?,
            metadata: serde_json::from_str(&self.metadata)?,
            created_at: parse_datetime(&self.created_at)?,
        })
    }
}

impl Database {
    // ==================== Autonomous Session Operations ====================

    /// Create a new autonomous session
    pub async fn create_autonomous_session(
        &self,
        session: &crate::autonomous_session::AutonomousSession,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO autonomous_sessions (
                id, state, started_at, updated_at, completed_at,
                current_epic_id, current_story_id, current_agent_id,
                config, work_queue, completed_items, metrics,
                error_message, blocked_reason, pause_reason
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(session.state.as_str())
        .bind(session.started_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .bind(session.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(&session.current_epic_id)
        .bind(&session.current_story_id)
        .bind(&session.current_agent_id)
        .bind(serde_json::to_string(&session.config)?)
        .bind(serde_json::to_string(&session.work_queue)?)
        .bind(serde_json::to_string(&session.completed_items)?)
        .bind(serde_json::to_string(&session.metrics)?)
        .bind(&session.error_message)
        .bind(&session.blocked_reason)
        .bind(&session.pause_reason)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update an autonomous session
    pub async fn update_autonomous_session(
        &self,
        session: &crate::autonomous_session::AutonomousSession,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE autonomous_sessions SET
                state = ?, updated_at = ?, completed_at = ?,
                current_epic_id = ?, current_story_id = ?, current_agent_id = ?,
                config = ?, work_queue = ?, completed_items = ?, metrics = ?,
                error_message = ?, blocked_reason = ?, pause_reason = ?
            WHERE id = ?
            "#,
        )
        .bind(session.state.as_str())
        .bind(session.updated_at.to_rfc3339())
        .bind(session.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(&session.current_epic_id)
        .bind(&session.current_story_id)
        .bind(&session.current_agent_id)
        .bind(serde_json::to_string(&session.config)?)
        .bind(serde_json::to_string(&session.work_queue)?)
        .bind(serde_json::to_string(&session.completed_items)?)
        .bind(serde_json::to_string(&session.metrics)?)
        .bind(&session.error_message)
        .bind(&session.blocked_reason)
        .bind(&session.pause_reason)
        .bind(&session.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get an autonomous session by ID
    pub async fn get_autonomous_session(
        &self,
        id: &str,
    ) -> Result<Option<crate::autonomous_session::AutonomousSession>> {
        let row = sqlx::query_as::<_, AutonomousSessionRow>(
            r#"
            SELECT id, state, started_at, updated_at, completed_at,
                   current_epic_id, current_story_id, current_agent_id,
                   config, work_queue, completed_items, metrics,
                   error_message, blocked_reason, pause_reason, created_at
            FROM autonomous_sessions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_session()).transpose()
    }

    /// Get the current active autonomous session (if any)
    pub async fn get_active_autonomous_session(
        &self,
    ) -> Result<Option<crate::autonomous_session::AutonomousSession>> {
        let row = sqlx::query_as::<_, AutonomousSessionRow>(
            r#"
            SELECT id, state, started_at, updated_at, completed_at,
                   current_epic_id, current_story_id, current_agent_id,
                   config, work_queue, completed_items, metrics,
                   error_message, blocked_reason, pause_reason, created_at
            FROM autonomous_sessions
            WHERE state NOT IN ('done', 'paused')
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_session()).transpose()
    }

    /// List autonomous sessions with optional state filter
    pub async fn list_autonomous_sessions(
        &self,
        state: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<crate::autonomous_session::AutonomousSession>> {
        let mut query = String::from(
            r#"
            SELECT id, state, started_at, updated_at, completed_at,
                   current_epic_id, current_story_id, current_agent_id,
                   config, work_queue, completed_items, metrics,
                   error_message, blocked_reason, pause_reason, created_at
            FROM autonomous_sessions
            WHERE 1=1
            "#,
        );

        if state.is_some() {
            query.push_str(" AND state = ?");
        }

        query.push_str(" ORDER BY started_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let mut q = sqlx::query_as::<_, AutonomousSessionRow>(&query);
        if let Some(s) = state {
            q = q.bind(s);
        }

        let rows = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(|r| r.into_session()).collect()
    }

    /// Delete an autonomous session
    pub async fn delete_autonomous_session(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM autonomous_sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record a state transition in the history
    pub async fn record_session_state_transition(
        &self,
        session_id: &str,
        from_state: crate::autonomous_session::AutonomousSessionState,
        to_state: crate::autonomous_session::AutonomousSessionState,
        reason: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO autonomous_session_history (
                session_id, from_state, to_state, reason, transitioned_at, metadata
            )
            VALUES (?, ?, ?, ?, datetime('now'), ?)
            "#,
        )
        .bind(session_id)
        .bind(from_state.as_str())
        .bind(to_state.as_str())
        .bind(reason)
        .bind(serde_json::to_string(&metadata.unwrap_or(serde_json::Value::Null))?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get state transition history for a session
    pub async fn get_session_state_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<crate::autonomous_session::SessionStateHistory>> {
        let rows = sqlx::query_as::<_, SessionHistoryRow>(
            r#"
            SELECT id, session_id, from_state, to_state, reason,
                   transitioned_at, metadata, created_at
            FROM autonomous_session_history
            WHERE session_id = ?
            ORDER BY transitioned_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_history()).collect()
    }

    /// Get sessions by state
    pub async fn get_sessions_by_state(
        &self,
        state: crate::autonomous_session::AutonomousSessionState,
    ) -> Result<Vec<crate::autonomous_session::AutonomousSession>> {
        self.list_autonomous_sessions(Some(state.as_str()), None)
            .await
    }

    /// Count sessions by state
    pub async fn count_sessions_by_state(&self) -> Result<std::collections::HashMap<String, i64>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT state, COUNT(*) as count FROM autonomous_sessions GROUP BY state",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().collect())
    }
}

// ==================== Agent Continuation Row ====================

#[derive(Debug, sqlx::FromRow)]
struct AgentContinuationRow {
    id: i64,
    agent_id: String,
    session_id: Option<String>,
    reason: String,
    message: String,
    context: String,
    status: String,
    created_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    result: Option<String>,
    error_message: Option<String>,
}

impl AgentContinuationRow {
    fn into_continuation(self) -> Result<crate::agent_continuation::AgentContinuation> {
        Ok(crate::agent_continuation::AgentContinuation {
            id: self.id,
            agent_id: self.agent_id,
            session_id: self.session_id,
            reason: crate::agent_continuation::ContinuationReason::from_str(&self.reason)?,
            message: self.message,
            context: serde_json::from_str(&self.context)?,
            status: crate::agent_continuation::ContinuationStatus::from_str(&self.status)?,
            created_at: parse_datetime(&self.created_at)?,
            started_at: self.started_at.map(|s| parse_datetime(&s)).transpose()?,
            completed_at: self.completed_at.map(|s| parse_datetime(&s)).transpose()?,
            result: self
                .result
                .map(|s| serde_json::from_str(&s))
                .transpose()?,
            error_message: self.error_message,
        })
    }
}

impl Database {
    // ==================== Agent Continuation Operations ====================

    /// Create a new agent continuation request
    pub async fn create_continuation(
        &self,
        continuation: &crate::agent_continuation::AgentContinuation,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO agent_continuations (
                agent_id, session_id, reason, message, context, status,
                started_at, completed_at, result, error_message
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&continuation.agent_id)
        .bind(&continuation.session_id)
        .bind(continuation.reason.as_str())
        .bind(&continuation.message)
        .bind(serde_json::to_string(&continuation.context)?)
        .bind(continuation.status.as_str())
        .bind(continuation.started_at.map(|dt| dt.to_rfc3339()))
        .bind(continuation.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(
            continuation
                .result
                .as_ref()
                .map(|r| serde_json::to_string(r))
                .transpose()?,
        )
        .bind(&continuation.error_message)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Update an agent continuation
    pub async fn update_continuation(
        &self,
        continuation: &crate::agent_continuation::AgentContinuation,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE agent_continuations SET
                status = ?, started_at = ?, completed_at = ?,
                result = ?, error_message = ?
            WHERE id = ?
            "#,
        )
        .bind(continuation.status.as_str())
        .bind(continuation.started_at.map(|dt| dt.to_rfc3339()))
        .bind(continuation.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(
            continuation
                .result
                .as_ref()
                .map(|r| serde_json::to_string(r))
                .transpose()?,
        )
        .bind(&continuation.error_message)
        .bind(continuation.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a continuation by ID
    pub async fn get_continuation(
        &self,
        id: i64,
    ) -> Result<Option<crate::agent_continuation::AgentContinuation>> {
        let row = sqlx::query_as::<_, AgentContinuationRow>(
            r#"
            SELECT id, agent_id, session_id, reason, message, context, status,
                   created_at, started_at, completed_at, result, error_message
            FROM agent_continuations
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_continuation()).transpose()
    }

    /// Get pending continuations for an agent
    pub async fn get_pending_continuations(
        &self,
        agent_id: &str,
    ) -> Result<Vec<crate::agent_continuation::AgentContinuation>> {
        let rows = sqlx::query_as::<_, AgentContinuationRow>(
            r#"
            SELECT id, agent_id, session_id, reason, message, context, status,
                   created_at, started_at, completed_at, result, error_message
            FROM agent_continuations
            WHERE agent_id = ? AND status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_continuation()).collect()
    }

    /// Get all continuations for an agent
    pub async fn get_continuations_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<crate::agent_continuation::AgentContinuation>> {
        let rows = sqlx::query_as::<_, AgentContinuationRow>(
            r#"
            SELECT id, agent_id, session_id, reason, message, context, status,
                   created_at, started_at, completed_at, result, error_message
            FROM agent_continuations
            WHERE agent_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.into_continuation()).collect()
    }

    /// Get the next pending continuation (oldest first)
    pub async fn get_next_pending_continuation(
        &self,
    ) -> Result<Option<crate::agent_continuation::AgentContinuation>> {
        let row = sqlx::query_as::<_, AgentContinuationRow>(
            r#"
            SELECT id, agent_id, session_id, reason, message, context, status,
                   created_at, started_at, completed_at, result, error_message
            FROM agent_continuations
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.into_continuation()).transpose()
    }

    /// Cancel all pending continuations for an agent
    pub async fn cancel_pending_continuations(&self, agent_id: &str) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE agent_continuations
            SET status = 'cancelled', completed_at = datetime('now')
            WHERE agent_id = ? AND status = 'pending'
            "#,
        )
        .bind(agent_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Count continuations by status
    pub async fn count_continuations_by_status(
        &self,
    ) -> Result<std::collections::HashMap<String, i64>> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT status, COUNT(*) as count FROM agent_continuations GROUP BY status",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().collect())
    }
}
