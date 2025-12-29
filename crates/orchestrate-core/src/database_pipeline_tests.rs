//! Database tests for pipeline operations

#[cfg(test)]
mod tests {
    use crate::{Database, Pipeline, PipelineRun, PipelineRunStatus, PipelineStage, PipelineStageStatus};

    #[tokio::test]
    async fn test_insert_pipeline() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new(
            "test-pipeline".to_string(),
            "name: test\nstages: []".to_string(),
        );

        let id = db.insert_pipeline(&pipeline).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn test_get_pipeline_by_id() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new(
            "get-test".to_string(),
            "name: get-test\nstages: []".to_string(),
        );

        let id = db.insert_pipeline(&pipeline).await.unwrap();
        let retrieved = db.get_pipeline(id).await.unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "get-test");
        assert_eq!(retrieved.definition, "name: get-test\nstages: []");
        assert!(retrieved.enabled);
    }

    #[tokio::test]
    async fn test_get_pipeline_by_name() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new(
            "get-by-name".to_string(),
            "name: get-by-name\nstages: []".to_string(),
        );

        db.insert_pipeline(&pipeline).await.unwrap();
        let retrieved = db.get_pipeline_by_name("get-by-name").await.unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "get-by-name");
    }

    #[tokio::test]
    async fn test_update_pipeline() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new(
            "update-test".to_string(),
            "name: update-test\nstages: []".to_string(),
        );

        let id = db.insert_pipeline(&pipeline).await.unwrap();
        let mut pipeline = db.get_pipeline(id).await.unwrap().unwrap();

        pipeline.definition = "name: updated\nstages: [build]".to_string();
        pipeline.enabled = false;

        db.update_pipeline(&pipeline).await.unwrap();

        let updated = db.get_pipeline(id).await.unwrap().unwrap();
        assert_eq!(updated.definition, "name: updated\nstages: [build]");
        assert!(!updated.enabled);
    }

    #[tokio::test]
    async fn test_list_pipelines() {
        let db = Database::in_memory().await.unwrap();

        // Insert multiple pipelines
        for i in 1..=3 {
            let pipeline = Pipeline::new(
                format!("pipeline-{}", i),
                format!("name: pipeline-{}\nstages: []", i),
            );
            db.insert_pipeline(&pipeline).await.unwrap();
        }

        let pipelines = db.list_pipelines().await.unwrap();
        assert_eq!(pipelines.len(), 3);
    }

    #[tokio::test]
    async fn test_list_enabled_pipelines() {
        let db = Database::in_memory().await.unwrap();

        // Insert enabled pipeline
        let mut pipeline1 = Pipeline::new("enabled-1".to_string(), "stages: []".to_string());
        db.insert_pipeline(&pipeline1).await.unwrap();

        // Insert disabled pipeline
        let pipeline2 = Pipeline::new("disabled-1".to_string(), "stages: []".to_string());
        let id = db.insert_pipeline(&pipeline2).await.unwrap();
        pipeline1 = db.get_pipeline(id).await.unwrap().unwrap();
        pipeline1.enabled = false;
        db.update_pipeline(&pipeline1).await.unwrap();

        let enabled = db.list_enabled_pipelines().await.unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "enabled-1");
    }

    #[tokio::test]
    async fn test_delete_pipeline() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("delete-test".to_string(), "stages: []".to_string());
        let id = db.insert_pipeline(&pipeline).await.unwrap();

        db.delete_pipeline(id).await.unwrap();

        let retrieved = db.get_pipeline(id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_insert_pipeline_run() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, Some("pull_request.merged".to_string()));
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        assert!(run_id > 0);
    }

    #[tokio::test]
    async fn test_get_pipeline_run() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, Some("push".to_string()));
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let retrieved = db.get_pipeline_run(run_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.pipeline_id, pipeline_id);
        assert_eq!(retrieved.status, PipelineRunStatus::Pending);
        assert_eq!(retrieved.trigger_event, Some("push".to_string()));
    }

    #[tokio::test]
    async fn test_update_pipeline_run() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let mut run = db.get_pipeline_run(run_id).await.unwrap().unwrap();
        run.mark_running();
        db.update_pipeline_run(&run).await.unwrap();

        let updated = db.get_pipeline_run(run_id).await.unwrap().unwrap();
        assert_eq!(updated.status, PipelineRunStatus::Running);
        assert!(updated.started_at.is_some());
    }

    #[tokio::test]
    async fn test_list_pipeline_runs() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        // Insert multiple runs
        for _ in 1..=3 {
            let run = PipelineRun::new(pipeline_id, None);
            db.insert_pipeline_run(&run).await.unwrap();
        }

        let runs = db.list_pipeline_runs(pipeline_id).await.unwrap();
        assert_eq!(runs.len(), 3);
    }

    #[tokio::test]
    async fn test_list_pipeline_runs_by_status() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        // Insert pending run
        let run1 = PipelineRun::new(pipeline_id, None);
        db.insert_pipeline_run(&run1).await.unwrap();

        // Insert running run
        let run2 = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run2).await.unwrap();
        let mut run2 = db.get_pipeline_run(run_id).await.unwrap().unwrap();
        run2.mark_running();
        db.update_pipeline_run(&run2).await.unwrap();

        let pending = db
            .list_pipeline_runs_by_status(PipelineRunStatus::Pending)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);

        let running = db
            .list_pipeline_runs_by_status(PipelineRunStatus::Running)
            .await
            .unwrap();
        assert_eq!(running.len(), 1);
    }

    #[tokio::test]
    async fn test_insert_pipeline_stage() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "build".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        assert!(stage_id > 0);
    }

    #[tokio::test]
    async fn test_get_pipeline_stage() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "test".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        let retrieved = db.get_pipeline_stage(stage_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.run_id, run_id);
        assert_eq!(retrieved.stage_name, "test");
        assert_eq!(retrieved.status, PipelineStageStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_pipeline_stage() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        let mut stage = db.get_pipeline_stage(stage_id).await.unwrap().unwrap();
        stage.mark_running(None);
        db.update_pipeline_stage(&stage).await.unwrap();

        let updated = db.get_pipeline_stage(stage_id).await.unwrap().unwrap();
        assert_eq!(updated.status, PipelineStageStatus::Running);
        assert!(updated.started_at.is_some());
    }

    #[tokio::test]
    async fn test_list_pipeline_stages() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        // Insert multiple stages
        for stage_name in &["build", "test", "deploy"] {
            let stage = PipelineStage::new(run_id, stage_name.to_string());
            db.insert_pipeline_stage(&stage).await.unwrap();
        }

        let stages = db.list_pipeline_stages(run_id).await.unwrap();
        assert_eq!(stages.len(), 3);
    }

    #[tokio::test]
    async fn test_list_pipeline_stages_by_status() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        // Insert pending stage
        let stage1 = PipelineStage::new(run_id, "build".to_string());
        db.insert_pipeline_stage(&stage1).await.unwrap();

        // Insert running stage
        let stage2 = PipelineStage::new(run_id, "test".to_string());
        let stage_id = db.insert_pipeline_stage(&stage2).await.unwrap();
        let mut stage2 = db.get_pipeline_stage(stage_id).await.unwrap().unwrap();
        stage2.mark_running(None);
        db.update_pipeline_stage(&stage2).await.unwrap();

        let pending = db
            .list_pipeline_stages_by_status(run_id, PipelineStageStatus::Pending)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);

        let running = db
            .list_pipeline_stages_by_status(run_id, PipelineStageStatus::Running)
            .await
            .unwrap();
        assert_eq!(running.len(), 1);
    }

    #[tokio::test]
    async fn test_get_pipeline_stage_by_name() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("test-pipeline".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "unique-stage".to_string());
        db.insert_pipeline_stage(&stage).await.unwrap();

        let retrieved = db
            .get_pipeline_stage_by_name(run_id, "unique-stage")
            .await
            .unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.stage_name, "unique-stage");
    }

    #[tokio::test]
    async fn test_pipeline_run_lifecycle() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("lifecycle-test".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        // Create run
        let run = PipelineRun::new(pipeline_id, Some("manual".to_string()));
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        // Mark as running
        let mut run = db.get_pipeline_run(run_id).await.unwrap().unwrap();
        run.mark_running();
        db.update_pipeline_run(&run).await.unwrap();

        // Mark as succeeded
        run.mark_succeeded();
        db.update_pipeline_run(&run).await.unwrap();

        let final_run = db.get_pipeline_run(run_id).await.unwrap().unwrap();
        assert_eq!(final_run.status, PipelineRunStatus::Succeeded);
        assert!(final_run.started_at.is_some());
        assert!(final_run.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_pipeline_stage_lifecycle() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("stage-lifecycle".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "build".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        // Mark as running (without agent for this test)
        let mut stage = db.get_pipeline_stage(stage_id).await.unwrap().unwrap();
        stage.mark_running(None);
        db.update_pipeline_stage(&stage).await.unwrap();

        // Mark as succeeded
        stage.mark_succeeded();
        db.update_pipeline_stage(&stage).await.unwrap();

        let final_stage = db.get_pipeline_stage(stage_id).await.unwrap().unwrap();
        assert_eq!(final_stage.status, PipelineStageStatus::Succeeded);
        assert!(final_stage.started_at.is_some());
        assert!(final_stage.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_cascade_delete_pipeline() {
        let db = Database::in_memory().await.unwrap();

        let pipeline = Pipeline::new("cascade-test".to_string(), "stages: []".to_string());
        let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = db.insert_pipeline_run(&run).await.unwrap();

        let stage = PipelineStage::new(run_id, "build".to_string());
        let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

        // Delete pipeline should cascade to runs and stages
        db.delete_pipeline(pipeline_id).await.unwrap();

        assert!(db.get_pipeline_run(run_id).await.unwrap().is_none());
        assert!(db.get_pipeline_stage(stage_id).await.unwrap().is_none());
    }
}
