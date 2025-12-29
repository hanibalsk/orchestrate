//! Approval gates for pipeline stages
//!
//! This module handles human-in-the-loop approval workflow for critical pipeline stages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{Error, Result};

/// Status of an approval request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Approval is pending
    Pending,
    /// Approval was granted
    Approved,
    /// Approval was rejected
    Rejected,
    /// Approval was delegated to another approver
    Delegated,
    /// Approval timed out
    TimedOut,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Delegated => "delegated",
            Self::TimedOut => "timed_out",
        }
    }

    /// Check if the approval is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Approved | Self::Rejected | Self::TimedOut)
    }
}

impl FromStr for ApprovalStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "delegated" => Ok(Self::Delegated),
            "timed_out" => Ok(Self::TimedOut),
            _ => Err(Error::Other(format!("Invalid approval status: {}", s))),
        }
    }
}

/// An approval request for a pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Database ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Stage ID this approval is for
    pub stage_id: i64,
    /// Run ID for the pipeline
    pub run_id: i64,
    /// Current status of the approval
    pub status: ApprovalStatus,
    /// Required approvers (comma-separated list)
    pub required_approvers: String,
    /// Number of approvals required (quorum)
    pub required_count: i32,
    /// Current approval count
    pub approval_count: i32,
    /// Current rejection count
    pub rejection_count: i32,
    /// Timeout in seconds (None means no timeout)
    pub timeout_seconds: Option<i64>,
    /// Default action if timeout occurs (approve/reject)
    pub timeout_action: Option<String>,
    /// When the approval times out
    pub timeout_at: Option<DateTime<Utc>>,
    /// When the approval was resolved
    pub resolved_at: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl ApprovalRequest {
    /// Create a new approval request
    pub fn new(
        stage_id: i64,
        run_id: i64,
        required_approvers: String,
        required_count: i32,
        timeout_seconds: Option<i64>,
        timeout_action: Option<String>,
    ) -> Self {
        let created_at = Utc::now();
        let timeout_at = timeout_seconds.map(|seconds| {
            created_at + chrono::Duration::seconds(seconds)
        });

        Self {
            id: None,
            stage_id,
            run_id,
            status: ApprovalStatus::Pending,
            required_approvers,
            required_count,
            approval_count: 0,
            rejection_count: 0,
            timeout_seconds,
            timeout_action,
            timeout_at,
            resolved_at: None,
            created_at,
        }
    }

    /// Check if the request has timed out
    pub fn has_timed_out(&self) -> bool {
        if let Some(timeout_at) = self.timeout_at {
            Utc::now() > timeout_at
        } else {
            false
        }
    }

    /// Check if quorum is reached for approval
    pub fn has_approval_quorum(&self) -> bool {
        self.approval_count >= self.required_count
    }

    /// Check if too many rejections to proceed
    pub fn has_rejection_quorum(&self) -> bool {
        // If any rejection when only 1 approver required, reject immediately
        if self.required_count == 1 && self.rejection_count > 0 {
            return true;
        }
        // Otherwise check if rejections make it impossible to reach quorum
        let total_approvers = self.required_approvers.split(',').count() as i32;
        let max_possible_approvals = total_approvers - self.rejection_count;
        max_possible_approvals < self.required_count
    }

    /// Mark as approved
    pub fn mark_approved(&mut self) {
        self.status = ApprovalStatus::Approved;
        self.resolved_at = Some(Utc::now());
    }

    /// Mark as rejected
    pub fn mark_rejected(&mut self) {
        self.status = ApprovalStatus::Rejected;
        self.resolved_at = Some(Utc::now());
    }

    /// Mark as timed out
    pub fn mark_timed_out(&mut self) {
        self.status = ApprovalStatus::TimedOut;
        self.resolved_at = Some(Utc::now());
    }

    /// Mark as delegated
    pub fn mark_delegated(&mut self) {
        self.status = ApprovalStatus::Delegated;
    }
}

/// An individual approval decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDecision {
    /// Database ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Approval request ID
    pub approval_id: i64,
    /// Approver identifier (email, username, etc.)
    pub approver: String,
    /// Decision: true=approve, false=reject
    pub decision: bool,
    /// Optional comment with the decision
    pub comment: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl ApprovalDecision {
    /// Create a new approval decision
    pub fn new(approval_id: i64, approver: String, decision: bool, comment: Option<String>) -> Self {
        Self {
            id: None,
            approval_id,
            approver,
            decision,
            comment,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_status_parsing() {
        assert_eq!(
            ApprovalStatus::from_str("pending").unwrap(),
            ApprovalStatus::Pending
        );
        assert_eq!(
            ApprovalStatus::from_str("approved").unwrap(),
            ApprovalStatus::Approved
        );
        assert_eq!(
            ApprovalStatus::from_str("rejected").unwrap(),
            ApprovalStatus::Rejected
        );
        assert_eq!(
            ApprovalStatus::from_str("delegated").unwrap(),
            ApprovalStatus::Delegated
        );
        assert_eq!(
            ApprovalStatus::from_str("timed_out").unwrap(),
            ApprovalStatus::TimedOut
        );

        assert!(ApprovalStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_approval_status_as_str() {
        assert_eq!(ApprovalStatus::Pending.as_str(), "pending");
        assert_eq!(ApprovalStatus::Approved.as_str(), "approved");
        assert_eq!(ApprovalStatus::Rejected.as_str(), "rejected");
        assert_eq!(ApprovalStatus::Delegated.as_str(), "delegated");
        assert_eq!(ApprovalStatus::TimedOut.as_str(), "timed_out");
    }

    #[test]
    fn test_approval_status_is_terminal() {
        assert!(!ApprovalStatus::Pending.is_terminal());
        assert!(ApprovalStatus::Approved.is_terminal());
        assert!(ApprovalStatus::Rejected.is_terminal());
        assert!(!ApprovalStatus::Delegated.is_terminal());
        assert!(ApprovalStatus::TimedOut.is_terminal());
    }

    #[test]
    fn test_approval_request_new() {
        let request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com,user2@example.com".to_string(),
            2,
            Some(3600),
            Some("approve".to_string()),
        );

        assert_eq!(request.stage_id, 1);
        assert_eq!(request.run_id, 2);
        assert_eq!(request.status, ApprovalStatus::Pending);
        assert_eq!(request.required_count, 2);
        assert_eq!(request.approval_count, 0);
        assert_eq!(request.rejection_count, 0);
        assert!(request.timeout_at.is_some());
        assert!(request.id.is_none());
    }

    #[test]
    fn test_approval_request_no_timeout() {
        let request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );

        assert!(request.timeout_at.is_none());
        assert!(!request.has_timed_out());
    }

    #[test]
    fn test_approval_request_has_approval_quorum() {
        let mut request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com,user2@example.com".to_string(),
            2,
            None,
            None,
        );

        assert!(!request.has_approval_quorum());

        request.approval_count = 1;
        assert!(!request.has_approval_quorum());

        request.approval_count = 2;
        assert!(request.has_approval_quorum());
    }

    #[test]
    fn test_approval_request_has_rejection_quorum_single_approver() {
        let mut request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );

        assert!(!request.has_rejection_quorum());

        request.rejection_count = 1;
        assert!(request.has_rejection_quorum());
    }

    #[test]
    fn test_approval_request_has_rejection_quorum_multiple_approvers() {
        let mut request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com,user2@example.com,user3@example.com".to_string(),
            2,
            None,
            None,
        );

        assert!(!request.has_rejection_quorum());

        request.rejection_count = 1;
        assert!(!request.has_rejection_quorum()); // Still possible to get 2 approvals

        request.rejection_count = 2;
        assert!(request.has_rejection_quorum()); // Only 1 approver left, can't reach quorum of 2
    }

    #[test]
    fn test_approval_request_mark_approved() {
        let mut request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );

        request.mark_approved();
        assert_eq!(request.status, ApprovalStatus::Approved);
        assert!(request.resolved_at.is_some());
    }

    #[test]
    fn test_approval_request_mark_rejected() {
        let mut request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );

        request.mark_rejected();
        assert_eq!(request.status, ApprovalStatus::Rejected);
        assert!(request.resolved_at.is_some());
    }

    #[test]
    fn test_approval_request_mark_timed_out() {
        let mut request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );

        request.mark_timed_out();
        assert_eq!(request.status, ApprovalStatus::TimedOut);
        assert!(request.resolved_at.is_some());
    }

    #[test]
    fn test_approval_request_mark_delegated() {
        let mut request = ApprovalRequest::new(
            1,
            2,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );

        request.mark_delegated();
        assert_eq!(request.status, ApprovalStatus::Delegated);
        assert!(request.resolved_at.is_none()); // Delegation doesn't resolve
    }

    #[test]
    fn test_approval_decision_new() {
        let decision = ApprovalDecision::new(
            1,
            "user@example.com".to_string(),
            true,
            Some("LGTM".to_string()),
        );

        assert_eq!(decision.approval_id, 1);
        assert_eq!(decision.approver, "user@example.com");
        assert!(decision.decision);
        assert_eq!(decision.comment, Some("LGTM".to_string()));
        assert!(decision.id.is_none());
    }
}
