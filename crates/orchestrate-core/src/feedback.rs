//! User feedback collection for agent outputs
//!
//! This module provides types and functionality for collecting user feedback
//! on agent outputs to enable closed-loop learning.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::{Error, Result};

/// Rating for feedback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackRating {
    /// Positive feedback (thumbs up)
    Positive,
    /// Negative feedback (thumbs down)
    Negative,
    /// Neutral feedback
    Neutral,
}

impl FeedbackRating {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
        }
    }

    /// Convert to a numeric score for aggregation
    pub fn score(&self) -> f64 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
            Self::Neutral => 0.0,
        }
    }
}

impl FromStr for FeedbackRating {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "positive" | "pos" | "+" | "up" | "thumbs_up" => Ok(Self::Positive),
            "negative" | "neg" | "-" | "down" | "thumbs_down" => Ok(Self::Negative),
            "neutral" | "0" | "meh" => Ok(Self::Neutral),
            _ => Err(Error::Other(format!("Invalid feedback rating: {}", s))),
        }
    }
}

impl std::fmt::Display for FeedbackRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Source of the feedback
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSource {
    /// Feedback from CLI
    Cli,
    /// Feedback from web UI
    Web,
    /// Feedback from Slack reaction
    Slack,
    /// Feedback from API
    Api,
    /// Automated feedback (from test results, etc.)
    Automated,
}

impl FeedbackSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Web => "web",
            Self::Slack => "slack",
            Self::Api => "api",
            Self::Automated => "automated",
        }
    }
}

impl FromStr for FeedbackSource {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "cli" => Ok(Self::Cli),
            "web" => Ok(Self::Web),
            "slack" => Ok(Self::Slack),
            "api" => Ok(Self::Api),
            "automated" | "auto" => Ok(Self::Automated),
            _ => Err(Error::Other(format!("Invalid feedback source: {}", s))),
        }
    }
}

impl std::fmt::Display for FeedbackSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// User feedback on agent output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    /// Unique feedback ID
    pub id: i64,
    /// Agent ID this feedback is for
    pub agent_id: Uuid,
    /// Optional message ID for specific output feedback
    pub message_id: Option<i64>,
    /// Feedback rating
    pub rating: FeedbackRating,
    /// Optional comment explaining the rating
    pub comment: Option<String>,
    /// Source of the feedback
    pub source: FeedbackSource,
    /// Who provided the feedback
    pub created_by: String,
    /// When the feedback was created
    pub created_at: DateTime<Utc>,
}

impl Feedback {
    /// Create new feedback
    pub fn new(agent_id: Uuid, rating: FeedbackRating, created_by: impl Into<String>) -> Self {
        Self {
            id: 0, // Will be set by database
            agent_id,
            message_id: None,
            rating,
            comment: None,
            source: FeedbackSource::Cli,
            created_by: created_by.into(),
            created_at: Utc::now(),
        }
    }

    /// Set the message ID
    pub fn with_message_id(mut self, message_id: i64) -> Self {
        self.message_id = Some(message_id);
        self
    }

    /// Set the comment
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Set the source
    pub fn with_source(mut self, source: FeedbackSource) -> Self {
        self.source = source;
        self
    }
}

/// Statistics about feedback for an agent or overall
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    /// Total feedback count
    pub total: i64,
    /// Positive feedback count
    pub positive: i64,
    /// Negative feedback count
    pub negative: i64,
    /// Neutral feedback count
    pub neutral: i64,
    /// Calculated score (-1.0 to 1.0)
    pub score: f64,
    /// Percentage positive (0-100)
    pub positive_percentage: f64,
}

impl FeedbackStats {
    /// Create stats from counts
    pub fn from_counts(positive: i64, negative: i64, neutral: i64) -> Self {
        let total = positive + negative + neutral;
        let score = if total > 0 {
            (positive as f64 - negative as f64) / total as f64
        } else {
            0.0
        };
        let positive_percentage = if total > 0 {
            (positive as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total,
            positive,
            negative,
            neutral,
            score,
            positive_percentage,
        }
    }

    /// Create empty stats
    pub fn empty() -> Self {
        Self {
            total: 0,
            positive: 0,
            negative: 0,
            neutral: 0,
            score: 0.0,
            positive_percentage: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_rating_from_str() {
        assert_eq!(
            FeedbackRating::from_str("positive").unwrap(),
            FeedbackRating::Positive
        );
        assert_eq!(
            FeedbackRating::from_str("pos").unwrap(),
            FeedbackRating::Positive
        );
        assert_eq!(
            FeedbackRating::from_str("+").unwrap(),
            FeedbackRating::Positive
        );
        assert_eq!(
            FeedbackRating::from_str("negative").unwrap(),
            FeedbackRating::Negative
        );
        assert_eq!(
            FeedbackRating::from_str("-").unwrap(),
            FeedbackRating::Negative
        );
        assert_eq!(
            FeedbackRating::from_str("neutral").unwrap(),
            FeedbackRating::Neutral
        );
        assert!(FeedbackRating::from_str("invalid").is_err());
    }

    #[test]
    fn test_feedback_rating_score() {
        assert_eq!(FeedbackRating::Positive.score(), 1.0);
        assert_eq!(FeedbackRating::Negative.score(), -1.0);
        assert_eq!(FeedbackRating::Neutral.score(), 0.0);
    }

    #[test]
    fn test_feedback_source_from_str() {
        assert_eq!(FeedbackSource::from_str("cli").unwrap(), FeedbackSource::Cli);
        assert_eq!(FeedbackSource::from_str("web").unwrap(), FeedbackSource::Web);
        assert_eq!(
            FeedbackSource::from_str("slack").unwrap(),
            FeedbackSource::Slack
        );
        assert_eq!(FeedbackSource::from_str("api").unwrap(), FeedbackSource::Api);
        assert_eq!(
            FeedbackSource::from_str("automated").unwrap(),
            FeedbackSource::Automated
        );
    }

    #[test]
    fn test_feedback_new() {
        let agent_id = Uuid::new_v4();
        let feedback = Feedback::new(agent_id, FeedbackRating::Positive, "user@example.com");

        assert_eq!(feedback.id, 0);
        assert_eq!(feedback.agent_id, agent_id);
        assert_eq!(feedback.rating, FeedbackRating::Positive);
        assert_eq!(feedback.created_by, "user@example.com");
        assert!(feedback.message_id.is_none());
        assert!(feedback.comment.is_none());
    }

    #[test]
    fn test_feedback_builder() {
        let agent_id = Uuid::new_v4();
        let feedback = Feedback::new(agent_id, FeedbackRating::Negative, "user")
            .with_message_id(123)
            .with_comment("This didn't work")
            .with_source(FeedbackSource::Web);

        assert_eq!(feedback.message_id, Some(123));
        assert_eq!(feedback.comment, Some("This didn't work".to_string()));
        assert_eq!(feedback.source, FeedbackSource::Web);
    }

    #[test]
    fn test_feedback_stats_from_counts() {
        let stats = FeedbackStats::from_counts(7, 2, 1);

        assert_eq!(stats.total, 10);
        assert_eq!(stats.positive, 7);
        assert_eq!(stats.negative, 2);
        assert_eq!(stats.neutral, 1);
        assert!((stats.score - 0.5).abs() < 0.01); // (7 - 2) / 10 = 0.5
        assert!((stats.positive_percentage - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_feedback_stats_empty() {
        let stats = FeedbackStats::empty();

        assert_eq!(stats.total, 0);
        assert_eq!(stats.score, 0.0);
        assert_eq!(stats.positive_percentage, 0.0);
    }

    #[test]
    fn test_feedback_stats_all_positive() {
        let stats = FeedbackStats::from_counts(10, 0, 0);

        assert_eq!(stats.score, 1.0);
        assert_eq!(stats.positive_percentage, 100.0);
    }

    #[test]
    fn test_feedback_stats_all_negative() {
        let stats = FeedbackStats::from_counts(0, 10, 0);

        assert_eq!(stats.score, -1.0);
        assert_eq!(stats.positive_percentage, 0.0);
    }
}
