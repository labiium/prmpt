//! Provides functionality for injecting code from a generated file (e.g., "curly.out")
//! back into the repository at specified file paths.

use log::{error, info, warn};
use std::{fs, io, path::PathBuf};

/// Injects code from the specified `input` file into the repository at `path`.
///
/// This function reads the entire input file, looking for sections that are wrapped
/// in code-block delimiters (by default "```"), and writes each code block to
/// the corresponding file path.
///
/// # Parameters
/// - `input`: The path to the file containing the generated code blocks.
/// - `path`: The base path to which file paths in the input file will be resolved.
///
/// # Returns
/// - `Result<(), io::Error>`: An `Ok` value if injection succeeds, or an error
///   otherwise.
pub fn inject(input: &str, path: &str) -> Result<(), io::Error> {
    // Read the input file content
    let contents = fs::read_to_string(input)?;
    let delimiter = "```";
    let mut lines = contents.lines();
    let mut file_path: Option<PathBuf> = None;
    let mut code_block = String::new();
    let mut in_code_block = false;

    info!("Starting to process the input file...");

    while let Some(line) = lines.next() {
        info!("Processing line: {:?}", line);

        // Detect file path in the format:
        //   ### `path/to/file`
        //   **`path/to/file`**
        //   or simply `path/to/file`
        if !in_code_block
            && ((line.trim_start().starts_with("### `") && line.trim_end().ends_with('`'))
                || (line.trim_start().starts_with("**`") && line.trim_end().ends_with("`**"))
                || (line.trim_start().starts_with('`')
                    && line.trim_end().ends_with('`')
                    && line.len() > 3))
        {
            let relative_path = format!("{}/{}", path, extract_path(line));
            if !relative_path.trim().is_empty() {
                file_path = Some(PathBuf::from(relative_path.trim()));
                info!("Detected file path: {:?}", file_path);
            } else {
                warn!("Detected an empty file path! Skipping...");
            }
        }
        // Detect code block delimiter
        else if line.trim_start().starts_with(delimiter) {
            in_code_block = !in_code_block;
            if !in_code_block {
                // Closing a code block
                if let Some(ref path) = file_path {
                    if !code_block.is_empty() {
                        if let Some(parent) = path.parent() {
                            info!("Creating directory: {:?}", parent);
                            if let Err(e) = fs::create_dir_all(parent) {
                                error!("Failed to create directory: {:?}", e);
                                return Err(e);
                            }
                        }
                        info!("Writing to file: {:?}", path);
                        if let Err(e) = fs::write(path, code_block.trim_end()) {
                            error!("Failed to write to file: {:?}", e);
                            return Err(e);
                        }
                        code_block.clear();
                    } else {
                        warn!("Empty code block detected for path: {:?}", path);
                    }
                } else {
                    warn!("Code block closed without a file path!");
                }
                file_path = None; // Reset file_path after writing the file
            } else {
                // Opening a new code block
                info!("Entering a code block...");
                code_block.clear(); // Clear the code block string when entering a new code block
            }
        }
        // Add line to code block
        else if in_code_block {
            code_block.push_str(line);
            code_block.push('\n');
        }
        // Outside code block
        else {
            info!("Outside code block and no file path detected, continuing...");
        }
    }

    info!("Finished processing the input file.");
    Ok(())
}

/// Helper function for extracting the path from a line
/// that looks like `### `path/to/file` or **`path/to/file`**, etc.
fn extract_path(input: &str) -> &str {
    if input.starts_with("### `") && input.ends_with('`') {
        &input[5..input.len() - 1]
    } else if input.starts_with("**`") && input.ends_with("`**") {
        &input[3..input.len() - 3]
    } else {
        &input[1..input.len() - 1]
    }
}
