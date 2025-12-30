//! Test coverage analysis and tracking
//!
//! This module provides functionality to:
//! - Run tests with coverage instrumentation
//! - Parse coverage reports (lcov, cobertura)
//! - Store coverage metrics in database
//! - Track coverage trends over time
//! - Identify untested code paths

use crate::{Database, Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Coverage report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoverageFormat {
    Lcov,
    Cobertura,
}

/// File coverage metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileCoverage {
    /// File path relative to project root
    pub file_path: String,
    /// Lines covered
    pub lines_covered: u32,
    /// Total lines
    pub lines_total: u32,
    /// Coverage percentage (0-100)
    pub coverage_percent: f64,
}

impl FileCoverage {
    /// Create new file coverage
    pub fn new(file_path: String, lines_covered: u32, lines_total: u32) -> Self {
        let coverage_percent = if lines_total > 0 {
            (lines_covered as f64 / lines_total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            file_path,
            lines_covered,
            lines_total,
            coverage_percent,
        }
    }
}

/// Module coverage metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleCoverage {
    /// Module name (e.g., "orchestrate-core", "orchestrate-web")
    pub module_name: String,
    /// Lines covered
    pub lines_covered: u32,
    /// Total lines
    pub lines_total: u32,
    /// Coverage percentage (0-100)
    pub coverage_percent: f64,
    /// Coverage threshold for this module (0-100)
    pub threshold: f64,
    /// Files in this module
    pub files: Vec<FileCoverage>,
}

impl ModuleCoverage {
    /// Create new module coverage
    pub fn new(module_name: String, threshold: f64) -> Self {
        Self {
            module_name,
            lines_covered: 0,
            lines_total: 0,
            coverage_percent: 0.0,
            threshold,
            files: Vec::new(),
        }
    }

    /// Add file coverage to module
    pub fn add_file(&mut self, file: FileCoverage) {
        self.lines_covered += file.lines_covered;
        self.lines_total += file.lines_total;
        self.coverage_percent = if self.lines_total > 0 {
            (self.lines_covered as f64 / self.lines_total as f64) * 100.0
        } else {
            0.0
        };
        self.files.push(file);
    }

    /// Check if module meets threshold
    pub fn meets_threshold(&self) -> bool {
        self.coverage_percent >= self.threshold
    }
}

/// Overall coverage report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Timestamp of report
    pub timestamp: DateTime<Utc>,
    /// Module coverage data
    pub modules: Vec<ModuleCoverage>,
    /// Overall coverage percentage
    pub overall_percent: f64,
}

impl CoverageReport {
    /// Create new empty coverage report
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            modules: Vec::new(),
            overall_percent: 0.0,
        }
    }

    /// Add module coverage
    pub fn add_module(&mut self, module: ModuleCoverage) {
        self.modules.push(module);
        self.recalculate_overall();
    }

    /// Recalculate overall coverage percentage
    fn recalculate_overall(&mut self) {
        let total_covered: u32 = self.modules.iter().map(|m| m.lines_covered).sum();
        let total_lines: u32 = self.modules.iter().map(|m| m.lines_total).sum();

        self.overall_percent = if total_lines > 0 {
            (total_covered as f64 / total_lines as f64) * 100.0
        } else {
            0.0
        };
    }
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Coverage service for running tests and analyzing coverage
pub struct CoverageService {
    db: Database,
}

impl CoverageService {
    /// Create new coverage service
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Run tests with coverage for a specific language/framework
    pub async fn run_tests_with_coverage(
        &self,
        project_root: &Path,
        language: &str,
    ) -> Result<PathBuf> {
        // This will execute the appropriate test command with coverage
        match language {
            "rust" => self.run_rust_coverage(project_root).await,
            "typescript" => self.run_typescript_coverage(project_root).await,
            "python" => self.run_python_coverage(project_root).await,
            _ => Err(Error::Other(format!("Unsupported language: {}", language))),
        }
    }

    /// Run Rust tests with coverage (using tarpaulin or llvm-cov)
    async fn run_rust_coverage(&self, project_root: &Path) -> Result<PathBuf> {
        // Implementation will run: cargo tarpaulin --out Lcov
        Err(Error::Other("Not implemented".to_string()))
    }

    /// Run TypeScript tests with coverage (using vitest or jest)
    async fn run_typescript_coverage(&self, project_root: &Path) -> Result<PathBuf> {
        // Implementation will run: npm test -- --coverage
        Err(Error::Other("Not implemented".to_string()))
    }

    /// Run Python tests with coverage (using pytest-cov)
    async fn run_python_coverage(&self, project_root: &Path) -> Result<PathBuf> {
        // Implementation will run: pytest --cov --cov-report=xml
        Err(Error::Other("Not implemented".to_string()))
    }

    /// Parse coverage report file
    pub async fn parse_coverage_report(
        &self,
        report_path: &Path,
        format: CoverageFormat,
    ) -> Result<CoverageReport> {
        match format {
            CoverageFormat::Lcov => self.parse_lcov(report_path).await,
            CoverageFormat::Cobertura => self.parse_cobertura(report_path).await,
        }
    }

    /// Parse lcov format coverage report
    async fn parse_lcov(&self, report_path: &Path) -> Result<CoverageReport> {
        // Implementation will parse lcov format
        Err(Error::Other("Not implemented".to_string()))
    }

    /// Parse cobertura XML format coverage report
    async fn parse_cobertura(&self, report_path: &Path) -> Result<CoverageReport> {
        // Implementation will parse cobertura XML format
        Err(Error::Other("Not implemented".to_string()))
    }

    /// Store coverage report in database
    pub async fn store_coverage(&self, report: &CoverageReport) -> Result<i64> {
        let mut tx = self.db.begin().await?;

        // Insert coverage report
        let report_id = sqlx::query(
            r#"
            INSERT INTO coverage_reports (timestamp, overall_percent)
            VALUES (?, ?)
            "#,
        )
        .bind(report.timestamp.to_rfc3339())
        .bind(report.overall_percent)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        // Insert module coverage
        for module in &report.modules {
            let module_id = sqlx::query(
                r#"
                INSERT INTO module_coverage (report_id, module_name, lines_covered, lines_total, coverage_percent, threshold)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(report_id)
            .bind(&module.module_name)
            .bind(module.lines_covered)
            .bind(module.lines_total)
            .bind(module.coverage_percent)
            .bind(module.threshold)
            .execute(&mut *tx)
            .await?
            .last_insert_rowid();

            // Insert file coverage
            for file in &module.files {
                sqlx::query(
                    r#"
                    INSERT INTO file_coverage (module_coverage_id, file_path, lines_covered, lines_total, coverage_percent)
                    VALUES (?, ?, ?, ?, ?)
                    "#,
                )
                .bind(module_id)
                .bind(&file.file_path)
                .bind(file.lines_covered)
                .bind(file.lines_total)
                .bind(file.coverage_percent)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(report_id)
    }

    /// Get coverage history for a module
    pub async fn get_coverage_history(
        &self,
        module_name: &str,
        limit: i64,
    ) -> Result<Vec<CoverageReport>> {
        // Get report IDs that contain this module
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT cr.id, cr.timestamp, cr.overall_percent
            FROM coverage_reports cr
            INNER JOIN module_coverage mc ON cr.id = mc.report_id
            WHERE mc.module_name = ?
            ORDER BY cr.timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(module_name)
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        let mut reports = Vec::new();

        for row in rows {
            let report_id: i64 = row.get(0);
            let timestamp: String = row.get(1);
            let overall_percent: f64 = row.get(2);

            let mut report = CoverageReport {
                timestamp: timestamp.parse().map_err(|_| {
                    Error::Other(format!("Failed to parse timestamp: {}", timestamp))
                })?,
                modules: Vec::new(),
                overall_percent,
            };

            // Get all modules for this report
            let module_rows = sqlx::query(
                r#"
                SELECT id, module_name, lines_covered, lines_total, coverage_percent, threshold
                FROM module_coverage
                WHERE report_id = ?
                "#,
            )
            .bind(report_id)
            .fetch_all(self.db.pool())
            .await?;

            for module_row in module_rows {
                let module_id: i64 = module_row.get(0);
                let module_name: String = module_row.get(1);
                let lines_covered: u32 = module_row.get::<i64, _>(2) as u32;
                let lines_total: u32 = module_row.get::<i64, _>(3) as u32;
                let coverage_percent: f64 = module_row.get(4);
                let threshold: f64 = module_row.get(5);

                let mut module = ModuleCoverage {
                    module_name,
                    lines_covered,
                    lines_total,
                    coverage_percent,
                    threshold,
                    files: Vec::new(),
                };

                // Get files for this module
                let file_rows = sqlx::query(
                    r#"
                    SELECT file_path, lines_covered, lines_total, coverage_percent
                    FROM file_coverage
                    WHERE module_coverage_id = ?
                    "#,
                )
                .bind(module_id)
                .fetch_all(self.db.pool())
                .await?;

                for file_row in file_rows {
                    let file_path: String = file_row.get(0);
                    let lines_covered: u32 = file_row.get::<i64, _>(1) as u32;
                    let lines_total: u32 = file_row.get::<i64, _>(2) as u32;
                    let coverage_percent: f64 = file_row.get(3);

                    module.files.push(FileCoverage {
                        file_path,
                        lines_covered,
                        lines_total,
                        coverage_percent,
                    });
                }

                report.modules.push(module);
            }

            reports.push(report);
        }

        Ok(reports)
    }

    /// Get latest coverage report
    pub async fn get_latest_coverage(&self) -> Result<Option<CoverageReport>> {
        let row = sqlx::query(
            r#"
            SELECT id, timestamp, overall_percent
            FROM coverage_reports
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(self.db.pool())
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let report_id: i64 = row.get(0);
        let timestamp: String = row.get(1);
        let overall_percent: f64 = row.get(2);

        let mut report = CoverageReport {
            timestamp: timestamp.parse().map_err(|_| {
                Error::Other(format!("Failed to parse timestamp: {}", timestamp))
            })?,
            modules: Vec::new(),
            overall_percent,
        };

        // Get all modules for this report
        let module_rows = sqlx::query(
            r#"
            SELECT id, module_name, lines_covered, lines_total, coverage_percent, threshold
            FROM module_coverage
            WHERE report_id = ?
            "#,
        )
        .bind(report_id)
        .fetch_all(self.db.pool())
        .await?;

        for module_row in module_rows {
            let module_id: i64 = module_row.get(0);
            let module_name: String = module_row.get(1);
            let lines_covered: u32 = module_row.get::<i64, _>(2) as u32;
            let lines_total: u32 = module_row.get::<i64, _>(3) as u32;
            let coverage_percent: f64 = module_row.get(4);
            let threshold: f64 = module_row.get(5);

            let mut module = ModuleCoverage {
                module_name,
                lines_covered,
                lines_total,
                coverage_percent,
                threshold,
                files: Vec::new(),
            };

            // Get files for this module
            let file_rows = sqlx::query(
                r#"
                SELECT file_path, lines_covered, lines_total, coverage_percent
                FROM file_coverage
                WHERE module_coverage_id = ?
                "#,
            )
            .bind(module_id)
            .fetch_all(self.db.pool())
            .await?;

            for file_row in file_rows {
                let file_path: String = file_row.get(0);
                let lines_covered: u32 = file_row.get::<i64, _>(1) as u32;
                let lines_total: u32 = file_row.get::<i64, _>(2) as u32;
                let coverage_percent: f64 = file_row.get(3);

                module.files.push(FileCoverage {
                    file_path,
                    lines_covered,
                    lines_total,
                    coverage_percent,
                });
            }

            report.modules.push(module);
        }

        Ok(Some(report))
    }

    /// Get module thresholds configuration
    pub async fn get_module_thresholds(&self) -> Result<HashMap<String, f64>> {
        let rows = sqlx::query(
            r#"
            SELECT module_name, threshold
            FROM coverage_thresholds
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut thresholds = HashMap::new();
        for row in rows {
            let module_name: String = row.get(0);
            let threshold: f64 = row.get(1);
            thresholds.insert(module_name, threshold);
        }

        Ok(thresholds)
    }

    /// Set coverage threshold for a module
    pub async fn set_module_threshold(&self, module_name: &str, threshold: f64) -> Result<()> {
        if threshold < 0.0 || threshold > 100.0 {
            return Err(Error::Other(
                "Threshold must be between 0 and 100".to_string(),
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO coverage_thresholds (module_name, threshold, updated_at)
            VALUES (?, ?, datetime('now'))
            ON CONFLICT(module_name) DO UPDATE SET
                threshold = excluded.threshold,
                updated_at = datetime('now')
            "#,
        )
        .bind(module_name)
        .bind(threshold)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Identify files with coverage below threshold
    pub async fn find_untested_files(
        &self,
        report: &CoverageReport,
    ) -> Vec<FileCoverage> {
        let mut untested = Vec::new();

        for module in &report.modules {
            for file in &module.files {
                if file.coverage_percent < module.threshold {
                    untested.push(file.clone());
                }
            }
        }

        untested.sort_by(|a, b| {
            a.coverage_percent
                .partial_cmp(&b.coverage_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        untested
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_coverage_new() {
        let coverage = FileCoverage::new("src/main.rs".to_string(), 50, 100);
        assert_eq!(coverage.file_path, "src/main.rs");
        assert_eq!(coverage.lines_covered, 50);
        assert_eq!(coverage.lines_total, 100);
        assert_eq!(coverage.coverage_percent, 50.0);
    }

    #[test]
    fn test_file_coverage_zero_lines() {
        let coverage = FileCoverage::new("empty.rs".to_string(), 0, 0);
        assert_eq!(coverage.coverage_percent, 0.0);
    }

    #[test]
    fn test_module_coverage_new() {
        let module = ModuleCoverage::new("orchestrate-core".to_string(), 80.0);
        assert_eq!(module.module_name, "orchestrate-core");
        assert_eq!(module.threshold, 80.0);
        assert_eq!(module.lines_covered, 0);
        assert_eq!(module.lines_total, 0);
        assert_eq!(module.coverage_percent, 0.0);
    }

    #[test]
    fn test_module_coverage_add_file() {
        let mut module = ModuleCoverage::new("orchestrate-core".to_string(), 80.0);
        let file1 = FileCoverage::new("src/main.rs".to_string(), 50, 100);
        let file2 = FileCoverage::new("src/lib.rs".to_string(), 30, 50);

        module.add_file(file1);
        assert_eq!(module.lines_covered, 50);
        assert_eq!(module.lines_total, 100);
        assert_eq!(module.coverage_percent, 50.0);

        module.add_file(file2);
        assert_eq!(module.lines_covered, 80);
        assert_eq!(module.lines_total, 150);
        assert!((module.coverage_percent - 53.333).abs() < 0.01);
    }

    #[test]
    fn test_module_meets_threshold() {
        let mut module = ModuleCoverage::new("orchestrate-core".to_string(), 80.0);
        assert!(!module.meets_threshold()); // 0% < 80%

        let file = FileCoverage::new("src/main.rs".to_string(), 85, 100);
        module.add_file(file);
        assert!(module.meets_threshold()); // 85% >= 80%
    }

    #[test]
    fn test_coverage_report_new() {
        let report = CoverageReport::new();
        assert_eq!(report.modules.len(), 0);
        assert_eq!(report.overall_percent, 0.0);
    }

    #[test]
    fn test_coverage_report_add_module() {
        let mut report = CoverageReport::new();

        let mut module1 = ModuleCoverage::new("orchestrate-core".to_string(), 80.0);
        module1.add_file(FileCoverage::new("src/main.rs".to_string(), 50, 100));

        let mut module2 = ModuleCoverage::new("orchestrate-web".to_string(), 70.0);
        module2.add_file(FileCoverage::new("src/app.ts".to_string(), 30, 100));

        report.add_module(module1);
        assert_eq!(report.modules.len(), 1);
        assert_eq!(report.overall_percent, 50.0);

        report.add_module(module2);
        assert_eq!(report.modules.len(), 2);
        assert_eq!(report.overall_percent, 40.0); // (50+30)/(100+100)
    }

    #[tokio::test]
    async fn test_run_tests_with_coverage_unsupported_language() {
        let db = Database::new(":memory:").await.unwrap();
        let service = CoverageService::new(db);

        let result = service
            .run_tests_with_coverage(Path::new("/tmp"), "unsupported")
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported language"));
    }

    #[tokio::test]
    async fn test_set_module_threshold_invalid() {
        let db = Database::new(":memory:").await.unwrap();
        let service = CoverageService::new(db);

        let result = service.set_module_threshold("test", -10.0).await;
        assert!(result.is_err());

        let result = service.set_module_threshold("test", 150.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_untested_files() {
        let db = Database::new(":memory:").await.unwrap();
        let service = CoverageService::new(db);

        let mut report = CoverageReport::new();
        let mut module = ModuleCoverage::new("test-module".to_string(), 80.0);

        module.add_file(FileCoverage::new("good.rs".to_string(), 90, 100)); // 90%
        module.add_file(FileCoverage::new("bad.rs".to_string(), 20, 100));  // 20%
        module.add_file(FileCoverage::new("ok.rs".to_string(), 80, 100));   // 80%

        report.add_module(module);

        let untested = service.find_untested_files(&report).await;
        assert_eq!(untested.len(), 1);
        assert_eq!(untested[0].file_path, "bad.rs");
        assert_eq!(untested[0].coverage_percent, 20.0);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_coverage() {
        let db = Database::new(":memory:").await.unwrap();
        let service = CoverageService::new(db);

        // Create a coverage report
        let mut report = CoverageReport::new();
        let mut module = ModuleCoverage::new("orchestrate-core".to_string(), 80.0);
        module.add_file(FileCoverage::new("src/main.rs".to_string(), 50, 100));
        module.add_file(FileCoverage::new("src/lib.rs".to_string(), 75, 100));
        report.add_module(module);

        // Store it
        let report_id = service.store_coverage(&report).await.unwrap();
        assert!(report_id > 0);

        // Retrieve it
        let retrieved = service.get_latest_coverage().await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.overall_percent, report.overall_percent);
        assert_eq!(retrieved.modules.len(), 1);
        assert_eq!(retrieved.modules[0].module_name, "orchestrate-core");
        assert_eq!(retrieved.modules[0].files.len(), 2);
    }

    #[tokio::test]
    async fn test_coverage_history() {
        let db = Database::new(":memory:").await.unwrap();
        let service = CoverageService::new(db);

        // Store multiple reports
        for i in 1..=5 {
            let mut report = CoverageReport::new();
            let mut module = ModuleCoverage::new("test-module".to_string(), 80.0);
            module.add_file(FileCoverage::new("test.rs".to_string(), i * 10, 100));
            report.add_module(module);

            service.store_coverage(&report).await.unwrap();
            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Get history
        let history = service
            .get_coverage_history("test-module", 3)
            .await
            .unwrap();

        assert_eq!(history.len(), 3);
        // Should be in descending order (newest first)
        assert!(history[0].timestamp >= history[1].timestamp);
        assert!(history[1].timestamp >= history[2].timestamp);
    }

    #[tokio::test]
    async fn test_module_thresholds() {
        let db = Database::new(":memory:").await.unwrap();
        let service = CoverageService::new(db);

        // Get default thresholds
        let thresholds = service.get_module_thresholds().await.unwrap();
        assert!(thresholds.contains_key("orchestrate-core"));
        assert_eq!(thresholds.get("orchestrate-core"), Some(&80.0));

        // Set new threshold
        service
            .set_module_threshold("new-module", 75.0)
            .await
            .unwrap();

        // Verify it was set
        let thresholds = service.get_module_thresholds().await.unwrap();
        assert_eq!(thresholds.get("new-module"), Some(&75.0));

        // Update existing threshold
        service
            .set_module_threshold("orchestrate-core", 90.0)
            .await
            .unwrap();

        let thresholds = service.get_module_thresholds().await.unwrap();
        assert_eq!(thresholds.get("orchestrate-core"), Some(&90.0));
    }
}
