use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
use tracing::debug;
use which::which;

use crate::languages::ToolCheck;
use crate::{error::Error, executor::LanguageExecutor};

pub struct JavaScriptExecutor {
    node_version: String,
}

impl JavaScriptExecutor {
    pub fn new(version: Option<String>) -> Self {
        Self {
            node_version: version.unwrap_or_else(|| "18".to_string()),
        }
    }
}

impl ToolCheck for JavaScriptExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["node", "npm"]
    }
}

#[async_trait]
impl LanguageExecutor for JavaScriptExecutor {
    fn file_extension(&self) -> &str {
        "js"
    }

    fn run_command(&self) -> &str {
        "node"
    }

    fn run_args(&self) -> Vec<String> {
        vec!["source.js".to_string()]
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Initialize npm project
        let package_json = sandbox_dir.join("package.json");
        let package_content = serde_json::json!({
            "name": "code-execution",
            "version": "1.0.0",
            "private": true
        });

        fs::write(
            &package_json,
            serde_json::to_string_pretty(&package_content).unwrap(),
        )
        .await
        .map_err(|e| Error::System(format!("Failed to create package.json: {}", e)))?;

        debug!("Created package.json at: {}", package_json.display());
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

        let mut install_args = vec!["install"];
        let dep_specs: Vec<String> = dependencies
            .iter()
            .map(|dep| match &dep.source {
                Some(source) => format!("{}@{}", dep.name, source),
                None => format!("{}@{}", dep.name, dep.version),
            })
            .collect();

        install_args.extend(dep_specs.iter().map(|s| s.as_str()));

        // Install dependencies
        let status = Command::new("npm")
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
        let target_path = sandbox_dir.join("source.js");
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
        // Reuse the implementation from ToolCheck
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
