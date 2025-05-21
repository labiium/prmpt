//! Contains utility functions for handling ignore patterns, directory structure,
//! and file handling for building prompts.

use glob::Pattern; // Still used by process_directory_structure
use std::path::Path; // Only Path is needed from std
use walkdir::WalkDir; // Still used by process_directory_structure
// Removed duplicate glob::Pattern
// Removed fs and io as they are no longer used.

// `get_gitignore_patterns` removed.
// `get_default_ignore_patterns` removed.
// `should_ignore` removed. 
// Note: `process_directory_structure` uses a local `should_ignore` or needs one.
// The subtask states: "You can keep process_directory_structure for now if it's used for the directory peak"
// The `process_directory_structure` in the original code calls the global `should_ignore`.
// This will cause a compile error. For now, I will remove the global `should_ignore` as instructed.
// A subsequent step will likely be to fix `process_directory_structure`.

/// Recursively builds a textual structure visualization for a directory.
/// This is used to output the tree-like structure seen in the generated prompt.
pub fn process_directory_structure(
    dir: &Path,
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    depth: usize,
    ignore_patterns: &[Pattern], // These are glob::Pattern
    prefix: &str,
    base_path: &Path,
) {
    // Local helper for process_directory_structure, as the global one is removed.
    // This replicates the behavior of the removed global `should_ignore`
    // for the specific needs of `process_directory_structure`.
    fn should_ignore_for_structure(path: &Path, base_path: &Path, ignore_patterns: &[Pattern]) -> bool {
        let relative_path = match path.strip_prefix(base_path) {
            Ok(p) => p.to_string_lossy(),
            Err(_) => return false,
        };
        let relative_path_str = relative_path.to_string();
        for pattern in ignore_patterns {
            let pattern_str = pattern.as_str();
            if pattern_str.contains('/') && pattern_str.contains('*') {
                if let Some(last_slash_pos) = pattern_str.rfind('/') {
                    let dir_part = &pattern_str[..=last_slash_pos];
                    let file_pattern = &pattern_str[last_slash_pos + 1..];
                    if file_pattern.is_empty() {
                        if relative_path_str.starts_with(dir_part) { return true; }
                        continue;
                    }
                    if relative_path_str.starts_with(dir_part) {
                        let remaining_path = &relative_path_str[dir_part.len()..];
                        if !remaining_path.contains('/') {
                            if let Ok(file_glob) = Pattern::new(file_pattern) {
                                if file_glob.matches(remaining_path) { return true; }
                            }
                        }
                        continue;
                    }
                }
            }
            if pattern.matches(&relative_path_str) { // Match against string form of relative_path
                return true;
            }
        }
        false
    }

    let entries: Vec<_> = WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        // Use the local helper function here
        .filter(|e| !should_ignore_for_structure(e.path(), base_path, ignore_patterns))
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
