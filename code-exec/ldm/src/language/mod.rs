use async_trait::async_trait;

pub mod go;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

use crate::{
    error::Result,
    types::{Package, Tool},
};

#[async_trait]
pub trait LanguageProvider: Send + Sync {
    /// Returns the name of the language
    fn name(&self) -> &'static str;

    /// Returns a list of required tools for this language
    fn required_tools(&self) -> Vec<Tool>;

    /// Returns a list of required packages for this language
    fn required_packages(&self) -> Vec<Package>;

    /// Validates that all required tools are installed and working
    async fn validate_installation(&self) -> Result<()>;

    /// Sets up the environment for this language (e.g., virtualenv for Python)
    async fn setup_environment(&mut self) -> Result<()>;

    /// Returns the command to run a file in this language
    fn get_run_command(&self, file_path: &str) -> Vec<String>;

    /// Returns the command to compile a file in this language (if applicable)
    fn get_compile_command(&self, file_path: &str) -> Option<Vec<String>>;

    /// Cleans up any temporary files or environment setup
    async fn cleanup(&self) -> Result<()>;
}
