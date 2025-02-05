use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
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
        // Initialize Go module
        let status = Command::new("go")
            .args(["mod", "init", "code-execution"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to initialize go.mod: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to initialize go.mod".to_string()));
        }

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

        // Add dependencies to go.mod
        for dep in dependencies {
            let dep_spec = match &dep.source {
                Some(source) => format!("{}@{}", dep.name, source),
                None => format!("{}@v{}", dep.name, dep.version),
            };

            let status = Command::new("go")
                .args(["get", &dep_spec])
                .current_dir(sandbox_dir)
                .status()
                .await
                .map_err(|e| {
                    Error::System(format!("Failed to add dependency {}: {}", dep.name, e))
                })?;

            if !status.success() {
                return Err(Error::System(format!(
                    "Failed to add dependency {}",
                    dep.name
                )));
            }
        }

        // Tidy up dependencies
        let status = Command::new("go")
            .args(["mod", "tidy"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to tidy dependencies: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to tidy dependencies".to_string()));
        }

        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Move source to main.go
        fs::rename(source_file, sandbox_dir.join("main.go"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Build the code
        let status = Command::new("go")
            .args(["build", "-o", "code-execution"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError("Go compilation failed".to_string()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::check_requirements;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_go_setup() -> Result<(), Error> {
        let executor = GoExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        assert!(dir.path().join("go.mod").exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_go_compilation() -> Result<(), Error> {
        let executor = GoExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        let source = r#"
            package main

            func main() {
                println("Hello, World!")
            }
        "#;

        let source_path = dir.path().join("tmp").join("source.go");
        fs::write(&source_path, source).await?;

        executor
            .compile(&dir.path().to_path_buf(), &source_path)
            .await?;
        assert!(dir.path().join("code-execution").exists());
        Ok(())
    }
}
