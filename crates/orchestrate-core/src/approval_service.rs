//! Approval service for managing approval workflow

use crate::{
    approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus},
    Database, Result,
};

/// Service for managing approval workflow
pub struct ApprovalService {
    db: Database,
}

impl ApprovalService {
    /// Create a new approval service
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Create an approval request for a stage
    pub async fn create_approval(
        &self,
        stage_id: i64,
        run_id: i64,
        approvers: Vec<String>,
        required_count: i32,
        timeout_seconds: Option<i64>,
        timeout_action: Option<String>,
    ) -> Result<ApprovalRequest> {
        let required_approvers = approvers.join(",");
        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            required_approvers,
            required_count,
            timeout_seconds,
            timeout_action,
        );

        self.db.create_approval_request(request).await
    }

    /// Submit an approval decision
    pub async fn approve(
        &self,
        approval_id: i64,
        approver: String,
        comment: Option<String>,
    ) -> Result<ApprovalRequest> {
        // Get the approval request
        let mut request = self
            .db
            .get_approval_request(approval_id)
            .await?
            .ok_or_else(|| crate::Error::Other("Approval request not found".to_string()))?;

        // Check if already resolved
        if request.status.is_terminal() {
            return Err(crate::Error::Other(format!(
                "Approval request already resolved with status: {:?}",
                request.status
            )));
        }

        // Check if approver is in the list
        if !request.required_approvers.split(',').any(|a| a == approver) {
            return Err(crate::Error::Other(format!(
                "User '{}' is not an authorized approver",
                approver
            )));
        }

        // Check if approver already voted
        let existing_decisions = self.db.get_approval_decisions(approval_id).await?;
        if existing_decisions.iter().any(|d| d.approver == approver) {
            return Err(crate::Error::Other(format!(
                "User '{}' has already submitted a decision",
                approver
            )));
        }

        // Record the decision
        let decision = ApprovalDecision::new(approval_id, approver, true, comment);
        self.db.create_approval_decision(decision).await?;

        // Update approval count
        request.approval_count += 1;

        // Check if quorum is reached
        if request.has_approval_quorum() {
            request.mark_approved();
        }

        // Save the updated request
        self.db.update_approval_request(&request).await?;

        Ok(request)
    }

    /// Submit a rejection decision
    pub async fn reject(
        &self,
        approval_id: i64,
        approver: String,
        reason: Option<String>,
    ) -> Result<ApprovalRequest> {
        // Get the approval request
        let mut request = self
            .db
            .get_approval_request(approval_id)
            .await?
            .ok_or_else(|| crate::Error::Other("Approval request not found".to_string()))?;

        // Check if already resolved
        if request.status.is_terminal() {
            return Err(crate::Error::Other(format!(
                "Approval request already resolved with status: {:?}",
                request.status
            )));
        }

        // Check if approver is in the list
        if !request.required_approvers.split(',').any(|a| a == approver) {
            return Err(crate::Error::Other(format!(
                "User '{}' is not an authorized approver",
                approver
            )));
        }

        // Check if approver already voted
        let existing_decisions = self.db.get_approval_decisions(approval_id).await?;
        if existing_decisions.iter().any(|d| d.approver == approver) {
            return Err(crate::Error::Other(format!(
                "User '{}' has already submitted a decision",
                approver
            )));
        }

        // Record the decision
        let decision = ApprovalDecision::new(approval_id, approver, false, reason);
        self.db.create_approval_decision(decision).await?;

        // Update rejection count
        request.rejection_count += 1;

        // Check if rejection quorum is reached
        if request.has_rejection_quorum() {
            request.mark_rejected();
        }

        // Save the updated request
        self.db.update_approval_request(&request).await?;

        Ok(request)
    }

    /// Delegate an approval to another user
    pub async fn delegate(
        &self,
        approval_id: i64,
        from_approver: String,
        to_approver: String,
    ) -> Result<ApprovalRequest> {
        // Get the approval request
        let mut request = self
            .db
            .get_approval_request(approval_id)
            .await?
            .ok_or_else(|| crate::Error::Other("Approval request not found".to_string()))?;

        // Check if already resolved
        if request.status.is_terminal() {
            return Err(crate::Error::Other(format!(
                "Approval request already resolved with status: {:?}",
                request.status
            )));
        }

        // Check if from_approver is in the list
        let approvers: Vec<String> = request
            .required_approvers
            .split(',')
            .map(|s| s.to_string())
            .collect();

        if !approvers.contains(&from_approver) {
            return Err(crate::Error::Other(format!(
                "User '{}' is not an authorized approver",
                from_approver
            )));
        }

        // Replace from_approver with to_approver
        let new_approvers: Vec<String> = approvers
            .into_iter()
            .map(|a| if a == from_approver { to_approver.clone() } else { a })
            .collect();

        request.required_approvers = new_approvers.join(",");
        request.mark_delegated();

        // Save the updated request
        self.db.update_approval_request(&request).await?;

        Ok(request)
    }

    /// Process timed out approvals
    pub async fn process_timeouts(&self) -> Result<Vec<ApprovalRequest>> {
        let timed_out = self.db.list_timed_out_approvals().await?;
        let mut processed = Vec::new();

        for mut request in timed_out {
            // Apply timeout action
            match request.timeout_action.as_deref() {
                Some("approve") => {
                    request.mark_approved();
                }
                Some("reject") | None => {
                    // Default to reject
                    request.mark_rejected();
                }
                _ => {
                    request.mark_timed_out();
                }
            }

            self.db.update_approval_request(&request).await?;
            processed.push(request);
        }

        Ok(processed)
    }

    /// List all pending approvals
    pub async fn list_pending(&self) -> Result<Vec<ApprovalRequest>> {
        self.db.list_pending_approvals().await
    }

    /// Get approval request by ID
    pub async fn get_approval(&self, approval_id: i64) -> Result<Option<ApprovalRequest>> {
        self.db.get_approval_request(approval_id).await
    }

    /// Get approval request by stage ID
    pub async fn get_approval_by_stage(&self, stage_id: i64) -> Result<Option<ApprovalRequest>> {
        self.db.get_approval_request_by_stage(stage_id).await
    }

    /// Get all decisions for an approval
    pub async fn get_decisions(&self, approval_id: i64) -> Result<Vec<ApprovalDecision>> {
        self.db.get_approval_decisions(approval_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Pipeline, PipelineRun, PipelineStage};

    async fn setup_test_approval(db: &Database) -> (i64, i64, i64) {
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        (pipeline_id, run_id, stage_id)
    }

    #[tokio::test]
    async fn test_create_approval() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        let request = service
            .create_approval(
                stage_id,
                run_id,
                vec!["user1@example.com".to_string(), "user2@example.com".to_string()],
                2,
                Some(3600),
                Some("approve".to_string()),
            )
            .await
            .unwrap();

        assert!(request.id.is_some());
        assert_eq!(request.status, ApprovalStatus::Pending);
        assert_eq!(request.required_count, 2);
    }

    #[tokio::test]
    async fn test_approve_single_approver() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        let request = service
            .create_approval(
                stage_id,
                run_id,
                vec!["user@example.com".to_string()],
                1,
                None,
                None,
            )
            .await
            .unwrap();

        let approval_id = request.id.unwrap();

        let updated = service
            .approve(
                approval_id,
                "user@example.com".to_string(),
                Some("LGTM".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(updated.status, ApprovalStatus::Approved);
        assert_eq!(updated.approval_count, 1);
    }

    #[tokio::test]
    async fn test_approve_multiple_approvers() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        let request = service
            .create_approval(
                stage_id,
                run_id,
                vec!["user1@example.com".to_string(), "user2@example.com".to_string()],
                2,
                None,
                None,
            )
            .await
            .unwrap();

        let approval_id = request.id.unwrap();

        // First approval
        let updated = service
            .approve(approval_id, "user1@example.com".to_string(), None)
            .await
            .unwrap();
        assert_eq!(updated.status, ApprovalStatus::Pending);
        assert_eq!(updated.approval_count, 1);

        // Second approval - should reach quorum
        let updated = service
            .approve(approval_id, "user2@example.com".to_string(), None)
            .await
            .unwrap();
        assert_eq!(updated.status, ApprovalStatus::Approved);
        assert_eq!(updated.approval_count, 2);
    }

    #[tokio::test]
    async fn test_reject() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        let request = service
            .create_approval(
                stage_id,
                run_id,
                vec!["user@example.com".to_string()],
                1,
                None,
                None,
            )
            .await
            .unwrap();

        let approval_id = request.id.unwrap();

        let updated = service
            .reject(
                approval_id,
                "user@example.com".to_string(),
                Some("Needs more testing".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(updated.status, ApprovalStatus::Rejected);
        assert_eq!(updated.rejection_count, 1);
    }

    #[tokio::test]
    async fn test_reject_quorum() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        let request = service
            .create_approval(
                stage_id,
                run_id,
                vec![
                    "user1@example.com".to_string(),
                    "user2@example.com".to_string(),
                    "user3@example.com".to_string(),
                ],
                2,
                None,
                None,
            )
            .await
            .unwrap();

        let approval_id = request.id.unwrap();

        // First rejection
        let updated = service
            .reject(approval_id, "user1@example.com".to_string(), None)
            .await
            .unwrap();
        assert_eq!(updated.status, ApprovalStatus::Pending);

        // Second rejection - should make it impossible to reach quorum
        let updated = service
            .reject(approval_id, "user2@example.com".to_string(), None)
            .await
            .unwrap();
        assert_eq!(updated.status, ApprovalStatus::Rejected);
    }

    #[tokio::test]
    async fn test_unauthorized_approver() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        let request = service
            .create_approval(
                stage_id,
                run_id,
                vec!["user1@example.com".to_string()],
                1,
                None,
                None,
            )
            .await
            .unwrap();

        let approval_id = request.id.unwrap();

        let result = service
            .approve(approval_id, "unauthorized@example.com".to_string(), None)
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not an authorized approver"));
    }

    #[tokio::test]
    async fn test_duplicate_decision() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        let request = service
            .create_approval(
                stage_id,
                run_id,
                vec!["user1@example.com".to_string(), "user2@example.com".to_string()],
                2,
                None,
                None,
            )
            .await
            .unwrap();

        let approval_id = request.id.unwrap();

        service
            .approve(approval_id, "user1@example.com".to_string(), None)
            .await
            .unwrap();

        let result = service
            .approve(approval_id, "user1@example.com".to_string(), None)
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already submitted a decision"));
    }

    #[tokio::test]
    async fn test_delegate() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        let request = service
            .create_approval(
                stage_id,
                run_id,
                vec!["user1@example.com".to_string(), "user2@example.com".to_string()],
                2,
                None,
                None,
            )
            .await
            .unwrap();

        let approval_id = request.id.unwrap();

        let updated = service
            .delegate(
                approval_id,
                "user1@example.com".to_string(),
                "user3@example.com".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(updated.status, ApprovalStatus::Delegated);
        assert!(updated.required_approvers.contains("user3@example.com"));
        assert!(!updated.required_approvers.contains("user1@example.com"));
    }

    #[tokio::test]
    async fn test_process_timeouts() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        // Create approval with negative timeout (already expired)
        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            "user@example.com".to_string(),
            1,
            Some(-10),
            Some("approve".to_string()),
        );
        db.create_approval_request(request).await.unwrap();

        let processed = service.process_timeouts().await.unwrap();

        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].status, ApprovalStatus::Approved);
    }

    #[tokio::test]
    async fn test_list_pending() {
        let db = Database::in_memory().await.unwrap();
        let service = ApprovalService::new(db.clone());

        let (_, run_id, stage_id) = setup_test_approval(&db).await;

        service
            .create_approval(
                stage_id,
                run_id,
                vec!["user@example.com".to_string()],
                1,
                None,
                None,
            )
            .await
            .unwrap();

        let pending = service.list_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, ApprovalStatus::Pending);
    }
}
