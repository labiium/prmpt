/// The top-level module file for the 'curly' library.
/// This file re-exports the modules and primary functions
/// so they can be used directly from `main.rs`.
pub mod curly;

pub use curly::config::{Config, load_config};
pub use curly::inject_code::inject;
pub use curly::run::directory_peak;
pub use curly::run::{run, run_and_write};
