//! Contains utility functions for directory structure visualization.
//! The ignore logic has been unified with the main processing in run.rs.

use glob::Pattern;
use std::path::Path;
use walkdir::WalkDir;

/// Recursively builds a textual structure visualization for a directory.
/// This is used to output the tree-like structure seen in the generated prompt.
///
/// Time complexity: O(n) where n is the number of files/directories in the tree
/// Space complexity: O(d * w) where d is depth and w is average width of directories
pub fn process_directory_structure(
    dir: &Path,
    output: &std::sync::Arc<std::sync::Mutex<String>>,
    _depth: usize,
    ignore_patterns: &[Pattern], // These are glob::Pattern for backward compatibility
    prefix: &str,
    base_path: &Path,
) {
    // Local helper for process_directory_structure.
    // This maintains the existing behavior for structure visualization
    // while the main file processing uses the unified ignore system.
    fn should_ignore_for_structure(
        path: &Path,
        base_path: &Path,
        ignore_patterns: &[Pattern],
    ) -> bool {
        let relative_path = match path.strip_prefix(base_path) {
            Ok(p) => p.to_string_lossy(),
            Err(_) => return false,
        };
        let relative_path_str = relative_path.to_string();

        for pattern in ignore_patterns {
            let pattern_str = pattern.as_str();

            // Handle complex patterns with directories and wildcards
            if pattern_str.contains('/') && pattern_str.contains('*') {
                if let Some(last_slash_pos) = pattern_str.rfind('/') {
                    let dir_part = &pattern_str[..=last_slash_pos];
                    let file_pattern = &pattern_str[last_slash_pos + 1..];

                    if file_pattern.is_empty() {
                        if relative_path_str.starts_with(dir_part) {
                            return true;
                        }
                        continue;
                    }

                    if let Some(remaining_path) = relative_path_str.strip_prefix(dir_part) {
                        if !remaining_path.contains('/') {
                            if let Ok(file_glob) = Pattern::new(file_pattern) {
                                if file_glob.matches(remaining_path) {
                                    return true;
                                }
                            }
                        }
                        continue;
                    }
                }
            }

            // Simple pattern matching
            if pattern.matches(&relative_path_str) {
                return true;
            }
        }
        false
    }

    let mut entries: Vec<_> = WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !should_ignore_for_structure(e.path(), base_path, ignore_patterns))
        .collect();

    // Ensure deterministic ordering of directory traversal
    entries.sort_by_key(|e| e.path().file_name().map(|n| n.to_os_string()));

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
                _depth + 1,
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
