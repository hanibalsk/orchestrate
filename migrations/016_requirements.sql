-- Requirements Capture Tables
-- For Epic 012: Requirements Capture Agent

-- Requirements table
CREATE TABLE IF NOT EXISTS requirements (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    requirement_type TEXT NOT NULL,
    priority TEXT NOT NULL,
    status TEXT NOT NULL,
    stakeholders TEXT NOT NULL DEFAULT '[]',
    actors TEXT NOT NULL DEFAULT '[]',
    acceptance_criteria TEXT NOT NULL DEFAULT '[]',
    dependencies TEXT NOT NULL DEFAULT '[]',
    related_requirements TEXT NOT NULL DEFAULT '[]',
    tags TEXT NOT NULL DEFAULT '[]',
    source TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_requirements_type ON requirements(requirement_type);
CREATE INDEX IF NOT EXISTS idx_requirements_status ON requirements(status);
CREATE INDEX IF NOT EXISTS idx_requirements_created ON requirements(created_at);

-- Clarifying questions for requirements refinement
CREATE TABLE IF NOT EXISTS clarifying_questions (
    id TEXT PRIMARY KEY,
    requirement_id TEXT NOT NULL,
    question TEXT NOT NULL,
    context TEXT NOT NULL,
    options TEXT NOT NULL DEFAULT '[]',
    answer TEXT,
    answered_at TEXT,
    FOREIGN KEY (requirement_id) REFERENCES requirements(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_clarifying_questions_req ON clarifying_questions(requirement_id);
CREATE INDEX IF NOT EXISTS idx_clarifying_questions_unanswered ON clarifying_questions(requirement_id, answer);

-- Generated stories from requirements
CREATE TABLE IF NOT EXISTS generated_stories (
    id TEXT PRIMARY KEY,
    requirement_id TEXT NOT NULL,
    title TEXT NOT NULL,
    user_type TEXT NOT NULL,
    goal TEXT NOT NULL,
    benefit TEXT NOT NULL,
    acceptance_criteria TEXT NOT NULL DEFAULT '[]',
    complexity TEXT NOT NULL,
    related_requirements TEXT NOT NULL DEFAULT '[]',
    suggested_epic TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (requirement_id) REFERENCES requirements(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_generated_stories_req ON generated_stories(requirement_id);
CREATE INDEX IF NOT EXISTS idx_generated_stories_created ON generated_stories(created_at);

-- Traceability links between artifacts
CREATE TABLE IF NOT EXISTS traceability_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    link_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(source_id, target_id, link_type)
);

CREATE INDEX IF NOT EXISTS idx_traceability_source ON traceability_links(source_id);
CREATE INDEX IF NOT EXISTS idx_traceability_target ON traceability_links(target_id);
CREATE INDEX IF NOT EXISTS idx_traceability_link_type ON traceability_links(link_type);

-- Impact analyses for requirement changes
CREATE TABLE IF NOT EXISTS impact_analyses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    requirement_id TEXT NOT NULL,
    affected_stories TEXT NOT NULL DEFAULT '[]',
    affected_code_files TEXT NOT NULL DEFAULT '[]',
    affected_tests TEXT NOT NULL DEFAULT '[]',
    estimated_effort TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    recommendations TEXT NOT NULL DEFAULT '[]',
    generated_at TEXT NOT NULL,
    FOREIGN KEY (requirement_id) REFERENCES requirements(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_impact_analyses_req ON impact_analyses(requirement_id);
CREATE INDEX IF NOT EXISTS idx_impact_analyses_generated ON impact_analyses(generated_at);
