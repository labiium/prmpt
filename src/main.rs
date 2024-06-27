use clap::{ArgGroup, Parser};
use curly::{run, Config};
use env_logger;
use log::LevelFilter;

/// A simple program to convert a code repository into an LLM prompt
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(group(
    ArgGroup::new("verbosity")
        .args(&["verbose", "quiet"])
))]
struct Cli {
    /// The path to the code repository give default value as current directory
    // #[arg(short, long)]
    // path: String,

    #[arg(short, long, default_value = ".")]
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

    let config = Config {
        path: cli.path,
        ignore: cli.ignore,
        output: cli.output,
        delimiter: cli.delimiter,
        language: cli.language,
    };

    run(config);
}
