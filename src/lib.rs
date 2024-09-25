use glob::Pattern;
use log::{debug, error, info, warn};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

// Import tree-sitter and the Python grammar
use tree_sitter::{Node, Parser};
use tree_sitter_python;

/// Configuration structure that holds various options
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub path: Option<String>,
    pub ignore: Option<Vec<String>>,
    pub output: Option<String>,
    pub delimiter: Option<String>,
    pub language: Option<String>,
    pub prompts: Option<Vec<String>>,
    pub docs_comments_only: Option<bool>,
    pub use_gitignore: Option<bool>, // Added field to control use of .gitignore
    // Additional fields can be added here
}

/// Load configurations from 'curly.yaml'
pub fn load_config() -> Result<HashMap<String, Config>, Box<dyn std::error::Error>> {
    let config_path = Path::new("curly.yaml");
    let contents = fs::read_to_string(config_path)?;
    let configs: HashMap<String, Config> = serde_yaml::from_str(&contents)?;
    Ok(configs)
}

/// Main function to run the program based on the provided configuration
pub fn run(config: Config) {
    let path = config.path.as_deref().unwrap_or(".").to_string();
    let repo_path = Path::new(&path);

    let output_file = config.output.as_deref().unwrap_or("curly.out").to_string();

    let delimiter = config.delimiter.as_deref().unwrap_or("```").to_string();

    // Initialize ignore patterns from the config
    let mut ignore_patterns: Vec<Pattern> = if let Some(ignore_list) = &config.ignore {
        ignore_list
            .iter()
            .map(|p| Pattern::new(p).unwrap())
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
        let mut output = output.lock().unwrap();
        for prompt in prompts {
            output.push_str(&format!("{}\n", prompt));
        }
        output.push_str("\n");
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
            .unwrap_or_else(|| OsStr::new(""))
            .to_string_lossy()
            .to_string()
    };

    // Start building the directory structure output
    {
        let mut output = output.lock().unwrap();
        output.push_str(&format!("{}\n", current_dir_name));
    }
    process_directory_structure(repo_path, &output, 0, &ignore_patterns, "");
    {
        let mut output = output.lock().unwrap();
        output.push_str("\n");
    }

    // Process files in the directory
    process_directory_files(
        repo_path,
        &output,
        repo_path,
        &ignore_patterns,
        &delimiter,
        &error_count,
        &config, // Pass the entire config
    );

    // Write the final output to the specified file
    let output = output.lock().unwrap();
    if let Err(e) = fs::write(&output_file, &*output) {
        error!("Unable to write to file {}: {}", output_file, e);
    }

    // Report any errors encountered during processing
    let error_count = error_count.lock().unwrap();
    if !error_count.is_empty() {
        for (dir, count) in error_count.iter() {
            warn!(
                "Directory '{}' had {} file(s) that could not be processed",
                dir, count
            );
        }
    }
}

/// Retrieve patterns from .gitignore if it exists
fn get_gitignore_patterns(repo_path: &Path) -> Result<Vec<Pattern>, io::Error> {
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

fn get_default_ignore_patterns(language: &str) -> Vec<Pattern> {
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

fn process_directory_structure(
    dir: &Path,
    output: &Arc<Mutex<String>>,
    depth: usize,
    ignore_patterns: &[Pattern],
    prefix: &str,
) {
    let entries: Vec<_> = WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !should_ignore(e.path(), ignore_patterns))
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
            process_directory_structure(path, output, depth + 1, ignore_patterns, &new_prefix);
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

fn process_directory_files(
    dir: &Path,
    output: &Arc<Mutex<String>>,
    base_path: &Path,
    ignore_patterns: &[Pattern],
    delimiter: &str,
    error_count: &Arc<Mutex<HashMap<String, usize>>>,
    config: &Config, // Receive the entire config
) {
    let files: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !should_ignore(e.path(), ignore_patterns))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    files.par_iter().for_each(|entry| {
        let path = entry.path();
        let mut local_output = String::new();
        if let Err(e) = process_file(
            path,
            &mut local_output,
            base_path,
            delimiter,
            config, // Pass the config to process_file
        ) {
            let dir = path.parent().unwrap().to_string_lossy().to_string();
            let mut error_count = error_count.lock().unwrap();
            *error_count.entry(dir).or_insert(0) += 1;
            debug!("Failed to process file {}: {}", path.display(), e);
        }

        let mut output = output.lock().unwrap();
        output.push_str(&local_output);
    });
}

fn should_ignore(path: &Path, ignore_patterns: &[Pattern]) -> bool {
    for pattern in ignore_patterns {
        if pattern.matches_path(path)
            || path
                .components()
                .any(|comp| pattern.matches(&comp.as_os_str().to_string_lossy()))
        {
            return true;
        }
    }
    false
}

fn process_file(
    file: &Path,
    output: &mut String,
    base_path: &Path,
    delimiter: &str,
    config: &Config, // Receive the config
) -> Result<(), std::io::Error> {
    let relative_path = file.strip_prefix(base_path).unwrap().to_string_lossy();

    // Only process files with .py extension if language is Python and 'docs_comments_only' is true
    let extension = file.extension().and_then(OsStr::to_str).unwrap_or("");

    if let Some(true) = config.docs_comments_only {
        if config.language.as_deref().unwrap_or("").to_lowercase() == "python" {
            if extension != "py" {
                return Ok(()); // Skip non-Python files
            }
            // Process Python file to extract signatures and docstrings
            let contents = fs::read_to_string(file)?;
            let signatures = extract_python_signatures(&contents);

            // If signatures are found, add them to the output
            if !signatures.trim().is_empty() {
                output.push_str(&format!("```{}\n", relative_path));
                output.push_str(&signatures);
                output.push_str(&format!("\n```\n\n"));
            }
            return Ok(());
        }
    }

    // Default processing
    output.push_str(&format!("```{}\n", relative_path));

    match fs::read_to_string(file) {
        Ok(contents) => output.push_str(&contents),
        Err(e) => {
            output.push_str("[Error reading file]");
            return Err(e);
        }
    }

    output.push_str(&format!("\n```\n\n"));
    Ok(())
}

// Function to extract function and class signatures along with docstrings from Python code using tree-sitter
fn extract_python_signatures(contents: &str) -> String {
    let mut parser = Parser::new();
    let language = tree_sitter_python::language();
    parser
        .set_language(language)
        .expect("Error loading Python grammar");

    let tree = parser.parse(contents, None).unwrap();
    let root_node = tree.root_node();

    let mut signatures = String::new();

    // Before processing other nodes, check for module-level docstring
    let module_docstring = extract_module_docstring(root_node, contents);
    if !module_docstring.is_empty() {
        signatures.push_str(&module_docstring);
        signatures.push('\n');
    }

    // Process definitions starting from the root node
    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        let child_signatures = extract_definitions(child, contents, 0);
        if !child_signatures.is_empty() {
            signatures.push_str(&child_signatures);
            signatures.push('\n');
        }
    }

    signatures
}

fn extract_module_docstring(root_node: Node, source_code: &str) -> String {
    let mut cursor = root_node.walk();
    for child in root_node.children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let mut expr_cursor = child.walk();
            for expr_child in child.named_children(&mut expr_cursor) {
                if expr_child.kind() == "string" {
                    let docstring_text = expr_child.utf8_text(source_code.as_bytes()).unwrap();
                    let (stripped_docstring, quote_type) = strip_quotes(docstring_text);
                    let indented_docstring = indent_docstring(stripped_docstring, "", quote_type);
                    return indented_docstring;
                }
            }
        } else if !child.kind().starts_with("comment") && !child.kind().starts_with("newline") {
            // If we encounter anything else, no module docstring is present
            break;
        }
    }
    String::new()
}

fn extract_signature_and_docstring(node: Node, source_code: &str, indent_level: usize) -> String {
    let mut signature = String::new();
    let indent = "    ".repeat(indent_level);

    if node.kind() == "function_definition" || node.kind() == "class_definition" {
        signature.push_str(&indent);

        let mut cursor = node.walk();
        let children = node.children(&mut cursor);
        let mut found_signature = false;
        for child in children {
            match child.kind() {
                "decorator" => {
                    let decorator_text = child.utf8_text(source_code.as_bytes()).unwrap();
                    signature.push_str(&indent);
                    signature.push_str(decorator_text);
                    signature.push('\n');
                }
                "def" | "class" => {
                    let text = child.utf8_text(source_code.as_bytes()).unwrap();
                    signature.push_str(text);
                    signature.push(' ');
                }
                "identifier" | "parameters" | ":" => {
                    let text = child.utf8_text(source_code.as_bytes()).unwrap();
                    signature.push_str(text);
                    if child.kind() == ":" {
                        signature.push('\n');
                    }
                    found_signature = true;
                }
                "block" => {
                    // After the signature, look for the docstring
                    if found_signature {
                        let docstring = extract_docstring(child, source_code, indent_level + 1);
                        if !docstring.is_empty() {
                            signature.push_str(&docstring);
                        }
                        break; // We only need the first block
                    }
                }
                _ => {}
            }
        }
    }

    signature
}

fn extract_definitions(node: Node, source_code: &str, indent_level: usize) -> String {
    let mut output = String::new();

    if node.kind() == "function_definition" || node.kind() == "class_definition" {
        let signature = extract_signature_and_docstring(node, source_code, indent_level);
        output.push_str(&signature);
        output.push('\n');
    }

    // If this is a class or function, process its body to find nested definitions
    if node.kind() == "class_definition" || node.kind() == "function_definition" {
        // Find the block node (the body)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "block" {
                // Process the statements inside the block
                let mut block_cursor = child.walk();
                for stmt in child.children(&mut block_cursor) {
                    let stmt_output = extract_definitions(stmt, source_code, indent_level + 1);
                    if !stmt_output.is_empty() {
                        output.push_str(&stmt_output);
                        output.push('\n');
                    }
                }
            }
        }
    }

    output
}

// Modify 'extract_docstring' to correctly handle docstring indentation
fn extract_docstring(block_node: Node, source_code: &str, indent_level: usize) -> String {
    let mut cursor = block_node.walk();
    let mut children = block_node.named_children(&mut cursor);

    let indent = "    ".repeat(indent_level);

    if let Some(first_child) = children.next() {
        if first_child.kind() == "expression_statement" {
            let mut expr_cursor = first_child.walk();
            for string_node in first_child.named_children(&mut expr_cursor) {
                if string_node.kind() == "string" {
                    // This is the docstring
                    let docstring_text = string_node.utf8_text(source_code.as_bytes()).unwrap();
                    // Strip the quotes and get the quote type
                    let (stripped_docstring, quote_type) = strip_quotes(docstring_text);
                    // Indent the docstring with the correct indentation and re-add quotes
                    let indented_docstring = indent_docstring(stripped_docstring, &indent, quote_type);
                    return indented_docstring;
                }
            }
        }
    }
    String::new()
}

// Add 'strip_quotes' function to remove quotes and get the quote type
fn strip_quotes(s: &str) -> (&str, &str) {
    let s = s.trim();
    if (s.starts_with("\"\"\"") && s.ends_with("\"\"\"")) || (s.starts_with("'''") && s.ends_with("'''")) {
        (&s[3..s.len() - 3], &s[..3]) // Return the inner content and the quote type
    } else if (s.starts_with("\"") && s.ends_with("\"")) || (s.starts_with("'") && s.ends_with("'")) {
        (&s[1..s.len() - 1], &s[..1])
    } else {
        (s, "")
    }
}

// Modify 'indent_docstring' to handle quotes properly and fix over-indentation
fn indent_docstring(docstring: &str, indent: &str, quote_type: &str) -> String {
    // Remove common leading whitespace from each line (dedent)
    let dedented_docstring = dedent(docstring);

    // Indent each line with the desired indentation
    let indented_docstring = dedented_docstring
        .lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<String>>()
        .join("\n");

    // Re-add the quotes with proper indentation
    if quote_type.is_empty() {
        indented_docstring
    } else {
        // For single-line docstrings, keep it in one line
        if !dedented_docstring.contains('\n') {
            format!("{}{}{}{}", indent, quote_type, dedented_docstring.trim(), quote_type)
        } else {
            // For multi-line docstrings, place quotes on separate lines
            format!(
                "{}{}\n{}\n{}{}",
                indent,
                quote_type,
                indented_docstring,
                indent,
                quote_type
            )
        }
    }
}

// Update 'dedent' function to handle empty or whitespace-only lines
fn dedent(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    // remove first index if it is empty or whitespace-only
    let lines = if lines.first().map_or(true, |line| line.trim().is_empty()) {
        &lines[1..]
    } else {
        &lines
    };

    // remove all whitespace from the beginning of each line
    let string = lines
        .iter()
        .map(|line| line.trim_start())
        .collect::<Vec<&str>>()
        .join("\n");

    string   
}

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

        // Detect file path in the format: ### `path/to/file`
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

fn extract_path(input: &str) -> &str {
    if input.starts_with("### `") && input.ends_with('`') {
        &input[5..input.len() - 1]
    } else if input.starts_with("**`") && input.ends_with("`**") {
        &input[3..input.len() - 3]
    } else {
        &input[1..input.len() - 1]
    }
}
