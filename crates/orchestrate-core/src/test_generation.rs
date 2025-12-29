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

/// Type of test to generate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestType {
    /// Unit tests for individual functions
    Unit,
    /// Integration tests for cross-module interactions
    Integration,
    /// End-to-end tests from user stories
    E2e,
    /// Property-based tests
    Property,
}

/// Represents a module boundary in the codebase
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Module name/path
    pub name: String,
    /// Public interfaces (functions, structs, traits)
    pub public_interfaces: Vec<InterfaceInfo>,
    /// Dependencies on other modules
    pub dependencies: Vec<String>,
    /// Source file path
    pub source_path: PathBuf,
}

/// Represents a public interface (function, struct, trait, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceInfo {
    /// Interface name
    pub name: String,
    /// Interface type (function, struct, trait, class, etc.)
    pub interface_type: InterfaceType,
    /// Whether this is async
    pub is_async: bool,
    /// Dependencies this interface uses
    pub uses: Vec<String>,
}

/// Type of interface
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceType {
    Function,
    Struct,
    Trait,
    Class,
    Module,
}

/// Result of integration test generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationTestResult {
    /// Module that was analyzed
    pub module: ModuleInfo,
    /// Generated test cases
    pub test_cases: Vec<TestCase>,
    /// Suggested test file location
    pub test_file_path: PathBuf,
    /// Test fixtures needed
    pub fixtures: Vec<TestFixture>,
}

/// Test fixture for integration tests
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestFixture {
    /// Fixture name
    pub name: String,
    /// Setup code
    pub setup_code: String,
    /// Teardown code
    pub teardown_code: String,
    /// Whether this fixture needs async support
    pub is_async: bool,
}

/// Type of E2E test platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum E2ETestPlatform {
    /// Playwright for web UI tests
    Playwright,
    /// Cypress for web UI tests
    Cypress,
    /// API tests using HTTP client
    Api,
    /// CLI tests
    Cli,
}

/// Acceptance criterion from a story
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    /// Criterion description
    pub description: String,
    /// Whether this is checked/completed
    pub checked: bool,
}

/// Result of E2E test generation from a story
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct E2ETestResult {
    /// Story ID that was used
    pub story_id: String,
    /// Story title
    pub story_title: String,
    /// Acceptance criteria parsed from the story
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    /// Generated test cases
    pub test_cases: Vec<TestCase>,
    /// Test platform detected/used
    pub platform: E2ETestPlatform,
    /// Test file path
    pub test_file_path: std::path::PathBuf,
    /// Test fixtures/setup
    pub fixtures: Vec<TestFixture>,
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

/// Type of property that can be tested
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    /// Roundtrip: parse(serialize(x)) == x or decode(encode(x)) == x
    Roundtrip,
    /// Idempotency: f(f(x)) == f(x)
    Idempotency,
    /// Commutativity: f(a, b) == f(b, a)
    Commutativity,
    /// Associativity: f(f(a, b), c) == f(a, f(b, c))
    Associativity,
    /// Identity: f(x, identity) == x
    Identity,
    /// Inverse: f(g(x)) == x where g is inverse of f
    Inverse,
}

/// Property-based test case definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyTestCase {
    /// Property being tested
    pub property: PropertyType,
    /// Function being tested
    pub function_name: String,
    /// Related function (for roundtrip, inverse, etc.)
    pub related_function: Option<String>,
    /// Generated test code
    pub code: String,
    /// Test name
    pub name: String,
}

/// Result of property-based test generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyTestResult {
    /// Source file analyzed
    pub source_file: PathBuf,
    /// Language detected
    pub language: Language,
    /// Functions analyzed
    pub functions: Vec<FunctionSignature>,
    /// Property test cases generated
    pub property_tests: Vec<PropertyTestCase>,
    /// Test file path
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

    /// Generate integration tests for a module
    pub async fn generate_integration_tests(
        &self,
        module_path: &Path,
    ) -> Result<IntegrationTestResult> {
        // Detect language
        let language = Language::from_path(module_path)?;

        // Read module source
        let source_code = tokio::fs::read_to_string(module_path)
            .await
            .map_err(|e| Error::Other(format!("Failed to read module: {}", e)))?;

        // Analyze module to extract boundaries and interfaces
        let module_info = self.analyze_module(&source_code, module_path, language)?;

        // Generate integration test cases for cross-module interactions
        let test_cases = self.generate_integration_test_cases(&module_info, language)?;

        // Generate test fixtures (setup/teardown)
        let fixtures = self.generate_test_fixtures(&module_info, language)?;

        // Determine integration test file location
        let test_file_path = self.determine_integration_test_location(module_path, language)?;

        Ok(IntegrationTestResult {
            module: module_info,
            test_cases,
            test_file_path,
            fixtures,
        })
    }

    /// Analyze module to identify boundaries and interfaces
    fn analyze_module(
        &self,
        source_code: &str,
        module_path: &Path,
        language: Language,
    ) -> Result<ModuleInfo> {
        match language {
            Language::Rust => self.analyze_rust_module(source_code, module_path),
            Language::TypeScript => self.analyze_typescript_module(source_code, module_path),
            Language::Python => self.analyze_python_module(source_code, module_path),
        }
    }

    /// Analyze Rust module for public interfaces
    fn analyze_rust_module(
        &self,
        source_code: &str,
        module_path: &Path,
    ) -> Result<ModuleInfo> {
        let mut public_interfaces = Vec::new();
        let mut dependencies = Vec::new();
        let lines: Vec<&str> = source_code.lines().collect();

        // Extract module name from path
        let module_name = module_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Find public functions
        for line in &lines {
            let trimmed = line.trim();

            // Look for pub fn
            if trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn ") {
                let is_async = trimmed.contains("async fn");
                if let Some(fn_pos) = trimmed.find("fn ") {
                    let after_fn = &trimmed[fn_pos + 3..];
                    if let Some(paren_pos) = after_fn.find('(') {
                        let name = after_fn[..paren_pos].trim().to_string();
                        public_interfaces.push(InterfaceInfo {
                            name,
                            interface_type: InterfaceType::Function,
                            is_async,
                            uses: Vec::new(),
                        });
                    }
                }
            }

            // Look for pub struct
            if trimmed.starts_with("pub struct ") {
                if let Some(struct_pos) = trimmed.find("struct ") {
                    let after_struct = &trimmed[struct_pos + 7..];
                    let name = after_struct
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_end_matches(&['<', '{', '('][..])
                        .to_string();
                    if !name.is_empty() {
                        public_interfaces.push(InterfaceInfo {
                            name,
                            interface_type: InterfaceType::Struct,
                            is_async: false,
                            uses: Vec::new(),
                        });
                    }
                }
            }

            // Look for pub trait
            if trimmed.starts_with("pub trait ") {
                if let Some(trait_pos) = trimmed.find("trait ") {
                    let after_trait = &trimmed[trait_pos + 6..];
                    let name = after_trait
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_end_matches(&['<', '{'][..])
                        .to_string();
                    if !name.is_empty() {
                        public_interfaces.push(InterfaceInfo {
                            name,
                            interface_type: InterfaceType::Trait,
                            is_async: false,
                            uses: Vec::new(),
                        });
                    }
                }
            }

            // Extract use statements as dependencies
            if trimmed.starts_with("use ") && !trimmed.contains("super::") {
                if let Some(use_pos) = trimmed.find("use ") {
                    let after_use = &trimmed[use_pos + 4..];
                    let dep = after_use
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !dep.is_empty() && !dependencies.contains(&dep) {
                        dependencies.push(dep);
                    }
                }
            }
        }

        Ok(ModuleInfo {
            name: module_name,
            public_interfaces,
            dependencies,
            source_path: module_path.to_path_buf(),
        })
    }

    /// Analyze TypeScript module
    fn analyze_typescript_module(
        &self,
        source_code: &str,
        module_path: &Path,
    ) -> Result<ModuleInfo> {
        let mut public_interfaces = Vec::new();
        let mut dependencies = Vec::new();
        let lines: Vec<&str> = source_code.lines().collect();

        let module_name = module_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        for line in &lines {
            let trimmed = line.trim();

            // Look for exported functions
            if trimmed.starts_with("export function ") || trimmed.starts_with("export async function ") {
                let is_async = trimmed.contains("async");
                if let Some(name) = self.extract_typescript_function_name(trimmed).ok() {
                    public_interfaces.push(InterfaceInfo {
                        name,
                        interface_type: InterfaceType::Function,
                        is_async,
                        uses: Vec::new(),
                    });
                }
            }

            // Look for exported classes
            if trimmed.starts_with("export class ") {
                if let Some(class_pos) = trimmed.find("class ") {
                    let after_class = &trimmed[class_pos + 6..];
                    let name = after_class
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .trim_end_matches(&['<', '{'][..])
                        .to_string();
                    if !name.is_empty() {
                        public_interfaces.push(InterfaceInfo {
                            name,
                            interface_type: InterfaceType::Class,
                            is_async: false,
                            uses: Vec::new(),
                        });
                    }
                }
            }

            // Extract imports as dependencies
            if trimmed.starts_with("import ") {
                if let Some(from_pos) = trimmed.find(" from ") {
                    let dep = trimmed[from_pos + 6..]
                        .trim()
                        .trim_matches(&['\'', '"', ';'][..])
                        .to_string();
                    if !dep.is_empty() && !dependencies.contains(&dep) {
                        dependencies.push(dep);
                    }
                }
            }
        }

        Ok(ModuleInfo {
            name: module_name,
            public_interfaces,
            dependencies,
            source_path: module_path.to_path_buf(),
        })
    }

    /// Analyze Python module
    fn analyze_python_module(
        &self,
        source_code: &str,
        module_path: &Path,
    ) -> Result<ModuleInfo> {
        let mut public_interfaces = Vec::new();
        let mut dependencies = Vec::new();
        let lines: Vec<&str> = source_code.lines().collect();

        let module_name = module_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        for line in &lines {
            let trimmed = line.trim();

            // Python doesn't have explicit public markers, but by convention
            // functions/classes not starting with _ are public
            if (trimmed.starts_with("def ") || trimmed.starts_with("async def "))
                && !trimmed.contains("def _")
                && !trimmed.contains("def __")
            {
                let is_async = trimmed.starts_with("async def");
                let after_def = if is_async {
                    &trimmed[10..]
                } else {
                    &trimmed[4..]
                };

                if let Some(paren_pos) = after_def.find('(') {
                    let name = after_def[..paren_pos].trim().to_string();
                    public_interfaces.push(InterfaceInfo {
                        name,
                        interface_type: InterfaceType::Function,
                        is_async,
                        uses: Vec::new(),
                    });
                }
            }

            // Look for class definitions
            if trimmed.starts_with("class ") && !trimmed.contains("class _") {
                if let Some(class_pos) = trimmed.find("class ") {
                    let after_class = &trimmed[class_pos + 6..];
                    let name = after_class
                        .split(&['(', ':'][..])
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !name.is_empty() {
                        public_interfaces.push(InterfaceInfo {
                            name,
                            interface_type: InterfaceType::Class,
                            is_async: false,
                            uses: Vec::new(),
                        });
                    }
                }
            }

            // Extract imports as dependencies
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                let dep = if trimmed.starts_with("import ") {
                    trimmed[7..]
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string()
                } else {
                    trimmed[5..]
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string()
                };
                if !dep.is_empty() && !dependencies.contains(&dep) {
                    dependencies.push(dep);
                }
            }
        }

        Ok(ModuleInfo {
            name: module_name,
            public_interfaces,
            dependencies,
            source_path: module_path.to_path_buf(),
        })
    }

    /// Generate integration test cases for module interactions
    fn generate_integration_test_cases(
        &self,
        module: &ModuleInfo,
        language: Language,
    ) -> Result<Vec<TestCase>> {
        let mut test_cases = Vec::new();

        // Generate tests for each public interface that has dependencies
        for interface in &module.public_interfaces {
            if interface.interface_type == InterfaceType::Function {
                test_cases.push(self.generate_integration_test_for_interface(
                    interface,
                    module,
                    language,
                )?);
            }
        }

        // If module has dependencies, generate cross-module interaction tests
        if !module.dependencies.is_empty() {
            test_cases.push(self.generate_cross_module_test(module, language)?);
        }

        Ok(test_cases)
    }

    /// Generate integration test for a specific interface
    fn generate_integration_test_for_interface(
        &self,
        interface: &InterfaceInfo,
        module: &ModuleInfo,
        language: Language,
    ) -> Result<TestCase> {
        let test_name = format!("test_{}_integration", interface.name);
        let code = match language {
            Language::Rust => self.generate_rust_integration_test(interface, module),
            Language::TypeScript => self.generate_typescript_integration_test(interface, module),
            Language::Python => self.generate_python_integration_test(interface, module),
        };

        Ok(TestCase {
            name: test_name,
            category: TestCategory::HappyPath,
            code,
        })
    }

    /// Generate Rust integration test code
    fn generate_rust_integration_test(
        &self,
        interface: &InterfaceInfo,
        module: &ModuleInfo,
    ) -> String {
        let async_marker = if interface.is_async {
            "#[tokio::test]\nasync "
        } else {
            "#[test]\n"
        };
        let await_suffix = if interface.is_async { ".await" } else { "" };

        format!(
            r#"{}fn test_{}_integration() {{
    // Arrange - Set up test fixtures and dependencies
    let test_data = setup_test_data();

    // Act - Call the function with cross-module dependencies
    let result = {}::{}(test_data){};

    // Assert - Verify cross-module interaction
    assert!(result.is_ok(), "Integration should succeed");

    // Cleanup
    teardown_test_data();
}}"#,
            async_marker, interface.name, module.name, interface.name, await_suffix
        )
    }

    /// Generate TypeScript integration test code
    fn generate_typescript_integration_test(
        &self,
        interface: &InterfaceInfo,
        _module: &ModuleInfo,
    ) -> String {
        let async_marker = if interface.is_async { "async " } else { "" };
        let await_prefix = if interface.is_async { "await " } else { "" };

        format!(
            r#"test('test_{}_integration', {}() => {{
  // Arrange - Set up test fixtures
  const testData = setupTestData();

  // Act - Call function with dependencies
  const result = {}{}({{}});

  // Assert - Verify integration
  expect(result).toBeDefined();

  // Cleanup
  teardownTestData();
}});"#,
            interface.name, async_marker, await_prefix, interface.name
        )
    }

    /// Generate Python integration test code
    fn generate_python_integration_test(
        &self,
        interface: &InterfaceInfo,
        _module: &ModuleInfo,
    ) -> String {
        let async_marker = if interface.is_async { "async " } else { "" };
        let await_prefix = if interface.is_async { "await " } else { "" };

        format!(
            r#"{}def test_{}_integration():
    # Arrange - Set up fixtures
    test_data = setup_test_data()

    # Act - Call function with dependencies
    result = {}{}(test_data)

    # Assert - Verify integration
    assert result is not None

    # Cleanup
    teardown_test_data()
"#,
            async_marker, interface.name, await_prefix, interface.name
        )
    }

    /// Generate cross-module interaction test
    fn generate_cross_module_test(
        &self,
        module: &ModuleInfo,
        language: Language,
    ) -> Result<TestCase> {
        let test_name = format!("test_{}_cross_module_interaction", module.name);
        let code = match language {
            Language::Rust => format!(
                r#"#[test]
fn test_{}_cross_module_interaction() {{
    // Arrange - Set up multiple module dependencies
    let module_a = setup_module_a();
    let module_b = setup_module_b();

    // Act - Test interaction between modules
    let result = {}::process(module_a, module_b);

    // Assert - Verify modules work together correctly
    assert!(result.is_ok());
}}"#,
                module.name, module.name
            ),
            Language::TypeScript => format!(
                r#"test('test_{}_cross_module_interaction', () => {{
  // Arrange
  const moduleA = setupModuleA();
  const moduleB = setupModuleB();

  // Act
  const result = process(moduleA, moduleB);

  // Assert
  expect(result).toBeDefined();
}});"#,
                module.name
            ),
            Language::Python => format!(
                r#"def test_{}_cross_module_interaction():
    # Arrange
    module_a = setup_module_a()
    module_b = setup_module_b()

    # Act
    result = process(module_a, module_b)

    # Assert
    assert result is not None
"#,
                module.name
            ),
        };

        Ok(TestCase {
            name: test_name,
            category: TestCategory::HappyPath,
            code,
        })
    }

    /// Generate test fixtures for integration tests
    fn generate_test_fixtures(
        &self,
        module: &ModuleInfo,
        language: Language,
    ) -> Result<Vec<TestFixture>> {
        let mut fixtures = Vec::new();

        // Generate main setup/teardown fixture
        let main_fixture = match language {
            Language::Rust => TestFixture {
                name: "test_data".to_string(),
                setup_code: format!(
                    r#"fn setup_test_data() -> TestData {{
    TestData {{
        // Initialize test data for {}
    }}
}}"#,
                    module.name
                ),
                teardown_code: r#"fn teardown_test_data() {
    // Clean up test data
}"#
                .to_string(),
                is_async: false,
            },
            Language::TypeScript => TestFixture {
                name: "testData".to_string(),
                setup_code: format!(
                    r#"function setupTestData() {{
  return {{
    // Initialize test data for {}
  }};
}}"#,
                    module.name
                ),
                teardown_code: r#"function teardownTestData() {
  // Clean up test data
}"#
                .to_string(),
                is_async: false,
            },
            Language::Python => TestFixture {
                name: "test_data".to_string(),
                setup_code: format!(
                    r#"def setup_test_data():
    return {{
        # Initialize test data for {}
    }}"#,
                    module.name
                ),
                teardown_code: r#"def teardown_test_data():
    # Clean up test data
    pass
"#
                .to_string(),
                is_async: false,
            },
        };

        fixtures.push(main_fixture);

        // If module has database dependencies, add database fixture
        if module.dependencies.iter().any(|d| {
            d.contains("database")
                || d.contains("db")
                || d.contains("sql")
                || d.contains("Database")
        }) {
            fixtures.push(self.generate_database_fixture(language)?);
        }

        Ok(fixtures)
    }

    /// Generate database fixture
    fn generate_database_fixture(&self, language: Language) -> Result<TestFixture> {
        match language {
            Language::Rust => Ok(TestFixture {
                name: "test_database".to_string(),
                setup_code: r#"async fn setup_test_database() -> Database {
    let db = Database::connect(":memory:").await.unwrap();
    db.migrate().await.unwrap();
    db
}"#
                .to_string(),
                teardown_code: r#"async fn teardown_test_database(db: Database) {
    db.close().await.unwrap();
}"#
                .to_string(),
                is_async: true,
            }),
            Language::TypeScript => Ok(TestFixture {
                name: "testDatabase".to_string(),
                setup_code: r#"async function setupTestDatabase() {
  const db = await Database.connect(':memory:');
  await db.migrate();
  return db;
}"#
                .to_string(),
                teardown_code: r#"async function teardownTestDatabase(db) {
  await db.close();
}"#
                .to_string(),
                is_async: true,
            }),
            Language::Python => Ok(TestFixture {
                name: "test_database".to_string(),
                setup_code: r#"async def setup_test_database():
    db = await Database.connect(':memory:')
    await db.migrate()
    return db
"#
                .to_string(),
                teardown_code: r#"async def teardown_test_database(db):
    await db.close()
"#
                .to_string(),
                is_async: true,
            }),
        }
    }

    /// Determine integration test file location
    fn determine_integration_test_location(
        &self,
        module_path: &Path,
        language: Language,
    ) -> Result<PathBuf> {
        let module_name = module_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::Other("Invalid module name".to_string()))?;

        match language {
            Language::Rust => {
                // For Rust, integration tests go in tests/ directory
                let project_root = module_path
                    .parent()
                    .and_then(|p| p.parent())
                    .ok_or_else(|| Error::Other("No project root found".to_string()))?;
                Ok(project_root
                    .join("tests")
                    .join(format!("{}_integration_test.rs", module_name)))
            }
            Language::TypeScript => {
                // For TypeScript, put in __tests__/integration/
                let parent = module_path
                    .parent()
                    .ok_or_else(|| Error::Other("No parent directory".to_string()))?;
                Ok(parent
                    .join("__tests__")
                    .join("integration")
                    .join(format!("{}.integration.test.ts", module_name)))
            }
            Language::Python => {
                // For Python, put in tests/integration/
                let parent = module_path
                    .parent()
                    .ok_or_else(|| Error::Other("No parent directory".to_string()))?;
                Ok(parent
                    .join("tests")
                    .join("integration")
                    .join(format!("test_{}_integration.py", module_name)))
            }
        }
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

    /// Generate E2E tests from a story
    pub async fn generate_e2e_tests_from_story(
        &self,
        story: &crate::Story,
        platform: Option<E2ETestPlatform>,
    ) -> Result<E2ETestResult> {
        // Parse acceptance criteria from the story
        let acceptance_criteria = self.parse_acceptance_criteria(story)?;

        // Detect test platform from story content or use provided platform
        let detected_platform = platform.unwrap_or_else(|| {
            self.detect_e2e_platform(&story.title, &story.description.as_deref().unwrap_or(""))
        });

        // Generate test cases for each acceptance criterion
        let test_cases = self.generate_e2e_test_cases(
            &story.title,
            &acceptance_criteria,
            detected_platform,
        )?;

        // Generate test fixtures
        let fixtures = self.generate_e2e_fixtures(detected_platform)?;

        // Determine test file location
        let test_file_path = self.determine_e2e_test_location(&story.id, detected_platform)?;

        Ok(E2ETestResult {
            story_id: story.id.clone(),
            story_title: story.title.clone(),
            acceptance_criteria,
            test_cases,
            platform: detected_platform,
            test_file_path,
            fixtures,
        })
    }

    /// Parse acceptance criteria from story
    fn parse_acceptance_criteria(&self, story: &crate::Story) -> Result<Vec<AcceptanceCriterion>> {
        let mut criteria = Vec::new();

        // If acceptance_criteria is in JSON format
        if let Some(ac_value) = &story.acceptance_criteria {
            if let Some(array) = ac_value.as_array() {
                for item in array {
                    if let Some(obj) = item.as_object() {
                        let description = obj
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let checked = obj
                            .get("checked")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        criteria.push(AcceptanceCriterion {
                            description,
                            checked,
                        });
                    }
                }
            }
        }

        // Also parse from description if in markdown format
        if let Some(desc) = &story.description {
            for line in desc.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("- [ ]") || trimmed.starts_with("- [x]") {
                    let checked = trimmed.starts_with("- [x]");
                    let description = if checked {
                        trimmed[5..].trim().to_string()
                    } else {
                        trimmed[5..].trim().to_string()
                    };
                    if !description.is_empty() {
                        criteria.push(AcceptanceCriterion {
                            description,
                            checked,
                        });
                    }
                }
            }
        }

        if criteria.is_empty() {
            // Default criterion from title
            criteria.push(AcceptanceCriterion {
                description: story.title.clone(),
                checked: false,
            });
        }

        Ok(criteria)
    }

    /// Detect E2E test platform from story content
    fn detect_e2e_platform(&self, title: &str, description: &str) -> E2ETestPlatform {
        let content = format!("{} {}", title, description).to_lowercase();

        if content.contains("web ui")
            || content.contains("webpage")
            || content.contains("browser")
            || content.contains("click")
            || content.contains("button")
            || content.contains("login")
        {
            E2ETestPlatform::Playwright
        } else if content.contains("api")
            || content.contains("endpoint")
            || content.contains("rest")
            || content.contains("http")
        {
            E2ETestPlatform::Api
        } else if content.contains("cli")
            || content.contains("command")
            || content.contains("orchestrate")
        {
            E2ETestPlatform::Cli
        } else {
            // Default to Playwright for web UI
            E2ETestPlatform::Playwright
        }
    }

    /// Generate E2E test cases from acceptance criteria
    fn generate_e2e_test_cases(
        &self,
        story_title: &str,
        criteria: &[AcceptanceCriterion],
        platform: E2ETestPlatform,
    ) -> Result<Vec<TestCase>> {
        let mut test_cases = Vec::new();

        for criterion in criteria {
            let test_name = self.generate_test_name_from_criterion(&criterion.description);
            let code = match platform {
                E2ETestPlatform::Playwright => {
                    self.generate_playwright_test(story_title, &criterion.description, &test_name)
                }
                E2ETestPlatform::Cypress => {
                    self.generate_cypress_test(story_title, &criterion.description, &test_name)
                }
                E2ETestPlatform::Api => {
                    self.generate_api_test(story_title, &criterion.description, &test_name)
                }
                E2ETestPlatform::Cli => {
                    self.generate_cli_test(story_title, &criterion.description, &test_name)
                }
            };

            test_cases.push(TestCase {
                name: test_name,
                category: TestCategory::HappyPath,
                code,
            });
        }

        Ok(test_cases)
    }

    /// Generate test name from acceptance criterion
    fn generate_test_name_from_criterion(&self, criterion: &str) -> String {
        // Convert criterion to snake_case test name
        let cleaned = criterion
            .to_lowercase()
            .replace("can ", "")
            .replace("should ", "")
            .replace("must ", "")
            .replace("will ", "");

        let words: Vec<&str> = cleaned
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();

        format!("test_{}", words.join("_"))
    }

    /// Generate Playwright test code
    fn generate_playwright_test(
        &self,
        _story_title: &str,
        criterion: &str,
        test_name: &str,
    ) -> String {
        format!(
            r#"test('{}', async ({{ page }}) => {{
  // Arrange
  await page.goto('/');

  // Act
  // TODO: Implement test steps for: {}

  // Assert
  // TODO: Add assertions
}});"#,
            test_name, criterion
        )
    }

    /// Generate Cypress test code
    fn generate_cypress_test(
        &self,
        _story_title: &str,
        criterion: &str,
        test_name: &str,
    ) -> String {
        format!(
            r#"it('{}', () => {{
  // Arrange
  cy.visit('/');

  // Act
  // TODO: Implement test steps for: {}

  // Assert
  // TODO: Add assertions
}});"#,
            test_name, criterion
        )
    }

    /// Generate API test code
    fn generate_api_test(&self, _story_title: &str, criterion: &str, test_name: &str) -> String {
        format!(
            r#"test('{}', async () => {{
  // Arrange
  const testData = {{}};

  // Act
  const response = await fetch('/api/endpoint', {{
    method: 'POST',
    headers: {{ 'Content-Type': 'application/json' }},
    body: JSON.stringify(testData),
  }});

  // Assert
  expect(response.status).toBe(200);
  // TODO: Implement test for: {}
}});"#,
            test_name, criterion
        )
    }

    /// Generate CLI test code
    fn generate_cli_test(&self, _story_title: &str, criterion: &str, test_name: &str) -> String {
        format!(
            r#"#[test]
fn {}() {{
    // Arrange
    let output = std::process::Command::new("orchestrate")
        .args(&["--help"])
        .output()
        .expect("Failed to execute command");

    // Act
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Assert
    assert!(output.status.success());
    // TODO: Implement test for: {}
}}"#,
            test_name, criterion
        )
    }

    /// Generate E2E test fixtures
    fn generate_e2e_fixtures(&self, platform: E2ETestPlatform) -> Result<Vec<TestFixture>> {
        let mut fixtures = Vec::new();

        match platform {
            E2ETestPlatform::Playwright | E2ETestPlatform::Cypress => {
                fixtures.push(TestFixture {
                    name: "browser_setup".to_string(),
                    setup_code: r#"async function setupBrowser() {
  // Setup browser context, authentication, etc.
}"#
                    .to_string(),
                    teardown_code: r#"async function teardownBrowser() {
  // Cleanup browser state
}"#
                    .to_string(),
                    is_async: true,
                });
            }
            E2ETestPlatform::Api => {
                fixtures.push(TestFixture {
                    name: "api_setup".to_string(),
                    setup_code: r#"async function setupApiTest() {
  // Setup test database, auth tokens, etc.
}"#
                    .to_string(),
                    teardown_code: r#"async function teardownApiTest() {
  // Cleanup test data
}"#
                    .to_string(),
                    is_async: true,
                });
            }
            E2ETestPlatform::Cli => {
                fixtures.push(TestFixture {
                    name: "cli_setup".to_string(),
                    setup_code: r#"fn setup_cli_test() {
    // Setup test environment, temp directories, etc.
}"#
                    .to_string(),
                    teardown_code: r#"fn teardown_cli_test() {
    // Cleanup test artifacts
}"#
                    .to_string(),
                    is_async: false,
                });
            }
        }

        Ok(fixtures)
    }

    /// Determine E2E test file location
    fn determine_e2e_test_location(
        &self,
        story_id: &str,
        platform: E2ETestPlatform,
    ) -> Result<PathBuf> {
        let sanitized_id = story_id
            .replace('.', "_")
            .replace('-', "_")
            .to_lowercase();

        match platform {
            E2ETestPlatform::Playwright => Ok(PathBuf::from(format!(
                "tests/e2e/playwright/{}.spec.ts",
                sanitized_id
            ))),
            E2ETestPlatform::Cypress => Ok(PathBuf::from(format!(
                "cypress/e2e/{}.cy.ts",
                sanitized_id
            ))),
            E2ETestPlatform::Api => Ok(PathBuf::from(format!(
                "tests/e2e/api/{}.test.ts",
                sanitized_id
            ))),
            E2ETestPlatform::Cli => Ok(PathBuf::from(format!(
                "tests/e2e/cli/{}_test.rs",
                sanitized_id
            ))),
        }
    }

    /// Format E2E test output
    pub fn format_e2e_test_output(&self, result: &E2ETestResult) -> Result<String> {
        match result.platform {
            E2ETestPlatform::Playwright => self.format_playwright_test_file(result),
            E2ETestPlatform::Cypress => self.format_cypress_test_file(result),
            E2ETestPlatform::Api => self.format_api_test_file(result),
            E2ETestPlatform::Cli => self.format_cli_test_file(result),
        }
    }

    /// Format Playwright test file
    fn format_playwright_test_file(&self, result: &E2ETestResult) -> Result<String> {
        let mut output = format!(
            r#"import {{ test, expect }} from '@playwright/test';

// Generated from Story: {}
// Story ID: {}

"#,
            result.story_title, result.story_id
        );

        // Add fixtures if any
        for fixture in &result.fixtures {
            output.push_str(&fixture.setup_code);
            output.push_str("\n\n");
        }

        // Add test cases
        for test_case in &result.test_cases {
            output.push_str(&test_case.code);
            output.push_str("\n\n");
        }

        Ok(output)
    }

    /// Format Cypress test file
    fn format_cypress_test_file(&self, result: &E2ETestResult) -> Result<String> {
        let mut output = format!(
            r#"// Generated from Story: {}
// Story ID: {}

describe('{}', () => {{
"#,
            result.story_title, result.story_id, result.story_title
        );

        // Add test cases
        for test_case in &result.test_cases {
            output.push_str("  ");
            output.push_str(&test_case.code.replace('\n', "\n  "));
            output.push_str("\n\n");
        }

        output.push_str("});\n");

        Ok(output)
    }

    /// Format API test file
    fn format_api_test_file(&self, result: &E2ETestResult) -> Result<String> {
        let mut output = format!(
            r#"// Generated from Story: {}
// Story ID: {}

import {{ describe, test, expect }} from 'vitest';

"#,
            result.story_title, result.story_id
        );

        // Add fixtures
        for fixture in &result.fixtures {
            output.push_str(&fixture.setup_code);
            output.push_str("\n\n");
        }

        // Add test cases
        for test_case in &result.test_cases {
            output.push_str(&test_case.code);
            output.push_str("\n\n");
        }

        Ok(output)
    }

    /// Format CLI test file
    fn format_cli_test_file(&self, result: &E2ETestResult) -> Result<String> {
        let mut output = format!(
            r#"// Generated from Story: {}
// Story ID: {}

#[cfg(test)]
mod tests {{
    use super::*;

"#,
            result.story_title, result.story_id
        );

        // Add test cases
        for test_case in &result.test_cases {
            output.push_str("    ");
            output.push_str(&test_case.code.replace('\n', "\n    "));
            output.push_str("\n\n");
        }

        output.push_str("}\n");

        Ok(output)
    }

    /// Generate property-based tests for a function or file
    pub async fn generate_property_tests(
        &self,
        file_path: &Path,
        target_function: Option<&str>,
    ) -> Result<PropertyTestResult> {
        // Detect language
        let language = Language::from_path(file_path)?;

        // Read source code
        let source_code = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| Error::Other(format!("Failed to read file: {}", e)))?;

        // Extract function signatures
        let functions = self.extract_functions(&source_code, language)?;

        // Filter to target function if specified
        let functions_to_test = if let Some(target) = target_function {
            functions
                .iter()
                .filter(|f| f.name == target)
                .cloned()
                .collect()
        } else {
            functions.clone()
        };

        // Identify suitable functions and generate property tests
        let property_tests = self.identify_and_generate_property_tests(
            &functions_to_test,
            &functions,
            language,
        )?;

        // Determine test file location
        let test_file_path = self.determine_property_test_location(file_path, language)?;

        Ok(PropertyTestResult {
            source_file: file_path.to_path_buf(),
            language,
            functions: functions_to_test,
            property_tests,
            test_file_path,
        })
    }

    /// Identify suitable functions for property testing and generate tests
    fn identify_and_generate_property_tests(
        &self,
        target_functions: &[FunctionSignature],
        all_functions: &[FunctionSignature],
        language: Language,
    ) -> Result<Vec<PropertyTestCase>> {
        let mut property_tests = Vec::new();

        for function in target_functions {
            // Check for roundtrip property (serialize/deserialize, encode/decode, parse/format)
            if let Some(related) = self.find_roundtrip_pair(&function.name, all_functions) {
                property_tests.push(self.generate_roundtrip_test(
                    function,
                    &related,
                    language,
                )?);
            }

            // Check for idempotency (functions that should produce same result when applied twice)
            if self.is_idempotent_candidate(function) {
                property_tests.push(self.generate_idempotency_test(function, language)?);
            }

            // Check for commutativity (binary operations where order doesn't matter)
            if self.is_commutative_candidate(function) {
                property_tests.push(self.generate_commutativity_test(function, language)?);
            }

            // Check for inverse property
            if let Some(inverse) = self.find_inverse_function(&function.name, all_functions) {
                property_tests.push(self.generate_inverse_test(
                    function,
                    &inverse,
                    language,
                )?);
            }
        }

        Ok(property_tests)
    }

    /// Find roundtrip pair (e.g., serialize/deserialize, encode/decode)
    fn find_roundtrip_pair(
        &self,
        function_name: &str,
        all_functions: &[FunctionSignature],
    ) -> Option<FunctionSignature> {
        let lower_name = function_name.to_lowercase();

        // Common roundtrip patterns
        let patterns = [
            ("serialize", "deserialize"),
            ("encode", "decode"),
            ("parse", "format"),
            ("parse", "serialize"),
            ("to_string", "from_string"),
            ("to_json", "from_json"),
            ("compress", "decompress"),
            ("encrypt", "decrypt"),
        ];

        for (first, second) in patterns {
            if lower_name.contains(first) {
                let complement = lower_name.replace(first, second);
                if let Some(pair) = all_functions.iter().find(|f| f.name.to_lowercase() == complement) {
                    return Some(pair.clone());
                }
            } else if lower_name.contains(second) {
                let complement = lower_name.replace(second, first);
                if let Some(pair) = all_functions.iter().find(|f| f.name.to_lowercase() == complement) {
                    return Some(pair.clone());
                }
            }
        }

        None
    }

    /// Check if function is a candidate for idempotency testing
    fn is_idempotent_candidate(&self, function: &FunctionSignature) -> bool {
        let lower_name = function.name.to_lowercase();

        // Functions that typically should be idempotent
        let idempotent_keywords = [
            "normalize",
            "sanitize",
            "clean",
            "trim",
            "sort",
            "unique",
            "dedupe",
            "format",
        ];

        idempotent_keywords.iter().any(|keyword| lower_name.contains(keyword))
            && function.return_type.is_some()
    }

    /// Check if function is a candidate for commutativity testing
    fn is_commutative_candidate(&self, function: &FunctionSignature) -> bool {
        // Binary operations with same-type parameters
        if function.parameters.len() != 2 {
            return false;
        }

        let lower_name = function.name.to_lowercase();

        // Common commutative operations
        let commutative_ops = ["add", "multiply", "max", "min", "gcd", "lcm", "union", "intersection"];

        commutative_ops.iter().any(|op| lower_name.contains(op))
    }

    /// Find inverse function (e.g., add/subtract, increment/decrement)
    fn find_inverse_function(
        &self,
        function_name: &str,
        all_functions: &[FunctionSignature],
    ) -> Option<FunctionSignature> {
        let lower_name = function_name.to_lowercase();

        let inverse_pairs = [
            ("add", "subtract"),
            ("increment", "decrement"),
            ("push", "pop"),
            ("insert", "remove"),
        ];

        for (first, second) in inverse_pairs {
            if lower_name.contains(first) {
                let inverse_name = lower_name.replace(first, second);
                if let Some(inverse) = all_functions.iter().find(|f| f.name.to_lowercase() == inverse_name) {
                    return Some(inverse.clone());
                }
            }
        }

        None
    }

    /// Generate roundtrip property test
    fn generate_roundtrip_test(
        &self,
        function: &FunctionSignature,
        related: &FunctionSignature,
        language: Language,
    ) -> Result<PropertyTestCase> {
        let test_name = format!("{}_roundtrip", function.name);

        let code = match language {
            Language::Rust => self.generate_rust_roundtrip_test(function, related),
            Language::TypeScript => self.generate_typescript_roundtrip_test(function, related),
            Language::Python => self.generate_python_roundtrip_test(function, related),
        };

        Ok(PropertyTestCase {
            property: PropertyType::Roundtrip,
            function_name: function.name.clone(),
            related_function: Some(related.name.clone()),
            code,
            name: test_name,
        })
    }

    /// Generate Rust roundtrip property test
    fn generate_rust_roundtrip_test(
        &self,
        function: &FunctionSignature,
        related: &FunctionSignature,
    ) -> String {
        format!(
            r#"proptest! {{
    #[test]
    fn {}_roundtrip(input: String) {{
        let result = {}(&input);
        if let Ok(value) = result {{
            let serialized = {}(&value);
            let reparsed = {}(&serialized).unwrap();
            assert_eq!(value, reparsed);
        }}
    }}
}}"#,
            function.name, function.name, related.name, function.name
        )
    }

    /// Generate TypeScript roundtrip property test using fast-check
    fn generate_typescript_roundtrip_test(
        &self,
        function: &FunctionSignature,
        related: &FunctionSignature,
    ) -> String {
        format!(
            r#"import {{ test }} from 'vitest';
import fc from 'fast-check';

test('{}_roundtrip', () => {{
    fc.assert(
        fc.property(fc.string(), (input) => {{
            const result = {}(input);
            if (result !== null && result !== undefined) {{
                const serialized = {}(result);
                const reparsed = {}(serialized);
                expect(reparsed).toEqual(result);
            }}
        }})
    );
}});"#,
            function.name, function.name, related.name, function.name
        )
    }

    /// Generate Python roundtrip property test using hypothesis
    fn generate_python_roundtrip_test(
        &self,
        function: &FunctionSignature,
        related: &FunctionSignature,
    ) -> String {
        format!(
            r#"from hypothesis import given
from hypothesis import strategies as st

@given(st.text())
def test_{}_roundtrip(input_str):
    result = {}(input_str)
    if result is not None:
        serialized = {}(result)
        reparsed = {}(serialized)
        assert reparsed == result"#,
            function.name, function.name, related.name, function.name
        )
    }

    /// Generate idempotency property test
    fn generate_idempotency_test(
        &self,
        function: &FunctionSignature,
        language: Language,
    ) -> Result<PropertyTestCase> {
        let test_name = format!("{}_idempotent", function.name);

        let code = match language {
            Language::Rust => self.generate_rust_idempotency_test(function),
            Language::TypeScript => self.generate_typescript_idempotency_test(function),
            Language::Python => self.generate_python_idempotency_test(function),
        };

        Ok(PropertyTestCase {
            property: PropertyType::Idempotency,
            function_name: function.name.clone(),
            related_function: None,
            code,
            name: test_name,
        })
    }

    /// Generate Rust idempotency test
    fn generate_rust_idempotency_test(&self, function: &FunctionSignature) -> String {
        format!(
            r#"proptest! {{
    #[test]
    fn {}_idempotent(input: String) {{
        let once = {}(&input);
        let twice = {}(&once);
        assert_eq!(once, twice);
    }}
}}"#,
            function.name, function.name, function.name
        )
    }

    /// Generate TypeScript idempotency test
    fn generate_typescript_idempotency_test(&self, function: &FunctionSignature) -> String {
        format!(
            r#"import {{ test }} from 'vitest';
import fc from 'fast-check';

test('{}_idempotent', () => {{
    fc.assert(
        fc.property(fc.string(), (input) => {{
            const once = {}(input);
            const twice = {}(once);
            expect(twice).toEqual(once);
        }})
    );
}});"#,
            function.name, function.name, function.name
        )
    }

    /// Generate Python idempotency test
    fn generate_python_idempotency_test(&self, function: &FunctionSignature) -> String {
        format!(
            r#"from hypothesis import given
from hypothesis import strategies as st

@given(st.text())
def test_{}_idempotent(input_str):
    once = {}(input_str)
    twice = {}(once)
    assert twice == once"#,
            function.name, function.name, function.name
        )
    }

    /// Generate commutativity property test
    fn generate_commutativity_test(
        &self,
        function: &FunctionSignature,
        language: Language,
    ) -> Result<PropertyTestCase> {
        let test_name = format!("{}_commutative", function.name);

        let code = match language {
            Language::Rust => self.generate_rust_commutativity_test(function),
            Language::TypeScript => self.generate_typescript_commutativity_test(function),
            Language::Python => self.generate_python_commutativity_test(function),
        };

        Ok(PropertyTestCase {
            property: PropertyType::Commutativity,
            function_name: function.name.clone(),
            related_function: None,
            code,
            name: test_name,
        })
    }

    /// Generate Rust commutativity test
    fn generate_rust_commutativity_test(&self, function: &FunctionSignature) -> String {
        format!(
            r#"proptest! {{
    #[test]
    fn {}_commutative(a: i32, b: i32) {{
        let result1 = {}(a, b);
        let result2 = {}(b, a);
        assert_eq!(result1, result2);
    }}
}}"#,
            function.name, function.name, function.name
        )
    }

    /// Generate TypeScript commutativity test
    fn generate_typescript_commutativity_test(&self, function: &FunctionSignature) -> String {
        format!(
            r#"import {{ test }} from 'vitest';
import fc from 'fast-check';

test('{}_commutative', () => {{
    fc.assert(
        fc.property(fc.integer(), fc.integer(), (a, b) => {{
            const result1 = {}(a, b);
            const result2 = {}(b, a);
            expect(result2).toEqual(result1);
        }})
    );
}});"#,
            function.name, function.name, function.name
        )
    }

    /// Generate Python commutativity test
    fn generate_python_commutativity_test(&self, function: &FunctionSignature) -> String {
        format!(
            r#"from hypothesis import given
from hypothesis import strategies as st

@given(st.integers(), st.integers())
def test_{}_commutative(a, b):
    result1 = {}(a, b)
    result2 = {}(b, a)
    assert result2 == result1"#,
            function.name, function.name, function.name
        )
    }

    /// Generate inverse property test
    fn generate_inverse_test(
        &self,
        function: &FunctionSignature,
        inverse: &FunctionSignature,
        language: Language,
    ) -> Result<PropertyTestCase> {
        let test_name = format!("{}_{}_inverse", function.name, inverse.name);

        let code = match language {
            Language::Rust => self.generate_rust_inverse_test(function, inverse),
            Language::TypeScript => self.generate_typescript_inverse_test(function, inverse),
            Language::Python => self.generate_python_inverse_test(function, inverse),
        };

        Ok(PropertyTestCase {
            property: PropertyType::Inverse,
            function_name: function.name.clone(),
            related_function: Some(inverse.name.clone()),
            code,
            name: test_name,
        })
    }

    /// Generate Rust inverse test
    fn generate_rust_inverse_test(
        &self,
        function: &FunctionSignature,
        inverse: &FunctionSignature,
    ) -> String {
        format!(
            r#"proptest! {{
    #[test]
    fn {}_{}_inverse(x: i32, y: i32) {{
        let result = {}(x, y);
        let back = {}(result, y);
        assert_eq!(back, x);
    }}
}}"#,
            function.name, inverse.name, function.name, inverse.name
        )
    }

    /// Generate TypeScript inverse test
    fn generate_typescript_inverse_test(
        &self,
        function: &FunctionSignature,
        inverse: &FunctionSignature,
    ) -> String {
        format!(
            r#"import {{ test }} from 'vitest';
import fc from 'fast-check';

test('{}_{}_inverse', () => {{
    fc.assert(
        fc.property(fc.integer(), fc.integer(), (x, y) => {{
            const result = {}(x, y);
            const back = {}(result, y);
            expect(back).toEqual(x);
        }})
    );
}});"#,
            function.name, inverse.name, function.name, inverse.name
        )
    }

    /// Generate Python inverse test
    fn generate_python_inverse_test(
        &self,
        function: &FunctionSignature,
        inverse: &FunctionSignature,
    ) -> String {
        format!(
            r#"from hypothesis import given
from hypothesis import strategies as st

@given(st.integers(), st.integers())
def test_{}_{}_inverse(x, y):
    result = {}(x, y)
    back = {}(result, y)
    assert back == x"#,
            function.name, inverse.name, function.name, inverse.name
        )
    }

    /// Determine property test file location
    fn determine_property_test_location(
        &self,
        source_file: &Path,
        language: Language,
    ) -> Result<PathBuf> {
        match language {
            Language::Rust => {
                // Place alongside source with _proptest suffix
                let mut path = source_file.to_path_buf();
                let stem = path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| Error::Other("Invalid file name".to_string()))?;
                path.set_file_name(format!("{}_proptest.rs", stem));
                Ok(path)
            }
            Language::TypeScript => {
                // Place in __tests__ directory
                let file_name = source_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| Error::Other("Invalid file name".to_string()))?;
                Ok(PathBuf::from(format!("__tests__/{}.property.test.ts", file_name)))
            }
            Language::Python => {
                // Place alongside with test_ prefix
                let file_name = source_file
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| Error::Other("Invalid file name".to_string()))?;
                let parent = source_file.parent().unwrap_or_else(|| Path::new("."));
                Ok(parent.join(format!("test_property_{}", file_name)))
            }
        }
    }

    /// Format property test output
    pub fn format_property_test_output(&self, result: &PropertyTestResult) -> Result<String> {
        match result.language {
            Language::Rust => self.format_rust_property_tests(result),
            Language::TypeScript => self.format_typescript_property_tests(result),
            Language::Python => self.format_python_property_tests(result),
        }
    }

    /// Format Rust property tests
    fn format_rust_property_tests(&self, result: &PropertyTestResult) -> Result<String> {
        let mut output = String::from(
            r#"//! Property-based tests
//! Generated by orchestrate test generator

use proptest::prelude::*;

"#,
        );

        for test in &result.property_tests {
            output.push_str(&test.code);
            output.push_str("\n\n");
        }

        Ok(output)
    }

    /// Format TypeScript property tests
    fn format_typescript_property_tests(&self, result: &PropertyTestResult) -> Result<String> {
        let mut output = String::from(
            r#"// Property-based tests
// Generated by orchestrate test generator

"#,
        );

        for test in &result.property_tests {
            output.push_str(&test.code);
            output.push_str("\n\n");
        }

        Ok(output)
    }

    /// Format Python property tests
    fn format_python_property_tests(&self, result: &PropertyTestResult) -> Result<String> {
        let mut output = String::from(
            r#"""Property-based tests
Generated by orchestrate test generator
"""

"#,
        );

        for test in &result.property_tests {
            output.push_str(&test.code);
            output.push_str("\n\n");
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

    // Integration test generation tests

    #[tokio::test]
    async fn test_analyze_rust_module_identifies_public_functions() {
        let service = TestGenerationService::new();
        let source = r#"
use std::collections::HashMap;
use crate::database::Database;

pub fn process_data(input: String) -> Result<String> {
    Ok(input)
}

pub async fn save_to_database(data: &str) -> Result<()> {
    // implementation
    Ok(())
}

pub struct DataProcessor {
    cache: HashMap<String, String>,
}

pub trait Processor {
    fn process(&self) -> Result<()>;
}
"#;
        let module_path = Path::new("/project/src/processor.rs");
        let module_info = service.analyze_rust_module(source, module_path).unwrap();

        assert_eq!(module_info.name, "processor");
        assert_eq!(module_info.public_interfaces.len(), 4);

        // Check for public functions
        let function_names: Vec<_> = module_info
            .public_interfaces
            .iter()
            .filter(|i| i.interface_type == InterfaceType::Function)
            .map(|i| i.name.as_str())
            .collect();
        assert!(function_names.contains(&"process_data"));
        assert!(function_names.contains(&"save_to_database"));

        // Check for async function
        let async_fn = module_info
            .public_interfaces
            .iter()
            .find(|i| i.name == "save_to_database")
            .unwrap();
        assert!(async_fn.is_async);

        // Check dependencies
        assert!(module_info.dependencies.iter().any(|d| d.contains("Database")));
    }

    #[tokio::test]
    async fn test_analyze_typescript_module_identifies_exports() {
        let service = TestGenerationService::new();
        let source = r#"
import { Database } from './database';
import { Logger } from './logger';

export async function processData(input: string): Promise<string> {
    return input;
}

export class DataProcessor {
    private cache: Map<string, string>;

    process(data: string): string {
        return data;
    }
}
"#;
        let module_path = Path::new("/project/src/processor.ts");
        let module_info = service
            .analyze_typescript_module(source, module_path)
            .unwrap();

        assert_eq!(module_info.name, "processor");
        assert_eq!(module_info.public_interfaces.len(), 2);

        // Check for exported function
        let function = module_info
            .public_interfaces
            .iter()
            .find(|i| i.interface_type == InterfaceType::Function)
            .unwrap();
        assert_eq!(function.name, "processData");
        assert!(function.is_async);

        // Check for exported class
        let class = module_info
            .public_interfaces
            .iter()
            .find(|i| i.interface_type == InterfaceType::Class)
            .unwrap();
        assert_eq!(class.name, "DataProcessor");

        // Check dependencies
        assert_eq!(module_info.dependencies.len(), 2);
        assert!(module_info.dependencies.contains(&"./database".to_string()));
    }

    #[tokio::test]
    async fn test_analyze_python_module_identifies_public_items() {
        let service = TestGenerationService::new();
        let source = r#"
import sqlite3
from typing import Optional

def process_data(input_str: str) -> str:
    return input_str

async def save_to_database(data: str) -> None:
    pass

class DataProcessor:
    def __init__(self):
        self.cache = {}

def _private_helper():
    pass
"#;
        let module_path = Path::new("/project/src/processor.py");
        let module_info = service.analyze_python_module(source, module_path).unwrap();

        assert_eq!(module_info.name, "processor");
        assert_eq!(module_info.public_interfaces.len(), 3); // 2 functions + 1 class

        // Check that private function is not included
        let names: Vec<_> = module_info
            .public_interfaces
            .iter()
            .map(|i| i.name.as_str())
            .collect();
        assert!(!names.contains(&"_private_helper"));
        assert!(names.contains(&"process_data"));
        assert!(names.contains(&"save_to_database"));

        // Check dependencies
        assert!(module_info.dependencies.contains(&"sqlite3".to_string()));
    }

    #[tokio::test]
    async fn test_generate_integration_tests_for_rust_module() {
        let service = TestGenerationService::new();

        // Create a temporary module file
        let temp_dir = tempfile::tempdir().unwrap();
        let module_file = temp_dir.path().join("processor.rs");

        tokio::fs::write(
            &module_file,
            r#"
use crate::database::Database;

pub async fn save_data(data: String) -> Result<()> {
    let db = Database::new();
    db.save(&data).await
}

pub fn process(input: &str) -> String {
    input.to_uppercase()
}
"#,
        )
        .await
        .unwrap();

        let result = service
            .generate_integration_tests(&module_file)
            .await
            .unwrap();

        assert_eq!(result.module.name, "processor");
        assert!(!result.test_cases.is_empty());

        // Should generate tests for public functions
        assert!(result.test_cases.iter().any(|t| t.name.contains("save_data")));
        assert!(result.test_cases.iter().any(|t| t.name.contains("process")));

        // Should include fixtures
        assert!(!result.fixtures.is_empty());
        assert!(result.fixtures.iter().any(|f| f.name == "test_data"));

        // Should have database fixture since module depends on Database
        assert!(result.fixtures.iter().any(|f| f.name == "test_database"));

        // Check test file path
        assert!(result
            .test_file_path
            .to_string_lossy()
            .contains("processor_integration_test.rs"));
    }

    #[test]
    fn test_generate_rust_integration_test_code() {
        let service = TestGenerationService::new();
        let interface = InterfaceInfo {
            name: "save_data".to_string(),
            interface_type: InterfaceType::Function,
            is_async: true,
            uses: vec!["Database".to_string()],
        };
        let module = ModuleInfo {
            name: "storage".to_string(),
            public_interfaces: vec![interface.clone()],
            dependencies: vec!["crate::database::Database".to_string()],
            source_path: PathBuf::from("/project/src/storage.rs"),
        };

        let code = service.generate_rust_integration_test(&interface, &module);

        assert!(code.contains("#[tokio::test]"));
        assert!(code.contains("async fn test_save_data_integration()"));
        assert!(code.contains(".await"));
        assert!(code.contains("setup_test_data"));
        assert!(code.contains("teardown_test_data"));
        assert!(code.contains("storage::save_data"));
    }

    #[test]
    fn test_generate_typescript_integration_test_code() {
        let service = TestGenerationService::new();
        let interface = InterfaceInfo {
            name: "saveData".to_string(),
            interface_type: InterfaceType::Function,
            is_async: true,
            uses: vec!["Database".to_string()],
        };
        let module = ModuleInfo {
            name: "storage".to_string(),
            public_interfaces: vec![interface.clone()],
            dependencies: vec!["./database".to_string()],
            source_path: PathBuf::from("/project/src/storage.ts"),
        };

        let code = service.generate_typescript_integration_test(&interface, &module);

        assert!(code.contains("test('test_saveData_integration'"));
        assert!(code.contains("async () =>"));
        assert!(code.contains("await "));
        assert!(code.contains("setupTestData"));
        assert!(code.contains("teardownTestData"));
    }

    #[test]
    fn test_generate_test_fixtures_includes_database_fixture() {
        let service = TestGenerationService::new();
        let module = ModuleInfo {
            name: "storage".to_string(),
            public_interfaces: vec![],
            dependencies: vec!["crate::database::Database".to_string()],
            source_path: PathBuf::from("/project/src/storage.rs"),
        };

        let fixtures = service
            .generate_test_fixtures(&module, Language::Rust)
            .unwrap();

        assert!(fixtures.len() >= 2); // test_data + test_database
        assert!(fixtures.iter().any(|f| f.name == "test_data"));
        assert!(fixtures.iter().any(|f| f.name == "test_database"));

        // Database fixture should be async
        let db_fixture = fixtures.iter().find(|f| f.name == "test_database").unwrap();
        assert!(db_fixture.is_async);
        assert!(db_fixture.setup_code.contains("async fn"));
        assert!(db_fixture.setup_code.contains("Database::connect"));
    }

    #[test]
    fn test_generate_cross_module_test() {
        let service = TestGenerationService::new();
        let module = ModuleInfo {
            name: "processor".to_string(),
            public_interfaces: vec![],
            dependencies: vec!["crate::storage".to_string(), "crate::logger".to_string()],
            source_path: PathBuf::from("/project/src/processor.rs"),
        };

        let test_case = service
            .generate_cross_module_test(&module, Language::Rust)
            .unwrap();

        assert_eq!(test_case.name, "test_processor_cross_module_interaction");
        assert_eq!(test_case.category, TestCategory::HappyPath);
        assert!(test_case.code.contains("#[test]"));
        assert!(test_case
            .code
            .contains("test_processor_cross_module_interaction"));
        assert!(test_case.code.contains("setup_module"));
    }

    #[test]
    fn test_determine_integration_test_location_rust() {
        let service = TestGenerationService::new();
        let module_path = Path::new("/project/src/storage/mod.rs");
        let test_location = service
            .determine_integration_test_location(module_path, Language::Rust)
            .unwrap();

        assert!(test_location
            .to_string_lossy()
            .contains("tests/mod_integration_test.rs"));
    }

    #[test]
    fn test_determine_integration_test_location_typescript() {
        let service = TestGenerationService::new();
        let module_path = Path::new("/project/src/storage.ts");
        let test_location = service
            .determine_integration_test_location(module_path, Language::TypeScript)
            .unwrap();

        assert_eq!(
            test_location,
            Path::new("/project/src/__tests__/integration/storage.integration.test.ts")
        );
    }

    #[test]
    fn test_determine_integration_test_location_python() {
        let service = TestGenerationService::new();
        let module_path = Path::new("/project/src/storage.py");
        let test_location = service
            .determine_integration_test_location(module_path, Language::Python)
            .unwrap();

        assert_eq!(
            test_location,
            Path::new("/project/src/tests/integration/test_storage_integration.py")
        );
    }

    // E2E test generation tests

    #[tokio::test]
    async fn test_generate_e2e_tests_from_story_with_playwright() {
        use crate::Story;
        use chrono::Utc;

        let service = TestGenerationService::new();
        let story = Story {
            id: "epic-005.4".to_string(),
            epic_id: "epic-005".to_string(),
            title: "User can login with valid credentials".to_string(),
            description: Some(
                r#"
Acceptance Criteria:
- [ ] User can navigate to login page
- [ ] User can enter email and password
- [ ] User can click login button
- [ ] User is redirected to dashboard after successful login
"#
                .to_string(),
            ),
            acceptance_criteria: None,
            status: crate::StoryStatus::Pending,
            agent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };

        let result = service
            .generate_e2e_tests_from_story(&story, Some(E2ETestPlatform::Playwright))
            .await
            .unwrap();

        assert_eq!(result.story_id, "epic-005.4");
        assert_eq!(result.platform, E2ETestPlatform::Playwright);
        assert_eq!(result.acceptance_criteria.len(), 4);
        assert_eq!(result.test_cases.len(), 4);
        assert!(!result.fixtures.is_empty());
        assert!(result
            .test_file_path
            .to_string_lossy()
            .contains("playwright"));
    }

    #[tokio::test]
    async fn test_parse_acceptance_criteria_from_markdown() {
        use crate::Story;
        use chrono::Utc;

        let service = TestGenerationService::new();
        let story = Story {
            id: "test-1".to_string(),
            epic_id: "test".to_string(),
            title: "Test Story".to_string(),
            description: Some(
                r#"
Some description here.

- [ ] First criterion
- [x] Second criterion (completed)
- [ ] Third criterion
"#
                .to_string(),
            ),
            acceptance_criteria: None,
            status: crate::StoryStatus::Pending,
            agent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };

        let criteria = service.parse_acceptance_criteria(&story).unwrap();

        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0].description, "First criterion");
        assert!(!criteria[0].checked);
        assert_eq!(criteria[1].description, "Second criterion (completed)");
        assert!(criteria[1].checked);
        assert_eq!(criteria[2].description, "Third criterion");
        assert!(!criteria[2].checked);
    }

    #[tokio::test]
    async fn test_parse_acceptance_criteria_from_json() {
        use crate::Story;
        use chrono::Utc;
        use serde_json::json;

        let service = TestGenerationService::new();
        let story = Story {
            id: "test-1".to_string(),
            epic_id: "test".to_string(),
            title: "Test Story".to_string(),
            description: None,
            acceptance_criteria: Some(json!([
                {"description": "Criterion 1", "checked": false},
                {"description": "Criterion 2", "checked": true}
            ])),
            status: crate::StoryStatus::Pending,
            agent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };

        let criteria = service.parse_acceptance_criteria(&story).unwrap();

        assert_eq!(criteria.len(), 2);
        assert_eq!(criteria[0].description, "Criterion 1");
        assert!(!criteria[0].checked);
        assert_eq!(criteria[1].description, "Criterion 2");
        assert!(criteria[1].checked);
    }

    #[test]
    fn test_detect_e2e_platform_web_ui() {
        let service = TestGenerationService::new();

        let platform = service.detect_e2e_platform(
            "User can login to the web UI",
            "User clicks the login button and is redirected to dashboard",
        );
        assert_eq!(platform, E2ETestPlatform::Playwright);
    }

    #[test]
    fn test_detect_e2e_platform_api() {
        let service = TestGenerationService::new();

        let platform =
            service.detect_e2e_platform("Create user API endpoint", "POST /api/users endpoint");
        assert_eq!(platform, E2ETestPlatform::Api);
    }

    #[test]
    fn test_detect_e2e_platform_cli() {
        let service = TestGenerationService::new();

        let platform = service.detect_e2e_platform(
            "orchestrate test generate command",
            "CLI command to generate tests",
        );
        assert_eq!(platform, E2ETestPlatform::Cli);
    }

    #[test]
    fn test_generate_test_name_from_criterion() {
        let service = TestGenerationService::new();

        let name = service.generate_test_name_from_criterion("User can login with credentials");
        assert_eq!(name, "test_user_login_with_credentials");

        let name2 = service.generate_test_name_from_criterion("Should display error message");
        assert_eq!(name2, "test_display_error_message");
    }

    #[test]
    fn test_generate_playwright_test() {
        let service = TestGenerationService::new();

        let code = service.generate_playwright_test(
            "User login",
            "User can login with valid credentials",
            "test_user_can_login",
        );

        assert!(code.contains("test('test_user_can_login'"));
        assert!(code.contains("async ({ page })"));
        assert!(code.contains("await page.goto"));
        assert!(code.contains("User can login with valid credentials"));
    }

    #[test]
    fn test_generate_cypress_test() {
        let service = TestGenerationService::new();

        let code = service.generate_cypress_test(
            "User login",
            "User can login with valid credentials",
            "test_user_can_login",
        );

        assert!(code.contains("it('test_user_can_login'"));
        assert!(code.contains("cy.visit"));
        assert!(code.contains("User can login with valid credentials"));
    }

    #[test]
    fn test_generate_api_test() {
        let service = TestGenerationService::new();

        let code = service.generate_api_test(
            "Create user",
            "API creates new user",
            "test_api_creates_user",
        );

        assert!(code.contains("test('test_api_creates_user'"));
        assert!(code.contains("async ()"));
        assert!(code.contains("fetch"));
        assert!(code.contains("API creates new user"));
    }

    #[test]
    fn test_generate_cli_test() {
        let service = TestGenerationService::new();

        let code = service.generate_cli_test(
            "Test generate command",
            "orchestrate test generate works",
            "test_orchestrate_generate",
        );

        assert!(code.contains("#[test]"));
        assert!(code.contains("fn test_orchestrate_generate()"));
        assert!(code.contains("Command::new(\"orchestrate\")"));
        assert!(code.contains("orchestrate test generate works"));
    }

    #[test]
    fn test_determine_e2e_test_location_playwright() {
        let service = TestGenerationService::new();

        let path = service
            .determine_e2e_test_location("epic-005.4", E2ETestPlatform::Playwright)
            .unwrap();
        assert_eq!(
            path,
            PathBuf::from("tests/e2e/playwright/epic_005_4.spec.ts")
        );
    }

    #[test]
    fn test_determine_e2e_test_location_cypress() {
        let service = TestGenerationService::new();

        let path = service
            .determine_e2e_test_location("epic-005.4", E2ETestPlatform::Cypress)
            .unwrap();
        assert_eq!(path, PathBuf::from("cypress/e2e/epic_005_4.cy.ts"));
    }

    #[test]
    fn test_determine_e2e_test_location_api() {
        let service = TestGenerationService::new();

        let path = service
            .determine_e2e_test_location("epic-005.4", E2ETestPlatform::Api)
            .unwrap();
        assert_eq!(path, PathBuf::from("tests/e2e/api/epic_005_4.test.ts"));
    }

    #[test]
    fn test_determine_e2e_test_location_cli() {
        let service = TestGenerationService::new();

        let path = service
            .determine_e2e_test_location("epic-005.4", E2ETestPlatform::Cli)
            .unwrap();
        assert_eq!(path, PathBuf::from("tests/e2e/cli/epic_005_4_test.rs"));
    }

    #[test]
    fn test_format_playwright_test_file() {
        let service = TestGenerationService::new();

        let result = E2ETestResult {
            story_id: "epic-005.4".to_string(),
            story_title: "User login".to_string(),
            acceptance_criteria: vec![],
            test_cases: vec![TestCase {
                name: "test_login".to_string(),
                category: TestCategory::HappyPath,
                code: "test('login', async ({ page }) => {});".to_string(),
            }],
            platform: E2ETestPlatform::Playwright,
            test_file_path: PathBuf::from("test.spec.ts"),
            fixtures: vec![],
        };

        let output = service.format_playwright_test_file(&result).unwrap();

        assert!(output.contains("import { test, expect } from '@playwright/test'"));
        assert!(output.contains("// Generated from Story: User login"));
        assert!(output.contains("// Story ID: epic-005.4"));
        assert!(output.contains("test('login'"));
    }

    #[test]
    fn test_format_cypress_test_file() {
        let service = TestGenerationService::new();

        let result = E2ETestResult {
            story_id: "epic-005.4".to_string(),
            story_title: "User login".to_string(),
            acceptance_criteria: vec![],
            test_cases: vec![TestCase {
                name: "test_login".to_string(),
                category: TestCategory::HappyPath,
                code: "it('login', () => {});".to_string(),
            }],
            platform: E2ETestPlatform::Cypress,
            test_file_path: PathBuf::from("test.cy.ts"),
            fixtures: vec![],
        };

        let output = service.format_cypress_test_file(&result).unwrap();

        assert!(output.contains("describe('User login'"));
        assert!(output.contains("// Story ID: epic-005.4"));
        assert!(output.contains("it('login'"));
    }

    #[test]
    fn test_format_cli_test_file() {
        let service = TestGenerationService::new();

        let result = E2ETestResult {
            story_id: "epic-005.4".to_string(),
            story_title: "CLI test generation".to_string(),
            acceptance_criteria: vec![],
            test_cases: vec![TestCase {
                name: "test_generate".to_string(),
                category: TestCategory::HappyPath,
                code: "#[test]\nfn test_generate() {}".to_string(),
            }],
            platform: E2ETestPlatform::Cli,
            test_file_path: PathBuf::from("test.rs"),
            fixtures: vec![],
        };

        let output = service.format_cli_test_file(&result).unwrap();

        assert!(output.contains("#[cfg(test)]"));
        assert!(output.contains("mod tests {"));
        assert!(output.contains("// Story ID: epic-005.4"));
        assert!(output.contains("fn test_generate()"));
    }

    // Property-based test generation tests
    #[test]
    fn test_find_roundtrip_pair_serialize_deserialize() {
        let service = TestGenerationService::new();
        let functions = vec![
            FunctionSignature {
                name: "serialize".to_string(),
                parameters: vec![],
                return_type: Some("String".to_string()),
                is_async: false,
                line_number: 1,
            },
            FunctionSignature {
                name: "deserialize".to_string(),
                parameters: vec![],
                return_type: Some("Value".to_string()),
                is_async: false,
                line_number: 5,
            },
        ];

        let pair = service.find_roundtrip_pair("serialize", &functions);
        assert!(pair.is_some());
        assert_eq!(pair.unwrap().name, "deserialize");
    }

    #[test]
    fn test_find_roundtrip_pair_encode_decode() {
        let service = TestGenerationService::new();
        let functions = vec![
            FunctionSignature {
                name: "encode".to_string(),
                parameters: vec![],
                return_type: Some("String".to_string()),
                is_async: false,
                line_number: 1,
            },
            FunctionSignature {
                name: "decode".to_string(),
                parameters: vec![],
                return_type: Some("Value".to_string()),
                is_async: false,
                line_number: 5,
            },
        ];

        let pair = service.find_roundtrip_pair("encode", &functions);
        assert!(pair.is_some());
        assert_eq!(pair.unwrap().name, "decode");
    }

    #[test]
    fn test_find_roundtrip_pair_no_match() {
        let service = TestGenerationService::new();
        let functions = vec![
            FunctionSignature {
                name: "serialize".to_string(),
                parameters: vec![],
                return_type: Some("String".to_string()),
                is_async: false,
                line_number: 1,
            },
        ];

        let pair = service.find_roundtrip_pair("serialize", &functions);
        assert!(pair.is_none());
    }

    #[test]
    fn test_is_idempotent_candidate_normalize() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "normalize_string".to_string(),
            parameters: vec![],
            return_type: Some("String".to_string()),
            is_async: false,
            line_number: 1,
        };

        assert!(service.is_idempotent_candidate(&function));
    }

    #[test]
    fn test_is_idempotent_candidate_trim() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "trim".to_string(),
            parameters: vec![],
            return_type: Some("String".to_string()),
            is_async: false,
            line_number: 1,
        };

        assert!(service.is_idempotent_candidate(&function));
    }

    #[test]
    fn test_is_idempotent_candidate_no_return() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "normalize".to_string(),
            parameters: vec![],
            return_type: None,
            is_async: false,
            line_number: 1,
        };

        assert!(!service.is_idempotent_candidate(&function));
    }

    #[test]
    fn test_is_commutative_candidate_add() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "add".to_string(),
            parameters: vec![
                Parameter {
                    name: "a".to_string(),
                    type_hint: Some("i32".to_string()),
                },
                Parameter {
                    name: "b".to_string(),
                    type_hint: Some("i32".to_string()),
                },
            ],
            return_type: Some("i32".to_string()),
            is_async: false,
            line_number: 1,
        };

        assert!(service.is_commutative_candidate(&function));
    }

    #[test]
    fn test_is_commutative_candidate_multiply() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "multiply".to_string(),
            parameters: vec![
                Parameter {
                    name: "a".to_string(),
                    type_hint: Some("i32".to_string()),
                },
                Parameter {
                    name: "b".to_string(),
                    type_hint: Some("i32".to_string()),
                },
            ],
            return_type: Some("i32".to_string()),
            is_async: false,
            line_number: 1,
        };

        assert!(service.is_commutative_candidate(&function));
    }

    #[test]
    fn test_is_commutative_candidate_wrong_param_count() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "add".to_string(),
            parameters: vec![
                Parameter {
                    name: "a".to_string(),
                    type_hint: Some("i32".to_string()),
                },
            ],
            return_type: Some("i32".to_string()),
            is_async: false,
            line_number: 1,
        };

        assert!(!service.is_commutative_candidate(&function));
    }

    #[test]
    fn test_find_inverse_function_add_subtract() {
        let service = TestGenerationService::new();
        let functions = vec![
            FunctionSignature {
                name: "add".to_string(),
                parameters: vec![],
                return_type: Some("i32".to_string()),
                is_async: false,
                line_number: 1,
            },
            FunctionSignature {
                name: "subtract".to_string(),
                parameters: vec![],
                return_type: Some("i32".to_string()),
                is_async: false,
                line_number: 5,
            },
        ];

        let inverse = service.find_inverse_function("add", &functions);
        assert!(inverse.is_some());
        assert_eq!(inverse.unwrap().name, "subtract");
    }

    #[test]
    fn test_generate_rust_roundtrip_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "parse".to_string(),
            parameters: vec![],
            return_type: Some("Value".to_string()),
            is_async: false,
            line_number: 1,
        };
        let related = FunctionSignature {
            name: "serialize".to_string(),
            parameters: vec![],
            return_type: Some("String".to_string()),
            is_async: false,
            line_number: 5,
        };

        let code = service.generate_rust_roundtrip_test(&function, &related);
        assert!(code.contains("proptest!"));
        assert!(code.contains("fn parse_roundtrip"));
        assert!(code.contains("parse(&input)"));
        assert!(code.contains("serialize(&value)"));
        assert!(code.contains("assert_eq!(value, reparsed)"));
    }

    #[test]
    fn test_generate_typescript_roundtrip_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "parse".to_string(),
            parameters: vec![],
            return_type: Some("Value".to_string()),
            is_async: false,
            line_number: 1,
        };
        let related = FunctionSignature {
            name: "serialize".to_string(),
            parameters: vec![],
            return_type: Some("String".to_string()),
            is_async: false,
            line_number: 5,
        };

        let code = service.generate_typescript_roundtrip_test(&function, &related);
        assert!(code.contains("import fc from 'fast-check'"));
        assert!(code.contains("test('parse_roundtrip'"));
        assert!(code.contains("fc.property(fc.string()"));
        assert!(code.contains("parse(input)"));
        assert!(code.contains("serialize(result)"));
    }

    #[test]
    fn test_generate_python_roundtrip_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "parse".to_string(),
            parameters: vec![],
            return_type: Some("Value".to_string()),
            is_async: false,
            line_number: 1,
        };
        let related = FunctionSignature {
            name: "serialize".to_string(),
            parameters: vec![],
            return_type: Some("String".to_string()),
            is_async: false,
            line_number: 5,
        };

        let code = service.generate_python_roundtrip_test(&function, &related);
        assert!(code.contains("from hypothesis import given"));
        assert!(code.contains("@given(st.text())"));
        assert!(code.contains("def test_parse_roundtrip"));
        assert!(code.contains("parse(input_str)"));
        assert!(code.contains("serialize(result)"));
    }

    #[test]
    fn test_generate_rust_idempotency_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "normalize".to_string(),
            parameters: vec![],
            return_type: Some("String".to_string()),
            is_async: false,
            line_number: 1,
        };

        let code = service.generate_rust_idempotency_test(&function);
        assert!(code.contains("proptest!"));
        assert!(code.contains("fn normalize_idempotent"));
        assert!(code.contains("normalize(&input)"));
        assert!(code.contains("normalize(&once)"));
        assert!(code.contains("assert_eq!(once, twice)"));
    }

    #[test]
    fn test_generate_rust_commutativity_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "add".to_string(),
            parameters: vec![],
            return_type: Some("i32".to_string()),
            is_async: false,
            line_number: 1,
        };

        let code = service.generate_rust_commutativity_test(&function);
        assert!(code.contains("proptest!"));
        assert!(code.contains("fn add_commutative"));
        assert!(code.contains("add(a, b)"));
        assert!(code.contains("add(b, a)"));
        assert!(code.contains("assert_eq!(result1, result2)"));
    }

    #[test]
    fn test_generate_rust_inverse_test() {
        let service = TestGenerationService::new();
        let function = FunctionSignature {
            name: "add".to_string(),
            parameters: vec![],
            return_type: Some("i32".to_string()),
            is_async: false,
            line_number: 1,
        };
        let inverse = FunctionSignature {
            name: "subtract".to_string(),
            parameters: vec![],
            return_type: Some("i32".to_string()),
            is_async: false,
            line_number: 5,
        };

        let code = service.generate_rust_inverse_test(&function, &inverse);
        assert!(code.contains("proptest!"));
        assert!(code.contains("fn add_subtract_inverse"));
        assert!(code.contains("add(x, y)"));
        assert!(code.contains("subtract(result, y)"));
        assert!(code.contains("assert_eq!(back, x)"));
    }

    #[test]
    fn test_determine_property_test_location_rust() {
        let service = TestGenerationService::new();
        let source = Path::new("src/parser.rs");

        let path = service
            .determine_property_test_location(source, Language::Rust)
            .unwrap();
        assert_eq!(path, PathBuf::from("src/parser_proptest.rs"));
    }

    #[test]
    fn test_determine_property_test_location_typescript() {
        let service = TestGenerationService::new();
        let source = Path::new("src/parser.ts");

        let path = service
            .determine_property_test_location(source, Language::TypeScript)
            .unwrap();
        assert_eq!(path, PathBuf::from("__tests__/parser.property.test.ts"));
    }

    #[test]
    fn test_determine_property_test_location_python() {
        let service = TestGenerationService::new();
        let source = Path::new("src/parser.py");

        let path = service
            .determine_property_test_location(source, Language::Python)
            .unwrap();
        assert_eq!(path, PathBuf::from("src/test_property_parser.py"));
    }

    #[test]
    fn test_format_rust_property_tests() {
        let service = TestGenerationService::new();
        let result = PropertyTestResult {
            source_file: PathBuf::from("src/test.rs"),
            language: Language::Rust,
            functions: vec![],
            property_tests: vec![PropertyTestCase {
                property: PropertyType::Roundtrip,
                function_name: "parse".to_string(),
                related_function: Some("serialize".to_string()),
                code: "proptest! { }".to_string(),
                name: "parse_roundtrip".to_string(),
            }],
            test_file_path: PathBuf::from("src/test_proptest.rs"),
        };

        let output = service.format_rust_property_tests(&result).unwrap();
        assert!(output.contains("Property-based tests"));
        assert!(output.contains("use proptest::prelude::*"));
        assert!(output.contains("proptest! { }"));
    }

    #[test]
    fn test_format_typescript_property_tests() {
        let service = TestGenerationService::new();
        let result = PropertyTestResult {
            source_file: PathBuf::from("src/test.ts"),
            language: Language::TypeScript,
            functions: vec![],
            property_tests: vec![PropertyTestCase {
                property: PropertyType::Idempotency,
                function_name: "normalize".to_string(),
                related_function: None,
                code: "test('normalize', () => {})".to_string(),
                name: "normalize_idempotent".to_string(),
            }],
            test_file_path: PathBuf::from("__tests__/test.property.test.ts"),
        };

        let output = service.format_typescript_property_tests(&result).unwrap();
        assert!(output.contains("Property-based tests"));
        assert!(output.contains("test('normalize'"));
    }

    #[test]
    fn test_format_python_property_tests() {
        let service = TestGenerationService::new();
        let result = PropertyTestResult {
            source_file: PathBuf::from("src/test.py"),
            language: Language::Python,
            functions: vec![],
            property_tests: vec![PropertyTestCase {
                property: PropertyType::Commutativity,
                function_name: "add".to_string(),
                related_function: None,
                code: "def test_add(): pass".to_string(),
                name: "add_commutative".to_string(),
            }],
            test_file_path: PathBuf::from("src/test_property_test.py"),
        };

        let output = service.format_python_property_tests(&result).unwrap();
        assert!(output.contains("Property-based tests"));
        assert!(output.contains("def test_add()"));
    }

    #[tokio::test]
    async fn test_generate_property_tests_for_file() {
        use tempfile::Builder;
        use std::io::Write;

        let service = TestGenerationService::new();

        // Create a temporary Rust file with functions suitable for property testing
        let mut temp_file = Builder::new()
            .suffix(".rs")
            .tempfile()
            .unwrap();
        writeln!(
            temp_file,
            r#"
pub fn serialize(value: &Value) -> String {{
    // implementation
}}

pub fn deserialize(input: &str) -> Result<Value> {{
    // implementation
}}

pub fn normalize(input: &str) -> String {{
    // implementation
}}

pub fn add(a: i32, b: i32) -> i32 {{
    a + b
}}
"#
        )
        .unwrap();

        let result = service
            .generate_property_tests(temp_file.path(), None)
            .await
            .unwrap();

        assert_eq!(result.language, Language::Rust);
        assert!(result.functions.len() >= 4);

        // Should find roundtrip property for serialize/deserialize
        let has_roundtrip = result
            .property_tests
            .iter()
            .any(|t| t.property == PropertyType::Roundtrip);
        assert!(has_roundtrip);

        // Should find idempotency for normalize
        let has_idempotency = result
            .property_tests
            .iter()
            .any(|t| t.property == PropertyType::Idempotency);
        assert!(has_idempotency);

        // Should find commutativity for add
        let has_commutativity = result
            .property_tests
            .iter()
            .any(|t| t.property == PropertyType::Commutativity);
        assert!(has_commutativity);
    }

    #[tokio::test]
    async fn test_generate_property_tests_for_specific_function() {
        use tempfile::Builder;
        use std::io::Write;

        let service = TestGenerationService::new();

        let mut temp_file = Builder::new()
            .suffix(".rs")
            .tempfile()
            .unwrap();
        writeln!(
            temp_file,
            r#"
pub fn encode(value: &str) -> String {{
    // implementation
}}

pub fn decode(input: &str) -> String {{
    // implementation
}}

pub fn other_function() {{
    // should not be tested
}}
"#
        )
        .unwrap();

        let result = service
            .generate_property_tests(temp_file.path(), Some("encode"))
            .await
            .unwrap();

        // Should only test the encode function
        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, "encode");

        // Should generate roundtrip test
        assert_eq!(result.property_tests.len(), 1);
        assert_eq!(result.property_tests[0].property, PropertyType::Roundtrip);
    }

    #[test]
    fn test_identify_and_generate_property_tests() {
        let service = TestGenerationService::new();

        let target_functions = vec![
            FunctionSignature {
                name: "parse".to_string(),
                parameters: vec![],
                return_type: Some("Value".to_string()),
                is_async: false,
                line_number: 1,
            },
            FunctionSignature {
                name: "trim".to_string(),
                parameters: vec![],
                return_type: Some("String".to_string()),
                is_async: false,
                line_number: 10,
            },
        ];

        let all_functions = vec![
            target_functions[0].clone(),
            FunctionSignature {
                name: "format".to_string(),
                parameters: vec![],
                return_type: Some("String".to_string()),
                is_async: false,
                line_number: 5,
            },
            target_functions[1].clone(),
        ];

        let property_tests = service
            .identify_and_generate_property_tests(&target_functions, &all_functions, Language::Rust)
            .unwrap();

        // Should generate roundtrip for parse/format and idempotency for trim
        assert!(property_tests.len() >= 2);

        let has_roundtrip = property_tests
            .iter()
            .any(|t| t.property == PropertyType::Roundtrip && t.function_name == "parse");
        assert!(has_roundtrip);

        let has_idempotency = property_tests
            .iter()
            .any(|t| t.property == PropertyType::Idempotency && t.function_name == "trim");
        assert!(has_idempotency);
    }
}
