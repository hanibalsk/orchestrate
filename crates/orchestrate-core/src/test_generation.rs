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
}
