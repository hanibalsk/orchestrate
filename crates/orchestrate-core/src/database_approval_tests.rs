//! Tests for approval database operations

#[cfg(test)]
mod tests {
    use crate::{
        approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus},
        Database, Pipeline, PipelineRun, PipelineStage,
    };

    #[tokio::test]
    async fn test_create_approval_request() {
        let db = Database::in_memory().await.unwrap();

        // Create a pipeline and run first
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        // Create approval request
        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            "user1@example.com,user2@example.com".to_string(),
            2,
            Some(3600),
            Some("approve".to_string()),
        );

        let created = db.create_approval_request(request).await.unwrap();

        assert!(created.id.is_some());
        assert_eq!(created.stage_id, stage_id);
        assert_eq!(created.run_id, run_id);
        assert_eq!(created.status, ApprovalStatus::Pending);
        assert_eq!(created.required_count, 2);
        assert_eq!(created.approval_count, 0);
        assert_eq!(created.rejection_count, 0);
    }

    #[tokio::test]
    async fn test_get_approval_request() {
        let db = Database::in_memory().await.unwrap();

        // Setup
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            "user@example.com".to_string(),
            1,
            None,
            None,
        );
        let created = db.create_approval_request(request).await.unwrap();
        let approval_id = created.id.unwrap();

        // Test get
        let fetched = db.get_approval_request(approval_id).await.unwrap();
        assert!(fetched.is_some());

        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, Some(approval_id));
        assert_eq!(fetched.stage_id, stage_id);
        assert_eq!(fetched.status, ApprovalStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_approval_request() {
        let db = Database::in_memory().await.unwrap();

        // Setup
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            "user@example.com".to_string(),
            1,
            None,
            None,
        );
        let mut created = db.create_approval_request(request).await.unwrap();

        // Update
        created.approval_count = 1;
        created.mark_approved();

        db.update_approval_request(&created).await.unwrap();

        // Verify
        let fetched = db
            .get_approval_request(created.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.status, ApprovalStatus::Approved);
        assert_eq!(fetched.approval_count, 1);
        assert!(fetched.resolved_at.is_some());
    }

    #[tokio::test]
    async fn test_get_approval_request_by_stage() {
        let db = Database::in_memory().await.unwrap();

        // Setup
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            "user@example.com".to_string(),
            1,
            None,
            None,
        );
        db.create_approval_request(request).await.unwrap();

        // Test get by stage
        let fetched = db.get_approval_request_by_stage(stage_id).await.unwrap();
        assert!(fetched.is_some());

        let fetched = fetched.unwrap();
        assert_eq!(fetched.stage_id, stage_id);
        assert_eq!(fetched.status, ApprovalStatus::Pending);
    }

    #[tokio::test]
    async fn test_list_pending_approvals() {
        let db = Database::in_memory().await.unwrap();

        // Setup pipeline and run
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        // Create multiple stages with approvals
        let stage1 = PipelineStage::new(run_id, "deploy-staging".to_string());
        let stage1_id = db.insert_pipeline_stage(&stage1).await.unwrap();

        let stage2 = PipelineStage::new(run_id, "deploy-prod".to_string());
        let stage2_id = db.insert_pipeline_stage(&stage2).await.unwrap();

        // Create pending approval
        let request1 = ApprovalRequest::new(
            stage1_id,
            run_id,
            "user@example.com".to_string(),
            1,
            None,
            None,
        );
        db.create_approval_request(request1).await.unwrap();

        // Create approved approval
        let mut request2 = ApprovalRequest::new(
            stage2_id,
            run_id,
            "user@example.com".to_string(),
            1,
            None,
            None,
        );
        request2.mark_approved();
        db.create_approval_request(request2).await.unwrap();

        // List pending
        let pending = db.list_pending_approvals().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].stage_id, stage1_id);
        assert_eq!(pending[0].status, ApprovalStatus::Pending);
    }

    #[tokio::test]
    async fn test_create_approval_decision() {
        let db = Database::in_memory().await.unwrap();

        // Setup
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            "user@example.com".to_string(),
            1,
            None,
            None,
        );
        let created_request = db.create_approval_request(request).await.unwrap();
        let approval_id = created_request.id.unwrap();

        // Create decision
        let decision = ApprovalDecision::new(
            approval_id,
            "user@example.com".to_string(),
            true,
            Some("LGTM".to_string()),
        );

        let created = db.create_approval_decision(decision).await.unwrap();

        assert!(created.id.is_some());
        assert_eq!(created.approval_id, approval_id);
        assert_eq!(created.approver, "user@example.com");
        assert!(created.decision);
        assert_eq!(created.comment, Some("LGTM".to_string()));
    }

    #[tokio::test]
    async fn test_get_approval_decisions() {
        let db = Database::in_memory().await.unwrap();

        // Setup
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            "user1@example.com,user2@example.com".to_string(),
            2,
            None,
            None,
        );
        let created_request = db.create_approval_request(request).await.unwrap();
        let approval_id = created_request.id.unwrap();

        // Create multiple decisions
        let decision1 = ApprovalDecision::new(
            approval_id,
            "user1@example.com".to_string(),
            true,
            Some("LGTM".to_string()),
        );
        db.create_approval_decision(decision1).await.unwrap();

        let decision2 = ApprovalDecision::new(
            approval_id,
            "user2@example.com".to_string(),
            true,
            Some("Looks good".to_string()),
        );
        db.create_approval_decision(decision2).await.unwrap();

        // Get decisions
        let decisions = db.get_approval_decisions(approval_id).await.unwrap();
        assert_eq!(decisions.len(), 2);
        assert!(decisions.iter().all(|d| d.decision));
    }

    #[tokio::test]
    async fn test_list_timed_out_approvals() {
        let db = Database::in_memory().await.unwrap();

        // Setup
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        // Create approval that has already timed out (1 second timeout in the past)
        let request = ApprovalRequest::new(
            stage_id,
            run_id,
            "user@example.com".to_string(),
            1,
            Some(-10), // Negative timeout means it's already in the past
            Some("approve".to_string()),
        );
        db.create_approval_request(request).await.unwrap();

        // List timed out
        let timed_out = db.list_timed_out_approvals().await.unwrap();
        assert_eq!(timed_out.len(), 1);
        assert_eq!(timed_out[0].stage_id, stage_id);
    }
}
