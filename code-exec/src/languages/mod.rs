//! Language-specific executor implementations

pub mod go;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

pub use go::GoExecutor;
pub use javascript::JavaScriptExecutor;
pub use python::PythonExecutor;
pub use rust::RustExecutor;
pub use typescript::TypeScriptExecutor;

use crate::error::Error;
use ldm::{analyze_source_code, InstallationConfig, InstallationManager, Package};
use which::which;

/// Trait for checking and installing required tools
pub trait ToolCheck {
    fn required_tools(&self) -> Vec<&str>;

    async fn install_missing_tools(&self) -> Result<(), Error> {
        let config = InstallationConfig::default();
        let manager = InstallationManager::new_for_current_os(config);

        // Convert tools to packages
        let packages: Vec<Package> = self
            .required_tools()
            .iter()
            .map(|&tool| Package {
                name: tool.to_string(),
                version: None,
                source: ldm::PackageSource::System,
            })
            .collect();

        // Try to install missing packages
        if let Some(pm) = manager.find_available_package_manager() {
            for package in packages {
                if !pm
                    .is_installed(&package)
                    .await
                    .map_err(|e| Error::System(e.to_string()))?
                {
                    pm.install(&package)
                        .await
                        .map_err(|e| Error::System(e.to_string()))?;
                }
            }
            Ok(())
        } else {
            Err(Error::System(
                "No package manager available for current OS".to_string(),
            ))
        }
    }

    fn check_tools(&self) -> Result<(), Error> {
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
}

/// Trait for analyzing and managing dependencies in source code
pub trait DependencyManager {
    /// Returns the language name for dependency analysis
    fn get_language(&self) -> &'static str;

    /// Analyzes source code to extract dependencies
    async fn analyze_dependencies(&self, source_code: &str) -> Result<Vec<Package>, Error> {
        let (lang, deps) = analyze_source_code(source_code)
            .await
            .map_err(|e| Error::System(e.to_string()))?;

        if lang != self.get_language() {
            return Err(Error::System(format!(
                "Language mismatch: expected {}, got {}",
                self.get_language(),
                lang
            )));
        }

        Ok(deps)
    }

    /// Installs dependencies for the given source code
    async fn install_dependencies(&self, source_code: &str) -> Result<(), Error> {
        let deps = self.analyze_dependencies(source_code).await?;
        let config = InstallationConfig::default();
        let manager = InstallationManager::new_for_current_os(config);

        if let Some(pm) = manager.find_available_package_manager() {
            for package in deps {
                if !pm
                    .is_installed(&package)
                    .await
                    .map_err(|e| Error::System(e.to_string()))?
                {
                    pm.install(&package)
                        .await
                        .map_err(|e| Error::System(e.to_string()))?;
                }
            }
            Ok(())
        } else {
            Err(Error::System(
                "No package manager available for current OS".to_string(),
            ))
        }
    }

    /// Validates that all required dependencies are installed
    async fn validate_dependencies(&self, source_code: &str) -> Result<(), Error> {
        let deps = self.analyze_dependencies(source_code).await?;
        let config = InstallationConfig::default();
        let manager = InstallationManager::new_for_current_os(config);

        if let Some(pm) = manager.find_available_package_manager() {
            for package in deps {
                if !pm
                    .is_installed(&package)
                    .await
                    .map_err(|e| Error::System(e.to_string()))?
                {
                    return Err(Error::System(format!(
                        "Missing dependency: {}",
                        package.name
                    )));
                }
            }
            Ok(())
        } else {
            Err(Error::System(
                "No package manager available for current OS".to_string(),
            ))
        }
    }
}

#[allow(dead_code)]
pub async fn check_requirements<T: ToolCheck>(executor: &T) -> Result<(), Error> {
    if let Err(_) = executor.check_tools() {
        // Try to install missing tools
        executor.install_missing_tools().await?;
    }
    Ok(())
}
