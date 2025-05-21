use curly::{Config, Generator, GenerateOperation}; // Corrected crate name
use insta::assert_yaml_snapshot;
use std::path::PathBuf;

// Helper function to construct path to test_repos
fn get_test_repo_path(repo_name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/snapshot_tests/test_repos");
    path.push(repo_name);
    path
}

#[test]
fn test_sample_project_1_default_snapshot() {
    let project_path = get_test_repo_path("sample_project_1");

    let config = Config {
        path: Some(project_path.to_string_lossy().to_string()),
        output: None, // We don't need to write a file for snapshot testing the output string
        ignore: Some(vec![]), // Default: no additional ignores beyond .gitignore
        delimiter: Some("```".to_string()),
        language: Some("python".to_string()), // Explicitly set for clarity
        docs_comments_only: Some(false),      // Default behavior
        docs_ignore: Some(vec![]),
        use_gitignore: Some(true),           // Test .gitignore processing
        display_outputs: Some(false),
        prompts: None,
    };

    let generator = Generator::default();
    let result = generator.run(&config);

    assert!(result.is_ok(), "Generator run failed: {:?}", result.err());
    let (output_string, errors) = result.unwrap();
    
    // Assert that there are no non-critical errors reported from the run
    // (e.g., files that couldn't be processed but didn't stop the whole operation)
    // Depending on strictness, this might be active or commented out.
    // For now, let's ensure it's empty for this controlled test case.
    assert!(errors.is_empty(), "Generator run reported errors: {:?}", errors);

    // Snapshot the main output string
    // The snapshot name will be derived from the test function name:
    // `generate_snapshots__test_sample_project_1_default_snapshot.snap`
    assert_yaml_snapshot!(output_string);
}

#[test]
fn test_sample_project_1_docs_only_snapshot() {
    let project_path = get_test_repo_path("sample_project_1");

    let config = Config {
        path: Some(project_path.to_string_lossy().to_string()),
        output: None,
        ignore: Some(vec![]),
        delimiter: Some("```".to_string()),
        language: Some("python".to_string()),
        docs_comments_only: Some(true), // Test docs_comments_only feature
        docs_ignore: Some(vec![]),
        use_gitignore: Some(true),
        display_outputs: Some(false),
        prompts: None,
    };

    let generator = Generator::default();
    let result = generator.run(&config);

    assert!(result.is_ok(), "Generator run failed for docs_only: {:?}", result.err());
    let (output_string, errors) = result.unwrap();
    assert!(errors.is_empty(), "Generator run for docs_only reported errors: {:?}", errors);

    // Snapshot for the docs_comments_only output
    // Snapshot name: `generate_snapshots__test_sample_project_1_docs_only_snapshot.snap`
    assert_yaml_snapshot!(output_string);
}
