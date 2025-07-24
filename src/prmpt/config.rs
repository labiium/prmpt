//! Holds the configuration structure (`Config`) and functionality to load configurations.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

/// Configuration structure that holds various options for generating or injecting code.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Path to the code repository.
    pub path: Option<String>,
    /// Glob patterns to ignore.
    pub ignore: Option<Vec<String>>,
    /// File path to write the generated prompt.
    pub output: Option<String>,
    /// Delimiter for code blocks in the prompt (e.g., "```").
    pub delimiter: Option<String>,
    /// The programming language of the repository (e.g. "rust", "python").
    pub language: Option<String>,
    /// Additional prompts that can be injected into the output for specific files.
    pub prompts: Option<Vec<String>>,
    /// If true, only documentation and comments are extracted (used for e.g. docs-only runs).
    pub docs_comments_only: Option<bool>,
    /// Patterns to ignore specifically in documentation comments.
    pub docs_ignore: Option<Vec<String>>,
    /// If true, respects patterns in a `.gitignore` file.
    pub use_gitignore: Option<bool>,
    /// If true, any outputs from Jupyter Notebook cells will be included in the generated prompt.
    pub display_outputs: Option<bool>,
}

pub const DEFAULT_CONFIG_KEY: &str = "base";

/// Creates a default base configuration with sensible defaults
fn create_default_base_config() -> Config {
    Config {
        path: Some(".".to_string()),
        ignore: None,
        output: Some("prmpt.out".to_string()),
        delimiter: Some("```".to_string()),
        language: None,
        prompts: None,
        docs_comments_only: None,
        docs_ignore: None,
        use_gitignore: Some(true),
        display_outputs: None,
    }
}

/// Loads configuration from a local `prmpt.yaml` file.
/// The file can contain a single configuration or multiple named configurations.
/// If no file exists or no 'base' config is found, returns a default 'base' config.
pub fn load_config() -> Result<HashMap<String, Config>, Box<dyn std::error::Error>> {
    let config_path = Path::new("prmpt.yaml");

    // If the config file doesn't exist, return default base config
    if !config_path.exists() {
        let mut configs = HashMap::new();
        configs.insert(DEFAULT_CONFIG_KEY.to_string(), create_default_base_config());
        return Ok(configs);
    }

    let contents = fs::read_to_string(config_path)?;

    // Parse the YAML generically first so we can determine its structure
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&contents)?;
    let mapping = yaml_value
        .as_mapping()
        .ok_or("prmpt.yaml must contain a mapping at the top level")?;

    // Set of valid Config field names to distinguish between a single config and a map of configs
    const CONFIG_FIELDS: &[&str] = &[
        "path",
        "ignore",
        "output",
        "delimiter",
        "language",
        "prompts",
        "docs_comments_only",
        "docs_ignore",
        "use_gitignore",
        "display_outputs",
    ];

    // Check if this is a mixed structure (top-level config fields + nested configs)
    let config_field_keys: Vec<_> = mapping
        .keys()
        .filter_map(|k| k.as_str())
        .filter(|key| CONFIG_FIELDS.contains(key))
        .collect();

    let non_config_keys: Vec<_> = mapping
        .keys()
        .filter_map(|k| k.as_str())
        .filter(|key| !CONFIG_FIELDS.contains(key))
        .collect();

    let mut configs = HashMap::new();

    if config_field_keys.is_empty() {
        // No top-level config fields, treat all keys as named configurations
        configs = serde_yaml::from_value(yaml_value)?;
    } else if non_config_keys.is_empty() {
        // All keys are config fields, treat as single config
        let single_config: Config = serde_yaml::from_value(yaml_value)?;
        configs.insert(DEFAULT_CONFIG_KEY.to_string(), single_config);
    } else {
        // Mixed structure: extract top-level config fields for 'base' and nested configs
        let mut base_config_map = serde_yaml::Mapping::new();
        let mut named_configs = HashMap::new();

        for (key, value) in mapping {
            if let Some(key_str) = key.as_str() {
                if CONFIG_FIELDS.contains(&key_str) {
                    // This is a config field, add to base config
                    base_config_map.insert(key.clone(), value.clone());
                } else {
                    // This is a named config
                    let named_config: Config = serde_yaml::from_value(value.clone())?;
                    named_configs.insert(key_str.to_string(), named_config);
                }
            }
        }

        // Create base config from extracted fields
        if !base_config_map.is_empty() {
            let base_config: Config =
                serde_yaml::from_value(serde_yaml::Value::Mapping(base_config_map))?;
            configs.insert(DEFAULT_CONFIG_KEY.to_string(), base_config);
        }

        // Add named configs
        configs.extend(named_configs);
    };

    // Ensure there's always a 'base' config available
    if !configs.contains_key(DEFAULT_CONFIG_KEY) {
        configs.insert(DEFAULT_CONFIG_KEY.to_string(), create_default_base_config());
    }

    Ok(configs)
}
