//! Test generation service for automated test creation
//!
//! This module provides functionality to analyze source code and generate
//! unit tests for functions across multiple languages (Rust, TypeScript, Python).

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Programming language support for test generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    Python,
}

impl Language {
    /// Detect language from file extension
    pub fn from_path(path: &Path) -> Result<Self> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| Error::Other("No file extension found".to_string()))?;

        match extension {
            "rs" => Ok(Language::Rust),
            "ts" | "tsx" => Ok(Language::TypeScript),
            "py" => Ok(Language::Python),
            _ => Err(Error::Other(format!(
                "Unsupported file extension: {}",
                extension
            ))),
        }
    }

    /// Get test file extension for this language
    pub fn test_extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::TypeScript => "test.ts",
            Language::Python => "test.py",
        }
    }

    /// Get typical test directory for this language
    pub fn test_directory(&self) -> &'static str {
        match self {
            Language::Rust => "tests",
            Language::TypeScript => "__tests__",
            Language::Python => "tests",
        }
    }
}

/// Represents a function signature extracted from source code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Function name
    pub name: String,
    /// Parameter names and types (simplified)
    pub parameters: Vec<Parameter>,
    /// Return type (if known)
    pub return_type: Option<String>,
    /// Whether function is async
    pub is_async: bool,
    /// Source code line number
    pub line_number: usize,
}

/// Function parameter
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_hint: Option<String>,
}

/// Test case to be generated
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCase {
    /// Test name
    pub name: String,
    /// Test category (happy_path, edge_case, error_condition)
    pub category: TestCategory,
    /// Generated test code
    pub code: String,
}

/// Category of test case
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCategory {
    HappyPath,
    EdgeCase,
    ErrorCondition,
}

/// Result of test generation for a file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestGenerationResult {
    /// Source file that was analyzed
    pub source_file: PathBuf,
    /// Target language
    pub language: Language,
    /// Functions found in the file
    pub functions: Vec<FunctionSignature>,
    /// Generated test cases
    pub test_cases: Vec<TestCase>,
    /// Suggested test file location
    pub test_file_path: PathBuf,
}

/// Service for generating unit tests
pub struct TestGenerationService;

impl TestGenerationService {
    /// Create a new test generation service
    pub fn new() -> Self {
        Self
    }

    /// Analyze a source file and generate unit tests
    pub async fn generate_tests(&self, file_path: &Path) -> Result<TestGenerationResult> {
        // Detect language from file extension
        let language = Language::from_path(file_path)?;

        // Read source file
        let source_code = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| Error::Other(format!("Failed to read file: {}", e)))?;

        // Extract function signatures
        let functions = self.extract_functions(&source_code, language)?;

        // Generate test cases for each function
        let mut test_cases = Vec::new();
        for function in &functions {
            test_cases.extend(self.generate_test_cases(function, language)?);
        }

        // Determine test file location
        let test_file_path = self.determine_test_location(file_path, language)?;

        Ok(TestGenerationResult {
            source_file: file_path.to_path_buf(),
            language,
            functions,
            test_cases,
            test_file_path,
        })
    }

    /// Extract function signatures from source code
    fn extract_functions(
        &self,
        source_code: &str,
        language: Language,
    ) -> Result<Vec<FunctionSignature>> {
        match language {
            Language::Rust => self.extract_rust_functions(source_code),
            Language::TypeScript => self.extract_typescript_functions(source_code),
            Language::Python => self.extract_python_functions(source_code),
        }
    }

    /// Extract Rust function signatures
    fn extract_rust_functions(&self, source_code: &str) -> Result<Vec<FunctionSignature>> {
        let mut functions = Vec::new();
        let lines: Vec<&str> = source_code.lines().collect();

        for (line_number, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip comments and non-function lines
            if trimmed.starts_with("//") || !trimmed.contains("fn ") {
                continue;
            }

            // Simple regex-like parsing for "pub fn name(" or "fn name("
            if let Some(fn_pos) = trimmed.find("fn ") {
                let after_fn = &trimmed[fn_pos + 3..];
                if let Some(paren_pos) = after_fn.find('(') {
                    let name = after_fn[..paren_pos].trim().to_string();

                    // Skip test functions themselves
                    if name.starts_with("test_") {
                        continue;
                    }

                    // Check if async
                    let is_async = trimmed.contains("async fn");

                    // Extract parameters (simplified - just count them)
                    let params = self.extract_rust_parameters(after_fn);

                    // Extract return type (simplified)
                    let return_type = self.extract_rust_return_type(after_fn);

                    functions.push(FunctionSignature {
                        name,
                        parameters: params,
                        return_type,
                        is_async,
                        line_number: line_number + 1,
                    });
                }
            }
        }

        Ok(functions)
    }

    /// Extract Rust function parameters
    fn extract_rust_parameters(&self, fn_signature: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        if let Some(paren_start) = fn_signature.find('(') {
            if let Some(paren_end) = fn_signature.find(')') {
                let params_str = &fn_signature[paren_start + 1..paren_end];

                if params_str.trim().is_empty() || params_str.trim() == "&self" || params_str.trim() == "self" {
                    return params;
                }

                for param in params_str.split(',') {
                    let parts: Vec<&str> = param.trim().split(':').collect();
                    if parts.len() >= 2 {
                        params.push(Parameter {
                            name: parts[0].trim().to_string(),
                            type_hint: Some(parts[1].trim().to_string()),
                        });
                    }
                }
            }
        }

        params
    }

    /// Extract Rust return type
    fn extract_rust_return_type(&self, fn_signature: &str) -> Option<String> {
        if let Some(arrow_pos) = fn_signature.find("->") {
            let after_arrow = &fn_signature[arrow_pos + 2..];
            if let Some(brace_pos) = after_arrow.find('{') {
                return Some(after_arrow[..brace_pos].trim().to_string());
            }
        }
        None
    }

    /// Extract TypeScript function signatures
    fn extract_typescript_functions(&self, source_code: &str) -> Result<Vec<FunctionSignature>> {
        let mut functions = Vec::new();
        let lines: Vec<&str> = source_code.lines().collect();

        for (line_number, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }

            // Look for function declarations
            if trimmed.contains("function ") || trimmed.contains("const ") && trimmed.contains("= (") {
                // Extract function name (simplified)
                let name = self.extract_typescript_function_name(trimmed)?;

                // Skip test functions
                if name.starts_with("test") || name.contains("Test") {
                    continue;
                }

                let is_async = trimmed.contains("async");

                functions.push(FunctionSignature {
                    name,
                    parameters: Vec::new(), // Simplified for now
                    return_type: None,
                    is_async,
                    line_number: line_number + 1,
                });
            }
        }

        Ok(functions)
    }

    /// Extract TypeScript function name
    fn extract_typescript_function_name(&self, line: &str) -> Result<String> {
        if let Some(fn_pos) = line.find("function ") {
            let after_fn = &line[fn_pos + 9..];
            if let Some(paren_pos) = after_fn.find('(') {
                return Ok(after_fn[..paren_pos].trim().to_string());
            }
        } else if let Some(const_pos) = line.find("const ") {
            let after_const = &line[const_pos + 6..];
            if let Some(eq_pos) = after_const.find('=') {
                return Ok(after_const[..eq_pos].trim().to_string());
            }
        }

        Err(Error::Other("Could not extract function name".to_string()))
    }

    /// Extract Python function signatures
    fn extract_python_functions(&self, source_code: &str) -> Result<Vec<FunctionSignature>> {
        let mut functions = Vec::new();
        let lines: Vec<&str> = source_code.lines().collect();

        for (line_number, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check for async def first, then regular def
            let is_async = trimmed.starts_with("async def ");
            let is_def = trimmed.starts_with("def ") || is_async;

            if is_def {
                let after_def = if is_async {
                    &trimmed[10..] // "async def " is 10 chars
                } else {
                    &trimmed[4..] // "def " is 4 chars
                };

                if let Some(paren_pos) = after_def.find('(') {
                    let name = after_def[..paren_pos].trim().to_string();

                    // Skip test functions
                    if name.starts_with("test_") {
                        continue;
                    }

                    functions.push(FunctionSignature {
                        name,
                        parameters: Vec::new(), // Simplified for now
                        return_type: None,
                        is_async,
                        line_number: line_number + 1,
                    });
                }
            }
        }

        Ok(functions)
    }

    /// Generate test cases for a function
    fn generate_test_cases(
        &self,
        function: &FunctionSignature,
        language: Language,
    ) -> Result<Vec<TestCase>> {
        let mut test_cases = Vec::new();

        // Generate happy path test
        test_cases.push(self.generate_happy_path_test(function, language)?);

        // Generate edge case tests
        test_cases.extend(self.generate_edge_case_tests(function, language)?);

        // Generate error condition tests
        test_cases.extend(self.generate_error_tests(function, language)?);

        Ok(test_cases)
    }

    /// Generate happy path test
    fn generate_happy_path_test(
        &self,
        function: &FunctionSignature,
        language: Language,
    ) -> Result<TestCase> {
        let test_name = format!("test_{}_happy_path", function.name);
        let code = match language {
            Language::Rust => self.generate_rust_test_code(function, &test_name, TestCategory::HappyPath),
            Language::TypeScript => self.generate_typescript_test_code(function, &test_name, TestCategory::HappyPath),
            Language::Python => self.generate_python_test_code(function, &test_name, TestCategory::HappyPath),
        };

        Ok(TestCase {
            name: test_name,
            category: TestCategory::HappyPath,
            code,
        })
    }

    /// Generate edge case tests
    fn generate_edge_case_tests(
        &self,
        function: &FunctionSignature,
        language: Language,
    ) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();

        // Generate empty input test
        let empty_test_name = format!("test_{}_empty_input", function.name);
        let empty_code = match language {
            Language::Rust => self.generate_rust_test_code(function, &empty_test_name, TestCategory::EdgeCase),
            Language::TypeScript => self.generate_typescript_test_code(function, &empty_test_name, TestCategory::EdgeCase),
            Language::Python => self.generate_python_test_code(function, &empty_test_name, TestCategory::EdgeCase),
        };

        tests.push(TestCase {
            name: empty_test_name,
            category: TestCategory::EdgeCase,
            code: empty_code,
        });

        Ok(tests)
    }

    /// Generate error condition tests
    fn generate_error_tests(
        &self,
        function: &FunctionSignature,
        language: Language,
    ) -> Result<Vec<TestCase>> {
        let mut tests = Vec::new();

        // Generate error condition test
        let error_test_name = format!("test_{}_error_condition", function.name);
        let error_code = match language {
            Language::Rust => self.generate_rust_test_code(function, &error_test_name, TestCategory::ErrorCondition),
            Language::TypeScript => self.generate_typescript_test_code(function, &error_test_name, TestCategory::ErrorCondition),
            Language::Python => self.generate_python_test_code(function, &error_test_name, TestCategory::ErrorCondition),
        };

        tests.push(TestCase {
            name: error_test_name,
            category: TestCategory::ErrorCondition,
            code: error_code,
        });

        Ok(tests)
    }

    /// Generate Rust test code
    fn generate_rust_test_code(
        &self,
        function: &FunctionSignature,
        test_name: &str,
        category: TestCategory,
    ) -> String {
        let async_marker = if function.is_async {
            "#[tokio::test]\nasync "
        } else {
            "#[test]\n"
        };

        let await_suffix = if function.is_async { ".await" } else { "" };

        let test_body = match category {
            TestCategory::HappyPath => {
                format!(
                    r#"    // Arrange
    let input = todo!("Setup test input");

    // Act
    let result = {}(input){};

    // Assert
    assert!(result.is_ok(), "Function should succeed with valid input");"#,
                    function.name, await_suffix
                )
            }
            TestCategory::EdgeCase => {
                format!(
                    r#"    // Arrange
    let input = todo!("Setup empty/boundary input");

    // Act
    let result = {}(input){};

    // Assert
    // TODO: Add appropriate assertion for edge case"#,
                    function.name, await_suffix
                )
            }
            TestCategory::ErrorCondition => {
                format!(
                    r#"    // Arrange
    let input = todo!("Setup invalid input");

    // Act
    let result = {}(input){};

    // Assert
    assert!(result.is_err(), "Function should fail with invalid input");"#,
                    function.name, await_suffix
                )
            }
        };

        format!(
            r#"{}fn {}() {{
{}
}}"#,
            async_marker, test_name, test_body
        )
    }

    /// Generate TypeScript test code
    fn generate_typescript_test_code(
        &self,
        function: &FunctionSignature,
        test_name: &str,
        category: TestCategory,
    ) -> String {
        let async_marker = if function.is_async { "async " } else { "" };
        let await_prefix = if function.is_async { "await " } else { "" };

        let test_body = match category {
            TestCategory::HappyPath => {
                format!(
                    r#"  // Arrange
  const input = null; // TODO: Setup test input

  // Act
  const result = {}{}(input);

  // Assert
  expect(result).toBeDefined();"#,
                    await_prefix, function.name
                )
            }
            TestCategory::EdgeCase => {
                format!(
                    r#"  // Arrange
  const input = null; // TODO: Setup empty/boundary input

  // Act
  const result = {}{}(input);

  // Assert
  // TODO: Add appropriate assertion for edge case"#,
                    await_prefix, function.name
                )
            }
            TestCategory::ErrorCondition => {
                format!(
                    r#"  // Arrange
  const input = null; // TODO: Setup invalid input

  // Act & Assert
  expect(() => {}{}(input)).toThrow();"#,
                    await_prefix, function.name
                )
            }
        };

        format!(
            r#"test('{}', {}() => {{
{}
}});"#,
            test_name, async_marker, test_body
        )
    }

    /// Generate Python test code
    fn generate_python_test_code(
        &self,
        function: &FunctionSignature,
        test_name: &str,
        category: TestCategory,
    ) -> String {
        let async_marker = if function.is_async { "async " } else { "" };
        let await_prefix = if function.is_async { "await " } else { "" };

        let test_body = match category {
            TestCategory::HappyPath => {
                format!(
                    r#"    # Arrange
    input_data = None  # TODO: Setup test input

    # Act
    result = {}{}(input_data)

    # Assert
    assert result is not None"#,
                    await_prefix, function.name
                )
            }
            TestCategory::EdgeCase => {
                format!(
                    r#"    # Arrange
    input_data = None  # TODO: Setup empty/boundary input

    # Act
    result = {}{}(input_data)

    # Assert
    # TODO: Add appropriate assertion for edge case"#,
                    await_prefix, function.name
                )
            }
            TestCategory::ErrorCondition => {
                format!(
                    r#"    # Arrange
    input_data = None  # TODO: Setup invalid input

    # Act & Assert
    with pytest.raises(Exception):
        {}{}(input_data)"#,
                    await_prefix, function.name
                )
            }
        };

        format!(
            r#"{}def {}():
{}
"#,
            async_marker, test_name, test_body
        )
    }

    /// Determine where to place test file
    fn determine_test_location(&self, source_file: &Path, language: Language) -> Result<PathBuf> {
        let file_name = source_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::Other("Invalid file name".to_string()))?;

        match language {
            Language::Rust => {
                // For Rust, tests can be in the same file or in tests/ directory
                // Default to same file (inline tests module)
                Ok(source_file.to_path_buf())
            }
            Language::TypeScript => {
                // For TypeScript, put in __tests__ directory
                let parent = source_file
                    .parent()
                    .ok_or_else(|| Error::Other("No parent directory".to_string()))?;
                Ok(parent.join("__tests__").join(format!(
                    "{}.{}",
                    file_name,
                    language.test_extension()
                )))
            }
            Language::Python => {
                // For Python, put in tests/ directory
                let parent = source_file
                    .parent()
                    .ok_or_else(|| Error::Other("No parent directory".to_string()))?;
                Ok(parent.join("tests").join(format!(
                    "{}_{}",
                    file_name,
                    language.test_extension()
                )))
            }
        }
    }

    /// Format test cases into a complete test file/module
    pub fn format_test_output(
        &self,
        result: &TestGenerationResult,
    ) -> Result<String> {
        match result.language {
            Language::Rust => self.format_rust_test_module(&result.test_cases),
            Language::TypeScript => self.format_typescript_test_file(&result.test_cases),
            Language::Python => self.format_python_test_file(&result.test_cases),
        }
    }

    /// Format Rust test module
    fn format_rust_test_module(&self, test_cases: &[TestCase]) -> Result<String> {
        let mut output = String::from(
            r#"#[cfg(test)]
mod tests {
    use super::*;

"#,
        );

        for test_case in test_cases {
            output.push_str("    ");
            output.push_str(&test_case.code.replace('\n', "\n    "));
            output.push_str("\n\n");
        }

        output.push('}');
        Ok(output)
    }

    /// Format TypeScript test file
    fn format_typescript_test_file(&self, test_cases: &[TestCase]) -> Result<String> {
        let mut output = String::from("import { describe, test, expect } from 'vitest';\n\n");

        for test_case in test_cases {
            output.push_str(&test_case.code);
            output.push_str("\n\n");
        }

        Ok(output)
    }

    /// Format Python test file
    fn format_python_test_file(&self, test_cases: &[TestCase]) -> Result<String> {
        let mut output = String::from("import pytest\n\n");

        for test_case in test_cases {
            output.push_str(&test_case.code);
            output.push('\n');
        }

        Ok(output)
    }
}

impl Default for TestGenerationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_path_rust() {
        let path = Path::new("src/main.rs");
        let lang = Language::from_path(path).unwrap();
        assert_eq!(lang, Language::Rust);
    }

    #[test]
    fn test_language_from_path_typescript() {
        let path = Path::new("src/index.ts");
        let lang = Language::from_path(path).unwrap();
        assert_eq!(lang, Language::TypeScript);
    }

    #[test]
    fn test_language_from_path_python() {
        let path = Path::new("src/main.py");
        let lang = Language::from_path(path).unwrap();
        assert_eq!(lang, Language::Python);
    }

    #[test]
    fn test_language_from_path_unsupported() {
        let path = Path::new("src/main.java");
        let result = Language::from_path(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_rust_simple_function() {
        let service = TestGenerationService::new();
        let source = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let functions = service.extract_rust_functions(source).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "add");
        assert_eq!(functions[0].parameters.len(), 2);
        assert_eq!(functions[0].return_type, Some("i32".to_string()));
        assert!(!functions[0].is_async);
    }

    #[test]
    fn test_extract_rust_async_function() {
        let service = TestGenerationService::new();
        let source = r#"
pub async fn fetch_data(url: String) -> Result<String> {
    // implementation
}
"#;
        let functions = service.extract_rust_functions(source).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "fetch_data");
        assert!(functions[0].is_async);
    }

    #[test]
    fn test_extract_rust_no_params_function() {
        let service = TestGenerationService::new();
        let source = r#"
fn get_default() -> i32 {
    42
}
"#;
        let functions = service.extract_rust_functions(source).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "get_default");
        assert_eq!(functions[0].parameters.len(), 0);
    }

    #[test]
    fn test_extract_rust_skip_test_functions() {
        let service = TestGenerationService::new();
        let source = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_add() {
    assert_eq!(add(1, 2), 3);
}
"#;
        let functions = service.extract_rust_functions(source).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "add");
    }

    #[test]
    fn test_extract_typescript_function() {
        let service = TestGenerationService::new();
        let source = r#"
function calculateSum(a: number, b: number): number {
    return a + b;
}
"#;
        let functions = service.extract_typescript_functions(source).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "calculateSum");
    }

    #[test]
    fn test_extract_typescript_async_function() {
        let service = TestGenerationService::new();
        let source = r#"
async function fetchData(url: string): Promise<string> {
    return await fetch(url);
}
"#;
        let functions = service.extract_typescript_functions(source).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "fetchData");
        assert!(functions[0].is_async);
    }

    #[test]
    fn test_extract_python_function() {
        let service = TestGenerationService::new();
        let source = r#"
def calculate_sum(a, b):
    return a + b
"#;
        let functions = service.extract_python_functions(source).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "calculate_sum");
    }

    #[test]
    fn test_extract_python_async_function() {
        let service = TestGenerationService::new();
        let source = r#"
async def fetch_data(url):
    return await client.get(url)
"#;
        let functions = service.extract_python_functions(source).unwrap();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "fetch_data");
        assert!(functions[0].is_async);
    }

    #[test]
    fn test_generate_rust_happy_path_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "add".to_string(),
            parameters: vec![],
            return_type: Some("i32".to_string()),
            is_async: false,
            line_number: 1,
        };

        let test_case = service
            .generate_happy_path_test(&function, Language::Rust)
            .unwrap();
        assert_eq!(test_case.name, "test_add_happy_path");
        assert_eq!(test_case.category, TestCategory::HappyPath);
        assert!(test_case.code.contains("#[test]"));
        assert!(test_case.code.contains("fn test_add_happy_path()"));
    }

    #[test]
    fn test_generate_typescript_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "calculateSum".to_string(),
            parameters: vec![],
            return_type: None,
            is_async: false,
            line_number: 1,
        };

        let test_case = service
            .generate_happy_path_test(&function, Language::TypeScript)
            .unwrap();
        assert!(test_case.code.contains("test('test_calculateSum_happy_path'"));
        assert!(test_case.code.contains("expect(result).toBeDefined()"));
    }

    #[test]
    fn test_generate_python_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "calculate_sum".to_string(),
            parameters: vec![],
            return_type: None,
            is_async: false,
            line_number: 1,
        };

        let test_case = service
            .generate_happy_path_test(&function, Language::Python)
            .unwrap();
        assert!(test_case.code.contains("def test_calculate_sum_happy_path()"));
        assert!(test_case.code.contains("assert result is not None"));
    }

    #[test]
    fn test_determine_test_location_rust() {
        let service = TestGenerationService::new();
        let source_file = Path::new("/project/src/lib.rs");
        let test_location = service
            .determine_test_location(source_file, Language::Rust)
            .unwrap();
        // Rust tests typically in same file
        assert_eq!(test_location, source_file);
    }

    #[test]
    fn test_determine_test_location_typescript() {
        let service = TestGenerationService::new();
        let source_file = Path::new("/project/src/index.ts");
        let test_location = service
            .determine_test_location(source_file, Language::TypeScript)
            .unwrap();
        assert_eq!(
            test_location,
            Path::new("/project/src/__tests__/index.test.ts")
        );
    }

    #[test]
    fn test_determine_test_location_python() {
        let service = TestGenerationService::new();
        let source_file = Path::new("/project/src/main.py");
        let test_location = service
            .determine_test_location(source_file, Language::Python)
            .unwrap();
        assert_eq!(
            test_location,
            Path::new("/project/src/tests/main_test.py")
        );
    }

    #[test]
    fn test_format_rust_test_module() {
        let service = TestGenerationService::new();
        let test_cases = vec![TestCase {
            name: "test_add".to_string(),
            category: TestCategory::HappyPath,
            code: "#[test]\nfn test_add() {\n    assert_eq!(1 + 1, 2);\n}".to_string(),
        }];

        let output = service.format_rust_test_module(&test_cases).unwrap();
        assert!(output.contains("#[cfg(test)]"));
        assert!(output.contains("mod tests {"));
        assert!(output.contains("use super::*;"));
    }

    #[test]
    fn test_format_typescript_test_file() {
        let service = TestGenerationService::new();
        let test_cases = vec![TestCase {
            name: "test_add".to_string(),
            category: TestCategory::HappyPath,
            code: "test('adds numbers', () => { expect(1 + 1).toBe(2); });".to_string(),
        }];

        let output = service.format_typescript_test_file(&test_cases).unwrap();
        assert!(output.contains("import { describe, test, expect } from 'vitest';"));
    }

    #[tokio::test]
    async fn test_generate_tests_for_rust_file() {
        let service = TestGenerationService::new();

        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        tokio::fs::write(
            &test_file,
            r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
"#,
        )
        .await
        .unwrap();

        let result = service.generate_tests(&test_file).await.unwrap();

        assert_eq!(result.language, Language::Rust);
        assert_eq!(result.functions.len(), 2);
        assert_eq!(result.functions[0].name, "add");
        assert_eq!(result.functions[1].name, "subtract");

        // Each function should have 3 test cases (happy path, edge case, error)
        assert_eq!(result.test_cases.len(), 6);
    }
}
