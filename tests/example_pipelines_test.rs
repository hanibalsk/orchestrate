//! Test that all example pipeline files are valid

use orchestrate_core::PipelineDefinition;
use std::fs;
use std::path::PathBuf;

fn get_example_path(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("pipelines")
        .join(filename)
}

#[test]
fn test_ci_pipeline_example() {
    let path = get_example_path("ci-pipeline.yaml");
    let pipeline = PipelineDefinition::from_yaml_file(&path)
        .expect("Failed to parse ci-pipeline.yaml");

    assert_eq!(pipeline.name, "ci-pipeline");
    assert!(!pipeline.stages.is_empty());
    assert!(!pipeline.triggers.is_empty());
}

#[test]
fn test_cd_pipeline_example() {
    let path = get_example_path("cd-pipeline.yaml");
    let pipeline = PipelineDefinition::from_yaml_file(&path)
        .expect("Failed to parse cd-pipeline.yaml");

    assert_eq!(pipeline.name, "cd-pipeline");
    assert!(!pipeline.stages.is_empty());
    assert!(!pipeline.variables.is_empty());
}

#[test]
fn test_release_pipeline_example() {
    let path = get_example_path("release-pipeline.yaml");
    let pipeline = PipelineDefinition::from_yaml_file(&path)
        .expect("Failed to parse release-pipeline.yaml");

    assert_eq!(pipeline.name, "release-pipeline");
    assert!(!pipeline.stages.is_empty());

    // Should have at least one approval stage
    let has_approval = pipeline.stages.iter().any(|s| s.requires_approval);
    assert!(has_approval);
}

#[test]
fn test_conditional_pipeline_example() {
    let path = get_example_path("conditional-pipeline.yaml");
    let pipeline = PipelineDefinition::from_yaml_file(&path)
        .expect("Failed to parse conditional-pipeline.yaml");

    assert_eq!(pipeline.name, "conditional-pipeline");
    assert!(!pipeline.stages.is_empty());

    // Should have at least one conditional stage
    let has_conditions = pipeline.stages.iter().any(|s| s.when.is_some());
    assert!(has_conditions);
}

#[test]
fn test_all_examples_valid() {
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("pipelines");

    let entries = fs::read_dir(examples_dir).expect("Failed to read examples directory");

    let mut count = 0;
    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            count += 1;
            PipelineDefinition::from_yaml_file(&path)
                .unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", path, e));
        }
    }

    assert!(count >= 4, "Expected at least 4 example pipeline files");
}
