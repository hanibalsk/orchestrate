//! Database layer for SQLite

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

use crate::{Agent, AgentState, AgentType, Epic, EpicStatus, Message, MessageRole, PrStatus, PullRequest, MergeStrategy, Result};

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

        // Enable WAL mode and set busy timeout
        sqlx::query("PRAGMA journal_mode=WAL")
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
