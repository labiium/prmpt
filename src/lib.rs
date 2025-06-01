/// The top-level module file for the 'curly' library.
/// This file re-exports the modules and primary functions
/// so they can be used directly from `main.rs`.
pub mod curly;

pub use curly::config::{Config, load_config, DEFAULT_CONFIG_KEY};
// pub use curly::inject_code::inject; // Replaced by Injector
pub use curly::inject_code::Injector;   // Added

pub use curly::run::directory_peak;
// pub use curly::run::run;             // Replaced by Generator
pub use curly::run::Generator;         // Added
pub use curly::run::run_and_write;     // run_and_write now uses GenerateOperation

pub use curly::traits::{GenerateOperation, InjectOperation}; // Added
