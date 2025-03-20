use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
use tracing::debug;
use which::which;

use crate::{error::Error, executor::LanguageExecutor, languages::ToolCheck};

pub struct PythonExecutor {
    python_version: String,
}

impl PythonExecutor {
    pub fn new(version: Option<String>) -> Self {
        Self {
            python_version: version.unwrap_or_else(|| "3.10".to_string()),
        }
    }
}

impl ToolCheck for PythonExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["python3", "pip3", "virtualenv"]
    }
}

#[async_trait]
impl LanguageExecutor for PythonExecutor {
    fn file_extension(&self) -> &str {
        "py"
    }

    fn run_command(&self) -> &str {
        "python3"
    }

    fn run_args(&self) -> Vec<String> {
        vec!["source.py".to_string()]
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create virtual environment with minimal output
        let status = Command::new("virtualenv")
            .args([
                "venv",
                "--quiet",
                "--no-download",
                "--no-periodic-update",
                "--no-vcs-ignore",
            ])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to create virtualenv: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to create virtualenv".to_string()));
        }

        debug!(
            "Created virtualenv at: {}",
            sandbox_dir.join("venv").display()
        );
        Ok(())
    }

    async fn install_dependencies(
        &self,
        sandbox_dir: &PathBuf,
        dependencies: &[crate::types::Dependency],
    ) -> Result<(), Error> {
        if dependencies.is_empty() {
            return Ok(());
        }

        // Activate virtualenv and install dependencies quietly
        let pip_path = sandbox_dir.join("venv/bin/pip");
        let mut install_args = vec![
            "install",
            "--quiet",
            "--no-cache-dir",
            "--no-warn-script-location",
        ];
        let dep_specs: Vec<String> = dependencies
            .iter()
            .map(|dep| match &dep.source {
                Some(source) => format!("{}@{}", dep.name, source),
                None => format!("{}=={}", dep.name, dep.version),
            })
            .collect();

        install_args.extend(dep_specs.iter().map(|s| s.as_str()));

        let status = Command::new(pip_path)
            .args(&install_args)
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to install dependencies: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to install dependencies".to_string()));
        }

        debug!("Installed dependencies: {:?}", dep_specs);
        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Move source to root directory
        let target_path = sandbox_dir.join("source.py");
        fs::rename(source_file, &target_path)
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;
        debug!("Moved source file to: {}", target_path.display());
        Ok(())
    }

    async fn check_tools(&self) -> Result<(), Error> {
        let missing: Vec<_> = self
            .required_tools()
            .iter()
            .filter(|tool| which(tool).is_err())
            .map(|s| (*s).to_string())
            .collect();

        if !missing.is_empty() {
            return Err(Error::System(format!(
                "Missing required tools: {}",
                missing.join(", ")
            )));
        }
        Ok(())
    }

    async fn install_missing_tools(&self) -> Result<(), Error> {
        ToolCheck::install_missing_tools(self).await
    }

    async fn ensure_directories(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        for dir in &["tmp", "src", "build"] {
            tokio::fs::create_dir_all(sandbox_dir.join(dir))
                .await
                .map_err(|e| Error::System(format!("Failed to create {} directory: {}", dir, e)))?;
        }
        Ok(())
    }
}
