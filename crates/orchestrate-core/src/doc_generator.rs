//! Documentation Generation Functions
//!
//! Functions for parsing code and generating various types of documentation.

use crate::documentation::{
    ApiEndpoint, ChangelogEntry, ChangeType, DocValidationIssue, DocValidationResult,
    ReadmeContent, ReadmeSection, ReadmeSectionContent,
};
use crate::Result;

/// Parse Rust source code to extract API endpoints
pub fn parse_api_endpoints_from_rust(source_code: &str) -> Result<Vec<ApiEndpoint>> {
    let mut endpoints = Vec::new();

    // Basic regex-based parsing for Rust actix-web routes
    // This is a simplified implementation - real one would use syn/proc_macro
    for line in source_code.lines() {
        let trimmed = line.trim();

        // Look for route annotations like #[get("/path")]
        if let Some(route_info) = parse_route_annotation(trimmed) {
            endpoints.push(route_info);
        }
    }

    Ok(endpoints)
}

/// Parse a route annotation to extract endpoint information
fn parse_route_annotation(line: &str) -> Option<ApiEndpoint> {
    // Match patterns like: #[get("/api/agents")]
    let methods = ["get", "post", "put", "delete", "patch"];

    for method in &methods {
        let pattern = format!("#[{}(\"", method);
        if line.starts_with(&pattern) {
            if let Some(end_idx) = line.find("\")]") {
                let start_idx = pattern.len();
                let path = &line[start_idx..end_idx];
                return Some(ApiEndpoint::new(method, path));
            }
        }
    }

    None
}

/// Parse git log to generate changelog entries
pub fn parse_git_commits(
    commit_messages: &str,
) -> Result<Vec<ChangelogEntry>> {
    let mut entries = Vec::new();

    for line in commit_messages.lines() {
        if line.is_empty() {
            continue;
        }

        // Parse format: "message|hash|author"
        let parts: Vec<&str> = line.splitn(3, '|').collect();
        if parts.len() < 3 {
            continue;
        }

        let message = parts[0];
        let hash = parts[1];
        let author = parts[2];

        // Parse conventional commit format: type(scope): description
        if let Some((commit_type, description, scope, breaking)) =
            parse_conventional_commit(message)
        {
            if let Some(change_type) = ChangeType::from_commit_type(&commit_type) {
                // Extract PR number from message like "feat: description (#123)"
                let pr_number = extract_pr_number(description);
                let clean_description = remove_pr_number(description);

                entries.push(ChangelogEntry {
                    change_type,
                    description: clean_description.to_string(),
                    commit_hash: Some(hash.to_string()),
                    pr_number,
                    issue_number: None,
                    author: Some(author.to_string()),
                    scope: scope.map(|s| s.to_string()),
                    breaking,
                });
            }
        }
    }

    Ok(entries)
}

/// Parse conventional commit format
fn parse_conventional_commit(message: &str) -> Option<(String, &str, Option<&str>, bool)> {
    // Check for breaking change marker
    let breaking = message.contains('!') || message.to_uppercase().contains("BREAKING");

    // Match: type(scope): description or type: description
    let re_with_scope = regex::Regex::new(r"^([a-z]+)\(([^)]+)\)(!?):\s*(.+)$").ok()?;
    let re_without_scope = regex::Regex::new(r"^([a-z]+)(!?):\s*(.+)$").ok()?;

    if let Some(caps) = re_with_scope.captures(message) {
        let commit_type = caps.get(1)?.as_str().to_string();
        let scope = Some(caps.get(2)?.as_str());
        let description = caps.get(4)?.as_str();
        Some((commit_type, description, scope, breaking))
    } else if let Some(caps) = re_without_scope.captures(message) {
        let commit_type = caps.get(1)?.as_str().to_string();
        let description = caps.get(3)?.as_str();
        Some((commit_type, description, None, breaking))
    } else {
        None
    }
}

/// Extract PR number from description
fn extract_pr_number(description: &str) -> Option<i64> {
    let re = regex::Regex::new(r"\(#(\d+)\)").ok()?;
    re.captures(description)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<i64>().ok())
}

/// Remove PR number from description
fn remove_pr_number(description: &str) -> &str {
    if let Some(idx) = description.rfind(" (#") {
        &description[..idx]
    } else {
        description
    }
}

/// Validate Rust documentation coverage
pub fn validate_rust_doc_coverage(source_code: &str, file_path: &str) -> DocValidationResult {
    let mut result = DocValidationResult::new();

    // Parse for public items
    for (line_num, line) in source_code.lines().enumerate() {
        let trimmed = line.trim();

        // Check for public functions
        if trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn ") {
            result.total_items += 1;

            // Check if previous lines contain doc comment
            let has_doc = check_previous_doc_comment(source_code, line_num);

            if has_doc {
                result.documented_items += 1;
            } else {
                // Extract function name
                if let Some(fn_name) = extract_function_name(trimmed) {
                    result.add_issue(DocValidationIssue {
                        file_path: file_path.to_string(),
                        line_number: Some(line_num + 1),
                        item_name: fn_name.to_string(),
                        item_type: crate::documentation::DocItemType::Function,
                        issue_type: crate::documentation::DocIssueType::MissingDoc,
                        message: format!("Public function '{}' is missing documentation", fn_name),
                    });
                }
            }
        }

        // Check for public structs
        if trimmed.starts_with("pub struct ") {
            result.total_items += 1;

            let has_doc = check_previous_doc_comment(source_code, line_num);

            if has_doc {
                result.documented_items += 1;
            } else {
                if let Some(struct_name) = extract_struct_name(trimmed) {
                    result.add_issue(DocValidationIssue {
                        file_path: file_path.to_string(),
                        line_number: Some(line_num + 1),
                        item_name: struct_name.to_string(),
                        item_type: crate::documentation::DocItemType::Struct,
                        issue_type: crate::documentation::DocIssueType::MissingDoc,
                        message: format!("Public struct '{}' is missing documentation", struct_name),
                    });
                }
            }
        }

        // Check for public enums
        if trimmed.starts_with("pub enum ") {
            result.total_items += 1;

            let has_doc = check_previous_doc_comment(source_code, line_num);

            if has_doc {
                result.documented_items += 1;
            } else {
                if let Some(enum_name) = extract_enum_name(trimmed) {
                    result.add_issue(DocValidationIssue {
                        file_path: file_path.to_string(),
                        line_number: Some(line_num + 1),
                        item_name: enum_name.to_string(),
                        item_type: crate::documentation::DocItemType::Enum,
                        issue_type: crate::documentation::DocIssueType::MissingDoc,
                        message: format!("Public enum '{}' is missing documentation", enum_name),
                    });
                }
            }
        }
    }

    result.calculate_coverage();
    result
}

/// Check if previous lines contain a doc comment
fn check_previous_doc_comment(source_code: &str, current_line: usize) -> bool {
    let lines: Vec<&str> = source_code.lines().collect();

    if current_line == 0 {
        return false;
    }

    // Check up to 10 lines above
    for i in (0..current_line).rev().take(10) {
        let trimmed = lines[i].trim();

        // Found doc comment
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            return true;
        }

        // Stop at non-comment, non-attribute line
        if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("#[") {
            break;
        }
    }

    false
}

/// Extract function name from line
fn extract_function_name(line: &str) -> Option<&str> {
    let after_fn = line.strip_prefix("pub fn ")
        .or_else(|| line.strip_prefix("pub async fn "))?;

    after_fn.split('(').next()
}

/// Extract struct name from line
fn extract_struct_name(line: &str) -> Option<&str> {
    let after_struct = line.strip_prefix("pub struct ")?;
    after_struct.split_whitespace().next()
        .and_then(|s| s.split('<').next())
        .and_then(|s| s.split('(').next())
}

/// Extract enum name from line
fn extract_enum_name(line: &str) -> Option<&str> {
    let after_enum = line.strip_prefix("pub enum ")?;
    after_enum.split_whitespace().next()
        .and_then(|s| s.split('<').next())
}

/// Generate README content from project structure
pub fn generate_readme_content(
    project_name: &str,
    description: Option<&str>,
    has_cargo_toml: bool,
    has_package_json: bool,
) -> ReadmeContent {
    let mut content = ReadmeContent::default();

    // Title section
    content.sections.push(ReadmeSectionContent {
        section_type: ReadmeSection::Title,
        heading: None,
        content: format!("# {}", project_name),
    });

    // Description
    if let Some(desc) = description {
        content.sections.push(ReadmeSectionContent {
            section_type: ReadmeSection::Description,
            heading: None,
            content: desc.to_string(),
        });
    }

    // Installation
    let install_content = if has_cargo_toml {
        "```bash\ncargo build --release\n```".to_string()
    } else if has_package_json {
        "```bash\nnpm install\n```".to_string()
    } else {
        "Installation instructions coming soon.".to_string()
    };

    content.sections.push(ReadmeSectionContent {
        section_type: ReadmeSection::Installation,
        heading: Some("## Installation".to_string()),
        content: install_content,
    });

    // Usage
    content.sections.push(ReadmeSectionContent {
        section_type: ReadmeSection::Usage,
        heading: Some("## Usage".to_string()),
        content: "Usage documentation coming soon.".to_string(),
    });

    // License
    content.sections.push(ReadmeSectionContent {
        section_type: ReadmeSection::License,
        heading: Some("## License".to_string()),
        content: "See LICENSE file for details.".to_string(),
    });

    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_route_annotation() {
        let line = r#"#[get("/api/agents")]"#;
        let endpoint = parse_route_annotation(line);
        assert!(endpoint.is_some());
        let endpoint = endpoint.unwrap();
        assert_eq!(endpoint.method, "GET");
        assert_eq!(endpoint.path, "/api/agents");
    }

    #[test]
    fn test_parse_route_annotation_post() {
        let line = r#"#[post("/api/agents")]"#;
        let endpoint = parse_route_annotation(line);
        assert!(endpoint.is_some());
        let endpoint = endpoint.unwrap();
        assert_eq!(endpoint.method, "POST");
        assert_eq!(endpoint.path, "/api/agents");
    }

    #[test]
    fn test_parse_route_annotation_no_match() {
        let line = "fn some_function() {";
        let endpoint = parse_route_annotation(line);
        assert!(endpoint.is_none());
    }

    #[test]
    fn test_parse_conventional_commit_with_scope() {
        let message = "feat(api): add new endpoint";
        let result = parse_conventional_commit(message);
        assert!(result.is_some());
        let (commit_type, description, scope, breaking) = result.unwrap();
        assert_eq!(commit_type, "feat");
        assert_eq!(description, "add new endpoint");
        assert_eq!(scope, Some("api"));
        assert!(!breaking);
    }

    #[test]
    fn test_parse_conventional_commit_without_scope() {
        let message = "fix: correct bug";
        let result = parse_conventional_commit(message);
        assert!(result.is_some());
        let (commit_type, description, scope, breaking) = result.unwrap();
        assert_eq!(commit_type, "fix");
        assert_eq!(description, "correct bug");
        assert_eq!(scope, None);
        assert!(!breaking);
    }

    #[test]
    fn test_parse_conventional_commit_breaking() {
        let message = "feat!: breaking change";
        let result = parse_conventional_commit(message);
        assert!(result.is_some());
        let (_, _, _, breaking) = result.unwrap();
        assert!(breaking);
    }

    #[test]
    fn test_extract_pr_number() {
        assert_eq!(extract_pr_number("Some description (#123)"), Some(123));
        assert_eq!(extract_pr_number("No PR here"), None);
        assert_eq!(extract_pr_number("Multiple (#42) numbers (#99)"), Some(42));
    }

    #[test]
    fn test_remove_pr_number() {
        assert_eq!(remove_pr_number("Some description (#123)"), "Some description");
        assert_eq!(remove_pr_number("No PR here"), "No PR here");
    }

    #[test]
    fn test_parse_git_commits() {
        let commits = "feat: add feature|abc123|John Doe\nfix: fix bug (#42)|def456|Jane Doe";
        let entries = parse_git_commits(commits).unwrap();

        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].change_type, ChangeType::Added);
        assert_eq!(entries[0].description, "add feature");
        assert_eq!(entries[0].commit_hash, Some("abc123".to_string()));
        assert_eq!(entries[0].author, Some("John Doe".to_string()));
        assert_eq!(entries[0].pr_number, None);

        assert_eq!(entries[1].change_type, ChangeType::Fixed);
        assert_eq!(entries[1].description, "fix bug");
        assert_eq!(entries[1].pr_number, Some(42));
    }

    #[test]
    fn test_extract_function_name() {
        assert_eq!(extract_function_name("pub fn test_func(arg: i32)"), Some("test_func"));
        assert_eq!(extract_function_name("pub async fn async_func()"), Some("async_func"));
        assert_eq!(extract_function_name("fn private_func()"), None);
    }

    #[test]
    fn test_extract_struct_name() {
        assert_eq!(extract_struct_name("pub struct MyStruct {"), Some("MyStruct"));
        assert_eq!(extract_struct_name("pub struct Generic<T> {"), Some("Generic"));
        assert_eq!(extract_struct_name("struct Private {"), None);
    }

    #[test]
    fn test_extract_enum_name() {
        assert_eq!(extract_enum_name("pub enum Status {"), Some("Status"));
        assert_eq!(extract_enum_name("pub enum Option<T> {"), Some("Option"));
        assert_eq!(extract_enum_name("enum Private {"), None);
    }

    #[test]
    fn test_validate_rust_doc_coverage_with_docs() {
        let source = r#"
/// This function does something
pub fn documented() {}

pub fn undocumented() {}

/// A struct
pub struct Documented {}

pub struct Undocumented {}
"#;

        let result = validate_rust_doc_coverage(source, "test.rs");

        assert_eq!(result.total_items, 4);
        assert_eq!(result.documented_items, 2);
        assert_eq!(result.coverage_percentage, 50.0);
        assert_eq!(result.issues.len(), 2);
    }

    #[test]
    fn test_validate_rust_doc_coverage_all_documented() {
        let source = r#"
/// Function 1
pub fn func1() {}

/// Function 2
pub async fn func2() {}
"#;

        let result = validate_rust_doc_coverage(source, "test.rs");

        assert_eq!(result.total_items, 2);
        assert_eq!(result.documented_items, 2);
        assert_eq!(result.coverage_percentage, 100.0);
        assert_eq!(result.issues.len(), 0);
        assert!(result.is_valid());
    }

    #[test]
    fn test_generate_readme_content_rust_project() {
        let readme = generate_readme_content(
            "My Project",
            Some("A great project"),
            true,  // has Cargo.toml
            false,
        );

        let markdown = readme.to_markdown();

        assert!(markdown.contains("# My Project"));
        assert!(markdown.contains("A great project"));
        assert!(markdown.contains("cargo build"));
        assert!(markdown.contains("## Installation"));
        assert!(markdown.contains("## Usage"));
        assert!(markdown.contains("## License"));
    }

    #[test]
    fn test_generate_readme_content_node_project() {
        let readme = generate_readme_content(
            "NodeApp",
            None,
            false,
            true,  // has package.json
        );

        let markdown = readme.to_markdown();

        assert!(markdown.contains("# NodeApp"));
        assert!(markdown.contains("npm install"));
    }
}
