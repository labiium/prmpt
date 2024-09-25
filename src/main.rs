use clap::{ArgGroup, Parser, Subcommand};
use curly::{inject, load_config, run, Config};
use env_logger;
use log::LevelFilter;

/// A simple program to convert a code repository into an LLM prompt and inject code into a repository
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(group(
    ArgGroup::new("verbosity")
        .args(&["verbose", "quiet"])
))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Verbose mode
    #[arg(long)]
    verbose: bool,

    /// Quiet mode
    #[arg(long)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run curly using a configuration from curly.config
    Run {
        /// Name of the configuration to run (defaults to 'base')
        name: Option<String>,
    },
    /// Inject code into the repository from the output file
    Inject {
        /// Path to the output file containing code to inject
        #[arg(
            short,
            long,
            default_value = "output.txt",
            help = "Specify the output file containing the code to inject."
        )]
        input: String,

        #[arg(
            short,
            long,
            default_value = ".",
            help = "Specify the path to the repository to inject the code into."
        )]
        path: String,
    },
    /// Generate the output from a repository
    Generate {
        /// The path to the code repository, default value is current directory
        #[arg(short, long, default_value = ".")]
        path: String,

        /// Patterns to ignore
        #[arg(short, long)]
        ignore: Vec<String>,

        /// Output file
        #[arg(short, long)]
        output: Option<String>,

        /// Custom code block delimiters
        #[arg(long, default_value = "```")]
        delimiter: String,

        /// Programming language of the repository
        #[arg(long)]
        language: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

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

    match cli.command {
        Some(Commands::Run { name }) => {
            // Load the configuration file
            match load_config() {
                Ok(configs) => {
                    let config_name = name.unwrap_or_else(|| "base".to_string());
                    if let Some(config) = configs.get(&config_name) {
                        // Call run() with the configuration
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
        Some(Commands::Inject { input, path }) => {
            if let Err(e) = inject(&input, &path) {
                eprintln!("Error injecting code: {}", e);
            }
        }
        Some(Commands::Generate {
            path,
            ignore,
            output,
            delimiter,
            language,
        }) => {
            let config = Config {
                path: Some(path),
                ignore: Some(ignore),
                output,
                delimiter: Some(delimiter),
                language,
                docs_comments_only: Some(false),
                prompts: None,
                use_gitignore: Some(false),
            };

            run(config);
        }
        None => {
            // Default to 'Run' command with 'base' configuration
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
