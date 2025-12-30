-- Test quality validation and mutation testing tables

-- Test quality reports
CREATE TABLE IF NOT EXISTS test_quality_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    quality_score REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(file_path, created_at)
);

CREATE INDEX idx_test_quality_reports_file ON test_quality_reports(file_path);
CREATE INDEX idx_test_quality_reports_created ON test_quality_reports(created_at);

-- Test quality issues
CREATE TABLE IF NOT EXISTS test_quality_issues (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    test_name TEXT NOT NULL,
    issue_type TEXT NOT NULL, -- weak_assertion, always_passes, implementation_focused, mutation_survived, improper_setup
    description TEXT NOT NULL,
    suggestion TEXT,
    line_number INTEGER,
    FOREIGN KEY (report_id) REFERENCES test_quality_reports(id) ON DELETE CASCADE
);

CREATE INDEX idx_test_quality_issues_report ON test_quality_issues(report_id);
CREATE INDEX idx_test_quality_issues_type ON test_quality_issues(issue_type);

-- Mutation test results
CREATE TABLE IF NOT EXISTS mutation_test_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id INTEGER NOT NULL,
    total_mutations INTEGER NOT NULL,
    mutations_caught INTEGER NOT NULL,
    mutations_survived INTEGER NOT NULL,
    mutation_score REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (report_id) REFERENCES test_quality_reports(id) ON DELETE CASCADE
);

CREATE INDEX idx_mutation_test_results_report ON mutation_test_results(report_id);

-- Mutation details (survived mutations)
CREATE TABLE IF NOT EXISTS mutation_details (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mutation_result_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    line_number INTEGER NOT NULL,
    mutation_type TEXT NOT NULL, -- arithmetic_operator, comparison_operator, logical_operator, return_value, statement_deletion, constant_replacement
    original TEXT NOT NULL,
    mutated TEXT NOT NULL,
    FOREIGN KEY (mutation_result_id) REFERENCES mutation_test_results(id) ON DELETE CASCADE
);

CREATE INDEX idx_mutation_details_result ON mutation_details(mutation_result_id);
CREATE INDEX idx_mutation_details_type ON mutation_details(mutation_type);
