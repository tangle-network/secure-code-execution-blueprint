use super::DependencyAnalyzer;
use crate::{
    error::Result,
    types::{Package, PackageSource},
};
use async_trait::async_trait;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
pub struct JavaScriptAnalyzer {
    import_re: Option<Regex>,
    version_re: Option<Regex>,
    default_versions: HashMap<String, String>,
}

impl JavaScriptAnalyzer {
    fn get_import_re(&mut self) -> &Regex {
        if self.import_re.is_none() {
            self.import_re = Some(
                Regex::new(r#"(?m)^(?:(?:const|let|var)\s*\{[^}]*\}\s*=\s*require\(['"]([^'"]+)['"]\)|(?:const|let|var)\s+\w+\s*=\s*require\(['"]([^'"]+)['"]\)|import\s+.*?from\s+['"]([^'"]+)['"])"#)
                    .unwrap(),
            );
        }
        self.import_re.as_ref().unwrap()
    }

    fn get_version_re(&mut self) -> &Regex {
        if self.version_re.is_none() {
            self.version_re =
                Some(Regex::new(r#"//\s*npm:\s*([@\w\-/.]+)@([\w\-~^.<>=]+)"#).unwrap());
        }
        self.version_re.as_ref().unwrap()
    }

    fn initialize_default_versions(&mut self) {
        if self.default_versions.is_empty() {
            let defaults = [
                // AWS Lambda
                ("@aws-sdk/client-lambda", "^3.350.0"),
                ("@aws-sdk/client-s3", "^3.350.0"),
                ("@aws-sdk/client-dynamodb", "^3.350.0"),
                // HTTP and API
                ("axios", "^1.4.0"),
                ("express", "^4.18.0"),
                ("fastify", "^4.17.0"),
                ("node-fetch", "^3.3.0"),
                // Image Processing
                ("sharp", "^0.32.0"),
                // Database
                ("mongodb", "^5.6.0"),
                ("mongoose", "^7.2.0"),
                ("pg", "^8.11.0"),
                ("redis", "^4.6.0"),
                // Utils
                ("lodash", "^4.17.21"),
                ("date-fns", "^2.30.0"),
                ("zod", "^3.21.0"),
                ("uuid", "^9.0.0"),
                // Cloud Functions
                ("@google-cloud/functions-framework", "^3.3.0"),
                ("@azure/functions", "^3.5.0"),
                // Testing
                ("jest", "^29.5.0"),
                ("supertest", "^6.3.0"),
            ];

            self.default_versions = defaults
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        }
    }

    fn is_local_import(&self, import_path: &str) -> bool {
        import_path.starts_with('.') || import_path.starts_with('/')
    }

    fn get_default_version(&self, package_name: &str) -> String {
        self.default_versions
            .get(package_name)
            .cloned()
            .unwrap_or_else(|| "^1.0.0".to_string())
    }
}

#[async_trait]
impl DependencyAnalyzer for JavaScriptAnalyzer {
    fn language(&self) -> &'static str {
        "javascript"
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

        // Then process imports
        let import_re = this.get_import_re().clone();
        for cap in import_re.captures_iter(source_code) {
            let import_path = cap
                .get(1)
                .or_else(|| cap.get(2))
                .or_else(|| cap.get(3))
                .unwrap()
                .as_str();
            if !this.is_local_import(import_path) {
                // Get the base package name (handle scoped packages)
                let package_name = if import_path.contains('/') {
                    import_path.split('/').take(2).collect::<Vec<_>>().join("/")
                } else {
                    import_path.to_string()
                };

                // Use explicit version if available, otherwise use default
                let version = explicit_versions
                    .get(&package_name)
                    .cloned()
                    .unwrap_or_else(|| this.get_default_version(&package_name));

                packages.insert(Package {
                    name: package_name,
                    version: Some(version),
                    source: PackageSource::Custom("npm".to_string()),
                });
            }
        }

        Ok(packages.into_iter().collect())
    }

    fn can_handle(&self, source_code: &str) -> bool {
        // JavaScript-specific patterns
        (source_code.contains("require(") || source_code.contains("import "))
            && !source_code.contains("import type")  // Not TypeScript
            && !source_code.contains("fn main()")    // Not Rust
            && !source_code.contains("package main") // Not Go
            && !source_code.contains("#!/usr/bin/env python") // Not Python
    }

    fn is_dependency_line(&self, line: &str) -> bool {
        let mut this = self.clone();
        this.get_import_re().is_match(line) || this.get_version_re().is_match(line)
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
                source: PackageSource::Custom("npm".to_string()),
            });
        }

        // Try import statement
        if let Some(cap) = this.get_import_re().captures(line) {
            let import_path = cap.get(1).or_else(|| cap.get(2)).unwrap().as_str();
            if !this.is_local_import(import_path) {
                let package_name = if import_path.contains('/') {
                    import_path.split('/').take(2).collect::<Vec<_>>().join("/")
                } else {
                    import_path.to_string()
                };

                return Some(Package {
                    name: package_name.clone(),
                    version: Some(this.get_default_version(&package_name)),
                    source: PackageSource::Custom("npm".to_string()),
                });
            }
        }

        None
    }
}
