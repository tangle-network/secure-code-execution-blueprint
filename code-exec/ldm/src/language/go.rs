use super::LanguageProvider;
use crate::{
    error::{Error, Result},
    types::{Package, PackageSource, Tool},
};
use async_trait::async_trait;
use std::{path::PathBuf, process::Command};
use tempfile::TempDir;
use tracing::{debug, info};

pub struct GoProvider {
    project_dir: Option<TempDir>,
    go_path: Option<PathBuf>,
}

impl Default for GoProvider {
    fn default() -> Self {
        Self {
            project_dir: None,
            go_path: None,
        }
    }
}

#[async_trait]
impl LanguageProvider for GoProvider {
    fn name(&self) -> &'static str {
        "go"
    }

    fn required_tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "go".to_string(),
            version: Some(">=1.16".to_string()),
            binary_name: "go".to_string(),
            required: true,
        }]
    }

    fn required_packages(&self) -> Vec<Package> {
        vec![Package {
            name: "golang".to_string(),
            version: None,
            source: PackageSource::System,
        }]
    }

    async fn validate_installation(&self) -> Result<()> {
        info!("Validating Go installation");

        // Check Go version
        let output = Command::new("go")
            .args(["version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check Go version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation("Go is not properly installed".into()));
        }

        // Check go env
        let output = Command::new("go")
            .args(["env"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check Go environment: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation(
                "Go environment is not properly set up".into(),
            ));
        }

        debug!("Go installation validated successfully");
        Ok(())
    }

    async fn setup_environment(&mut self) -> Result<()> {
        info!("Setting up Go environment");

        // Create a temporary project directory
        let project_dir = TempDir::new().map_err(|e| {
            Error::Environment(format!("Failed to create temporary directory: {}", e))
        })?;

        // Initialize a new Go module
        let status = Command::new("go")
            .args(["mod", "init", "temp"])
            .current_dir(project_dir.path())
            .status()
            .map_err(|e| Error::Environment(format!("Failed to initialize Go module: {}", e)))?;

        if !status.success() {
            return Err(Error::Environment("Failed to initialize Go module".into()));
        }

        // Store project directory and set GOPATH
        self.project_dir = Some(project_dir);
        self.go_path = Some(self.project_dir.as_ref().unwrap().path().join("gopath"));

        // Create GOPATH directory structure
        let src_dir = self.go_path.as_ref().unwrap().join("src");
        std::fs::create_dir_all(&src_dir)
            .map_err(|e| Error::Environment(format!("Failed to create GOPATH structure: {}", e)))?;

        debug!("Go environment set up successfully");
        Ok(())
    }

    fn get_run_command(&self, file_path: &str) -> Vec<String> {
        vec!["go".to_string(), "run".to_string(), file_path.to_string()]
    }

    fn get_compile_command(&self, file_path: &str) -> Option<Vec<String>> {
        Some(vec![
            "go".to_string(),
            "build".to_string(),
            "-o".to_string(),
            "output".to_string(),
            file_path.to_string(),
        ])
    }

    async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up Go environment");

        // Clean up GOPATH if it exists
        if let Some(ref go_path) = self.go_path {
            if go_path.exists() {
                debug!("Removing GOPATH directory: {}", go_path.display());
                std::fs::remove_dir_all(go_path).map_err(|e| {
                    Error::Environment(format!("Failed to remove GOPATH directory: {}", e))
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
