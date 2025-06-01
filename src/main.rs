use clap::{Parser, Subcommand, Args};
use env_logger;
use log::LevelFilter;

// Import all necessary functions/types from our library
// These are re-exported at the crate root by src/lib.rs
use curly::{
    Config,
    load_config,
    DEFAULT_CONFIG_KEY, // Added import
    // inject, // Will use Injector::inject
    // run_and_write, // Will use the updated run_and_write that takes a Generator
    Generator,  // Added
    Injector,   // Added
    GenerateOperation, // Added, for clarity or direct use
    InjectOperation,   // Added
};
use curly::run_and_write; // Corrected path for the utility function
use std::path::Path; // For Injector path arguments


/// A simple program to convert a code repository into an LLM prompt and inject code into a repository
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Optional config name to run if no subcommand is provided
    config_name: Option<String>,

    /// Verbose mode
    #[arg(long, global = true)]
    verbose: bool,

    /// Quiet mode
    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generates a prompt from a code repository
    Generate(GenerateArgs),
    /// Injects code into a repository from a file
    Inject(InjectArgs),
    // Potentially a 'Run' subcommand for explicit config execution later
    // Run(RunArgs), 
}

/// Arguments for the `generate` subcommand
#[derive(Args)]
struct GenerateArgs {
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

    /// Only extract documentation and comments
    #[arg(long)]
    docs_comments_only: bool,

    /// Use .gitignore file for ignore patterns
    #[arg(long)]
    use_gitignore: bool,
    
    /// Display outputs from Jupyter notebooks
    #[arg(long)]
    display_outputs: bool,
}

/// Arguments for the `inject` subcommand
#[derive(Args)]
struct InjectArgs {
    /// Path to the file containing code to inject
    #[arg(short, long, default_value = "curly.in")]
    input: String,

    /// Path to the repository to inject the code into
    #[arg(short, long, default_value = ".")]
    path: String,
}

// Define reserved keywords for subcommands to avoid conflict with config names if needed
// This might not be strictly necessary if config loading is handled when no subcommand is parsed.
// const RESERVED_SUBCOMMANDS: &[&str] = &["generate", "inject"];


fn main() {
    let cli = Cli::parse();

    // Set up logging based on verbosity flags
    if cli.verbose {
        env_logger::builder().filter_level(LevelFilter::Debug).init();
    } else if cli.quiet {
        env_logger::builder().filter_level(LevelFilter::Error).init();
    } else {
        env_logger::builder().filter_level(LevelFilter::Warn).init();
    }

    match cli.command {
        Some(Commands::Generate(args)) => {
            let config = Config {
                path: Some(args.path),
                ignore: Some(args.ignore),
                output: args.output,
                delimiter: Some(args.delimiter),
                language: args.language,
                docs_comments_only: Some(args.docs_comments_only),
                docs_ignore: Some(args.docs_ignore),
                use_gitignore: Some(args.use_gitignore),
                display_outputs: Some(args.display_outputs),
                prompts: None, // Prompts are usually part of curly.yaml, not direct CLI flags here.
            };
            let generator = Generator::default();
            if let Err(e) = run_and_write(&generator, &config) {
                eprintln!("Error generating prompt: {:?}", e); // Use {:?} for anyhow::Error
                std::process::exit(1);
            }
        }
        Some(Commands::Inject(args)) => {
            let injector = Injector::default();
            if let Err(e) = injector.inject(Path::new(&args.input), Path::new(&args.path)) {
                eprintln!("Error injecting code: {:?}", e); // Use {:?} for anyhow::Error
                std::process::exit(1);
            }
        }
        None => {
            // No subcommand was provided, try to load config based on `cli.config_name`
            let config_to_load = cli.config_name.as_deref().unwrap_or(DEFAULT_CONFIG_KEY);
            match load_config() {
                Ok(configs) => {
                    if let Some(config) = configs.get(config_to_load) {
                        let generator = Generator::default();
                        // Use the updated run_and_write with the loaded config
                        if let Err(e) = run_and_write(&generator, &config.clone()) {
                             eprintln!("Error generating prompt from config '{}': {:?}", config_to_load, e);
                             std::process::exit(1);
                        }
                    } else {
                        eprintln!("Configuration '{}' not found in curly.yaml.", config_to_load);
                        // Show help if default 'base' config not found and no args given
                        if cli.config_name.is_none() {
                             // This is tricky, clap might have already shown help if no args were truly given.
                             // If `curly` is run with no args, `cli.config_name` is None.
                             // If `curly some_config` is run, `cli.config_name` is Some("some_config").
                             // If `clap` fails to parse `some_config` as a subcommand, it might treat it as `cli.config_name`.
                             // For now, this error message is sufficient.
                             // A more robust solution would be to use `#[command(flatten)] args: ConfigArgs`
                             // and `#[command(flatten)] command: Option<Commands>`
                             // then check if command is None and ConfigArgs are default to imply "load base config".
                             // Or require `curly run base` explicitly.
                        }
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load curly.yaml: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

// Old InjectCli and GenerateCli structs are removed as their fields are now in InjectArgs and GenerateArgs.
