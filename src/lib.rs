/// The top-level module file for the 'curly' library.
/// This file re-exports the modules and primary functions
/// so they can be used directly from `main.rs`.
pub mod curly;

pub use curly::config::{load_config, Config};
pub use curly::inject_code::inject;
pub use curly::run::run;
