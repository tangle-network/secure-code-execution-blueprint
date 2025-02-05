use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
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
        vec![
            "-c".to_string(),
            "import sys; sys.path.append('tmp'); import main".to_string(),
        ]
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create virtual environment
        let status = Command::new("virtualenv")
            .args(["venv"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to create virtualenv: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to create virtualenv".to_string()));
        }

        // Create symlink to python in tmp directory
        let venv_python = sandbox_dir.join("venv/bin/python3");
        let tmp_python = sandbox_dir.join("tmp/python3");
        std::os::unix::fs::symlink(&venv_python, &tmp_python)
            .map_err(|e| Error::System(format!("Failed to create python symlink: {}", e)))?;

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

        // Write dependencies to requirements.txt
        let requirements = dependencies
            .iter()
            .map(|dep| match &dep.source {
                Some(source) => format!("{}@{}", dep.name, source),
                None => format!("{}=={}", dep.name, dep.version),
            })
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(sandbox_dir.join("requirements.txt"), requirements)
            .await
            .map_err(|e| Error::System(format!("Failed to write requirements.txt: {}", e)))?;

        // Install dependencies using pip
        let status = Command::new(sandbox_dir.join("venv/bin/pip"))
            .args(["install", "-r", "requirements.txt"])
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
        // Move source file to tmp/main.py
        fs::rename(source_file, sandbox_dir.join("tmp/main.py"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Python is interpreted, but we can check syntax
        let status = Command::new(sandbox_dir.join("venv/bin/python3"))
            .args(["-m", "py_compile", "tmp/main.py"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError(
                "Python syntax check failed".to_string(),
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
    async fn test_python_setup() -> Result<(), Error> {
        let executor = PythonExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        assert!(dir.path().join("venv").exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_python_dependencies() -> Result<(), Error> {
        let executor = PythonExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        let deps = vec![crate::types::Dependency {
            name: "requests".to_string(),
            version: "2.31.0".to_string(),
            source: None,
        }];

        executor
            .install_dependencies(&dir.path().to_path_buf(), &deps)
            .await?;

        // Verify installation
        let status = Command::new(dir.path().join("venv/bin/pip"))
            .args(["show", "requests"])
            .status()
            .await?;

        assert!(status.success());
        Ok(())
    }
}
