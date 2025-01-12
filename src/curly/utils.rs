//! Contains utility functions for handling ignore patterns, directory structure,
//! and file handling for building prompts.

use glob::Pattern;
use std::{fs, io, path::Path};
use walkdir::WalkDir;

/// Reads a `.gitignore` file from the repository path, returning the list of patterns.
/// Returns an error if the file cannot be read, or an empty list if the file does not exist.
pub fn get_gitignore_patterns(repo_path: &Path) -> Result<Vec<Pattern>, io::Error> {
    let gitignore_path = repo_path.join(".gitignore");
    let mut patterns = Vec::new();
    if gitignore_path.exists() {
        let contents = fs::read_to_string(gitignore_path)?;
        for line in contents.lines() {
            let trimmed_line = line.trim();
            if !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
                patterns.push(Pattern::new(trimmed_line).unwrap());
            }
        }
    }
    Ok(patterns)
}

/// Returns a vector of default patterns to ignore based on the given language name.
pub fn get_default_ignore_patterns(language: &str) -> Vec<Pattern> {
    match language.to_lowercase().as_str() {
        "rust" => vec![
            Pattern::new("target").unwrap(),
            Pattern::new("*.rs.bk").unwrap(),
            Pattern::new("Cargo.lock").unwrap(),
        ],
        "python" => vec![
            Pattern::new("*.pyc").unwrap(),
            Pattern::new("__pycache__").unwrap(),
            Pattern::new(".venv").unwrap(),
            Pattern::new("venv").unwrap(),
        ],
        "javascript" | "typescript" => vec![
            Pattern::new("node_modules").unwrap(),
            Pattern::new("*.min.js").unwrap(),
            Pattern::new("dist").unwrap(),
        ],
        "java" => vec![
            Pattern::new("*.class").unwrap(),
            Pattern::new("*.jar").unwrap(),
            Pattern::new("target").unwrap(),
            Pattern::new(".idea").unwrap(),
        ],
        "c++" => vec![
            Pattern::new("*.o").unwrap(),
            Pattern::new("*.obj").unwrap(),
            Pattern::new("*.exe").unwrap(),
            Pattern::new("build").unwrap(),
        ],
        "go" => vec![
            Pattern::new("*.out").unwrap(),
            Pattern::new("*.test").unwrap(),
            Pattern::new("vendor").unwrap(),
        ],
        "php" => vec![
            Pattern::new("*.log").unwrap(),
            Pattern::new("vendor").unwrap(),
            Pattern::new("composer.lock").unwrap(),
        ],
        _ => vec![],
    }
}

/// Determines whether a given path should be ignored based on a list of glob `Pattern`s.
/// The path is first made relative to `base_path` before matching. If any pattern
/// matches, the file or directory is ignored.
pub fn should_ignore(path: &Path, base_path: &Path, ignore_patterns: &[Pattern]) -> bool {
    let relative_path = match path.strip_prefix(base_path) {
        Ok(rel_path) => rel_path,
        Err(_) => path,
    };

    for pattern in ignore_patterns {
        if pattern.matches_path(relative_path)
            || relative_path
                .components()
                .any(|comp| pattern.matches(&comp.as_os_str().to_string_lossy()))
        {
            return true;
        }
    }
    false
}

/// Recursively builds a textual structure visualization for a directory.
/// This is used to output the tree-like structure seen in the generated prompt.
pub fn process_directory_structure(
    dir: &Path,
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    depth: usize,
    ignore_patterns: &[Pattern],
    prefix: &str,
    base_path: &Path,
) {
    let entries: Vec<_> = WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !should_ignore(e.path(), base_path, ignore_patterns))
        .collect();

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let is_last = i == entries.len() - 1;

        if path.is_dir() {
            let dir_name = path.file_name().unwrap().to_string_lossy();
            {
                let mut output = output.lock().unwrap();
                output.push_str(&format!(
                    "{}{}── {}\n",
                    prefix,
                    if is_last { "└" } else { "├" },
                    dir_name
                ));
            }
            let new_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });
            process_directory_structure(
                path,
                output,
                depth + 1,
                ignore_patterns,
                &new_prefix,
                base_path,
            );
        } else if path.is_file() {
            let file_name = path.file_name().unwrap().to_string_lossy();
            let mut output = output.lock().unwrap();
            output.push_str(&format!(
                "{}{}── {}\n",
                prefix,
                if is_last { "└" } else { "├" },
                file_name
            ));
        }
    }
}
