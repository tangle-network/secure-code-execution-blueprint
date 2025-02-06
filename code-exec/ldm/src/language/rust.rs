use super::LanguageProvider;
use crate::{
    error::{Error, Result},
    types::{Package, PackageSource, Tool},
};
use async_trait::async_trait;
use std::{path::PathBuf, process::Command};
use tempfile::TempDir;
use tracing::{debug, info};

pub struct RustProvider {
    project_dir: Option<TempDir>,
    target_dir: Option<PathBuf>,
}

impl Default for RustProvider {
    fn default() -> Self {
        Self {
            project_dir: None,
            target_dir: None,
        }
    }
}

#[async_trait]
impl LanguageProvider for RustProvider {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn required_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "rustc".to_string(),
                version: None,
                binary_name: "rustc".to_string(),
                required: true,
            },
            Tool {
                name: "cargo".to_string(),
                version: None,
                binary_name: "cargo".to_string(),
                required: true,
            },
        ]
    }

    fn required_packages(&self) -> Vec<Package> {
        vec![Package {
            name: "curl".to_string(), // For rustup installation
            version: None,
            source: PackageSource::System,
        }]
    }

    async fn validate_installation(&self) -> Result<()> {
        info!("Validating Rust installation");

        // Check rustc version
        let output = Command::new("rustc")
            .args(["--version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check Rust version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation("Rust is not properly installed".into()));
        }

        // Check cargo version
        let output = Command::new("cargo")
            .args(["--version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check Cargo version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation("Cargo is not properly installed".into()));
        }

        debug!("Rust installation validated successfully");
        Ok(())
    }

    async fn setup_environment(&mut self) -> Result<()> {
        info!("Setting up Rust environment");

        // Create a temporary project directory
        let project_dir = TempDir::new().map_err(|e| {
            Error::Environment(format!("Failed to create temporary directory: {}", e))
        })?;

        // Initialize a new Cargo project
        let status = Command::new("cargo")
            .args(["init", "--bin"])
            .current_dir(project_dir.path())
            .status()
            .map_err(|e| {
                Error::Environment(format!("Failed to initialize Cargo project: {}", e))
            })?;

        if !status.success() {
            return Err(Error::Environment(
                "Failed to initialize Cargo project".into(),
            ));
        }

        // Store project directory and target directory
        self.project_dir = Some(project_dir);
        self.target_dir = Some(self.project_dir.as_ref().unwrap().path().join("target"));

        debug!("Rust environment set up successfully");
        Ok(())
    }

    fn get_run_command(&self, file_path: &str) -> Vec<String> {
        if let Some(ref project_dir) = self.project_dir {
            // If we're in a Cargo project, use cargo run
            vec!["cargo".to_string(), "run".to_string()]
        } else {
            // Otherwise, compile and run directly
            vec![
                "rustc".to_string(),
                file_path.to_string(),
                "-o".to_string(),
                "output".to_string(),
            ]
        }
    }

    fn get_compile_command(&self, file_path: &str) -> Option<Vec<String>> {
        if let Some(ref project_dir) = self.project_dir {
            // If we're in a Cargo project, use cargo build
            Some(vec!["cargo".to_string(), "build".to_string()])
        } else {
            // Otherwise, compile directly
            Some(vec!["rustc".to_string(), file_path.to_string()])
        }
    }

    async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up Rust environment");

        // Clean up target directory if it exists
        if let Some(ref target_dir) = self.target_dir {
            if target_dir.exists() {
                debug!("Removing target directory: {}", target_dir.display());
                std::fs::remove_dir_all(target_dir).map_err(|e| {
                    Error::Environment(format!("Failed to remove target directory: {}", e))
                })?;
            }
        }

        // The TempDir will be automatically cleaned up when dropped
        if let Some(project_dir) = &self.project_dir {
            debug!(
                "Removing project directory: {}",
                project_dir.path().display()
            );
        }

        Ok(())
    }
}
