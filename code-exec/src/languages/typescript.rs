use async_trait::async_trait;
use gadget_std::{end_timer, start_timer};
use std::path::PathBuf;
use tokio::{fs, process::Command};
use which::which;

use crate::{
    error::Error, executor::LanguageExecutor, languages::ToolCheck, ExecutionResult,
    ExecutionStatus,
};

pub struct TypeScriptExecutor {
    node_version: String,
    ts_version: String,
}

impl TypeScriptExecutor {
    pub fn new(node_version: Option<String>, ts_version: Option<String>) -> Self {
        Self {
            node_version: node_version.unwrap_or_else(|| "18".to_string()),
            ts_version: ts_version.unwrap_or_else(|| "5.0".to_string()),
        }
    }
}

impl ToolCheck for TypeScriptExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["node", "npm", "tsc"]
    }
}

#[async_trait]
impl LanguageExecutor for TypeScriptExecutor {
    fn file_extension(&self) -> &str {
        "ts"
    }

    fn run_command(&self) -> &str {
        "node"
    }

    fn run_args(&self) -> Vec<String> {
        vec!["dist/index.js".to_string()]
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create tsconfig.json
        let tsconfig = serde_json::json!({
            "compilerOptions": {
                "target": "ES2022",
                "module": "ESNext",
                "moduleResolution": "node",
                "esModuleInterop": true,
                "strict": true,
                "skipLibCheck": true,
                "outDir": "./dist",
                "rootDir": "./src"
            },
            "include": ["src/**/*"],
            "exclude": ["node_modules"]
        });

        fs::write(
            sandbox_dir.join("tsconfig.json"),
            serde_json::to_string_pretty(&tsconfig).unwrap(),
        )
        .await
        .map_err(|e| Error::System(format!("Failed to write tsconfig.json: {}", e)))?;

        // Create package.json
        let package_json = serde_json::json!({
            "name": "code-execution",
            "version": "1.0.0",
            "private": true,
            "type": "module"
        });

        fs::write(
            sandbox_dir.join("package.json"),
            serde_json::to_string_pretty(&package_json).unwrap(),
        )
        .await
        .map_err(|e| Error::System(format!("Failed to create package.json: {}", e)))?;

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

        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Move source to src directory
        fs::create_dir_all(sandbox_dir.join("src"))
            .await
            .map_err(|e| Error::System(format!("Failed to create src directory: {}", e)))?;

        fs::rename(source_file, sandbox_dir.join("src/index.ts"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Install typescript locally first
        let status = Command::new("npm")
            .args(["install", "--save-dev", "typescript"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to install typescript: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to install typescript".to_string()));
        }

        // Use local tsc from node_modules
        let status = Command::new("npx")
            .args(["tsc"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError(
                "TypeScript compilation failed".to_string(),
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
        for dir in &["tmp", "src", "build", "dist"] {
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
    async fn test_typescript_setup() -> Result<(), Error> {
        let executor = TypeScriptExecutor::new(None, None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        assert!(dir.path().join("tsconfig.json").exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_typescript_compilation() -> Result<(), Error> {
        let executor = TypeScriptExecutor::new(None, None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        let source = r#"
            const message: string = "Hello, World!";
            console.log(message);
        "#;

        let source_path = dir.path().join("tmp").join("source.ts");
        fs::write(&source_path, source).await?;

        executor
            .compile(&dir.path().to_path_buf(), &source_path)
            .await?;
        assert!(dir.path().join("dist/index.js").exists());
        Ok(())
    }
}
