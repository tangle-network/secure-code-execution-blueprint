use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
use which::which;

use crate::{error::Error, executor::LanguageExecutor, languages::ToolCheck};

pub struct PhpExecutor {
    php_version: String,
}

impl PhpExecutor {
    pub fn new(version: Option<String>) -> Self {
        Self {
            php_version: version.unwrap_or_else(|| "8.2".to_string()),
        }
    }
}

impl ToolCheck for PhpExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["php", "composer"]
    }
}

#[async_trait]
impl LanguageExecutor for PhpExecutor {
    fn file_extension(&self) -> &str {
        "php"
    }

    fn run_command(&self) -> &str {
        "php"
    }

    fn run_args(&self) -> Vec<String> {
        vec!["main.php".to_string()]
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create composer.json
        let composer = serde_json::json!({
            "name": "code-execution/app",
            "type": "project",
            "require": {
                "php": format!(">={}", self.php_version)
            },
            "autoload": {
                "psr-4": {
                    "App\\": "src/"
                }
            }
        });

        fs::write(
            sandbox_dir.join("composer.json"),
            serde_json::to_string_pretty(&composer).unwrap(),
        )
        .await
        .map_err(|e| Error::System(format!("Failed to write composer.json: {}", e)))?;

        // Initialize Composer
        let status = Command::new("composer")
            .args(["install", "--no-interaction"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to initialize Composer: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to initialize Composer".to_string()));
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

        // Update composer.json with dependencies
        let mut composer: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(sandbox_dir.join("composer.json")).await?)
                .map_err(|e| Error::System(format!("Failed to read composer.json: {}", e)))?;

        let require = composer
            .get_mut("require")
            .unwrap()
            .as_object_mut()
            .unwrap();

        for dep in dependencies {
            let version = match &dep.source {
                Some(source) => format!("dev-master#{}", source),
                None => dep.version.clone(),
            };
            require.insert(dep.name.clone(), version.into());
        }

        fs::write(
            sandbox_dir.join("composer.json"),
            serde_json::to_string_pretty(&composer).unwrap(),
        )
        .await
        .map_err(|e| Error::System(format!("Failed to write composer.json: {}", e)))?;

        // Install dependencies
        let status = Command::new("composer")
            .args(["update", "--no-interaction"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to install dependencies: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to install dependencies".to_string()));
        }

        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Move source file to main.php
        fs::rename(source_file, sandbox_dir.join("main.php"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // PHP is interpreted, but we can check syntax
        let status = Command::new("php")
            .args(["-l", "main.php"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError(
                "PHP syntax check failed".to_string(),
            ));
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
    async fn test_php_setup() -> Result<(), Error> {
        let executor = PhpExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        assert!(dir.path().join("composer.json").exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_php_syntax() -> Result<(), Error> {
        let executor = PhpExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        let source = r#"<?php
            function greet($name = 'World') {
                echo "Hello, $name!";
            }
            greet();
        "#;

        let source_path = dir.path().join("tmp").join("source.php");
        fs::write(&source_path, source).await?;

        executor
            .compile(&dir.path().to_path_buf(), &source_path)
            .await?;
        assert!(dir.path().join("main.php").exists());
        Ok(())
    }
}
