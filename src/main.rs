use clap::{ArgGroup, Parser, Subcommand};
use curly::{run, inject, Config};
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
    command: Option<Commands>,  // Make command optional

    /// Verbose mode
    #[arg(long)]
    verbose: bool,

    /// Quiet mode
    #[arg(long)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
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
    /// Inject code into the repository from the output file
    Inject {
        /// Path to the output file containing code to inject
        #[arg(short, long, default_value = "output.txt", help = "Specify the output file containing the code to inject.\n\nProvide the relative file path at the top of the block.\nFollow the relative file path with the code, ensuring there is no additional text in between.\nThink logically, breaking down the problem step by step within the comments of the code.\n\n### Example:\n\n`src/lib.rs`\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```\n\n`src/main.py`\n```python\nprint(\"hello\")\n```\n\nAdd this to your code prompt to be able to use curly to deserialize it.")]
        input: String,

        #[arg(short, long, default_value = ".", help = "Specify the path to the repository to inject the code into.")]
        path: String,
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

    match cli.command.unwrap_or(Commands::Generate {
        path: ".".into(),
        ignore: Vec::new(),
        output: None,
        delimiter: "```".into(),
        language: None,
    }) {
        Commands::Generate {
            path,
            ignore,
            output,
            delimiter,
            language,
        } => {
            let config = Config {
                path,
                ignore,
                output,
                delimiter,
                language,
            };

            run(config);
        }
        Commands::Inject { input, path } => {
            if let Err(e) = inject(&input, &path) {
                eprintln!("Error injecting code: {}", e);
            }
        }
    }
}
