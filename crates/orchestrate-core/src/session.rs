//! Session management for Claude API

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A Claude API session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID (internal)
    pub id: String,
    /// Agent this session belongs to
    pub agent_id: Uuid,
    /// Parent session ID (if forked)
    pub parent_id: Option<String>,
    /// Claude API session ID
    pub api_session_id: Option<String>,
    /// Total tokens used in this session
    pub total_tokens: i64,
    /// Whether this is a forked session
    pub is_forked: bool,
    /// When the session was forked
    pub forked_at: Option<DateTime<Utc>>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// When the session was closed
    pub closed_at: Option<DateTime<Utc>>,
}

impl Session {
    /// Create a new session
    pub fn new(agent_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id,
            parent_id: None,
            api_session_id: None,
            total_tokens: 0,
            is_forked: false,
            forked_at: None,
            created_at: Utc::now(),
            closed_at: None,
        }
    }

    /// Create a forked session from a parent
    pub fn fork(parent: &Session, new_agent_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id: new_agent_id,
            parent_id: Some(parent.id.clone()),
            api_session_id: None, // Will be set when fork is executed
            total_tokens: parent.total_tokens, // Inherit token count
            is_forked: true,
            forked_at: Some(Utc::now()),
            created_at: Utc::now(),
            closed_at: None,
        }
    }

    /// Add tokens to the session
    pub fn add_tokens(&mut self, tokens: i64) {
        self.total_tokens += tokens;
    }

    /// Check if session is open
    pub fn is_open(&self) -> bool {
        self.closed_at.is_none()
    }

    /// Close the session
    pub fn close(&mut self) {
        self.closed_at = Some(Utc::now());
    }
}
