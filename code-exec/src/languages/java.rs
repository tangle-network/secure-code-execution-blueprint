use async_trait::async_trait;
use std::path::PathBuf;
use tokio::{fs, process::Command};
use which::which;

use crate::{error::Error, executor::LanguageExecutor, languages::ToolCheck};

pub struct JavaExecutor {
    java_version: String,
}

impl JavaExecutor {
    pub fn new(version: Option<String>) -> Self {
        Self {
            java_version: version.unwrap_or_else(|| "17".to_string()),
        }
    }
}

impl ToolCheck for JavaExecutor {
    fn required_tools(&self) -> Vec<&str> {
        vec!["java", "javac", "mvn"]
    }
}

#[async_trait]
impl LanguageExecutor for JavaExecutor {
    fn file_extension(&self) -> &str {
        "java"
    }

    fn run_command(&self) -> &str {
        "java"
    }

    fn run_args(&self) -> Vec<String> {
        vec![
            "-cp".to_string(),
            "target/classes".to_string(),
            "Main".to_string(),
        ]
    }

    async fn setup_environment(&self, sandbox_dir: &PathBuf) -> Result<(), Error> {
        // Create pom.xml
        let pom = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>code.execution</groupId>
    <artifactId>code-execution</artifactId>
    <version>1.0-SNAPSHOT</version>

    <properties>
        <maven.compiler.source>{}</maven.compiler.source>
        <maven.compiler.target>{}</maven.compiler.target>
        <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
    </properties>

    <dependencies>
    </dependencies>
</project>"#,
            self.java_version, self.java_version
        );

        fs::write(sandbox_dir.join("pom.xml"), pom)
            .await
            .map_err(|e| Error::System(format!("Failed to write pom.xml: {}", e)))?;

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

        // Read existing pom.xml
        let mut pom = fs::read_to_string(sandbox_dir.join("pom.xml"))
            .await
            .map_err(|e| Error::System(format!("Failed to read pom.xml: {}", e)))?;

        // Add dependencies
        let deps_xml = dependencies
            .iter()
            .map(|dep| {
                let (group_id, artifact_id) = dep.name.split_once(':').unwrap_or(("", ""));
                match &dep.source {
                    Some(source) => format!(
                        r#"        <dependency>
            <groupId>{}</groupId>
            <artifactId>{}</artifactId>
            <version>{}</version>
            <systemPath>{}</systemPath>
            <scope>system</scope>
        </dependency>"#,
                        group_id, artifact_id, dep.version, source
                    ),
                    None => format!(
                        r#"        <dependency>
            <groupId>{}</groupId>
            <artifactId>{}</artifactId>
            <version>{}</version>
        </dependency>"#,
                        group_id, artifact_id, dep.version
                    ),
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Insert dependencies before closing dependencies tag
        if let Some(pos) = pom.find("</dependencies>") {
            pom.insert_str(pos, &deps_xml);
        }

        // Write updated pom.xml
        fs::write(sandbox_dir.join("pom.xml"), pom)
            .await
            .map_err(|e| Error::System(format!("Failed to write pom.xml: {}", e)))?;

        // Run Maven install
        let status = Command::new("mvn")
            .args(["dependency:resolve"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::System(format!("Failed to resolve dependencies: {}", e)))?;

        if !status.success() {
            return Err(Error::System("Failed to resolve dependencies".to_string()));
        }

        Ok(())
    }

    async fn compile(&self, sandbox_dir: &PathBuf, source_file: &PathBuf) -> Result<(), Error> {
        // Create src/main/java directory
        fs::create_dir_all(sandbox_dir.join("src/main/java"))
            .await
            .map_err(|e| Error::System(format!("Failed to create source directory: {}", e)))?;

        // Move source file to src/main/java/Main.java
        fs::rename(source_file, sandbox_dir.join("src/main/java/Main.java"))
            .await
            .map_err(|e| Error::System(format!("Failed to move source file: {}", e)))?;

        // Compile with Maven
        let status = Command::new("mvn")
            .args(["compile"])
            .current_dir(sandbox_dir)
            .status()
            .await
            .map_err(|e| Error::CompilationError(e.to_string()))?;

        if !status.success() {
            return Err(Error::CompilationError(
                "Maven compilation failed".to_string(),
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
    async fn test_java_setup() -> Result<(), Error> {
        let executor = JavaExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        assert!(dir.path().join("pom.xml").exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_java_compilation() -> Result<(), Error> {
        let executor = JavaExecutor::new(None);
        check_requirements(&executor).await?;

        let dir = tempdir().unwrap();
        executor
            .ensure_directories(&dir.path().to_path_buf())
            .await?;
        executor
            .setup_environment(&dir.path().to_path_buf())
            .await?;

        let source = r#"
            public class Main {
                public static void main(String[] args) {
                    System.out.println("Hello, World!");
                }
            }
        "#;

        let source_path = dir.path().join("tmp").join("source.java");
        fs::write(&source_path, source).await?;

        executor
            .compile(&dir.path().to_path_buf(), &source_path)
            .await?;
        assert!(dir.path().join("target/classes/Main.class").exists());
        Ok(())
    }
}
