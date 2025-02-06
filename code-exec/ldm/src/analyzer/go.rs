use super::DependencyAnalyzer;
use crate::{
    error::Result,
    types::{Package, PackageSource},
};
use async_trait::async_trait;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone)]
pub struct GoAnalyzer {
    import_re: Option<Regex>,
    version_re: Option<Regex>,
    default_versions: HashMap<String, String>,
}

impl GoAnalyzer {
    fn get_import_re(&mut self) -> &Regex {
        if self.import_re.is_none() {
            // Match both single imports and multi-line import blocks
            self.import_re = Some(
                Regex::new(
                    r#"(?m)^[\s\n]*(?:import\s+"([^"]+)"|(?:import\s*\(\s*)?(?:[_.\w]+\s+)?"([^"]+)")"#
                )
                .unwrap(),
            );
        }
        self.import_re.as_ref().unwrap()
    }

    fn get_version_re(&mut self) -> &Regex {
        if self.version_re.is_none() {
            self.version_re =
                Some(Regex::new(r#"//\s*go:\s*require\s+([^\s]+)\s+v([\w\-.]+)"#).unwrap());
        }
        self.version_re.as_ref().unwrap()
    }

    fn initialize_default_versions(&mut self) {
        if self.default_versions.is_empty() {
            let defaults = [
                // AWS
                ("github.com/aws/aws-lambda-go", "1.41.0"),
                ("github.com/aws/aws-sdk-go-v2", "1.18.0"),
                // HTTP and API
                ("github.com/gin-gonic/gin", "1.9.0"),
                ("github.com/go-chi/chi", "5.0.8"),
                ("github.com/gorilla/mux", "1.8.0"),
                ("github.com/valyala/fasthttp", "1.47.0"),
                // Database
                ("github.com/lib/pq", "1.10.9"),
                ("go.mongodb.org/mongo-driver", "1.11.0"),
                ("github.com/go-redis/redis", "8.11.5"),
                ("gorm.io/gorm", "1.25.0"),
                // Utils
                ("github.com/spf13/viper", "1.15.0"),
                ("github.com/sirupsen/logrus", "1.9.0"),
                ("github.com/stretchr/testify", "1.8.0"),
                ("github.com/google/uuid", "1.3.0"),
                ("go.uber.org/zap", "1.24.0"),
                // Cloud Functions
                ("cloud.google.com/go", "1.13.0"),
                (
                    "github.com/GoogleCloudPlatform/functions-framework-go",
                    "1.7.0",
                ),
                ("github.com/Azure/azure-sdk-for-go", "68.0.0"),
            ];

            self.default_versions = defaults
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        }
    }

    fn is_std_import(&self, import_path: &str) -> bool {
        !import_path.contains('.')
    }

    fn get_default_version(&self, package_name: &str) -> String {
        self.default_versions
            .get(package_name)
            .cloned()
            .unwrap_or_else(|| "1.0.0".to_string())
    }

    fn is_definitely_go(&self, source_code: &str) -> bool {
        // Check for definitive Go patterns in order of specificity
        let patterns = [
            // Must have a package declaration
            (r"(?m)^[\s\n]*package\s+\w+\s*$", true),
            // Must have proper import syntax if imports exist
            (
                r#"(?m)^[\s\n]*import\s*\(\s*$|^[\s\n]*import\s+"[^"]+"\s*$"#,
                true,
            ),
            // Common Go patterns that should exist
            (r"(?m)^[\s\n]*type\s+\w+\s+struct\s*{", false),
            (
                r"(?m)^[\s\n]*func\s+\w+\s*\([^)]*\)\s*(?:\([^)]*\))?\s*{",
                false,
            ),
            // Patterns that indicate it's NOT Go
            (
                r"(?m)^def\s+\w+|^class\s+\w+|^import\s+os|#!/usr/bin/env\s+python",
                false,
            ),
        ];

        let mut required_matches = 0;
        let mut has_negative_pattern = false;

        for (pattern, required) in patterns.iter() {
            if let Ok(re) = Regex::new(pattern) {
                let matches = re.is_match(source_code);
                if *required {
                    if !matches {
                        return false;
                    }
                    required_matches += 1;
                } else if matches && pattern.contains("NOT Go") {
                    has_negative_pattern = true;
                }
            }
        }

        // Must have all required patterns and no negative patterns
        required_matches >= 2 && !has_negative_pattern
    }

    fn get_base_package(&self, import_path: &str) -> String {
        // For GitHub repos, always use github.com/org/repo
        if import_path.starts_with("github.com/") {
            let parts: Vec<&str> = import_path.split('/').collect();
            if parts.len() >= 3 {
                return parts[..3].join("/");
            }
        }

        // For other repos (like go.uber.org), use up to the second component
        // e.g., "go.uber.org/zap" -> "go.uber.org/zap"
        let parts: Vec<&str> = import_path.split('/').collect();
        if parts.len() >= 2 {
            parts[..2].join("/")
        } else {
            import_path.to_string()
        }
    }
}

#[async_trait]
impl DependencyAnalyzer for GoAnalyzer {
    fn language(&self) -> &'static str {
        "go"
    }

    async fn analyze_dependencies(&self, source_code: &str) -> Result<Vec<Package>> {
        let mut this = self.clone();
        this.initialize_default_versions();
        let mut packages = HashSet::new();
        let mut explicit_versions = HashMap::new();

        // First, collect any explicit version declarations
        for cap in this.get_version_re().captures_iter(source_code) {
            let name = cap.get(1).unwrap().as_str();
            let version = format!("v{}", cap.get(2).unwrap().as_str());
            explicit_versions.insert(name.to_string(), version);
        }

        // Then process imports
        let import_re = this.get_import_re().clone();
        let mut in_import_block = false;

        for line in source_code.lines() {
            let line = line.trim();

            // Check for import block start/end
            if line.starts_with("import (") {
                in_import_block = true;
                continue;
            } else if line == ")" && in_import_block {
                in_import_block = false;
                continue;
            }

            // Process imports
            if let Some(cap) = import_re.captures(line) {
                let import_path = if in_import_block {
                    // Inside import block, look for just the quoted path
                    if let Some(m) = cap.get(2) {
                        Some(m)
                    } else {
                        cap.get(1)
                    }
                } else {
                    // Single-line import
                    cap.get(1)
                };

                if let Some(import_path) = import_path {
                    let import_path = import_path.as_str();
                    if !this.is_std_import(import_path) {
                        let base_package = this.get_base_package(import_path);

                        let version = explicit_versions
                            .get(&base_package)
                            .cloned()
                            .unwrap_or_else(|| this.get_default_version(&base_package));
                        packages.insert(Package {
                            name: base_package,
                            version: Some(version),
                            source: PackageSource::Custom("go".to_string()),
                        });
                    } else {
                    }
                } else {
                }
            }
        }

        Ok(packages.into_iter().collect())
    }

    fn is_dependency_line(&self, line: &str) -> bool {
        let mut this = self.clone();
        this.get_import_re().is_match(line) || this.get_version_re().is_match(line)
    }

    fn can_handle(&self, source_code: &str) -> bool {
        self.is_definitely_go(source_code)
    }

    fn extract_package_info(&self, line: &str) -> Option<Package> {
        let mut this = self.clone();
        this.initialize_default_versions();

        // Try version comment first
        if let Some(cap) = this.get_version_re().captures(line) {
            let name = cap.get(1).unwrap().as_str();
            let version = format!("v{}", cap.get(2).unwrap().as_str());
            return Some(Package {
                name: name.to_string(),
                version: Some(version),
                source: PackageSource::Custom("go".to_string()),
            });
        }

        // Try import statement
        if let Some(cap) = this.get_import_re().captures(line) {
            if let Some(import_path) = cap.get(1) {
                let import_path = import_path.as_str();
                if !this.is_std_import(import_path) {
                    return Some(Package {
                        name: import_path.to_string(),
                        version: Some(this.get_default_version(import_path)),
                        source: PackageSource::Custom("go".to_string()),
                    });
                }
            }
        }

        None
    }
}
