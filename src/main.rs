use std::fs;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use clap::{Parser, ArgGroup};
use walkdir::{DirEntry, WalkDir};
use glob::Pattern;
use rayon::prelude::*;
use log::{info, warn, error, debug, LevelFilter};
use env_logger;

/// A simple program to convert a code repository into an LLM prompt
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(group(
    ArgGroup::new("verbosity")
        .args(&["verbose", "quiet"])
))]
struct Cli {
    /// The path to the code repository
    #[arg(short, long)]
    path: String,

    /// Patterns to ignore
    #[arg(short, long)]
    ignore: Vec<String>,

    /// Output file
    #[arg(short, long)]
    output: Option<String>,

    /// Verbose mode
    #[arg(long)]
    verbose: bool,

    /// Quiet mode
    #[arg(long)]
    quiet: bool,

    /// Custom code block delimiters
    #[arg(long, default_value = "```")]
    delimiter: String,

    /// Programming language of the repository
    #[arg(long)]
    language: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        env_logger::builder().filter_level(LevelFilter::Debug).init();
    } else if cli.quiet {
        env_logger::builder().filter_level(LevelFilter::Error).init();
    } else {
        env_logger::builder().filter_level(LevelFilter::Warn).init();
    }

    let repo_path = Path::new(&cli.path);
    let current_dir_name = if cli.path == "." {
        std::env::current_dir().unwrap().file_name().unwrap().to_string_lossy().to_string()
    } else {
        repo_path.file_name().unwrap_or_else(|| OsStr::new("")).to_string_lossy().to_string()
    };

    if repo_path.is_dir() {
        let output = Arc::new(Mutex::new(String::new()));
        let error_count = Arc::new(Mutex::new(HashMap::new()));

        let mut ignore_patterns: Vec<Pattern> = cli.ignore.iter().map(|p| Pattern::new(p).unwrap()).collect();

        // Add default version control patterns
        ignore_patterns.push(Pattern::new(".git").unwrap());

        if let Some(language) = cli.language.as_deref() {
            let default_patterns = get_default_ignore_patterns(language);
            ignore_patterns.extend(default_patterns);
        }

        // First pass: print the directory structure
        {
            let mut output = output.lock().unwrap();
            output.push_str(&format!("{}\n", current_dir_name));
        }
        process_directory_structure(repo_path, &output, 0, &ignore_patterns, "");
        {
            let mut output = output.lock().unwrap();
            output.push_str("\n");
        }

        // Second pass: print the file contents
        process_directory_files(repo_path, &output, repo_path, &ignore_patterns, &cli.delimiter, &error_count);

        let output = output.lock().unwrap();
        if let Some(output_file) = &cli.output {
            if let Err(e) = fs::write(output_file, &*output) {
                error!("Unable to write to file {}: {}", output_file, e);
            }
        } else {
            println!("{}", output);
        }

        let error_count = error_count.lock().unwrap();
        if !error_count.is_empty() {
            for (dir, count) in error_count.iter() {
                warn!("Directory '{}' had {} file(s) that could not be processed", dir, count);
            }
        }
    } else {
        error!("The provided path is not a directory");
    }
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


fn process_directory_structure(dir: &Path, output: &Arc<Mutex<String>>, depth: usize, ignore_patterns: &[Pattern], prefix: &str) {
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
                output.push_str(&format!("{}{}── {}\n", prefix, if is_last { "└" } else { "├" }, dir_name));
            }
            let new_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });
            process_directory_structure(path, output, depth + 1, ignore_patterns, &new_prefix);
        } else if path.is_file() {
            let file_name = path.file_name().unwrap().to_string_lossy();
            let mut output = output.lock().unwrap();
            output.push_str(&format!("{}{}── {}\n", prefix, if is_last { "└" } else { "├" }, file_name));
        }
    }
}

fn process_directory_files(
    dir: &Path, 
    output: &Arc<Mutex<String>>, 
    base_path: &Path, 
    ignore_patterns: &[Pattern], 
    delimiter: &str,
    error_count: &Arc<Mutex<HashMap<String, usize>>>
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
        if let Err(e) = process_file(path, &mut local_output, base_path, delimiter) {
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
    let path_str = path.to_string_lossy();
    for pattern in ignore_patterns {
        if pattern.matches_path(path) || path.components().any(|comp| pattern.matches(&comp.as_os_str().to_string_lossy())) {
            return true;
        }
    }
    false
}

fn process_file(file: &Path, output: &mut String, base_path: &Path, delimiter: &str) -> Result<(), std::io::Error> {
    let relative_path = file.strip_prefix(base_path).unwrap().to_string_lossy();

    output.push_str(&format!("{}{}\n", delimiter, relative_path));

    match fs::read_to_string(file) {
        Ok(contents) => output.push_str(&contents),
        Err(e) => {
            output.push_str("[Error reading file]");
            return Err(e);
        }
    }

    output.push_str(&format!("\n{}\n\n\n", delimiter));
    Ok(())
}
