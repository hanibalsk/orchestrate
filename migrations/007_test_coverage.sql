-- Test Coverage Tracking
-- Stores coverage metrics over time and module thresholds

-- Coverage reports table
CREATE TABLE IF NOT EXISTS coverage_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    overall_percent REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_coverage_reports_timestamp ON coverage_reports(timestamp);

-- Module coverage table
CREATE TABLE IF NOT EXISTS module_coverage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_id INTEGER NOT NULL,
    module_name TEXT NOT NULL,
    lines_covered INTEGER NOT NULL,
    lines_total INTEGER NOT NULL,
    coverage_percent REAL NOT NULL,
    threshold REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (report_id) REFERENCES coverage_reports(id) ON DELETE CASCADE
);

CREATE INDEX idx_module_coverage_report ON module_coverage(report_id);
CREATE INDEX idx_module_coverage_module ON module_coverage(module_name);

-- File coverage table
CREATE TABLE IF NOT EXISTS file_coverage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    module_coverage_id INTEGER NOT NULL,
    file_path TEXT NOT NULL,
    lines_covered INTEGER NOT NULL,
    lines_total INTEGER NOT NULL,
    coverage_percent REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (module_coverage_id) REFERENCES module_coverage(id) ON DELETE CASCADE
);

CREATE INDEX idx_file_coverage_module ON file_coverage(module_coverage_id);
CREATE INDEX idx_file_coverage_path ON file_coverage(file_path);

-- Module threshold configuration
CREATE TABLE IF NOT EXISTS coverage_thresholds (
    module_name TEXT PRIMARY KEY,
    threshold REAL NOT NULL CHECK(threshold >= 0 AND threshold <= 100),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Insert default thresholds
INSERT OR IGNORE INTO coverage_thresholds (module_name, threshold) VALUES
    ('orchestrate-core', 80.0),
    ('orchestrate-web', 70.0),
    ('orchestrate-cli', 75.0);
