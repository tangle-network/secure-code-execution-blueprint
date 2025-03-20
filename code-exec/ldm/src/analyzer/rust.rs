use super::DependencyAnalyzer;
use crate::{
    error::Result,
    types::{Package, PackageSource},
};
use async_trait::async_trait;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
pub struct RustAnalyzer {
    use_re: Option<Regex>,
    version_re: Option<Regex>,
    default_versions: HashMap<String, String>,
}

impl RustAnalyzer {
    fn get_use_re(&mut self) -> &Regex {
        if self.use_re.is_none() {
            // Match both simple and complex use statements
            self.use_re = Some(
                Regex::new(
                    r#"(?mx)
                    ^[\s\n]*use\s+
                    ([a-zA-Z0-9_]+)  # Base crate name
                    (?:::[a-zA-Z0-9_:,\s{}\[\]]+)?  # Optional path components
                    \s*;  # Ending semicolon
                    |^[\s\n]*extern\s+crate\s+([a-zA-Z0-9_]+)\s*;  # extern crate form
                    "#,
                )
                .unwrap(),
            );
        }
        self.use_re.as_ref().unwrap()
    }

    fn get_version_re(&mut self) -> &Regex {
        if self.version_re.is_none() {
            self.version_re = Some(
                Regex::new(r#"//\s*cargo-version:\s*([a-zA-Z0-9_-]+)\s*=\s*"([0-9.]+)""#).unwrap(),
            );
        }
        self.version_re.as_ref().unwrap()
    }

    fn get_canonical_crate_name(&self, name: &str) -> String {
        // First, check if it's a known crate name
        let known_crates = [
            // Exact matches (when crate name differs from path)
            ("serde_json", "serde_json"),
            ("tokio_test", "tokio_test"),
            ("lambda_runtime", "lambda_runtime"),
            // AWS crates (always use hyphens)
            ("aws_config", "aws-config"),
            ("aws_sdk_lambda", "aws-sdk-lambda"),
            ("aws_sdk_s3", "aws-sdk-s3"),
            ("aws_sdk_dynamodb", "aws-sdk-dynamodb"),
            // Standard crates (keep underscores)
            ("serde", "serde"),
            ("tokio", "tokio"),
            ("async_trait", "async-trait"),
            ("futures", "futures"),
        ];

        // Check for exact matches first
        if let Some((_, canonical)) = known_crates.iter().find(|(k, _)| *k == name) {
            return canonical.to_string();
        }

        // Check if it's in default versions
        if self.default_versions.contains_key(name) {
            return name.to_string();
        }

        // For AWS crates, convert to hyphenated form
        if name.starts_with("aws_") {
            return name.replace('_', "-");
        }

        // For everything else, preserve the original form
        name.to_string()
    }

    fn initialize_default_versions(&mut self) {
        if self.default_versions.is_empty() {
            let defaults = [
                // AWS
                ("aws-config", "0.55"),
                ("aws-sdk-lambda", "0.28"),
                ("aws-sdk-s3", "0.28"),
                ("aws-sdk-dynamodb", "0.28"),
                ("lambda_runtime", "0.8"),
                // Async
                ("tokio", "1.28"),
                ("async-trait", "0.1"),
                ("futures", "0.3"),
                // HTTP and API
                ("axum", "0.6"),
                ("reqwest", "0.11"),
                ("hyper", "0.14"),
                ("tower", "0.4"),
                // Serialization
                ("serde", "1.0"),
                ("serde_json", "1.0"),
                // Database
                ("sqlx", "0.7"),
                ("mongodb", "2.5"),
                ("redis", "0.23"),
                // Utils
                ("chrono", "0.4"),
                ("uuid", "1.3"),
                ("tracing", "0.1"),
                ("anyhow", "1.0"),
                ("thiserror", "1.0"),
                // Testing
                ("tokio_test", "0.4"),
                ("mockall", "0.11"),
            ];

            self.default_versions = defaults
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        }
    }

    fn is_std_import(&self, import_path: &str) -> bool {
        import_path.starts_with("std::")
    }

    fn get_default_version(&self, package_name: &str) -> String {
        self.default_versions
            .get(package_name)
            .cloned()
            .unwrap_or_else(|| "0.1".to_string())
    }

    fn extract_crate_name(&self, import_path: &str) -> Option<String> {
        let parts: Vec<&str> = import_path.split("::").collect();
        if parts.is_empty() || self.is_std_import(import_path) {
            None
        } else {
            Some(parts[0].to_string())
        }
    }
}

#[async_trait]
impl DependencyAnalyzer for RustAnalyzer {
    fn language(&self) -> &'static str {
        "rust"
    }

    async fn analyze_dependencies(&self, source_code: &str) -> Result<Vec<Package>> {
        let mut this = self.clone();
        this.initialize_default_versions();
        let mut packages = HashSet::new();
        let mut explicit_versions = HashMap::new();

        // First, collect any explicit version declarations
        for cap in this.get_version_re().captures_iter(source_code) {
            let name = cap.get(1).unwrap().as_str();
            let version = cap.get(2).unwrap().as_str();
            explicit_versions.insert(name.to_string(), version.to_string());
        }

        // Then process use statements
        let use_re = this.get_use_re().clone();
        for cap in use_re.captures_iter(source_code) {
            let crate_name = cap.get(1).or_else(|| cap.get(2));
            if let Some(crate_name) = crate_name {
                let crate_name = crate_name.as_str();
                if !this.is_std_import(crate_name) {
                    let canonical_name = this.get_canonical_crate_name(crate_name);
                    let version = explicit_versions
                        .get(&canonical_name)
                        .cloned()
                        .unwrap_or_else(|| this.get_default_version(&canonical_name));

                    packages.insert(Package {
                        name: canonical_name,
                        version: Some(version),
                        source: PackageSource::Custom("cargo".to_string()),
                    });
                }
            }
        }

        Ok(packages.into_iter().collect())
    }

    fn can_handle(&self, source_code: &str) -> bool {
        // Rust-specific patterns
        source_code.contains("fn main()")
            || source_code.contains("#[derive")
            || (source_code.contains("use ") && source_code.contains("::"))
    }

    fn is_dependency_line(&self, line: &str) -> bool {
        let mut this = self.clone();
        this.get_use_re().is_match(line) || this.get_version_re().is_match(line)
    }

    fn extract_package_info(&self, line: &str) -> Option<Package> {
        let mut this = self.clone();
        this.initialize_default_versions();

        // Try version comment first
        if let Some(cap) = this.get_version_re().captures(line) {
            let name = cap.get(1).unwrap().as_str();
            let version = cap.get(2).unwrap().as_str();
            return Some(Package {
                name: name.to_string(),
                version: Some(version.to_string()),
                source: PackageSource::Custom("cargo".to_string()),
            });
        }

        // Try use statement
        if let Some(cap) = this.get_use_re().captures(line) {
            let import_path = cap.get(1).unwrap().as_str();
            if let Some(crate_name) = this.extract_crate_name(import_path) {
                return Some(Package {
                    name: crate_name.clone(),
                    version: Some(this.get_default_version(&crate_name)),
                    source: PackageSource::Custom("cargo".to_string()),
                });
            }
        }

        None
    }
}
