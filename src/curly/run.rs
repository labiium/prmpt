//! The primary function for generating prompts from a code repository based on a given `Config`.
//! Includes logic for scanning directories, applying ignore patterns, extracting documentation
//! or source code, and writing the results to an output file.

use log::{debug, error, warn};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};
use walkdir::WalkDir;

use super::config::Config;
use super::parse_python::{extract_python_signatures, maybe_read_notebook};
use super::utils::{
    get_default_ignore_patterns, get_gitignore_patterns, process_directory_structure, should_ignore,
};
use glob::Pattern;

/// Main function to run the code generation logic based on the provided configuration.
///
/// Steps:
/// 1. Build or retrieve ignore patterns.
/// 2. Write any provided `prompts` into the output.
/// 3. Output a visual directory structure.
/// 4. Traverse all files and collect code into the prompt.
/// 5. Write the final result to `config.output` (defaults to "curly.out").
/// 
/// Returns a tuple of the final output and any errors encountered during processing.
pub fn run(config: Config) -> (String, String) {
    let path = config.path.as_deref().unwrap_or(".").to_string();
    let repo_path = Path::new(&path);

    let output_file = config.output.as_deref().unwrap_or("curly.out").to_string();
    let delimiter = config.delimiter.as_deref().unwrap_or("```").to_string();

    // Initialize ignore patterns from the config
    let mut ignore_patterns: Vec<Pattern> = if let Some(ignore_list) = &config.ignore {
        ignore_list
            .iter()
            .filter_map(|p| Pattern::new(p).ok())
            .collect()
    } else {
        Vec::new()
    };
    ignore_patterns.push(Pattern::new(&output_file).unwrap());
    ignore_patterns.push(Pattern::new(".git").unwrap());
    ignore_patterns.push(Pattern::new("curly.yaml").unwrap());

    // Conditionally add patterns from .gitignore file based on 'use_gitignore' setting
    if config.use_gitignore.unwrap_or(true) {
        if let Ok(gitignore_patterns) = get_gitignore_patterns(repo_path) {
            ignore_patterns.extend(gitignore_patterns);
        }
    }

    // Add default patterns based on language
    if let Some(language) = config.language.as_deref() {
        let default_patterns = get_default_ignore_patterns(language);
        ignore_patterns.extend(default_patterns);
    }

    let output = Arc::new(Mutex::new(String::new()));
    let error_count = Arc::new(Mutex::new(HashMap::new()));

    // Include prompts in the output if provided
    if let Some(prompts) = &config.prompts {
        let mut output_guard = output.lock().unwrap();
        for prompt in prompts {
            output_guard.push_str(&format!("{}\n", prompt));
        }
        output_guard.push_str("\n");
    }

    // Get the current directory name
    let current_dir_name = if path == "." {
        std::env::current_dir()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    } else {
        repo_path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new(""))
            .to_string_lossy()
            .to_string()
    };

    // Start building the directory structure output
    {
        let mut output_guard = output.lock().unwrap();
        output_guard.push_str(&format!("{}\n", current_dir_name));
    }
    process_directory_structure(repo_path, &output, 0, &ignore_patterns, "", repo_path);
    {
        let mut output_guard = output.lock().unwrap();
        output_guard.push_str("\n");
    }

    // Process files in the directory
    process_directory_files(
        repo_path,
        &output,
        repo_path,
        &ignore_patterns,
        &delimiter,
        &error_count,
        &config,
    );

    // Report any errors encountered during processing
    let mut errors = String::new();
    let error_count_guard = error_count.lock().unwrap();
    if !error_count_guard.is_empty() {
        for (dir, count) in error_count_guard.iter() {
            errors.push_str(&format!(
                "Directory '{}' had {} file(s) that could not be processed\n",
                dir, count
            ));
        }
    }
    (output.lock().unwrap().clone(), errors)
}

pub fn run_and_write(config: Config) {
    let output_file = config.output.as_deref().unwrap_or("curly.out").to_string();

    // Write the final output to the specified file
    let (output_final, errors)  = run(config);
    if let Err(e) = fs::write(&output_file, &*output_final) {
        error!("Unable to write to file {}: {}", output_file, e);
    }

    warn!("{}", errors);
}

/// Iterates over files in a directory and processes each one, collecting the results into `output`.
fn process_directory_files(
    dir: &Path,
    output: &Arc<Mutex<String>>,
    base_path: &Path,
    ignore_patterns: &[Pattern],
    delimiter: &str,
    error_count: &Arc<Mutex<HashMap<String, usize>>>,
    config: &Config,
) {
    let files: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !should_ignore(e.path(), base_path, ignore_patterns))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    files.par_iter().for_each(|entry| {
        let path = entry.path();
        let mut local_output = String::new();
        if let Err(e) = process_file(path, &mut local_output, base_path, delimiter, config) {
            let dir = path.parent().unwrap().to_string_lossy().to_string();
            let mut error_count_guard = error_count.lock().unwrap();
            *error_count_guard.entry(dir).or_insert(0) += 1;
            debug!("Failed to process file {}: {}", path.display(), e);
        }
        let mut output_guard = output.lock().unwrap();
        output_guard.push_str(&local_output);
    });
}

/// Processes a single file, adding its contents (or relevant docstrings) to `output`.
/// Respects the `docs_comments_only` setting and can handle Jupyter notebooks if needed.
fn process_file(
    file: &Path,
    output: &mut String,
    base_path: &Path,
    delimiter: &str,
    config: &Config,
) -> Result<(), std::io::Error> {
    let relative_path = file.strip_prefix(base_path).unwrap().to_string_lossy();

    // If the user wants to ignore certain patterns for docstrings
    let docs_ignore_patterns = if let Some(docs_ignore_list) = &config.docs_ignore {
        docs_ignore_list
            .iter()
            .filter_map(|p| Pattern::new(p).ok())
            .collect::<Vec<Pattern>>()
    } else {
        Vec::new()
    };

    let should_ignore_docs_only = docs_ignore_patterns
        .iter()
        .any(|pattern| pattern.matches(&relative_path) || pattern.matches_path(file));

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
            let contents = fs::read_to_string(file)?;
            let signatures = extract_python_signatures(&contents);

            if !signatures.trim().is_empty() {
                output.push_str(&format!("{}{}\n", delimiter, relative_path));
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
                output.push_str(&format!("{}{}\n", delimiter, relative_path));

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
    output.push_str(&format!("{}{}\n", delimiter, relative_path));
    match fs::read_to_string(file) {
        Ok(contents) => output.push_str(&contents),
        Err(e) => {
            output.push_str("[Error reading file]");
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
    process_directory_structure(path, &output, 0, &Vec::new(), "", path);
    let output_guard = output.lock().unwrap();
    output_guard.clone().to_string()
}