use super::LanguageProvider;
use crate::{
    error::{Error, Result},
    types::{Package, PackageSource, Tool},
};
use async_trait::async_trait;
use std::{path::PathBuf, process::Command};
use tempfile::TempDir;
use tracing::{debug, info};

pub struct PythonProvider {
    venv_dir: Option<TempDir>,
    python_path: Option<PathBuf>,
}

impl Default for PythonProvider {
    fn default() -> Self {
        Self {
            venv_dir: None,
            python_path: None,
        }
    }
}

#[async_trait]
impl LanguageProvider for PythonProvider {
    fn name(&self) -> &'static str {
        "python"
    }

    fn required_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "python".to_string(),
                version: None,
                binary_name: "python3".to_string(),
                required: true,
            },
            Tool {
                name: "pip".to_string(),
                version: None,
                binary_name: "pip3".to_string(),
                required: true,
            },
            Tool {
                name: "virtualenv".to_string(),
                version: None,
                binary_name: "virtualenv".to_string(),
                required: true,
            },
        ]
    }

    fn required_packages(&self) -> Vec<Package> {
        vec![
            Package {
                name: "python3".to_string(),
                version: None,
                source: PackageSource::System,
            },
            Package {
                name: "python3-pip".to_string(),
                version: None,
                source: PackageSource::System,
            },
            Package {
                name: "python3-virtualenv".to_string(),
                version: None,
                source: PackageSource::System,
            },
        ]
    }

    async fn validate_installation(&self) -> Result<()> {
        info!("Validating Python installation");

        // Check Python version
        let output = Command::new("python3")
            .args(["--version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check Python version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation("Python is not properly installed".into()));
        }

        // Check pip
        let output = Command::new("pip3")
            .args(["--version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check pip version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation("pip is not properly installed".into()));
        }

        // Check virtualenv
        let output = Command::new("virtualenv")
            .args(["--version"])
            .output()
            .map_err(|e| Error::Validation(format!("Failed to check virtualenv version: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Validation(
                "virtualenv is not properly installed".into(),
            ));
        }

        debug!("Python installation validated successfully");
        Ok(())
    }

    async fn setup_environment(&mut self) -> Result<()> {
        info!("Setting up Python environment");

        // Create a temporary directory for the virtualenv
        let venv_dir = TempDir::new().map_err(|e| {
            Error::Environment(format!("Failed to create temporary directory: {}", e))
        })?;

        // Create virtualenv
        let status = Command::new("virtualenv")
            .arg(venv_dir.path())
            .status()
            .map_err(|e| Error::Environment(format!("Failed to create virtualenv: {}", e)))?;

        if !status.success() {
            return Err(Error::Environment("Failed to create virtualenv".into()));
        }

        // Get the Python path in the virtualenv
        let python_path = venv_dir.path().join("bin").join("python");

        // Store the virtualenv directory and Python path
        self.venv_dir = Some(venv_dir);
        self.python_path = Some(python_path);

        debug!("Python environment set up successfully");
        Ok(())
    }

    fn get_run_command(&self, file_path: &str) -> Vec<String> {
        if let Some(ref python_path) = self.python_path {
            vec![
                python_path.to_string_lossy().to_string(),
                file_path.to_string(),
            ]
        } else {
            vec!["python3".to_string(), file_path.to_string()]
        }
    }

    fn get_compile_command(&self, _file_path: &str) -> Option<Vec<String>> {
        None // Python is an interpreted language
    }

    async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up Python environment");

        // The TempDir will be automatically cleaned up when dropped
        if let Some(venv_dir) = &self.venv_dir {
            debug!(
                "Removing virtualenv directory: {}",
                venv_dir.path().display()
            );
        }

        Ok(())
    }
}
