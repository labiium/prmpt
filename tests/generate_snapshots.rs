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

#[test]
fn test_config_and_ignore_snapshot() {
    let project_path_str = get_test_repo_path("config_and_ignore_test")
        .to_string_lossy()
        .to_string();

    // Change current directory to the test project path so curly.yaml is found
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&project_path_str).unwrap();

    // Load config from curly.yaml in the test project directory
    let configs = curly::load_config().expect("Failed to load curly.yaml for config_and_ignore_test");
    let config = configs.get(curly::DEFAULT_CONFIG_KEY) // Using DEFAULT_CONFIG_KEY as per new format
        .expect("Config not found under default key for config_and_ignore_test")
        .clone(); // Clone to own it

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    // Ensure the path in the loaded config is updated to be absolute, or relative to the original dir
    // For simplicity, let's assume load_config doesn't populate `path` if it's not in YAML,
    // or if it does, it's relative. We need to ensure the Generator runs in the context of project_path_str.
    // The easiest way is to set it explicitly after loading.
    let mut effective_config = config;
    effective_config.path = Some(project_path_str.clone());
    // Output is set in curly.yaml, but for snapshot string testing, it's not strictly needed.
    // Let's clear it to ensure we are testing the string output, not file writing.
    effective_config.output = None;


    let generator = Generator::default();
    // Pass a reference to the config
    let result = generator.run(&effective_config);

    assert!(result.is_ok(), "Generator run failed: {:?}", result.err());
    let (output_string, errors) = result.unwrap();

    assert!(errors.is_empty(), "Generator run reported errors: {:?}", errors);

    // Snapshot name will be `generate_snapshots__test_config_and_ignore_snapshot.snap`
    assert_yaml_snapshot!(output_string);
}

#[test]
fn test_config_and_ignore_false_snapshot() {
    let project_path_str = get_test_repo_path("config_and_ignore_false_test")
        .to_string_lossy()
        .to_string();

    // Change current directory to the test project path so curly.yaml is found
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&project_path_str).unwrap();

    // Load config from curly.yaml in the test project directory
    let configs = curly::load_config().expect("Failed to load curly.yaml for config_and_ignore_false_test");
    let config = configs.get(curly::DEFAULT_CONFIG_KEY)
        .expect("Config not found under default key for config_and_ignore_false_test")
        .clone();

    // Restore original directory
    std::env::set_current_dir(original_dir).unwrap();

    let mut effective_config = config;
    effective_config.path = Some(project_path_str.clone());
    effective_config.output = None; // For snapshot string testing

    let generator = Generator::default();
    let result = generator.run(&effective_config);

    assert!(result.is_ok(), "Generator run failed: {:?}", result.err());
    let (output_string, errors) = result.unwrap();

    assert!(errors.is_empty(), "Generator run reported errors: {:?}", errors);

    // Snapshot name will be `generate_snapshots__test_config_and_ignore_false_snapshot.snap`
    assert_yaml_snapshot!(output_string);
}
