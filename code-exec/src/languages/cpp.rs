use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
use which::which;

use crate::{error::Error, executor::LanguageExecutor, languages::ToolCheck};

pub struct CppExecutor {
    std_version: String,
    compiler: String,
}

impl CppExecutor {
    pub fn new(std_version: Option<String>, compiler: Option<String>) -> Self {
        Self {
            std_version: std_version.unwrap_or_else(|| "17".to_string()),
            compiler: compiler.unwrap_or_else(|| "g++".to_string()),
        }
    }
}

impl ToolCheck for CppExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["g++", "cmake", "make"]
    }
}

#[async_trait]
impl LanguageExecutor for CppExecutor {
    fn file_extension(&self) -> &str {
        "cpp"
    }

    fn run_command(&self) -> &str {
        "./code_execution"
    }

    fn run_args(&self) -> Vec<String> {
        Vec::new()
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create CMakeLists.txt
        let cmake_content = format!(
            r#"cmake_minimum_required(VERSION 3.10)
project(code_execution)

set(CMAKE_CXX_STANDARD {})
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

add_executable(code_execution main.cpp)

# Add external dependencies
find_package(Threads REQUIRED)
target_link_libraries(code_execution PRIVATE Threads::Threads)
"#,
            self.std_version
        );

        fs::write(sandbox_dir.join("CMakeLists.txt"), cmake_content)
            .await
            .map_err(|e| Error::System(format!("Failed to write CMakeLists.txt: {}", e)))?;

        Ok(())
    }

    async fn install_dependencies(
        &self,
        _sandbox_dir: &PathBuf,
        _dependencies: &[crate::types::Dependency],
    ) -> Result<(), Error> {
        // C++ dependencies are typically handled through the system package manager
        // and should be installed before running the code
        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Move source to main.cpp
        fs::rename(source_file, sandbox_dir.join("main.cpp"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Create build directory
        fs::create_dir_all(sandbox_dir.join("build"))
            .await
            .map_err(|e| Error::System(format!("Failed to create build directory: {}", e)))?;

        // Run CMake
        let status = Command::new("cmake")
            .args(["-S", ".", "-B", "build"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError(
                "CMake configuration failed".to_string(),
            ));
        }

        // Build with Make
        let status = Command::new("make")
            .current_dir(sandbox_dir.join("build"))
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError(
                "C++ compilation failed".to_string(),
            ));
        }

        // Move executable to sandbox root
        fs::rename(
            sandbox_dir.join("build/code_execution"),
            sandbox_dir.join("code_execution"),
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
    async fn test_cpp_setup() -> Result<(), Error> {
        let executor = CppExecutor::new(None, None);
        check_requirements(&executor).await?; // This will install tools if missing

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        assert!(dir.path().join("CMakeLists.txt").exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_cpp_compilation() -> Result<(), Error> {
        let executor = CppExecutor::new(None, None);
        check_requirements(&executor).await?; // This will install tools if missing

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        let source = r#"
            #include <iostream>
            
            int main() {
                std::cout << "Hello, World!" << std::endl;
                return 0;
            }
        "#;

        let source_path = dir.path().join("tmp").join("source.cpp");
        fs::write(&source_path, source).await?;

        executor
            .compile(&dir.path().to_path_buf(), &source_path)
            .await?;
        assert!(dir.path().join("code_execution").exists());
        Ok(())
    }
}
