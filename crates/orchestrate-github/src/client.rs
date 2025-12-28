//! GitHub API client (via gh CLI)

use anyhow::Result;
use serde::Deserialize;
use std::process::Command;

/// GitHub client using gh CLI
pub struct GitHubClient {
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
}

impl GitHubClient {
    /// Create a new GitHub client for the current repository
    pub fn new() -> Result<Self> {
        let output = Command::new("gh")
            .args(["repo", "view", "--json", "owner,name"])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to get repo info: {}", String::from_utf8_lossy(&output.stderr));
        }

        #[derive(Deserialize)]
        struct RepoInfo {
            owner: Owner,
            name: String,
        }

        #[derive(Deserialize)]
        struct Owner {
            login: String,
        }

        let info: RepoInfo = serde_json::from_slice(&output.stdout)?;
        Ok(Self {
            owner: info.owner.login,
            repo: info.name,
        })
    }

    /// Create a PR
    pub fn create_pr(&self, title: &str, body: &str, base: &str) -> Result<i32> {
        let output = Command::new("gh")
            .args(["pr", "create", "--title", title, "--body", body, "--base", base])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to create PR: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Get PR number
        let output = Command::new("gh")
            .args(["pr", "view", "--json", "number", "-q", ".number"])
            .output()?;

        let number: i32 = String::from_utf8_lossy(&output.stdout).trim().parse()?;
        Ok(number)
    }

    /// Get PR state
    pub fn get_pr_state(&self, number: i32) -> Result<PrState> {
        let output = Command::new("gh")
            .args([
                "pr", "view",
                &number.to_string(),
                "--json", "state,mergeable,reviewDecision",
            ])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to get PR state: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// Get CI check status
    pub fn get_checks(&self, number: i32) -> Result<Vec<Check>> {
        let output = Command::new("gh")
            .args([
                "pr", "checks",
                &number.to_string(),
                "--json", "name,conclusion,status",
            ])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to get checks: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// Merge a PR
    pub fn merge_pr(&self, number: i32, strategy: &str) -> Result<()> {
        let strategy_arg = match strategy {
            "squash" => "--squash",
            "rebase" => "--rebase",
            "merge" => "--merge",
            _ => "--squash",
        };

        let output = Command::new("gh")
            .args([
                "pr", "merge",
                &number.to_string(),
                strategy_arg,
                "--delete-branch",
            ])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to merge PR: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    /// Get unresolved review threads
    pub fn get_unresolved_threads(&self, number: i32) -> Result<Vec<ReviewThread>> {
        let query = format!(
            r#"
            query {{
                repository(owner: "{}", name: "{}") {{
                    pullRequest(number: {}) {{
                        reviewThreads(first: 100) {{
                            nodes {{
                                id
                                isResolved
                                path
                                line
                                comments(first: 10) {{
                                    nodes {{
                                        body
                                        author {{ login }}
                                    }}
                                }}
                            }}
                        }}
                    }}
                }}
            }}
            "#,
            self.owner, self.repo, number
        );

        let output = Command::new("gh")
            .args(["api", "graphql", "-f", &format!("query={}", query)])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to get threads: {}", String::from_utf8_lossy(&output.stderr));
        }

        #[derive(Deserialize)]
        struct Response {
            data: Data,
        }

        #[derive(Deserialize)]
        struct Data {
            repository: Repository,
        }

        #[derive(Deserialize)]
        struct Repository {
            #[serde(rename = "pullRequest")]
            pull_request: PullRequest,
        }

        #[derive(Deserialize)]
        struct PullRequest {
            #[serde(rename = "reviewThreads")]
            review_threads: Threads,
        }

        #[derive(Deserialize)]
        struct Threads {
            nodes: Vec<ThreadNode>,
        }

        #[derive(Deserialize)]
        struct ThreadNode {
            id: String,
            #[serde(rename = "isResolved")]
            is_resolved: bool,
            path: Option<String>,
            line: Option<i32>,
            comments: Comments,
        }

        #[derive(Deserialize)]
        struct Comments {
            nodes: Vec<CommentNode>,
        }

        #[derive(Deserialize)]
        struct CommentNode {
            body: String,
            author: Author,
        }

        #[derive(Deserialize)]
        struct Author {
            login: String,
        }

        let response: Response = serde_json::from_slice(&output.stdout)?;
        let threads = response
            .data
            .repository
            .pull_request
            .review_threads
            .nodes
            .into_iter()
            .filter(|t| !t.is_resolved)
            .map(|t| ReviewThread {
                id: t.id,
                path: t.path,
                line: t.line,
                comments: t
                    .comments
                    .nodes
                    .into_iter()
                    .map(|c| ThreadComment {
                        author: c.author.login,
                        body: c.body,
                    })
                    .collect(),
            })
            .collect();

        Ok(threads)
    }

    /// Resolve a review thread
    pub fn resolve_thread(&self, thread_id: &str) -> Result<()> {
        let mutation = format!(
            r#"
            mutation {{
                resolveReviewThread(input: {{threadId: "{}"}}) {{
                    thread {{ isResolved }}
                }}
            }}
            "#,
            thread_id
        );

        let output = Command::new("gh")
            .args(["api", "graphql", "-f", &format!("query={}", mutation)])
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to resolve thread: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct PrState {
    pub state: String,
    pub mergeable: Option<String>,
    #[serde(rename = "reviewDecision")]
    pub review_decision: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Check {
    pub name: String,
    pub conclusion: Option<String>,
    pub status: String,
}

#[derive(Debug)]
pub struct ReviewThread {
    pub id: String,
    pub path: Option<String>,
    pub line: Option<i32>,
    pub comments: Vec<ThreadComment>,
}

#[derive(Debug)]
pub struct ThreadComment {
    pub author: String,
    pub body: String,
}
