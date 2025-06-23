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

/// Loads configuration from a local `curly.yaml` file.
/// The file can contain a single configuration or multiple named configurations.
pub fn load_config() -> Result<HashMap<String, Config>, Box<dyn std::error::Error>> {
    let config_path = Path::new("curly.yaml");
    let contents = fs::read_to_string(config_path)?;

    // Parse the YAML generically first so we can determine its structure
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&contents)?;
    let mapping = yaml_value.as_mapping().ok_or("curly.yaml must contain a mapping at the top level")?;

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

    let all_keys_are_fields = mapping.keys().all(|k| {
        k.as_str()
            .map(|key| CONFIG_FIELDS.contains(&key))
            .unwrap_or(false)
    });

    if all_keys_are_fields {
        // Deserialize directly into a single Config
        let single_config: Config = serde_yaml::from_value(yaml_value)?;
        let mut configs = HashMap::new();
        configs.insert(DEFAULT_CONFIG_KEY.to_string(), single_config);
        Ok(configs)
    } else {
        // Treat each top level key as a named configuration
        let multiple_configs: HashMap<String, Config> = serde_yaml::from_value(yaml_value)?;
        Ok(multiple_configs)
    }
}
