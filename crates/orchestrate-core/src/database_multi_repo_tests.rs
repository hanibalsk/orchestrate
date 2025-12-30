//! Tests for multi-repository database operations

use crate::{
    CoordinatedRelease, CrossRepoBranch, Database, LinkedPr, LinkedPrGroup, LinkedPrStatus,
    ReleaseStatus, RepoBranchStatus, RepoConfig, RepoDependencyGraph, RepoProvider, RepoRelease,
    RepoStatus, Repository,
};
use chrono::Utc;

#[tokio::test]
async fn test_insert_repository() {
    let db = Database::in_memory().await.unwrap();

    let repo = Repository::new("api", "https://github.com/org/api");

    db.insert_repository(&repo).await.unwrap();

    let retrieved = db.get_repository_by_name("api").await.unwrap().unwrap();
    assert_eq!(retrieved.name, "api");
    assert_eq!(retrieved.url, "https://github.com/org/api");
    assert_eq!(retrieved.provider, RepoProvider::GitHub);
    assert_eq!(retrieved.status, RepoStatus::Inactive);
}

#[tokio::test]
async fn test_update_repository() {
    let db = Database::in_memory().await.unwrap();

    let mut repo = Repository::new("api", "https://github.com/org/api");
    db.insert_repository(&repo).await.unwrap();

    repo.status = RepoStatus::Active;
    repo.local_path = Some("/path/to/api".to_string());

    db.update_repository(&repo).await.unwrap();

    let retrieved = db.get_repository_by_name("api").await.unwrap().unwrap();
    assert_eq!(retrieved.status, RepoStatus::Active);
    assert_eq!(retrieved.local_path, Some("/path/to/api".to_string()));
}

#[tokio::test]
async fn test_list_repositories() {
    let db = Database::in_memory().await.unwrap();

    let repo1 = Repository::new("api", "https://github.com/org/api");
    let repo2 = Repository::new("web", "https://github.com/org/web");

    db.insert_repository(&repo1).await.unwrap();
    db.insert_repository(&repo2).await.unwrap();

    let repos = db.list_repositories().await.unwrap();
    assert_eq!(repos.len(), 2);
}

#[tokio::test]
async fn test_delete_repository() {
    let db = Database::in_memory().await.unwrap();

    let repo = Repository::new("api", "https://github.com/org/api");
    db.insert_repository(&repo).await.unwrap();

    db.delete_repository("api").await.unwrap();

    let retrieved = db.get_repository_by_name("api").await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_add_repository_dependency() {
    let db = Database::in_memory().await.unwrap();

    let repo1 = Repository::new("api", "https://github.com/org/api");
    let repo2 = Repository::new("core", "https://github.com/org/core");

    db.insert_repository(&repo1).await.unwrap();
    db.insert_repository(&repo2).await.unwrap();

    db.add_repository_dependency("api", "core").await.unwrap();

    let deps = db.get_repository_dependencies("api").await.unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0], "core");
}

#[tokio::test]
async fn test_get_dependency_graph() {
    let db = Database::in_memory().await.unwrap();

    let repo1 = Repository::new("api", "https://github.com/org/api");
    let repo2 = Repository::new("web", "https://github.com/org/web");
    let repo3 = Repository::new("core", "https://github.com/org/core");

    db.insert_repository(&repo1).await.unwrap();
    db.insert_repository(&repo2).await.unwrap();
    db.insert_repository(&repo3).await.unwrap();

    db.add_repository_dependency("api", "core").await.unwrap();
    db.add_repository_dependency("web", "api").await.unwrap();

    let graph = db.get_dependency_graph().await.unwrap();
    assert_eq!(graph.repositories.len(), 3);
    assert_eq!(graph.repositories.get("api").unwrap().len(), 1);
    assert_eq!(graph.repositories.get("web").unwrap().len(), 1);
    assert_eq!(graph.repositories.get("core").unwrap().len(), 0);
}

#[tokio::test]
async fn test_remove_repository_dependency() {
    let db = Database::in_memory().await.unwrap();

    let repo1 = Repository::new("api", "https://github.com/org/api");
    let repo2 = Repository::new("core", "https://github.com/org/core");

    db.insert_repository(&repo1).await.unwrap();
    db.insert_repository(&repo2).await.unwrap();

    db.add_repository_dependency("api", "core").await.unwrap();
    db.remove_repository_dependency("api", "core")
        .await
        .unwrap();

    let deps = db.get_repository_dependencies("api").await.unwrap();
    assert_eq!(deps.len(), 0);
}

#[tokio::test]
async fn test_create_cross_repo_branch() {
    let db = Database::in_memory().await.unwrap();

    let branch = CrossRepoBranch {
        name: "feature/new-api".to_string(),
        repos: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    db.insert_cross_repo_branch(&branch).await.unwrap();

    let retrieved = db
        .get_cross_repo_branch("feature/new-api")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.name, "feature/new-api");
}

#[tokio::test]
async fn test_update_repo_branch_status() {
    let db = Database::in_memory().await.unwrap();

    let repo = Repository::new("api", "https://github.com/org/api");
    db.insert_repository(&repo).await.unwrap();

    let branch = CrossRepoBranch {
        name: "feature/new-api".to_string(),
        repos: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.insert_cross_repo_branch(&branch).await.unwrap();

    let status = RepoBranchStatus {
        repo_name: "api".to_string(),
        branch_exists: true,
        commits_ahead: Some(3),
        commits_behind: Some(0),
        has_conflicts: false,
        pr_number: Some(123),
        pr_status: Some("open".to_string()),
    };

    db.update_repo_branch_status("feature/new-api", &status)
        .await
        .unwrap();

    let branch = db
        .get_cross_repo_branch("feature/new-api")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(branch.repos.len(), 1);
    assert_eq!(branch.repos[0].pr_number, Some(123));
}

#[tokio::test]
async fn test_list_cross_repo_branches() {
    let db = Database::in_memory().await.unwrap();

    let branch1 = CrossRepoBranch {
        name: "feature/api".to_string(),
        repos: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let branch2 = CrossRepoBranch {
        name: "feature/web".to_string(),
        repos: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    db.insert_cross_repo_branch(&branch1).await.unwrap();
    db.insert_cross_repo_branch(&branch2).await.unwrap();

    let branches = db.list_cross_repo_branches().await.unwrap();
    assert_eq!(branches.len(), 2);
}

#[tokio::test]
async fn test_create_linked_pr_group() {
    let db = Database::in_memory().await.unwrap();

    let group = LinkedPrGroup {
        id: "group-1".to_string(),
        name: "Multi-repo feature".to_string(),
        prs: vec![],
        merge_order: vec![],
        status: LinkedPrStatus::Open,
        created_at: Utc::now(),
    };

    db.insert_linked_pr_group(&group).await.unwrap();

    let retrieved = db.get_linked_pr_group("group-1").await.unwrap().unwrap();
    assert_eq!(retrieved.id, "group-1");
    assert_eq!(retrieved.status, LinkedPrStatus::Open);
}

#[tokio::test]
async fn test_add_linked_pr() {
    let db = Database::in_memory().await.unwrap();

    let repo = Repository::new("api", "https://github.com/org/api");
    db.insert_repository(&repo).await.unwrap();

    let group = LinkedPrGroup {
        id: "group-1".to_string(),
        name: "Multi-repo feature".to_string(),
        prs: vec![],
        merge_order: vec![],
        status: LinkedPrStatus::Open,
        created_at: Utc::now(),
    };
    db.insert_linked_pr_group(&group).await.unwrap();

    let pr = LinkedPr {
        repo_name: "api".to_string(),
        pr_number: 123,
        title: "Add new feature".to_string(),
        status: "open".to_string(),
        mergeable: true,
    };

    db.add_linked_pr("group-1", &pr, 0).await.unwrap();

    let group = db.get_linked_pr_group("group-1").await.unwrap().unwrap();
    assert_eq!(group.prs.len(), 1);
    assert_eq!(group.prs[0].pr_number, 123);
}

#[tokio::test]
async fn test_update_linked_pr_group_status() {
    let db = Database::in_memory().await.unwrap();

    let group = LinkedPrGroup {
        id: "group-1".to_string(),
        name: "Multi-repo feature".to_string(),
        prs: vec![],
        merge_order: vec![],
        status: LinkedPrStatus::Open,
        created_at: Utc::now(),
    };
    db.insert_linked_pr_group(&group).await.unwrap();

    db.update_linked_pr_group_status("group-1", LinkedPrStatus::ReadyToMerge)
        .await
        .unwrap();

    let group = db.get_linked_pr_group("group-1").await.unwrap().unwrap();
    assert_eq!(group.status, LinkedPrStatus::ReadyToMerge);
}

#[tokio::test]
async fn test_list_linked_pr_groups() {
    let db = Database::in_memory().await.unwrap();

    let group1 = LinkedPrGroup {
        id: "group-1".to_string(),
        name: "Feature 1".to_string(),
        prs: vec![],
        merge_order: vec![],
        status: LinkedPrStatus::Open,
        created_at: Utc::now(),
    };

    let group2 = LinkedPrGroup {
        id: "group-2".to_string(),
        name: "Feature 2".to_string(),
        prs: vec![],
        merge_order: vec![],
        status: LinkedPrStatus::Merged,
        created_at: Utc::now(),
    };

    db.insert_linked_pr_group(&group1).await.unwrap();
    db.insert_linked_pr_group(&group2).await.unwrap();

    let groups = db.list_linked_pr_groups(None).await.unwrap();
    assert_eq!(groups.len(), 2);

    let open_groups = db
        .list_linked_pr_groups(Some(LinkedPrStatus::Open))
        .await
        .unwrap();
    assert_eq!(open_groups.len(), 1);
}

#[tokio::test]
async fn test_create_coordinated_release() {
    let db = Database::in_memory().await.unwrap();

    let release = CoordinatedRelease {
        version: "1.0.0".to_string(),
        repos: vec![],
        status: ReleaseStatus::Pending,
        changelog: "Initial release".to_string(),
        created_at: Utc::now(),
        completed_at: None,
    };

    db.insert_coordinated_release(&release).await.unwrap();

    let releases = db.list_coordinated_releases(None).await.unwrap();
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].version, "1.0.0");
}

#[tokio::test]
async fn test_add_repo_release() {
    let db = Database::in_memory().await.unwrap();

    let repo = Repository::new("api", "https://github.com/org/api");
    db.insert_repository(&repo).await.unwrap();

    let release = CoordinatedRelease {
        version: "1.0.0".to_string(),
        repos: vec![],
        status: ReleaseStatus::Pending,
        changelog: "Initial release".to_string(),
        created_at: Utc::now(),
        completed_at: None,
    };
    let release_id = db.insert_coordinated_release(&release).await.unwrap();

    let repo_release = RepoRelease {
        repo_name: "api".to_string(),
        version: "1.0.0".to_string(),
        status: ReleaseStatus::Pending,
        tag: Some("v1.0.0".to_string()),
        release_url: None,
    };

    db.add_repo_release(release_id, &repo_release).await.unwrap();

    let release = db.get_coordinated_release(release_id).await.unwrap().unwrap();
    assert_eq!(release.repos.len(), 1);
    assert_eq!(release.repos[0].version, "1.0.0");
}

#[tokio::test]
async fn test_update_repo_release_status() {
    let db = Database::in_memory().await.unwrap();

    let repo = Repository::new("api", "https://github.com/org/api");
    db.insert_repository(&repo).await.unwrap();

    let release = CoordinatedRelease {
        version: "1.0.0".to_string(),
        repos: vec![],
        status: ReleaseStatus::Pending,
        changelog: "Initial release".to_string(),
        created_at: Utc::now(),
        completed_at: None,
    };
    let release_id = db.insert_coordinated_release(&release).await.unwrap();

    let repo_release = RepoRelease {
        repo_name: "api".to_string(),
        version: "1.0.0".to_string(),
        status: ReleaseStatus::Pending,
        tag: Some("v1.0.0".to_string()),
        release_url: None,
    };

    db.add_repo_release(release_id, &repo_release).await.unwrap();

    db.update_repo_release_status(release_id, "api", ReleaseStatus::Completed)
        .await
        .unwrap();

    let release = db.get_coordinated_release(release_id).await.unwrap().unwrap();
    assert_eq!(release.repos[0].status, ReleaseStatus::Completed);
}

#[tokio::test]
async fn test_update_coordinated_release_status() {
    let db = Database::in_memory().await.unwrap();

    let release = CoordinatedRelease {
        version: "1.0.0".to_string(),
        repos: vec![],
        status: ReleaseStatus::Pending,
        changelog: "Initial release".to_string(),
        created_at: Utc::now(),
        completed_at: None,
    };
    let release_id = db.insert_coordinated_release(&release).await.unwrap();

    db.update_coordinated_release_status(release_id, ReleaseStatus::InProgress)
        .await
        .unwrap();

    let release = db.get_coordinated_release(release_id).await.unwrap().unwrap();
    assert_eq!(release.status, ReleaseStatus::InProgress);
}

#[tokio::test]
async fn test_complete_coordinated_release() {
    let db = Database::in_memory().await.unwrap();

    let release = CoordinatedRelease {
        version: "1.0.0".to_string(),
        repos: vec![],
        status: ReleaseStatus::Pending,
        changelog: "Initial release".to_string(),
        created_at: Utc::now(),
        completed_at: None,
    };
    let release_id = db.insert_coordinated_release(&release).await.unwrap();

    db.complete_coordinated_release(release_id).await.unwrap();

    let release = db.get_coordinated_release(release_id).await.unwrap().unwrap();
    assert_eq!(release.status, ReleaseStatus::Completed);
    assert!(release.completed_at.is_some());
}

#[tokio::test]
async fn test_detect_package_dependencies() {
    let db = Database::in_memory().await.unwrap();

    let repo1 = Repository::new("api", "https://github.com/org/api");
    let repo2 = Repository::new("core", "https://github.com/org/core");

    db.insert_repository(&repo1).await.unwrap();
    db.insert_repository(&repo2).await.unwrap();

    // Simulate package.json detection
    let dependencies = vec!["@org/core".to_string()];

    db.detect_dependencies_from_packages("api", &dependencies)
        .await
        .unwrap();

    let deps = db.get_repository_dependencies("api").await.unwrap();
    assert_eq!(deps.len(), 1);
}

#[tokio::test]
async fn test_cascade_delete_repository() {
    let db = Database::in_memory().await.unwrap();

    let repo1 = Repository::new("api", "https://github.com/org/api");
    let repo2 = Repository::new("core", "https://github.com/org/core");

    db.insert_repository(&repo1).await.unwrap();
    db.insert_repository(&repo2).await.unwrap();

    db.add_repository_dependency("api", "core").await.unwrap();

    // Delete should cascade
    db.delete_repository("api").await.unwrap();

    let deps = db.get_repository_dependencies("api").await.unwrap();
    assert_eq!(deps.len(), 0);
}
