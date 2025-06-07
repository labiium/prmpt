//! The primary function for generating prompts from a code repository based on a given `Config`.
//! Includes logic for scanning directories, applying ignore patterns, extracting documentation
//! or source code, and writing the results to an output file.

use log::{debug, error, warn};
// use rayon::prelude::*; // Removed as build_parallel().run() provides parallelism
use std::{
    collections::HashMap,
    // fs, // Removed unused import (std_fs is used)
    path::{Path, PathBuf}, 
    sync::{Arc, Mutex},
};
// use walkdir::WalkDir; // Removed
use ignore::WalkBuilder; // Added
use std::fs as std_fs; // Used for fs::canonicalize and fs::read_to_string
use ignore::overrides::OverrideBuilder; // Added this import

use super::config::Config;
use super::parse_python::{extract_python_signatures, maybe_read_notebook};
// Removed get_default_ignore_patterns, get_gitignore_patterns, should_ignore from utils import
// process_directory_structure is still used.
use super::utils::{process_directory_structure}; 
// use glob::Pattern; // Removed as main ignore logic uses `ignore` crate now. Still used by process_directory_structure internally.
use crate::curly::traits::GenerateOperation; // Import the trait
use anyhow::{Context, Error}; // For the Result type & context

/// Struct for implementing the GenerateOperation trait.
#[derive(Default)] 
pub struct Generator;

impl GenerateOperation for Generator {
    /// Runs the generation process based on the provided configuration.
    /// This method encapsulates the original `run` function's logic.
    fn run(&self, config: &Config) -> Result<(String, Vec<String>), Error> {
        let path_str = config.path.as_deref().unwrap_or(".");
        let repo_path = Path::new(path_str);

        // Canonicalize repo_path for robust path handling
        let canonical_repo_path = std_fs::canonicalize(repo_path)
            .with_context(|| format!("Failed to canonicalize repository path: '{}'", repo_path.display()))?;

        let output_file_name = config.output.as_deref().unwrap_or("curly.out");
        let delimiter = config.delimiter.as_deref().unwrap_or("```");

        let mut ignore_patterns_for_structure: Vec<glob::Pattern> = 
            if let Some(ignore_list) = &config.ignore {
                ignore_list
                    .iter()
                    .filter_map(|p| glob::Pattern::new(p).ok())
                    .collect()
            } else {
                Vec::new()
            };
        ignore_patterns_for_structure.push(glob::Pattern::new(output_file_name).unwrap());
        ignore_patterns_for_structure.push(glob::Pattern::new(".git").unwrap());
        ignore_patterns_for_structure.push(glob::Pattern::new("curly.yaml").unwrap());
        ignore_patterns_for_structure.push(glob::Pattern::new(".gitignore").unwrap()); // Added this line

        let output_arc = Arc::new(Mutex::new(String::new()));
        let error_count_arc = Arc::new(Mutex::new(HashMap::new()));

        if let Some(prompts) = &config.prompts {
            let mut output_guard = output_arc.lock().unwrap();
            for prompt in prompts {
                output_guard.push_str(&format!("{}\n", prompt));
            }
            output_guard.push_str("\n");
        }

        let current_dir_name = if path_str == "." {
            std::env::current_dir()
                .context("Failed to get current directory")?
                .file_name()
                .ok_or_else(|| Error::msg("Failed to get current directory name (file_name is None)"))?
                .to_string_lossy()
                .into_owned()
        } else {
            canonical_repo_path // Use the canonicalized path here
                .file_name()
                .ok_or_else(|| Error::msg(format!("Failed to get file name from repo_path: {}", canonical_repo_path.display())))?
                .to_string_lossy()
                .into_owned()
        };
        
        {
            let mut output_guard = output_arc.lock().unwrap();
            output_guard.push_str(&format!("{}\n", current_dir_name));
        }
        process_directory_structure(&canonical_repo_path, &output_arc, 0, &ignore_patterns_for_structure, "", &canonical_repo_path);
        {
            let mut output_guard = output_arc.lock().unwrap();
            output_guard.push_str("\n");
        }

        process_directory_files(
            &canonical_repo_path, 
            &output_arc,
            &canonical_repo_path, 
            delimiter,
            &error_count_arc,
            config, 
            output_file_name,
        );

        let mut errors = vec!();
        let error_count_guard = error_count_arc.lock().unwrap();
        if !error_count_guard.is_empty() {
            for (dir, count) in error_count_guard.iter() {
                errors.push(format!(
                    "Directory '{}' had {} file(s) that could not be processed\n",
                    dir, count
                ));
            }
        }
        let final_output_string = output_arc.lock().unwrap().clone();
        Ok((final_output_string, errors))
    }
}

// The old `run` function is removed as its logic is now in `Generator::run`.

/// Utility function to run the generation and write the output to a file.
/// This function now uses the GenerateOperation trait.
pub fn run_and_write(generator: &impl GenerateOperation, config: &Config) -> Result<(), Error> {
    let output_file_name = config.output.as_deref().unwrap_or("curly.out").to_string();

    match generator.run(config) {
        Ok((output_final, errors)) => {
            if let Err(e) = std_fs::write(&output_file_name, &*output_final) {
                return Err(Error::new(e).context(format!("Unable to write to file {}", output_file_name)));
            }
            if !errors.is_empty() {
                // Log non-critical errors from the run process
                for error_msg in errors {
                    warn!("{}", error_msg.trim_end()); // Trim newline if present
                }
            }
            Ok(())
        }
        Err(e) => {
            // Log the critical error from the generator itself
            error!("Generator operation failed: {:?}", e);
            Err(e.context("Generator operation failed in run_and_write"))
        }
    }
}

/// Helper function to provide language-specific default ignore patterns for the `ignore` crate.
/// These patterns should be in .gitignore format.
fn get_default_ignore_patterns_for_ignore(language: &str) -> Vec<String> {
    match language.to_lowercase().as_str() {
        "python" => vec![
            "__pycache__/".to_string(), "*.pyc".to_string(), "*.pyo".to_string(), "*.pyd".to_string(),
            ".Python".to_string(), "build/".to_string(), "develop-eggs/".to_string(), "dist/".to_string(),
            "downloads/".to_string(), "eggs/".to_string(), ".eggs/".to_string(), "lib/".to_string(),
            "lib64/".to_string(), "parts/".to_string(), "sdist/".to_string(), "var/".to_string(),
            "wheels/".to_string(), "share/python-wheels/".to_string(), "*.egg-info/".to_string(),
            ".installed.cfg".to_string(), "*.egg".to_string(), "MANIFEST".to_string(),
            ".env".to_string(), ".venv".to_string(), "env/".to_string(), "venv/".to_string(),
            "ENV/".to_string(), "VENV/".to_string(), ".pytest_cache/".to_string(),
            ".mypy_cache/".to_string(), ".dmypy.json".to_string(), "dmypy.json".to_string(),
            ".coverage".to_string(), "htmlcov/".to_string(), "instance/".to_string(),
            ".webassets-cache".to_string(),
        ],
        "javascript" => vec![
            "node_modules/".to_string(), "npm-debug.log*".to_string(), "yarn-debug.log*".to_string(),
            "yarn-error.log*".to_string(), "dist/".to_string(), "build/".to_string(), ".DS_Store".to_string(),
        ],
        "rust" => vec!["target".to_string(), "Cargo.lock".to_string()], // Changed "target/" to "target"
        _ => Vec::new(),
    }
}

/// Iterates over files in a directory and processes each one, collecting the results into `output`.
fn process_directory_files(
    dir: &Path,
    output: &Arc<Mutex<String>>,
    base_path: &Path, // Used for stripping prefix from paths for display
    // ignore_patterns: &[Pattern], // Removed
    delimiter: &str,
    error_count: &Arc<Mutex<HashMap<String, usize>>>,
    config: &Config,
    output_file_name: &str, // Added to ignore the output file specifically
) {
    let mut walker_builder = WalkBuilder::new(dir);
    walker_builder.add_custom_ignore_filename(".curlyignore"); // Support .curlyignore

    // Create an OverrideBuilder and add all patterns to it.
    // Use OverrideBuilder for ignore crate patterns. Prefixing globs with '!' makes
    // them act as ignore rules instead of whitelists.
    let mut override_builder = OverrideBuilder::new(dir);

    // Add patterns to ensure specific files/dirs are ignored.
    // Ensure the output file itself is ignored
    if let Err(e) = override_builder.add(&format!("!{}", output_file_name)) {
        warn!("Failed to add output file ignore pattern '{}': {}", output_file_name, e);
    }
    if let Err(e) = override_builder.add("!.git") {
        warn!("Failed to add .git ignore pattern: {}", e);
    }
    if let Err(e) = override_builder.add("!.gitignore") {
        warn!("Failed to add .gitignore ignore pattern: {}", e);
    }
    if let Err(e) = override_builder.add("!curly.yaml") {
        warn!("Failed to add curly.yaml ignore pattern: {}", e);
    }

    // Add patterns from config.ignore
    if let Some(ignore_list) = &config.ignore {
        for pattern_str in ignore_list {
            if let Err(e) = override_builder.add(&format!("!{}", pattern_str)) {
                warn!("Failed to add custom ignore pattern '{}': {}", pattern_str, e);
            }
        }
    }

    // Add language-specific default ignore patterns
    if let Some(language) = config.language.as_deref() {
        let default_patterns = get_default_ignore_patterns_for_ignore(language);
        for pattern_str in default_patterns {
            if let Err(e) = override_builder.add(&format!("!{}", pattern_str)) {
                warn!("Failed to add default ignore pattern '{}': {}", pattern_str, e);
            }
        }
    }

    match override_builder.build() {
        Ok(ov) => {
            walker_builder.overrides(ov);
        }
        Err(e) => {
            warn!("Failed to build overrides: {}", e);
        }
    }

    // Control .gitignore usage based on config
    if !config.use_gitignore.unwrap_or(true) {
        walker_builder.git_ignore(false);
        walker_builder.git_global(false);
        walker_builder.git_exclude(false); // Also disable per-repository core.excludesFile
        walker_builder.parents(false); // Disable parent ignore files
        walker_builder.require_git(false); // Don't require a .git directory for parent search
    }
    // Otherwise, .gitignore files are respected by default (and parent search is enabled).
    
    // Canonicalize base_path for robust prefix stripping, important if `dir` could be a symlink
    // or contains `..` components.
    let canonical_base_path = match std_fs::canonicalize(base_path) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to canonicalize base_path {}: {}. Using original.", base_path.display(), e);
            PathBuf::from(base_path) // Fallback to original base_path
        }
    };

    let walker = walker_builder.build();
    let mut entries: Vec<_> = walker.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path().to_path_buf());

    for entry in entries {
        let path = entry.path();
        if path.is_file() {
            let mut local_output = String::new();
            if let Err(e) = process_file(path, &mut local_output, &canonical_base_path, delimiter, config) {
                let dir_key = path.parent().unwrap_or_else(|| Path::new("")).to_string_lossy().to_string();
                let mut error_count_guard = error_count.lock().unwrap();
                *error_count_guard.entry(dir_key).or_insert(0) += 1;
                debug!("Failed to process file {}: {}", path.display(), e);
            } else if !local_output.is_empty() {
                let mut output_guard = output.lock().unwrap();
                output_guard.push_str(&local_output);
            }
        }
    }
}

/// Processes a single file, adding its contents (or relevant docstrings) to `output`.
/// Respects the `docs_comments_only` setting and can handle Jupyter notebooks if needed.
fn process_file(
    file: &Path,
    output: &mut String,
    base_path: &Path, // Now potentially canonicalized
    delimiter: &str,
    config: &Config,
) -> Result<(), std::io::Error> {
    // Attempt to strip the prefix using the (potentially canonicalized) base_path.
    let relative_path_display = match file.strip_prefix(base_path) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => {
            // Fallback: if stripping fails (e.g., symlink points outside, though `ignore` crate might handle this)
            // use the full path. This ensures we always have a valid path string.
            file.to_string_lossy().to_string()
        }
    };
    let relative_path_str = &relative_path_display;


    // If the user wants to ignore certain patterns for docstrings (uses glob::Pattern)
    let docs_ignore_patterns = if let Some(docs_ignore_list) = &config.docs_ignore {
        docs_ignore_list
            .iter()
            .filter_map(|p| glob::Pattern::new(p).ok()) // Keep using glob for this specific feature
            .collect::<Vec<glob::Pattern>>()
    } else {
        Vec::new()
    };

    let should_ignore_docs_only = docs_ignore_patterns
        .iter()
            .any(|pattern| pattern.matches(relative_path_str) || pattern.matches_path(file));

    // If docs_comments_only is enabled and the language is Python, extract docstrings only
    if let Some(true) = config.docs_comments_only {
        if !should_ignore_docs_only
            && config.language.as_deref().unwrap_or("").to_lowercase() == "python"
        {
            let extension = file
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("");
            if extension != "py" {
                return Ok(()); // Skip non-Python files
            }

            if should_ignore_docs_only {
                return Ok(());
            }

            // Process Python file to extract signatures and docstrings
            let contents = std_fs::read_to_string(file)?; // Use std_fs
            let signatures = extract_python_signatures(&contents);

            if !signatures.trim().is_empty() {
                output.push_str(&format!("{}{}\n", delimiter, relative_path_str));
                output.push_str(&signatures);
                output.push_str(&format!("\n{}\n\n", delimiter));
            }
            return Ok(());
        }
    }

    // If the file is a .ipynb, parse the notebook
    if let Some(ext) = file.extension().and_then(std::ffi::OsStr::to_str) {
        if ext == "ipynb" {
            if let Some(notebook_json) = maybe_read_notebook(&file.to_string_lossy()) {
                output.push_str(&format!("{}{}\n", delimiter, relative_path_str));

                // Attempt to read cells from the notebook
                if let Some(cells) = notebook_json.get("cells").and_then(|c| c.as_array()) {
                    for (i, cell) in cells.iter().enumerate() {
                        let cell_type = cell.get("cell_type").and_then(|ct| ct.as_str());
                        if let Some(cell_type) = cell_type {
                            match cell_type {
                                "code" => {
                                    if let Some(src) = cell.get("source").and_then(|s| s.as_array())
                                    {
                                        output.push_str(&format!("// Cell #{} (code)\n", i));
                                        for line_val in src {
                                            if let Some(line_str) = line_val.as_str() {
                                                output.push_str(line_str);
                                            }
                                        }
                                        output.push_str("\n");
                                    }
                                    // If display_outputs is enabled, print outputs
                                    if config.display_outputs.unwrap_or(false) {
                                        if let Some(outputs) =
                                            cell.get("outputs").and_then(|o| o.as_array())
                                        {
                                            output.push_str(&format!("// Cell #{} (outputs)\n", i));
                                            for output_obj in outputs {
                                                // Attempt to extract common output types
                                                if let Some(text) = output_obj
                                                    .get("text")
                                                    .and_then(|t| t.as_array())
                                                {
                                                    for text_line in text {
                                                        if let Some(line_str) = text_line.as_str() {
                                                            output.push_str(line_str);
                                                        }
                                                    }
                                                    output.push_str("\n");
                                                } else if let Some(data) = output_obj
                                                    .get("data")
                                                    .and_then(|d| d.as_object())
                                                {
                                                    // For example: text/plain outputs
                                                    if let Some(text_plain) = data.get("text/plain")
                                                    {
                                                        if let Some(text_arr) =
                                                            text_plain.as_array()
                                                        {
                                                            for text_line in text_arr {
                                                                if let Some(line_str) =
                                                                    text_line.as_str()
                                                                {
                                                                    output.push_str(line_str);
                                                                }
                                                            }
                                                            output.push_str("\n");
                                                        } else if let Some(text_str) =
                                                            text_plain.as_str()
                                                        {
                                                            output.push_str(text_str);
                                                            output.push_str("\n");
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                "markdown" => {
                                    if let Some(src) = cell.get("source").and_then(|s| s.as_array())
                                    {
                                        output.push_str(&format!("// Cell #{} (markdown)\n", i));
                                        for line_val in src {
                                            if let Some(line_str) = line_val.as_str() {
                                                output.push_str(line_str);
                                            }
                                        }
                                        output.push_str("\n");
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                output.push_str(&format!("\n{}\n\n", delimiter));
            }
            return Ok(());
        }
    }

    // Default case: read the file and include its entire contents.
    output.push_str(&format!("{}{}\n", delimiter, relative_path_str));
    match std_fs::read_to_string(file) { // Use std_fs
        Ok(contents) => output.push_str(&contents),
        Err(e) => {
            output.push_str(&format!("[Error reading file: {}]", e)); // Include error message
            return Err(e);
        }
    }
    output.push_str(&format!("\n{}\n\n", delimiter));
    Ok(())
}


// A function which returns the directory structurre of a given path
pub fn directory_peak(dir_path: &str) -> String {
    let path = Path::new(dir_path);
    let output = Arc::new(Mutex::new(String::new()));
    // These are glob patterns, used by process_directory_structure
    let ignore_patterns_for_peak = vec!( // Renamed to avoid confusion
        glob::Pattern::new("curly.out").unwrap(),
        glob::Pattern::new(".git").unwrap(),
        glob::Pattern::new("curly.yaml").unwrap(),
        glob::Pattern::new("node_modules").unwrap(),
        glob::Pattern::new("target").unwrap(),
        glob::Pattern::new("dist").unwrap(),
        glob::Pattern::new("build").unwrap(),
        glob::Pattern::new("venv").unwrap(),
        glob::Pattern::new("env").unwrap()
    );
    
    process_directory_structure(path, &output, 0, &ignore_patterns_for_peak, "", path);
    let output_guard = output.lock().unwrap();
    output_guard.clone().to_string()
}