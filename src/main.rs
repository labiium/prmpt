use clap::Parser;
use env_logger;
use log::LevelFilter;

// Import all necessary functions/types from our library
use curly::{
    Config,
    inject,      // The inject function
    load_config, // The load_config function
    run,         // The run function
};

/// A simple program to convert a code repository into an LLM prompt and inject code into a repository
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The command or config name
    command_or_config: Option<String>,

    /// Additional arguments
    #[arg(allow_hyphen_values = true)]
    args: Vec<String>,

    /// Verbose mode
    #[arg(long)]
    verbose: bool,

    /// Quiet mode
    #[arg(long)]
    quiet: bool,
}

// Define reserved keywords to prevent them from being used as config names
const RESERVED_KEYWORDS: &[&str] = &["inject", "generate", "help", "--help", "-h"];

fn main() {
    let cli = Cli::parse();

    // Set up logging based on verbosity flags
    if cli.verbose {
        env_logger::builder()
            .filter_level(LevelFilter::Debug)
            .init();
    } else if cli.quiet {
        env_logger::builder()
            .filter_level(LevelFilter::Error)
            .init();
    } else {
        env_logger::builder().filter_level(LevelFilter::Warn).init();
    }

    let command_or_config = cli.command_or_config.as_deref();

    match command_or_config {
        Some("inject") => {
            // Parse arguments for 'inject' command
            let inject_cli = match InjectCli::try_parse_from(
                std::iter::once("inject").chain(cli.args.iter().map(|s| s.as_str())),
            ) {
                Ok(cli) => cli,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = inject(&inject_cli.input, &inject_cli.path) {
                eprintln!("Error injecting code: {}", e);
            }
        }
        Some("generate") => {
            // Parse arguments for 'generate' command
            let generate_cli = match GenerateCli::try_parse_from(
                std::iter::once("generate").chain(cli.args.iter().map(|s| s.as_str())),
            ) {
                Ok(cli) => cli,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            // Build a Config object from the parsed CLI arguments
            let config = Config {
                path: Some(generate_cli.path),
                ignore: Some(generate_cli.ignore),
                output: generate_cli.output,
                delimiter: Some(generate_cli.delimiter),
                language: generate_cli.language,
                docs_comments_only: Some(false),
                docs_ignore: Some(generate_cli.docs_ignore),
                use_gitignore: Some(false),
                display_outputs: Some(false),
                prompts: None,
            };

            run(config);
        }
        Some(reserved) if RESERVED_KEYWORDS.contains(&reserved) => {
            // Prevent usage of reserved keywords as config names
            eprintln!(
                "'{}' is a reserved keyword and cannot be used as a config name.",
                reserved
            );
            std::process::exit(1);
        }
        Some(config_name) => {
            // Load and run the configuration named 'config_name'
            match load_config() {
                Ok(configs) => {
                    if let Some(config) = configs.get(config_name) {
                        run(config.clone());
                    } else {
                        eprintln!("Configuration '{}' not found in curly.yaml", config_name);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load curly.yaml: {}", e);
                }
            }
        }
        None => {
            // No command or config name provided, run default config 'base'
            match load_config() {
                Ok(configs) => {
                    if let Some(config) = configs.get("base") {
                        run(config.clone());
                    } else {
                        eprintln!("Configuration 'base' not found in curly.yaml");
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load curly.yaml: {}", e);
                }
            }
        }
    }
}

/// CLI parser for the `inject` command
#[derive(Parser)]
struct InjectCli {
    /// Path to the output file containing code to inject
    #[arg(short, long, default_value = "curly.in")]
    input: String,

    /// Path to the repository to inject the code into
    #[arg(short, long, default_value = ".")]
    path: String,
}

/// CLI parser for the `generate` command
#[derive(Parser)]
struct GenerateCli {
    /// The path to the code repository, default value is current directory
    #[arg(short, long, default_value = ".")]
    path: String,

    /// Patterns to ignore
    #[arg(short, long)]
    ignore: Vec<String>,

    /// Patterns to ignore in documentation comments
    #[arg(long)]
    docs_ignore: Vec<String>,

    /// Output file
    #[arg(short, long)]
    output: Option<String>,

    /// Custom code block delimiters
    #[arg(long, default_value = "```")]
    delimiter: String,

    /// Programming language of the repository
    #[arg(long)]
    language: Option<String>,
}
