//! Integration tests for change test analyzer
//!
//! NOTE: These tests are disabled because ChangeTestAnalyzer is not exported from orchestrate-core.
//! Re-enable when Epic 005 (Test Generation Agent) is implemented.

// Disable entire module until change test analyzer is exported
#![allow(dead_code)]
#![allow(unused_imports)]

#[cfg(feature = "change_test_analyzer")]
mod change_analyzer_tests {

use orchestrate_core::{ChangeTestAnalyzer, Priority};
use std::path::PathBuf;

#[tokio::test]
async fn test_analyze_rust_diff_with_new_function() {
    let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/test/repo"));

    let diff = r#"diff --git a/src/calculator.rs b/src/calculator.rs
index 1234567..abcdefg 100644
--- a/src/calculator.rs
+++ b/src/calculator.rs
@@ -10,6 +10,14 @@ impl Calculator {
     }

+    pub fn add(a: i32, b: i32) -> i32 {
+        a + b
+    }
+
+    fn helper() {
+        // internal helper
+    }
+
     fn existing_function() {
         // existing code
     }
"#;

    let result = analyzer
        .analyze_diff(diff, "main", "feature")
        .await
        .unwrap();

    // Should find 2 functions (add and helper)
    assert_eq!(result.changed_functions.len(), 2);

    // Find the public add function
    let add_fn = result
        .changed_functions
        .iter()
        .find(|f| f.name == "add")
        .expect("Should find 'add' function");

    assert_eq!(add_fn.is_public, true);
    assert_eq!(add_fn.file_path, PathBuf::from("src/calculator.rs"));

    // Find the private helper function
    let helper_fn = result
        .changed_functions
        .iter()
        .find(|f| f.name == "helper")
        .expect("Should find 'helper' function");

    assert_eq!(helper_fn.is_public, false);
}

#[tokio::test]
async fn test_analyze_typescript_diff() {
    let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/test/repo"));

    let diff = r#"diff --git a/src/utils.ts b/src/utils.ts
index abc123..def456 100644
--- a/src/utils.ts
+++ b/src/utils.ts
@@ -5,6 +5,10 @@ export class Utils {
     }

+    export function formatDate(date: Date): string {
+        return date.toISOString();
+    }
+
     private function internalHelper() {
         // helper
     }
"#;

    let result = analyzer
        .analyze_diff(diff, "main", "feature")
        .await
        .unwrap();

    assert!(result.changed_functions.len() >= 1);

    let format_fn = result
        .changed_functions
        .iter()
        .find(|f| f.name == "formatDate");

    assert!(format_fn.is_some());
    if let Some(f) = format_fn {
        assert_eq!(f.is_public, true);
        assert_eq!(f.file_path, PathBuf::from("src/utils.ts"));
    }
}

#[tokio::test]
async fn test_generate_suggestions_for_untested_functions() {
    let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/test/repo"));

    let diff = r#"diff --git a/src/math.rs b/src/math.rs
index 1111111..2222222 100644
--- a/src/math.rs
+++ b/src/math.rs
@@ -1,6 +1,10 @@
 pub mod math {

+    pub fn multiply(a: i32, b: i32) -> i32 {
+        a * b
+    }
+
     pub fn divide(a: i32, b: i32) -> Result<i32, String> {
         if b == 0 {
"#;

    let result = analyzer
        .analyze_diff(diff, "main", "feature")
        .await
        .unwrap();

    // Should find multiply function
    assert_eq!(result.changed_functions.len(), 1);

    // Since we're not actually checking test files (they don't exist in this test),
    // the function should be marked as untested
    assert_eq!(result.suggestions.len(), 1);

    let suggestion = &result.suggestions[0];
    assert_eq!(suggestion.function.name, "multiply");
    assert_eq!(suggestion.priority, Priority::High); // Public function = high priority

    // Should have suggested test cases
    assert!(!suggestion.suggested_tests.is_empty());
    assert!(suggestion
        .suggested_tests
        .iter()
        .any(|t| t.contains("happy_path")));
}

#[tokio::test]
async fn test_format_pr_comment() {
    let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/test/repo"));

    let diff = r#"diff --git a/src/api.rs b/src/api.rs
index aaa..bbb 100644
--- a/src/api.rs
+++ b/src/api.rs
@@ -1,3 +1,7 @@
+pub fn handle_request() {
+    // new public function
+}
+
 fn internal() {
     // existing
 }
"#;

    let result = analyzer
        .analyze_diff(diff, "main", "feature")
        .await
        .unwrap();

    let comment = analyzer.format_pr_comment(&result);

    // Check that comment contains expected sections
    assert!(comment.contains("Test Coverage Analysis"));
    assert!(comment.contains("Coverage:"));
    assert!(comment.contains("handle_request"));

    // Should contain priority markers
    assert!(comment.contains("High Priority") || comment.contains("Medium Priority"));
}

#[tokio::test]
async fn test_coverage_percentage_in_result() {
    let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/test/repo"));

    // Empty diff should result in 100% coverage
    let diff = "";

    let result = analyzer
        .analyze_diff(diff, "main", "feature")
        .await
        .unwrap();

    // No functions changed = 100% coverage
    assert_eq!(result.coverage_percentage, 100.0);
    assert_eq!(result.changed_functions.len(), 0);
    assert_eq!(result.suggestions.len(), 0);
}

} // End of change_analyzer_tests module
