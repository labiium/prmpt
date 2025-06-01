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

    // Attempt to deserialize as a single Config first
    if let Ok(single_config) = serde_yaml::from_str::<Config>(&contents) {
        let mut configs = HashMap::new();
        configs.insert(DEFAULT_CONFIG_KEY.to_string(), single_config);
        return Ok(configs);
    }

    // If single Config deserialization fails, try as HashMap<String, Config>
    match serde_yaml::from_str::<HashMap<String, Config>>(&contents) {
        Ok(multiple_configs) => Ok(multiple_configs),
        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
    }
}
