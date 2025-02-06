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
        // Create Cargo.toml
        let cargo_toml = sandbox_dir.join("Cargo.toml");
        let cargo_content = r#"[package]
name = "code-execution"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

        fs::write(&cargo_toml, cargo_content)
            .await
            .map_err(|e| Error::System(format!("Failed to create Cargo.toml: {}", e)))?;

        // Initialize toolchain with minimal output
        let status = Command::new("rustup")
            .args(["default", "stable"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to set Rust toolchain: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to set Rust toolchain".to_string()));
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
        // Move source to src/main.rs
        let src_dir = sandbox_dir.join("src");
        fs::create_dir_all(&src_dir)
            .await
            .map_err(|e| Error::System(format!("Failed to create src directory: {}", e)))?;

        fs::rename(source_file, src_dir.join("main.rs"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Build the code
        let status = Command::new("cargo")
            .args([
                "build",
                "--release",
                "--quiet",
                "--color=never",
                "--message-format=short",
            ])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError(
                "Rust compilation failed".to_string(),
            ));
        }

        // Copy binary to root directory
        fs::copy(
            sandbox_dir.join("target/release/code-execution"),
            sandbox_dir.join("code-execution"),
        )
        .await
        .map_err(|e| Error::System(format!("Failed to copy binary: {}", e)))?;

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
