//! Provides functionality for injecting code from a generated file (e.g., "curly.out")
//! back into the repository at specified file paths.

use log::{error, info, warn};
use std::{
    fs, // Removed io
    path::Path,
};
use rand::Rng;
use rand::distr::Alphanumeric; 
use rand::rngs::ThreadRng;
use crate::curly::traits::InjectOperation; // Import the trait
use anyhow::{Context, Error}; // For the Result type & context

/// Struct for implementing the InjectOperation trait.
#[derive(Default)]
pub struct Injector;

impl InjectOperation for Injector {
    /// Injects code from a specified input file into a target repository path.
    /// This method encapsulates the original `inject` function's logic.
    fn inject(&self, input_path: &Path, repo_path: &Path) -> Result<(), Error> {
        // Canonicalize the base repo path
        let base_path_canon = fs::canonicalize(repo_path)
            .with_context(|| format!("Failed to canonicalize base repository path: '{}'", repo_path.display()))?;
        info!("Canonicalized base repository path: {:?}", base_path_canon);

        let contents = fs::read_to_string(input_path)
            .with_context(|| format!("Failed to read input file: '{}'", input_path.display()))?;
        
        let delimiter = "```";
        let mut lines = contents.lines();
        let mut current_file_target_path_str: Option<String> = None;
        let mut code_block = String::new();
        let mut in_code_block = false;

        info!("Starting to process the input file for injection: {:?}", input_path);

        while let Some(line) = lines.next() {
            if !in_code_block
                && ((line.trim_start().starts_with("### `") && line.trim_end().ends_with('`'))
                    || (line.trim_start().starts_with("**`") && line.trim_end().ends_with("`**"))
                    || (line.trim_start().starts_with('`')
                        && line.trim_end().ends_with('`')
                        && line.len() > 3))
            {
                let extracted_path_str = extract_path(line);
                if !extracted_path_str.trim().is_empty() {
                    current_file_target_path_str = Some(extracted_path_str.to_string());
                    info!("Detected relative file path for injection: {:?}", current_file_target_path_str);
                } else {
                    warn!("Detected an empty file path! Skipping...");
                    current_file_target_path_str = None;
                }
            }
            else if line.trim_start().starts_with(delimiter) {
                // Handle optional path on the same line as the opening code fence
                if !in_code_block {
                    if let Some(path_on_fence) = extract_path_from_fence(line, delimiter) {
                        current_file_target_path_str = Some(path_on_fence.to_string());
                        info!("Detected relative file path for injection: {:?}", current_file_target_path_str);
                    }
                }
                in_code_block = !in_code_block;
                if !in_code_block { // Closing a code block
                    if let Some(ref target_file_rel_str) = current_file_target_path_str {
                        if code_block.is_empty() {
                            warn!("Empty code block detected for path: {:?}", target_file_rel_str);
                            current_file_target_path_str = None; 
                            continue;
                        }

                        let full_target_path = base_path_canon.join(target_file_rel_str);
                        let target_filename = match full_target_path.file_name() {
                            Some(name) => name.to_os_string(),
                            None => {
                                error!("Could not extract filename from path: {:?}", full_target_path);
                                current_file_target_path_str = None; code_block.clear(); continue;
                            }
                        };
                        
                        let parent_dir_for_file = full_target_path.parent().unwrap_or_else(|| Path::new(""));

                        // Ensure parent directory exists, trying to canonicalize it.
                        let canonical_parent_dir = if parent_dir_for_file.as_os_str().is_empty() || parent_dir_for_file == base_path_canon.as_path() {
                             base_path_canon.clone()
                        } else if !parent_dir_for_file.is_absolute() && parent_dir_for_file.starts_with(&base_path_canon) {
                            // If parent_dir_for_file is already relative to base_path_canon correctly
                            fs::create_dir_all(&parent_dir_for_file)
                                .with_context(|| format!("Failed to create parent directory: {:?}", parent_dir_for_file))?;
                            fs::canonicalize(&parent_dir_for_file)
                                .with_context(|| format!("Failed to canonicalize parent directory: {:?}", parent_dir_for_file))?
                        } else {
                             // This case handles when full_target_path.parent() is absolute or needs careful joining
                             // For simplicity, assume it's relative or directly under base_path_canon as handled by base_path_canon.join()
                             // If it's already absolute and within repo_path, fs::create_dir_all should handle it.
                             // This part might need more robust handling for complex symlink scenarios outside typical usage.
                             fs::create_dir_all(parent_dir_for_file)
                                 .with_context(|| format!("Failed to create parent directory: {:?}", parent_dir_for_file))?;
                             fs::canonicalize(parent_dir_for_file)
                                 .with_context(|| format!("Failed to canonicalize parent directory: {:?}", parent_dir_for_file))?
                        };


                        let final_file_path_canon = canonical_parent_dir.join(&target_filename);
                        info!("Final canonical file path for injection: {:?}", final_file_path_canon);

                        let mut rng = ThreadRng::default();
                        let random_string: String = (&mut rng)
                            .sample_iter(&Alphanumeric)
                            .take(6)
                            .map(char::from)
                            .collect();
                        let temp_filename = format!(".{}.tmp.{}", target_filename.to_string_lossy(), random_string);
                        let temp_file_path = canonical_parent_dir.join(temp_filename);

                        info!("Writing to temporary file: {:?}", temp_file_path);
                        match fs::write(&temp_file_path, code_block.trim_end()) {
                            Ok(_) => {
                                info!("Successfully wrote to temporary file. Renaming to: {:?}", final_file_path_canon);
                                if let Err(e) = fs::rename(&temp_file_path, &final_file_path_canon) {
                                    let rename_err = Error::new(e).context(format!("Failed to rename temporary file {:?} to {:?}", temp_file_path, final_file_path_canon));
                                    error!("{:?}", rename_err);
                                    if let Err(remove_err) = fs::remove_file(&temp_file_path) {
                                        error!("Additionally, failed to remove temporary file {:?}: {}", temp_file_path, remove_err);
                                    }
                                    // Decide if this is a critical error for the whole inject operation
                                    // For now, log and continue, but one could return rename_err here.
                                } else {
                                    info!("Successfully injected code into {:?}", final_file_path_canon);
                                }
                            }
                            Err(e) => {
                                error!("Failed to write to temporary file {:?}: {}", temp_file_path, e);
                                if temp_file_path.exists() {
                                    if let Err(remove_err) = fs::remove_file(&temp_file_path) {
                                        error!("Additionally, failed to remove temporary file {:?}: {}", temp_file_path, remove_err);
                                    }
                                }
                            }
                        }
                        code_block.clear();
                    } else {
                        warn!("Code block closed without a file path being set!");
                    }
                    current_file_target_path_str = None; 
                } else {
                    info!("Entering a code block...");
                    code_block.clear();
                }
            }
            else if !in_code_block {
                let trimmed = line.trim();
                if !trimmed.is_empty()
                    && !trimmed.starts_with('#')
                    && !trimmed.starts_with(delimiter)
                    && !trimmed.contains(' ')
                {
                    current_file_target_path_str = Some(trimmed.to_string());
                    info!("Detected relative file path for injection: {:?}", current_file_target_path_str);
                }
            }
            else if in_code_block {
                code_block.push_str(line);
                code_block.push('\n');
            }
        }
        info!("Finished processing the input file for injection.");
        Ok(())
    }
}

// The old `inject` function is removed as its logic is now in `Injector::inject`.

/// Helper function for extracting the path from a line
/// that looks like `### `path/to/file` or **`path/to/file`**, etc.
fn extract_path(input: &str) -> &str {
    // Trim leading/trailing whitespace which might affect path extraction
    let trimmed_input = input.trim();
    if trimmed_input.starts_with("### `") && trimmed_input.ends_with('`') {
        &trimmed_input[5..trimmed_input.len() - 1]
    } else if trimmed_input.starts_with("**`") && trimmed_input.ends_with("`**") {
        &trimmed_input[3..trimmed_input.len() - 3]
    } else if trimmed_input.starts_with('`') && trimmed_input.ends_with('`') { // Generic backtick case
        &trimmed_input[1..trimmed_input.len() - 1]
    } else {
        trimmed_input // Fallback if no known pattern matches, assume the line itself is the path
    }
} // Added missing closing brace

/// Attempt to extract a file path from a line that begins with the code block
/// delimiter. This supports Curly's own output format where the file path
/// directly follows the opening fence, e.g. "```src/lib.rs" or
/// "```rust src/lib.rs".
fn extract_path_from_fence<'a>(line: &'a str, delimiter: &str) -> Option<&'a str> {
    let remainder = line.trim_start().strip_prefix(delimiter)?.trim();
    if remainder.is_empty() {
        return None;
    }
    // If multiple tokens exist after the delimiter, assume the last one is the path
    let tokens: Vec<&str> = remainder.split_whitespace().collect();
    if tokens.len() == 1 {
        let t = tokens[0];
        if t.contains('/') || t.contains('.') {
            return Some(t);
        }
        return None;
    }
    tokens.last().copied()
}
