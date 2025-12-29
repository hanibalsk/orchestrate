//! Error types for orchestrate-core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Worktree not found: {0}")]
    WorktreeNotFound(String),

    #[error("PR not found: {0}")]
    PrNotFound(i32),

    #[error("Epic not found: {0}")]
    EpicNotFound(String),

    #[error("Invalid state transition: {0} -> {1}")]
    InvalidStateTransition(String, String),

    #[error("Agent already exists: {0}")]
    AgentAlreadyExists(String),

    #[error("Worktree already exists: {0}")]
    WorktreeAlreadyExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid environment type: {0}")]
    InvalidEnvironmentType(String),

    #[error("Environment not found: {0}")]
    EnvironmentNotFound(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
