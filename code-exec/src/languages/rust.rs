use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
use tracing::info;

use crate::{error::Error, executor::LanguageExecutor, languages::ToolCheck};

pub struct RustExecutor {
    toolchain: String,
}

impl RustExecutor {
    pub fn new(toolchain: Option<String>) -> Self {
        Self {
            toolchain: toolchain.unwrap_or_else(|| "stable".to_string()),
        }
    }

    async fn create_cargo_toml(
        &self,
        sandbox_dir: &PathBuf,
        dependencies: &[crate::types::Dependency],
    ) -> Result<(), Error> {
        let mut manifest = toml::Table::new();

        // Add package section
        let mut package = toml::Table::new();
        package.insert("name".into(), "code-execution".into());
        package.insert("version".into(), "0.1.0".into());
        package.insert("edition".into(), "2021".into());
        manifest.insert("package".into(), package.into());

        // Add dependencies section
        let mut deps = toml::Table::new();
        for dep in dependencies {
            let mut dep_spec = toml::Table::new();
            match &dep.source {
                Some(source) => {
                    dep_spec.insert("git".into(), source.clone().into());
                }
                None => {
                    dep_spec.insert("version".into(), dep.version.clone().into());
                }
            }
            deps.insert(dep.name.clone(), dep_spec.into());
        }
        manifest.insert("dependencies".into(), deps.into());

        fs::write(
            sandbox_dir.join("Cargo.toml"),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .await
        .map_err(|e| Error::System(format!("Failed to write Cargo.toml: {}", e)))?;

        Ok(())
    }
}

impl ToolCheck for RustExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["rustc", "cargo", "rustup"]
    }
}

#[async_trait]
impl LanguageExecutor for RustExecutor {
    fn file_extension(&self) -> &str {
        "rs"
    }

    fn run_command(&self) -> &str {
        "./target/release/code-execution"
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create src directory
        fs::create_dir_all(sandbox_dir.join("src"))
            .await
            .map_err(|e| Error::System(format!("Failed to create src directory: {}", e)))?;

        // Create basic Cargo.toml
        self.create_cargo_toml(sandbox_dir, &[]).await?;

        // Install specified toolchain
        let status = Command::new("rustup")
            .args(["default", &self.toolchain])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to set Rust toolchain: {}", e)))?;

        if !status.success() {
            return Err(Error::System(format!(
                "Failed to set Rust toolchain {}",
                self.toolchain
            )));
        }

        Ok(())
    }

    async fn install_dependencies(
        &self,
        sandbox_dir: &PathBuf,
        dependencies: &[crate::types::Dependency],
    ) -> Result<(), Error> {
        self.create_cargo_toml(sandbox_dir, dependencies).await?;
        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Move source file to src/main.rs
        fs::rename(source_file, sandbox_dir.join("src").join("main.rs"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Build in release mode
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError("Cargo build failed".to_string()));
        }

        Ok(())
    }

    async fn check_tools(&self) -> Result<(), Error> {
        let missing: Vec<_> = self
            .required_tools()
            .iter()
            .filter(|tool| which::which(tool).is_err())
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
        for dir in &["tmp", "src", "target"] {
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
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_rust_setup() {
        let dir = tempdir().unwrap();
        let executor = RustExecutor::new(None);

        assert!(executor
            .setup_environment(&dir.path().to_path_buf())
            .await
            .is_ok());
        assert!(dir.path().join("Cargo.toml").exists());
    }

    #[tokio::test]
    async fn test_rust_compilation() {
        let dir = tempdir().unwrap();
        let executor = RustExecutor::new(None);

        executor
            .setup_environment(&dir.path().to_path_buf())
            .await
            .unwrap();

        let source = r#"
            fn main() {
                println!("Hello, World!");
            }
        "#;

        let source_path = dir.path().join("tmp").join("source.rs");
        fs::write(&source_path, source).await.unwrap();

        assert!(executor
            .compile(&dir.path().to_path_buf(), &source_path)
            .await
            .is_ok());
        assert!(dir.path().join("target/release/code-execution").exists());
    }
}
