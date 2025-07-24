//! Provides functionality for injecting code from a generated file (e.g., "prmpt.out")
//! back into the repository at specified file paths.

use crate::prmpt::traits::InjectOperation; // Import the trait
use anyhow::{Context, Error, Result};
use log::{error, info, warn};
use rand::distr::Alphanumeric;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::{
    fs,
    path::{Path, PathBuf},
}; // For the Result type & context

/// Parser states for processing injection file content
#[derive(Debug, PartialEq)]
enum ParserState {
    ExpectingPath,
    InCodeBlock,
}

/// Represents a parsed code block with its target file path
#[derive(Debug)]
struct CodeBlock {
    target_path: String,
    content: String,
}

/// Struct for implementing the InjectOperation trait.
#[derive(Default)]
pub struct Injector;

/// Parser for processing injection file content
struct InjectionParser {
    state: ParserState,
    current_target_path: Option<String>,
    current_code_block: String,
    blocks: Vec<CodeBlock>,
}

impl InjectionParser {
    fn new() -> Self {
        Self {
            state: ParserState::ExpectingPath,
            current_target_path: None,
            current_code_block: String::new(),
            blocks: Vec::new(),
        }
    }

    fn parse(mut self, content: &str) -> Vec<CodeBlock> {
        let delimiter = "```";

        for line in content.lines() {
            match self.state {
                ParserState::ExpectingPath => {
                    if line.trim_start().starts_with(delimiter) {
                        // Handle optional path on the same line as the opening code fence
                        if let Some(path_on_fence) = extract_path_from_fence(line, delimiter) {
                            self.current_target_path = Some(path_on_fence.to_string());
                        }
                        self.state = ParserState::InCodeBlock;
                        self.current_code_block.clear();
                    } else if self.is_path_line(line) {
                        let extracted_path = extract_path(line);
                        if !extracted_path.trim().is_empty() {
                            self.current_target_path = Some(extracted_path.to_string());
                        } else {
                            warn!("Detected an empty file path! Skipping...");
                            self.current_target_path = None;
                        }
                    } else {
                        // Check for inline file paths
                        let trimmed = line.trim();
                        if !trimmed.is_empty()
                            && !trimmed.starts_with('#')
                            && !trimmed.contains(' ')
                        {
                            self.current_target_path = Some(trimmed.to_string());
                        }
                    }
                }
                ParserState::InCodeBlock => {
                    if line.trim_start().starts_with(delimiter) {
                        // End of code block
                        self.finalize_current_block();
                        self.state = ParserState::ExpectingPath;
                    } else {
                        self.current_code_block.push_str(line);
                        self.current_code_block.push('\n');
                    }
                }
            }
        }

        // Handle case where file ends without closing delimiter
        if self.state == ParserState::InCodeBlock {
            self.finalize_current_block();
        }

        self.blocks
    }

    fn is_path_line(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        (trimmed.starts_with("### `") && line.trim_end().ends_with('`'))
            || (trimmed.starts_with("**`") && line.trim_end().ends_with("`**"))
            || (trimmed.starts_with('`') && line.trim_end().ends_with('`') && line.len() > 3)
    }

    fn finalize_current_block(&mut self) {
        if let Some(ref target_path) = self.current_target_path {
            if !self.current_code_block.trim().is_empty() {
                self.blocks.push(CodeBlock {
                    target_path: target_path.clone(),
                    content: self.current_code_block.trim_end().to_string(),
                });
            } else {
                warn!("Empty code block detected for path: {:?}", target_path);
            }
        } else {
            warn!("Code block closed without a file path being set!");
        }
        self.current_target_path = None;
        self.current_code_block.clear();
    }
}

impl InjectOperation for Injector {
    /// Injects code from a specified input file into a target repository path.
    /// This method encapsulates the original `inject` function's logic with security improvements.
    fn inject(&self, input_path: &Path, repo_path: &Path) -> Result<(), Error> {
        // Canonicalize the base repo path
        let base_path_canon = fs::canonicalize(repo_path).with_context(|| {
            format!(
                "Failed to canonicalize base repository path: '{}'",
                repo_path.display()
            )
        })?;
        info!("Canonicalized base repository path: {:?}", base_path_canon);

        let contents = fs::read_to_string(input_path)
            .with_context(|| format!("Failed to read input file: '{}'", input_path.display()))?;

        info!(
            "Starting to process the input file for injection: {:?}",
            input_path
        );

        // Parse the input file using the new parser
        let parser = InjectionParser::new();
        let code_blocks = parser.parse(&contents);

        // Process each code block
        for block in code_blocks {
            self.inject_code_block(&block, &base_path_canon)?;
        }

        info!("Finished processing the input file for injection.");
        Ok(())
    }
}

impl Injector {
    /// Injects a single code block into the target file system
    /// Time complexity: O(1) for path validation, O(n) for file I/O where n is content size
    /// Space complexity: O(m) where m is the size of the code block content
    fn inject_code_block(&self, block: &CodeBlock, base_path_canon: &PathBuf) -> Result<()> {
        // Construct the full target path
        let full_target_path = base_path_canon.join(&block.target_path);

        // Extract filename
        let target_filename = match full_target_path.file_name() {
            Some(name) => name.to_os_string(),
            None => {
                error!(
                    "Could not extract filename from path: {:?}",
                    full_target_path
                );
                return Ok(()); // Skip this file and continue
            }
        };

        let parent_dir_for_file = full_target_path.parent().unwrap_or_else(|| Path::new(""));

        // Ensure parent directory exists and canonicalize it
        let canonical_parent_dir = if parent_dir_for_file.as_os_str().is_empty()
            || parent_dir_for_file == base_path_canon.as_path()
        {
            base_path_canon.clone()
        } else {
            fs::create_dir_all(parent_dir_for_file).with_context(|| {
                format!(
                    "Failed to create parent directory: {:?}",
                    parent_dir_for_file
                )
            })?;
            fs::canonicalize(parent_dir_for_file).with_context(|| {
                format!(
                    "Failed to canonicalize parent directory: {:?}",
                    parent_dir_for_file
                )
            })?
        };

        let final_file_path_canon = canonical_parent_dir.join(&target_filename);

        // SECURITY CHECK: Verify the final path is still within the base repository
        if !final_file_path_canon.starts_with(base_path_canon) {
            error!(
                "Security risk: Attempted to write to a path outside the repository: {:?}. \
                Target path: {:?}, Base path: {:?}",
                final_file_path_canon, block.target_path, base_path_canon
            );
            return Ok(()); // Skip this file and continue to the next
        }

        info!(
            "Final canonical file path for injection: {:?}",
            final_file_path_canon
        );

        // Generate a secure temporary filename
        let mut rng = ThreadRng::default();
        let random_string: String = (&mut rng)
            .sample_iter(&Alphanumeric)
            .take(8) // Increased from 6 to 8 for better collision resistance
            .map(char::from)
            .collect();
        let temp_filename = format!(
            ".{}.tmp.{}",
            target_filename.to_string_lossy(),
            random_string
        );
        let temp_file_path = canonical_parent_dir.join(temp_filename);

        info!("Writing to temporary file: {:?}", temp_file_path);

        // Write to temporary file and atomically rename
        fs::write(&temp_file_path, &block.content)
            .with_context(|| format!("Failed to write to temporary file: {:?}", temp_file_path))?;

        info!(
            "Successfully wrote to temporary file. Renaming to: {:?}",
            final_file_path_canon
        );

        fs::rename(&temp_file_path, &final_file_path_canon).with_context(|| {
            // Clean up temporary file on failure
            let _ = fs::remove_file(&temp_file_path);
            format!(
                "Failed to rename temporary file {:?} to {:?}",
                temp_file_path, final_file_path_canon
            )
        })?;

        info!(
            "Successfully injected code into {:?}",
            final_file_path_canon
        );
        Ok(())
    }
}

/// Helper function for extracting the path from a line
/// that looks like `### `path/to/file` or **`path/to/file`**, etc.
fn extract_path(input: &str) -> &str {
    // Trim leading/trailing whitespace which might affect path extraction
    let trimmed_input = input.trim();
    if trimmed_input.starts_with("### `") && trimmed_input.ends_with('`') {
        &trimmed_input[5..trimmed_input.len() - 1]
    } else if trimmed_input.starts_with("**`") && trimmed_input.ends_with("`**") {
        &trimmed_input[3..trimmed_input.len() - 3]
    } else if trimmed_input.starts_with('`') && trimmed_input.ends_with('`') {
        // Generic backtick case
        &trimmed_input[1..trimmed_input.len() - 1]
    } else {
        trimmed_input // Fallback if no known pattern matches, assume the line itself is the path
    }
}

/// Attempt to extract a file path from a line that begins with the code block
/// delimiter. This supports prmpt's own output format where the file path
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
