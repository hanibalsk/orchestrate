//! Security Report Generation
//!
//! Generates security reports in multiple formats:
//! - SARIF format for GitHub Security tab
//! - JSON format for CI integration
//! - HTML format for human review

use crate::security::{SarifReport, SecurityScan, Severity};
use serde::{Deserialize, Serialize};

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Sarif,
    Json,
    Html,
}

impl std::str::FromStr for ReportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sarif" => Ok(Self::Sarif),
            "json" => Ok(Self::Json),
            "html" => Ok(Self::Html),
            _ => Err(format!("Unknown report format: {}", s)),
        }
    }
}

impl std::fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sarif => write!(f, "sarif"),
            Self::Json => write!(f, "json"),
            Self::Html => write!(f, "html"),
        }
    }
}

/// Security report generator
pub trait SecurityReportGenerator {
    /// Generate report from security scan
    fn generate(&self, scan: &SecurityScan) -> Result<String, String>;

    /// Get report format
    fn format(&self) -> ReportFormat;

    /// Get file extension for this format
    fn extension(&self) -> &str {
        match self.format() {
            ReportFormat::Sarif => "sarif",
            ReportFormat::Json => "json",
            ReportFormat::Html => "html",
        }
    }
}

/// JSON report generator
pub struct JsonReportGenerator;

impl JsonReportGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityReportGenerator for JsonReportGenerator {
    fn generate(&self, scan: &SecurityScan) -> Result<String, String> {
        serde_json::to_string_pretty(scan)
            .map_err(|e| format!("Failed to generate JSON report: {}", e))
    }

    fn format(&self) -> ReportFormat {
        ReportFormat::Json
    }
}

/// HTML report generator
pub struct HtmlReportGenerator {
    include_css: bool,
}

impl HtmlReportGenerator {
    pub fn new() -> Self {
        Self { include_css: true }
    }

    pub fn without_css(mut self) -> Self {
        self.include_css = false;
        self
    }

    fn generate_css(&self) -> &str {
        r#"
<style>
    body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
        line-height: 1.6;
        max-width: 1200px;
        margin: 0 auto;
        padding: 20px;
        background: #f5f5f5;
    }
    .header {
        background: white;
        padding: 20px;
        border-radius: 8px;
        margin-bottom: 20px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    }
    .header h1 {
        margin: 0 0 10px 0;
        color: #333;
    }
    .scan-info {
        color: #666;
        font-size: 14px;
    }
    .summary {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 15px;
        margin-bottom: 20px;
    }
    .summary-card {
        background: white;
        padding: 20px;
        border-radius: 8px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    }
    .summary-card h3 {
        margin: 0 0 10px 0;
        font-size: 14px;
        color: #666;
        text-transform: uppercase;
    }
    .summary-card .count {
        font-size: 36px;
        font-weight: bold;
        margin: 0;
    }
    .critical { color: #d32f2f; }
    .high { color: #f57c00; }
    .medium { color: #fbc02d; }
    .low { color: #7cb342; }
    .vulnerability {
        background: white;
        padding: 20px;
        border-radius: 8px;
        margin-bottom: 15px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        border-left: 4px solid #ddd;
    }
    .vulnerability.critical { border-left-color: #d32f2f; }
    .vulnerability.high { border-left-color: #f57c00; }
    .vulnerability.medium { border-left-color: #fbc02d; }
    .vulnerability.low { border-left-color: #7cb342; }
    .vuln-header {
        display: flex;
        justify-content: space-between;
        align-items: start;
        margin-bottom: 10px;
    }
    .vuln-title {
        font-size: 18px;
        font-weight: bold;
        margin: 0;
    }
    .severity-badge {
        padding: 4px 12px;
        border-radius: 4px;
        font-size: 12px;
        font-weight: bold;
        text-transform: uppercase;
    }
    .severity-badge.critical { background: #ffebee; color: #d32f2f; }
    .severity-badge.high { background: #fff3e0; color: #f57c00; }
    .severity-badge.medium { background: #fffde7; color: #fbc02d; }
    .severity-badge.low { background: #f1f8e9; color: #7cb342; }
    .vuln-meta {
        color: #666;
        font-size: 14px;
        margin-bottom: 10px;
    }
    .vuln-description {
        margin: 10px 0;
        color: #333;
    }
    .auto-fixable {
        display: inline-block;
        background: #e3f2fd;
        color: #1976d2;
        padding: 4px 8px;
        border-radius: 4px;
        font-size: 12px;
        margin-top: 10px;
    }
    .remediation {
        background: #f5f5f5;
        padding: 15px;
        border-radius: 4px;
        margin-top: 10px;
    }
    .remediation h4 {
        margin: 0 0 10px 0;
        font-size: 14px;
        color: #333;
    }
    .remediation code {
        background: #fff;
        padding: 2px 6px;
        border-radius: 3px;
        font-family: 'Courier New', monospace;
    }
    .section {
        background: white;
        padding: 20px;
        border-radius: 8px;
        margin-bottom: 20px;
        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    }
    .section h2 {
        margin: 0 0 15px 0;
        color: #333;
        border-bottom: 2px solid #eee;
        padding-bottom: 10px;
    }
    .secret-item, .license-item {
        background: #fff3e0;
        padding: 15px;
        border-radius: 4px;
        margin-bottom: 10px;
        border-left: 4px solid #f57c00;
    }
</style>
        "#
    }

    fn severity_class(&self, severity: &Severity) -> &str {
        match severity {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Unknown => "low",
        }
    }
}

impl Default for HtmlReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityReportGenerator for HtmlReportGenerator {
    fn generate(&self, scan: &SecurityScan) -> Result<String, String> {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
        html.push_str("    <meta charset=\"UTF-8\">\n");
        html.push_str("    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
        html.push_str("    <title>Security Scan Report</title>\n");

        if self.include_css {
            html.push_str(self.generate_css());
        }

        html.push_str("</head>\n<body>\n");

        // Header
        html.push_str("    <div class=\"header\">\n");
        html.push_str("        <h1>Security Scan Report</h1>\n");
        html.push_str(&format!("        <div class=\"scan-info\">Scan ID: {}</div>\n", scan.id));
        html.push_str(&format!("        <div class=\"scan-info\">Status: {:?}</div>\n", scan.status));
        html.push_str(&format!("        <div class=\"scan-info\">Started: {}</div>\n", scan.started_at.format("%Y-%m-%d %H:%M:%S UTC")));
        if let Some(completed) = scan.completed_at {
            html.push_str(&format!("        <div class=\"scan-info\">Completed: {}</div>\n", completed.format("%Y-%m-%d %H:%M:%S UTC")));
        }
        if let Some(duration) = scan.duration_seconds {
            html.push_str(&format!("        <div class=\"scan-info\">Duration: {:.2}s</div>\n", duration));
        }
        html.push_str("    </div>\n");

        // Summary
        html.push_str("    <div class=\"summary\">\n");
        html.push_str(&format!(
            "        <div class=\"summary-card\">\n            <h3>Total Vulnerabilities</h3>\n            <p class=\"count\">{}</p>\n        </div>\n",
            scan.summary.total_vulnerabilities
        ));
        html.push_str(&format!(
            "        <div class=\"summary-card\">\n            <h3>Critical</h3>\n            <p class=\"count critical\">{}</p>\n        </div>\n",
            scan.summary.critical_count
        ));
        html.push_str(&format!(
            "        <div class=\"summary-card\">\n            <h3>High</h3>\n            <p class=\"count high\">{}</p>\n        </div>\n",
            scan.summary.high_count
        ));
        html.push_str(&format!(
            "        <div class=\"summary-card\">\n            <h3>Medium</h3>\n            <p class=\"count medium\">{}</p>\n        </div>\n",
            scan.summary.medium_count
        ));
        html.push_str(&format!(
            "        <div class=\"summary-card\">\n            <h3>Low</h3>\n            <p class=\"count low\">{}</p>\n        </div>\n",
            scan.summary.low_count
        ));
        html.push_str(&format!(
            "        <div class=\"summary-card\">\n            <h3>Auto-Fixable</h3>\n            <p class=\"count\">{}</p>\n        </div>\n",
            scan.summary.auto_fixable_count
        ));
        html.push_str("    </div>\n");

        // Vulnerabilities
        if !scan.vulnerabilities.is_empty() {
            html.push_str("    <div class=\"section\">\n");
            html.push_str("        <h2>Vulnerabilities</h2>\n");

            for vuln in &scan.vulnerabilities {
                let severity_class = self.severity_class(&vuln.severity);
                html.push_str(&format!("        <div class=\"vulnerability {}\">\n", severity_class));
                html.push_str("            <div class=\"vuln-header\">\n");
                html.push_str(&format!("                <h3 class=\"vuln-title\">{}</h3>\n", vuln.title));
                html.push_str(&format!("                <span class=\"severity-badge {}\">{}</span>\n", severity_class, vuln.severity));
                html.push_str("            </div>\n");

                // Metadata
                html.push_str("            <div class=\"vuln-meta\">\n");
                if let Some(cve) = &vuln.cve_id {
                    html.push_str(&format!("                CVE: {} | ", cve));
                }
                html.push_str(&format!("Type: {}", vuln.vulnerability_type));
                if let Some(package) = &vuln.package_name {
                    html.push_str(&format!(" | Package: {}", package));
                    if let Some(version) = &vuln.installed_version {
                        html.push_str(&format!(" ({})", version));
                    }
                }
                if let Some(file) = &vuln.file_path {
                    html.push_str(&format!(" | File: {}", file));
                    if let Some(line) = vuln.line_number {
                        html.push_str(&format!(":{}", line));
                    }
                }
                html.push_str("\n            </div>\n");

                // Description
                if !vuln.description.is_empty() {
                    html.push_str(&format!("            <div class=\"vuln-description\">{}</div>\n", vuln.description));
                }

                // Auto-fixable badge
                if vuln.auto_fixable {
                    html.push_str("            <div class=\"auto-fixable\">✓ Auto-fixable</div>\n");
                }

                // Remediation
                if vuln.fixed_version.is_some() || vuln.fix_command.is_some() {
                    html.push_str("            <div class=\"remediation\">\n");
                    html.push_str("                <h4>Remediation</h4>\n");
                    if let Some(fixed) = &vuln.fixed_version {
                        html.push_str(&format!("                <p>Upgrade to version: <code>{}</code></p>\n", fixed));
                    }
                    if let Some(cmd) = &vuln.fix_command {
                        html.push_str(&format!("                <p>Fix command: <code>{}</code></p>\n", cmd));
                    }
                    html.push_str("            </div>\n");
                }

                html.push_str("        </div>\n");
            }

            html.push_str("    </div>\n");
        }

        // Secrets
        if !scan.secrets.is_empty() {
            html.push_str("    <div class=\"section\">\n");
            html.push_str("        <h2>Detected Secrets</h2>\n");

            for secret in &scan.secrets {
                html.push_str("        <div class=\"secret-item\">\n");
                html.push_str(&format!("            <strong>{}</strong><br>\n", secret.secret_type));
                html.push_str(&format!("            File: {} (line {})<br>\n", secret.file_path, secret.line_number));
                if secret.is_in_history {
                    html.push_str("            <strong style=\"color: #d32f2f;\">⚠ Found in git history</strong>\n");
                }
                html.push_str("        </div>\n");
            }

            html.push_str("    </div>\n");
        }

        // License issues
        if !scan.license_issues.is_empty() {
            html.push_str("    <div class=\"section\">\n");
            html.push_str("        <h2>License Issues</h2>\n");

            for issue in &scan.license_issues {
                html.push_str("        <div class=\"license-item\">\n");
                html.push_str(&format!("            <strong>{}</strong> ({})<br>\n", issue.package_name, issue.license));
                html.push_str(&format!("            Type: {:?}<br>\n", issue.issue_type));
                if !issue.description.is_empty() {
                    html.push_str(&format!("            {}\n", issue.description));
                }
                html.push_str("        </div>\n");
            }

            html.push_str("    </div>\n");
        }

        html.push_str("</body>\n</html>");

        Ok(html)
    }

    fn format(&self) -> ReportFormat {
        ReportFormat::Html
    }
}

/// Generate report in specified format
pub fn generate_report(scan: &SecurityScan, format: ReportFormat) -> Result<String, String> {
    match format {
        ReportFormat::Sarif => {
            let report = SarifReport::from_scan(scan, "orchestrate-security");
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("Failed to generate SARIF report: {}", e))
        }
        ReportFormat::Json => JsonReportGenerator::new().generate(scan),
        ReportFormat::Html => HtmlReportGenerator::new().generate(scan),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{ScanType, SecurityScan, Vulnerability, DetectedSecret, SecretType};

    #[test]
    fn test_report_format_from_str() {
        assert_eq!("sarif".parse::<ReportFormat>().unwrap(), ReportFormat::Sarif);
        assert_eq!("json".parse::<ReportFormat>().unwrap(), ReportFormat::Json);
        assert_eq!("html".parse::<ReportFormat>().unwrap(), ReportFormat::Html);
        assert_eq!("SARIF".parse::<ReportFormat>().unwrap(), ReportFormat::Sarif);
        assert!("invalid".parse::<ReportFormat>().is_err());
    }

    #[test]
    fn test_json_report_generation() {
        let mut scan = SecurityScan::new(vec![ScanType::Dependencies], "test");
        scan.add_vulnerability(
            Vulnerability::dependency("lodash", "4.17.20", Severity::Critical)
                .with_cve("CVE-2021-23337")
                .with_fix("4.17.21"),
        );
        scan.complete();

        let generator = JsonReportGenerator::new();
        let report = generator.generate(&scan).unwrap();

        assert!(report.contains("CVE-2021-23337"));
        assert!(report.contains("lodash"));
        assert!(report.contains("4.17.21"));

        // Verify it's valid JSON
        let _: serde_json::Value = serde_json::from_str(&report).unwrap();
    }

    #[test]
    fn test_html_report_generation() {
        let mut scan = SecurityScan::new(vec![ScanType::Code], "test");
        scan.add_vulnerability(
            Vulnerability::code("XSS Vulnerability", "src/app.rs", 42, Severity::High)
                .with_description("Unescaped user input in HTML context"),
        );
        scan.complete();

        let generator = HtmlReportGenerator::new();
        let report = generator.generate(&scan).unwrap();

        assert!(report.contains("<!DOCTYPE html>"));
        assert!(report.contains("XSS Vulnerability"));
        assert!(report.contains("src/app.rs"));
        assert!(report.contains("HIGH"));
        assert!(report.contains("Unescaped user input"));
    }

    #[test]
    fn test_html_report_with_secrets() {
        let mut scan = SecurityScan::new(vec![ScanType::Secrets], "test");
        scan.add_secret(DetectedSecret::new(
            SecretType::AwsAccessKey,
            ".env",
            10,
            "AKIA***",
        ).in_history("abc123"));
        scan.complete();

        let generator = HtmlReportGenerator::new();
        let report = generator.generate(&scan).unwrap();

        assert!(report.contains("Detected Secrets"));
        assert!(report.contains("AWS Access Key"));
        assert!(report.contains(".env"));
        assert!(report.contains("git history"));
    }

    #[test]
    fn test_html_report_without_css() {
        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        scan.complete();

        let generator = HtmlReportGenerator::new().without_css();
        let report = generator.generate(&scan).unwrap();

        assert!(report.contains("<!DOCTYPE html>"));
        assert!(!report.contains("<style>"));
    }

    #[test]
    fn test_sarif_report_generation() {
        let mut scan = SecurityScan::new(vec![ScanType::Code], "test");
        let mut vuln = Vulnerability::code("SQL Injection", "src/db.rs", 100, Severity::Critical)
            .with_description("Unsafe SQL query construction");
        vuln.title = "SQL Injection".to_string(); // Ensure title is set
        scan.add_vulnerability(vuln);
        scan.complete();

        let report = generate_report(&scan, ReportFormat::Sarif).unwrap();

        assert!(report.contains("sarif-schema"));
        assert!(report.contains("Unsafe SQL query construction"));
        assert!(report.contains("src/db.rs"));

        // Verify it's valid JSON
        let _: serde_json::Value = serde_json::from_str(&report).unwrap();
    }

    #[test]
    fn test_generate_report_all_formats() {
        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        scan.add_vulnerability(
            Vulnerability::dependency("axios", "0.21.0", Severity::High)
                .with_cve("CVE-2021-3749")
                .with_fix("0.21.2"),
        );
        scan.complete();

        // SARIF
        let sarif = generate_report(&scan, ReportFormat::Sarif).unwrap();
        assert!(sarif.contains("sarif-schema"));

        // JSON
        let json = generate_report(&scan, ReportFormat::Json).unwrap();
        assert!(json.contains("axios"));

        // HTML
        let html = generate_report(&scan, ReportFormat::Html).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_report_extension() {
        assert_eq!(JsonReportGenerator::new().extension(), "json");
        assert_eq!(HtmlReportGenerator::new().extension(), "html");
    }
}
