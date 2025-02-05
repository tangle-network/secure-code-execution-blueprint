use async_trait::async_trait;
use regex;
use std::path::PathBuf;
use tokio::{fs, process::Command};
use which::which;

use crate::languages::ToolCheck;
use crate::ExecutionResult;
use crate::{error::Error, executor::LanguageExecutor};

pub struct SwiftExecutor {
    swift_version: String,
}

impl SwiftExecutor {
    pub fn new(version: Option<String>) -> Self {
        // Get installed Swift version
        let installed_version = std::process::Command::new("swift")
            .arg("--version")
            .output()
            .map(|output| {
                let version_str = String::from_utf8_lossy(&output.stdout);
                let re = regex::Regex::new(r"Swift version (\d+\.\d+\.\d+)").unwrap();
                re.captures(&version_str)
                    .and_then(|cap| cap.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "5.8".to_string())
            })
            .unwrap_or_else(|_| "5.8".to_string());

        Self {
            swift_version: version.unwrap_or(installed_version),
        }
    }

    async fn create_package_swift(
        &self,
        sandbox_dir: &PathBuf,
        dependencies: &[crate::types::Dependency],
    ) -> Result<(), Error> {
        let mut package_content = format!(
            r#"// swift-tools-version:{}
import PackageDescription

let package = Package(
    name: "code-execution",
    platforms: [
        .macOS(.v13)
    ],
    dependencies: [
"#,
            self.swift_version
        );

        // Add dependencies
        for dep in dependencies {
            let dep_spec = match &dep.source {
                Some(source) => format!(
                    r#"        .package(url: "{}", from: "{}"),
"#,
                    source, dep.version
                ),
                None => format!(
                    r#"        .package(name: "{}", from: "{}"),
"#,
                    dep.name, dep.version
                ),
            };
            package_content.push_str(&dep_spec);
        }

        package_content.push_str(
            r#"    ],
    targets: [
        .executableTarget(
            name: "code-execution",
            dependencies: []
        )
    ]
)
"#,
        );

        fs::write(sandbox_dir.join("Package.swift"), package_content)
            .await
            .map_err(|e| Error::System(format!("Failed to write Package.swift: {}", e)))?;

        Ok(())
    }
}

impl ToolCheck for SwiftExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["swift", "swiftc"]
    }
}

#[async_trait]
impl LanguageExecutor for SwiftExecutor {
    fn file_extension(&self) -> &str {
        "swift"
    }

    fn run_command(&self) -> &str {
        "./code-execution"
    }

    fn run_args(&self) -> Vec<String> {
        Vec::new()
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create Package.swift
        let package = format!(
            r#"// swift-tools-version:{}
import PackageDescription

let package = Package(
    name: "code-execution",
    platforms: [.macOS(.v12)],
    products: [
        .executable(name: "code-execution", targets: ["code-execution"])
    ],
    targets: [
        .executableTarget(name: "code-execution", path: "Sources")
    ]
)
"#,
            self.swift_version
        );

        fs::write(sandbox_dir.join("Package.swift"), package)
            .await
            .map_err(|e| Error::System(format!("Failed to write Package.swift: {}", e)))?;

        // Create Sources directory
        fs::create_dir_all(sandbox_dir.join("Sources"))
            .await
            .map_err(|e| Error::System(format!("Failed to create Sources directory: {}", e)))?;

        Ok(())
    }

    async fn install_dependencies(
        &self,
        _sandbox_dir: &PathBuf,
        _dependencies: &[crate::types::Dependency],
    ) -> Result<(), Error> {
        // Swift Package Manager dependencies would be added here
        // For now, we don't support external dependencies
        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Move source to Sources/main.swift
        fs::rename(source_file, sandbox_dir.join("Sources/main.swift"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Build with Swift Package Manager
        let status = Command::new("swift")
            .args(["build", "-c", "release"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError(
                "Swift compilation failed".to_string(),
            ));
        }

        // Move executable to sandbox root
        fs::rename(
            sandbox_dir.join(".build/release/code-execution"),
            sandbox_dir.join("code-execution"),
        )
        .await
        .map_err(|e| Error::System(format!("Failed to move executable: {}", e)))?;

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
    async fn test_swift_setup() -> Result<(), Error> {
        let executor = SwiftExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        assert!(dir.path().join("Package.swift").exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_swift_compilation() -> Result<(), Error> {
        let executor = SwiftExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        let source = r#"
            print("Hello, World!")
        "#;

        let source_path = dir.path().join("tmp").join("source.swift");
        fs::write(&source_path, source).await?;

        executor
            .compile(&dir.path().to_path_buf(), &source_path)
            .await?;
        assert!(dir.path().join("code-execution").exists());
        Ok(())
    }
}
