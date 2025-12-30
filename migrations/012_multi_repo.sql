-- Multi-Repository Orchestration Schema
-- Migration 012

-- Repositories table
CREATE TABLE IF NOT EXISTS repositories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    url TEXT NOT NULL,
    local_path TEXT,
    default_branch TEXT NOT NULL DEFAULT 'main',
    provider TEXT NOT NULL CHECK (provider IN ('github', 'gitlab', 'bitbucket', 'other')),
    status TEXT NOT NULL DEFAULT 'inactive' CHECK (status IN ('active', 'inactive', 'error', 'syncing')),
    last_synced TEXT,
    config TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Repository dependencies table
CREATE TABLE IF NOT EXISTS repository_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    depends_on_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(repo_id, depends_on_id),
    CHECK(repo_id != depends_on_id)
);

-- Cross-repository branches table
CREATE TABLE IF NOT EXISTS cross_repo_branches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Branch status per repository
CREATE TABLE IF NOT EXISTS repo_branch_status (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cross_branch_id INTEGER NOT NULL REFERENCES cross_repo_branches(id) ON DELETE CASCADE,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    branch_exists INTEGER NOT NULL DEFAULT 0,
    commits_ahead INTEGER,
    commits_behind INTEGER,
    has_conflicts INTEGER NOT NULL DEFAULT 0,
    pr_number INTEGER,
    pr_status TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(cross_branch_id, repo_id)
);

-- Linked PR groups
CREATE TABLE IF NOT EXISTS linked_pr_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'ready_to_merge', 'partially_merged', 'merged', 'blocked')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Individual linked PRs
CREATE TABLE IF NOT EXISTS linked_prs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id TEXT NOT NULL REFERENCES linked_pr_groups(id) ON DELETE CASCADE,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    pr_number INTEGER NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    mergeable INTEGER NOT NULL DEFAULT 0,
    merge_order INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(group_id, repo_id)
);

-- Coordinated releases
CREATE TABLE IF NOT EXISTS coordinated_releases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'failed', 'rolled_back')),
    changelog TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Repository releases
CREATE TABLE IF NOT EXISTS repo_releases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    release_id INTEGER NOT NULL REFERENCES coordinated_releases(id) ON DELETE CASCADE,
    repo_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'failed', 'rolled_back')),
    tag TEXT,
    release_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(release_id, repo_id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_repositories_name ON repositories(name);
CREATE INDEX IF NOT EXISTS idx_repositories_status ON repositories(status);
CREATE INDEX IF NOT EXISTS idx_repo_dependencies_repo ON repository_dependencies(repo_id);
CREATE INDEX IF NOT EXISTS idx_repo_dependencies_depends ON repository_dependencies(depends_on_id);
CREATE INDEX IF NOT EXISTS idx_cross_repo_branches_name ON cross_repo_branches(name);
CREATE INDEX IF NOT EXISTS idx_repo_branch_status_branch ON repo_branch_status(cross_branch_id);
CREATE INDEX IF NOT EXISTS idx_linked_prs_group ON linked_prs(group_id);
CREATE INDEX IF NOT EXISTS idx_coordinated_releases_status ON coordinated_releases(status);
CREATE INDEX IF NOT EXISTS idx_repo_releases_release ON repo_releases(release_id);
