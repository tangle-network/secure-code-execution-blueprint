//! Language Dependency Manager (LDM)
//!
//! A robust, cross-platform dependency management system for code execution environments.
//! This library handles the installation, verification, and maintenance of language-specific
//! tools and dependencies across different operating systems and package managers.

pub mod analyzer;
pub mod error;
pub mod language;
pub mod manager;
pub mod package_manager;
mod types;

#[cfg(test)]
mod tests;

pub use analyzer::{analyze_source_code, DependencyAnalyzer};
pub use error::{Error, Result};
pub use language::LanguageProvider;
pub use manager::InstallationManager;
pub use package_manager::PackageManager;
pub use types::{
    InstallationConfig, InstallationProgress, InstallationStatus, Package, PackageSource, Tool,
};

// Re-export commonly used language providers
pub use language::{
    go::GoProvider, javascript::JavaScriptProvider, python::PythonProvider, rust::RustProvider,
    typescript::TypeScriptProvider,
};

// Re-export commonly used package managers
pub use package_manager::{apt::AptPackageManager, brew::HomebrewPackageManager};
