use crate::{error::Result, types::Package};
use async_trait::async_trait;

pub mod go;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

#[async_trait]
pub trait DependencyAnalyzer: Send + Sync {
    /// Returns the name of the language this analyzer is for
    fn language(&self) -> &'static str;

    /// Analyzes source code to extract dependencies
    async fn analyze_dependencies(&self, source_code: &str) -> Result<Vec<Package>>;

    /// Returns true if this analyzer can handle the given source code
    fn can_handle(&self, source_code: &str) -> bool;

    /// Returns true if the given line appears to be a dependency declaration
    fn is_dependency_line(&self, line: &str) -> bool;

    /// Extracts package information from a dependency line
    fn extract_package_info(&self, line: &str) -> Option<Package>;
}

/// Analyzes source code to determine its language and extract dependencies
pub async fn analyze_source_code(source_code: &str) -> Result<(String, Vec<Package>)> {
    let analyzers: Vec<Box<dyn DependencyAnalyzer>> = vec![
        Box::new(python::PythonAnalyzer::default()),
        Box::new(javascript::JavaScriptAnalyzer::default()),
        Box::new(typescript::TypeScriptAnalyzer::default()),
        Box::new(rust::RustAnalyzer::default()),
        Box::new(go::GoAnalyzer::default()),
    ];

    for analyzer in analyzers {
        if analyzer.can_handle(source_code) {
            let deps = analyzer.analyze_dependencies(source_code).await?;
            return Ok((analyzer.language().to_string(), deps));
        }
    }

    Err(crate::error::Error::Validation(
        "Could not determine language of source code".into(),
    ))
}
