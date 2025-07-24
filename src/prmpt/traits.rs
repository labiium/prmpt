use crate::Config;
use anyhow::Error;
use std::path::Path; // Using anyhow::Error

/// Trait for the 'generate' operation.
pub trait GenerateOperation {
    /// Runs the generation process based on the provided configuration.
    ///
    /// # Arguments
    /// * `config`: A reference to the `Config` object specifying generation parameters.
    ///
    /// # Returns
    /// A `Result` containing a tuple of (generated_output_string, error_messages_vector) on success,
    /// or an `anyhow::Error` on critical failure.
    fn run(&self, config: &Config) -> Result<(String, Vec<String>), Error>;
}

/// Trait for the 'inject' operation.
pub trait InjectOperation {
    /// Injects code from a specified input file into a target repository path.
    ///
    /// # Arguments
    /// * `input_path`: Path to the file containing the code blocks to be injected.
    /// * `repo_path`: Path to the base of the repository where code will be injected.
    ///
    /// # Returns
    /// An `Ok(())` on successful injection of all parts, or an `anyhow::Error` if
    /// a critical error occurs or any part of the injection fails.
    fn inject(&self, input_path: &Path, repo_path: &Path) -> Result<(), Error>;
}
