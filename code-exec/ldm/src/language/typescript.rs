use super::LanguageProvider;
use crate::{
    error::{Error, Result},
    types::{Package, PackageSource, Tool},
};
use async_trait::async_trait;
use std::{path::PathBuf, process::Command};
use tempfile::TempDir;
use tracing::{debug, info};

pub struct TypeScriptProvider {
    project_dir: Option<TempDir>,
    node_modules: Option<PathBuf>,
}

impl Default for TypeScriptProvider {
    fn default() -> Self {
        Self {
            project_dir: None,
            node_modules: None,
        }
    }
}

#[async_trait]
impl LanguageProvider for TypeScriptProvider {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn required_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "node".to_string(),
                version: Some(">=14.0.0".to_string()),
                binary_name: "node".to_string(),
                required: true,
            },
            Tool {
                name: "npm".to_string(),
                version: Some(">=6.0.0".to_string()),
                binary_name: "npm".to_string(),
                required: true,
            },
            Tool {
                name: "tsc".to_string(),
                version: None,
                binary_name: "tsc".to_string(),
                required: true,
            },
        ]
    }

    fn required_packages(&self) -> Vec<Package> {
        vec![
            Package {
                name: "nodejs".to_string(),
                version: None,
                source: PackageSource::System,
            },
            Package {
                name: "npm".to_string(),
                version: None,
                source: PackageSource::System,
            },
            Package {
                name: "typescript".to_string(),
                version: None,
                source: PackageSource::System,
            },
        ]
    }

    async fn validate_installation(&self) -> Result<()> {
        info!("Validating TypeScript installation");

        // Check Node.js version
        let output = Command::new("node")
            .args(["--version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check Node.js version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation(
                "Node.js is not properly installed".into(),
            ));
        }

        // Check npm version
        let output = Command::new("npm")
            .args(["--version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check npm version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation("npm is not properly installed".into()));
        }

        // Check TypeScript compiler
        let output = Command::new("tsc")
            .args(["--version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check TypeScript version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation(
                "TypeScript is not properly installed".into(),
            ));
        }

        debug!("TypeScript installation validated successfully");
        Ok(())
    }

    async fn setup_environment(&mut self) -> Result<()> {
        info!("Setting up TypeScript environment");

        // Create a temporary project directory
        let project_dir = TempDir::new().map_err(|e| {
            Error::Environment(format!("Failed to create temporary directory: {}", e))
        })?;

        // Initialize package.json
        let status = Command::new("npm")
            .args(["init", "-y"])
            .current_dir(project_dir.path())
            .status()
            .map_err(|e| Error::Environment(format!("Failed to initialize package.json: {}", e)))?;

        if !status.success() {
            return Err(Error::Environment(
                "Failed to initialize package.json".into(),
            ));
        }

        // Initialize tsconfig.json
        let status = Command::new("tsc")
            .args(["--init"])
            .current_dir(project_dir.path())
            .status()
            .map_err(|e| {
                Error::Environment(format!("Failed to initialize tsconfig.json: {}", e))
            })?;

        if !status.success() {
            return Err(Error::Environment(
                "Failed to initialize tsconfig.json".into(),
            ));
        }

        // Store project directory and node_modules path
        self.project_dir = Some(project_dir);
        self.node_modules = Some(
            self.project_dir
                .as_ref()
                .unwrap()
                .path()
                .join("node_modules"),
        );

        debug!("TypeScript environment set up successfully");
        Ok(())
    }

    fn get_run_command(&self, file_path: &str) -> Vec<String> {
        // First compile TypeScript to JavaScript, then run with Node.js
        vec!["ts-node".to_string(), file_path.to_string()]
    }

    fn get_compile_command(&self, file_path: &str) -> Option<Vec<String>> {
        Some(vec!["tsc".to_string(), file_path.to_string()])
    }

    async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up TypeScript environment");

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
