/// The top-level module file for the 'prmpt' library.
/// This file re-exports the modules and primary functions
/// so they can be used directly from `main.rs`.
pub mod prmpt;

pub use prmpt::config::{load_config, Config, DEFAULT_CONFIG_KEY};
// pub use prmpt::inject_code::inject; // Replaced by Injector
pub use prmpt::inject_code::Injector; // Added

pub use prmpt::run::directory_peak;
// pub use prmpt::run::run;             // Replaced by Generator
pub use prmpt::run::run_and_write;
pub use prmpt::run::Generator; // Added // run_and_write now uses GenerateOperation

pub use prmpt::traits::{GenerateOperation, InjectOperation}; // Added
