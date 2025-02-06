use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
use tracing::debug;
use which::which;

use crate::{error::Error, executor::LanguageExecutor, languages::ToolCheck};

pub struct GoExecutor {
    go_version: String,
}

impl GoExecutor {
    pub fn new(version: Option<String>) -> Self {
        Self {
            go_version: version.unwrap_or_else(|| "1.21".to_string()),
        }
    }

    async fn write_go_mod(
        &self,
        sandbox_dir: &PathBuf,
        dependencies: &[crate::types::Dependency],
    ) -> Result<(), Error> {
        let mut content = String::from("module code-execution\n\ngo 1.21\n\n");
        if !dependencies.is_empty() {
            content.push_str("require (\n");
            for dep in dependencies {
                let version = match &dep.source {
                    Some(source) => source.clone(),
                    None => format!("v{}", dep.version),
                };
                content.push_str(&format!("\t{} {}\n", dep.name, version));
            }
            content.push_str(")\n");
        }

        fs::write(sandbox_dir.join("go.mod"), content.clone())
            .await
            .map_err(|e| Error::System(format!("Failed to write go.mod: {}", e)))?;

        println!("Created go.mod with content:\n{}", content);
        Ok(())
    }
}

impl ToolCheck for GoExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["go"]
    }
}

#[async_trait]
impl LanguageExecutor for GoExecutor {
    fn file_extension(&self) -> &str {
        "go"
    }

    fn run_command(&self) -> &str {
        "./code-execution"
    }

    fn run_args(&self) -> Vec<String> {
        Vec::new()
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create an empty go.mod file - we'll update it during dependency installation
        self.write_go_mod(sandbox_dir, &[]).await?;
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

        // Update go.mod with dependencies
        self.write_go_mod(sandbox_dir, dependencies).await?;

        // Run go mod tidy to download dependencies and create go.sum
        let output = Command::new("go")
            .args(["mod", "tidy"])
            .current_dir(sandbox_dir)
            .output()
            .await
            .map_err(|e| Error::System(format!("Failed to run go mod tidy: {}", e)))?;

        if !output.status.success() {
            return Err(Error::System(format!(
                "Failed to run go mod tidy: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Verify dependencies
        let output = Command::new("go")
            .args(["mod", "verify"])
            .current_dir(sandbox_dir)
            .output()
            .await
            .map_err(|e| Error::System(format!("Failed to verify dependencies: {}", e)))?;

        if !output.status.success() {
            return Err(Error::System(format!(
                "Failed to verify dependencies: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        println!("Successfully installed Go dependencies");
        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Move source to main.go
        fs::rename(source_file, sandbox_dir.join("main.go"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Build with verbose output to help diagnose issues
        let output = Command::new("go")
            .args(["build", "-v", "-o", "code-execution"])
            .current_dir(sandbox_dir)
            .output()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !output.status.success() {
            println!(
                "Go compilation failed. stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            println!(
                "Go compilation failed. stdout: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            return Err(Error::CompilationError(format!(
                "Go compilation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

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
