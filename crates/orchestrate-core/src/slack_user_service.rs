//! Slack User Mention Service
//!
//! Story 8: User Mention Support
//! - Map GitHub users to Slack users
//! - Automatically mention users on their PRs
//! - Mention code owners on relevant PRs
//! - Mention assignees on failures
//! - Send DMs for urgent notifications

use crate::{
    error::Result,
    slack::{NotificationType, SlackMessage, UserMapping},
    slack_service::SlackService,
    Database,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Code owner for a file pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeOwner {
    pub id: String,
    pub pattern: String,
    pub github_username: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CodeOwner {
    pub fn new(pattern: impl Into<String>, github_username: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pattern: pattern.into(),
            github_username: github_username.into(),
            created_at: chrono::Utc::now(),
        }
    }
}

/// Service for managing user mentions in Slack notifications
pub struct SlackUserService {
    db: Database,
    slack_service: SlackService,
}

impl SlackUserService {
    /// Create a new Slack user service
    pub fn new(db: Database, slack_service: SlackService) -> Self {
        Self { db, slack_service }
    }

    /// Map a GitHub user to a Slack user
    pub async fn map_user(
        &self,
        github_username: &str,
        slack_user_id: &str,
        slack_username: &str,
    ) -> Result<UserMapping> {
        // Get or create default connection
        let connection = self.slack_service.get_active_connection().await?;
        let connection_id = if let Some(conn) = connection {
            conn.id
        } else {
            // Create a default connection for testing
            let conn = crate::slack::SlackConnection::new("T_DEFAULT", "Default", "xoxb-test");
            self.slack_service.save_connection(&conn).await?;
            conn.id
        };

        let mut mapping = UserMapping::new(github_username, slack_user_id);
        mapping.slack_username = slack_username.to_string();

        // Save with connection_id
        sqlx::query(
            r#"
            INSERT INTO slack_user_mappings (
                id, connection_id, github_username, slack_user_id, slack_username,
                notify_on_pr, notify_on_mention, notify_on_failure, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(connection_id, github_username) DO UPDATE SET
                slack_user_id = excluded.slack_user_id,
                slack_username = excluded.slack_username,
                notify_on_pr = excluded.notify_on_pr,
                notify_on_mention = excluded.notify_on_mention,
                notify_on_failure = excluded.notify_on_failure
            "#,
        )
        .bind(&mapping.id)
        .bind(&connection_id)
        .bind(&mapping.github_username)
        .bind(&mapping.slack_user_id)
        .bind(&mapping.slack_username)
        .bind(mapping.notify_on_pr)
        .bind(mapping.notify_on_mention)
        .bind(mapping.notify_on_failure)
        .bind(&mapping.created_at)
        .execute(self.db.pool())
        .await?;

        Ok(mapping)
    }

    /// Get Slack user ID for a GitHub username
    pub async fn get_slack_user_id(&self, github_username: &str) -> Result<Option<String>> {
        let mapping = self.slack_service.get_user_mapping(github_username).await?;
        Ok(mapping.map(|m| m.slack_user_id))
    }

    /// Get all user mappings
    pub async fn list_user_mappings(&self) -> Result<Vec<UserMapping>> {
        let rows = sqlx::query(
            r#"
            SELECT id, github_username, slack_user_id, slack_username,
                   notify_on_pr, notify_on_mention, notify_on_failure, created_at
            FROM slack_user_mappings
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut mappings = Vec::new();
        for row in rows {
            mappings.push(UserMapping {
                id: row.try_get("id")?,
                github_username: row.try_get("github_username")?,
                slack_user_id: row.try_get("slack_user_id")?,
                slack_username: row.try_get("slack_username")?,
                notify_on_pr: row.try_get::<i32, _>("notify_on_pr")? == 1,
                notify_on_mention: row.try_get::<i32, _>("notify_on_mention")? == 1,
                notify_on_failure: row.try_get::<i32, _>("notify_on_failure")? == 1,
                created_at: row.try_get::<String, _>("created_at")?.parse()?,
            });
        }

        Ok(mappings)
    }

    /// Delete a user mapping
    pub async fn delete_user_mapping(&self, github_username: &str) -> Result<()> {
        sqlx::query("DELETE FROM slack_user_mappings WHERE github_username = ?")
            .bind(github_username)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// Add code owner
    pub async fn add_code_owner(&self, pattern: &str, github_username: &str) -> Result<CodeOwner> {
        let owner = CodeOwner::new(pattern, github_username);

        sqlx::query(
            r#"
            INSERT INTO slack_code_owners (id, pattern, github_username, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&owner.id)
        .bind(&owner.pattern)
        .bind(&owner.github_username)
        .bind(&owner.created_at)
        .execute(self.db.pool())
        .await?;

        Ok(owner)
    }

    /// Get code owners for files
    pub async fn get_code_owners_for_files(&self, files: &[String]) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT pattern, github_username
            FROM slack_code_owners
            ORDER BY github_username
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut owners = Vec::new();
        for row in rows {
            let pattern: String = row.try_get("pattern")?;
            let github_username: String = row.try_get("github_username")?;

            // Simple pattern matching - in production would use glob patterns
            for file in files {
                if file.contains(&pattern) || pattern == "*" {
                    if !owners.contains(&github_username) {
                        owners.push(github_username.clone());
                    }
                    break;
                }
            }
        }

        Ok(owners)
    }

    /// List all code owners
    pub async fn list_code_owners(&self) -> Result<Vec<CodeOwner>> {
        let rows = sqlx::query(
            r#"
            SELECT id, pattern, github_username, created_at
            FROM slack_code_owners
            ORDER BY pattern
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut owners = Vec::new();
        for row in rows {
            owners.push(CodeOwner {
                id: row.try_get("id")?,
                pattern: row.try_get("pattern")?,
                github_username: row.try_get("github_username")?,
                created_at: row.try_get::<String, _>("created_at")?.parse()?,
            });
        }

        Ok(owners)
    }

    /// Delete a code owner
    pub async fn delete_code_owner(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM slack_code_owners WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// Mention users in a message
    pub fn add_mentions_to_message(&self, message: SlackMessage, slack_user_ids: &[String]) -> SlackMessage {
        if slack_user_ids.is_empty() {
            return message;
        }

        let mentions = slack_user_ids
            .iter()
            .map(|id| format!("<@{}>", id))
            .collect::<Vec<_>>()
            .join(" ");

        let mut updated = message;
        updated.text = format!("{}\n{}", mentions, updated.text);
        updated
    }

    /// Send a direct message to a user
    pub async fn send_dm(
        &self,
        slack_user_id: &str,
        message: SlackMessage,
        notification_type: NotificationType,
    ) -> Result<()> {
        // In Slack, DMs are sent to the user's ID as the channel
        let mut dm_message = message;
        dm_message.channel = slack_user_id.to_string();

        self.slack_service
            .send_notification(notification_type, dm_message, None, None)
            .await?;

        Ok(())
    }

    /// Check if user should receive DM for urgent notifications
    pub async fn should_send_dm(&self, github_username: &str, notification_type: &NotificationType) -> Result<bool> {
        let mapping = self.slack_service.get_user_mapping(github_username).await?;

        if let Some(mapping) = mapping {
            // Check notification preferences
            match notification_type {
                NotificationType::AgentFailed => Ok(mapping.notify_on_failure),
                NotificationType::PrCreated | NotificationType::PrMerged => Ok(mapping.notify_on_pr),
                NotificationType::ApprovalRequired => Ok(true), // Always DM for approvals
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    async fn setup_test_db() -> Database {
        Database::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_map_user() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        let mapping = service
            .map_user("github-user", "U12345", "slack-user")
            .await
            .unwrap();

        assert_eq!(mapping.github_username, "github-user");
        assert_eq!(mapping.slack_user_id, "U12345");
        assert_eq!(mapping.slack_username, "slack-user");
    }

    #[tokio::test]
    async fn test_get_slack_user_id() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        service
            .map_user("github-user", "U12345", "slack-user")
            .await
            .unwrap();

        let slack_id = service.get_slack_user_id("github-user").await.unwrap();
        assert_eq!(slack_id, Some("U12345".to_string()));

        let not_found = service.get_slack_user_id("nonexistent").await.unwrap();
        assert_eq!(not_found, None);
    }

    #[tokio::test]
    async fn test_list_user_mappings() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        service.map_user("user1", "U1", "slack1").await.unwrap();
        service.map_user("user2", "U2", "slack2").await.unwrap();

        let mappings = service.list_user_mappings().await.unwrap();
        assert_eq!(mappings.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_user_mapping() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        service
            .map_user("github-user", "U12345", "slack-user")
            .await
            .unwrap();

        service.delete_user_mapping("github-user").await.unwrap();

        let slack_id = service.get_slack_user_id("github-user").await.unwrap();
        assert_eq!(slack_id, None);
    }

    #[tokio::test]
    async fn test_add_code_owner() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        let owner = service.add_code_owner("src/api/*", "api-team").await.unwrap();
        assert_eq!(owner.pattern, "src/api/*");
        assert_eq!(owner.github_username, "api-team");
    }

    #[tokio::test]
    async fn test_get_code_owners_for_files() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        service.add_code_owner("src/api", "api-team").await.unwrap();
        service.add_code_owner("src/db", "db-team").await.unwrap();

        let owners = service
            .get_code_owners_for_files(&["src/api/routes.rs".to_string()])
            .await
            .unwrap();

        assert!(owners.contains(&"api-team".to_string()));
        assert!(!owners.contains(&"db-team".to_string()));
    }

    #[tokio::test]
    async fn test_list_code_owners() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        service.add_code_owner("src/api/*", "api-team").await.unwrap();
        service.add_code_owner("src/db/*", "db-team").await.unwrap();

        let owners = service.list_code_owners().await.unwrap();
        assert_eq!(owners.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_code_owner() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        let owner = service.add_code_owner("src/api/*", "api-team").await.unwrap();
        service.delete_code_owner(&owner.id).await.unwrap();

        let owners = service.list_code_owners().await.unwrap();
        assert_eq!(owners.len(), 0);
    }

    #[tokio::test]
    async fn test_add_mentions_to_message() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        let message = SlackMessage::new("#test", "Hello");
        let mentioned = service.add_mentions_to_message(message, &["U1".to_string(), "U2".to_string()]);

        assert!(mentioned.text.contains("<@U1>"));
        assert!(mentioned.text.contains("<@U2>"));
    }

    #[tokio::test]
    async fn test_should_send_dm_for_failure() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        service.map_user("github-user", "U12345", "slack-user").await.unwrap();

        let should_dm = service
            .should_send_dm("github-user", &NotificationType::AgentFailed)
            .await
            .unwrap();

        assert!(should_dm); // By default notify_on_failure is true
    }

    #[tokio::test]
    async fn test_should_send_dm_for_approval() {
        let db = setup_test_db().await;
        let slack_service = SlackService::new_for_testing(db.clone());
        let service = SlackUserService::new(db, slack_service);

        service.map_user("github-user", "U12345", "slack-user").await.unwrap();

        let should_dm = service
            .should_send_dm("github-user", &NotificationType::ApprovalRequired)
            .await
            .unwrap();

        assert!(should_dm); // Always DM for approvals
    }
}
