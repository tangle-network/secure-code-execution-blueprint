use super::DependencyAnalyzer;
use crate::{
    error::Result,
    types::{Package, PackageSource},
};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Default, Clone)]
pub struct PythonAnalyzer {
    import_re: Option<Regex>,
    pip_re: Option<Regex>,
    default_versions: HashMap<String, String>,
}

impl PythonAnalyzer {
    fn get_import_re(&mut self) -> &Regex {
        if self.import_re.is_none() {
            self.import_re = Some(
                Regex::new(
                    r"(?m)^\s*(?:from\s+([a-zA-Z0-9_.-]+)(?:\s+import\s+.*)?|import\s+([a-zA-Z0-9_.-]+)(?:\s+as\s+.*)?)"
                )
                .unwrap(),
            );
        }
        self.import_re.as_ref().unwrap()
    }

    fn get_pip_re(&mut self) -> &Regex {
        if self.pip_re.is_none() {
            self.pip_re = Some(
                Regex::new(
                    r"#\s*pip\s*:\s*([a-zA-Z0-9_-]+)(?:>=|==|<=|>|<|~=|!=)?([0-9a-zA-Z.-]*)",
                )
                .unwrap(),
            );
        }
        self.pip_re.as_ref().unwrap()
    }

    fn is_stdlib_module(&self, module: &str) -> bool {
        // List of common standard library modules
        static STDLIB: &[&str] = &[
            "os",
            "sys",
            "re",
            "math",
            "random",
            "datetime",
            "time",
            "json",
            "collections",
            "itertools",
            "functools",
            "typing",
            "pathlib",
            "subprocess",
            "argparse",
            "logging",
            "unittest",
            "threading",
        ];

        STDLIB.contains(&module)
    }

    fn initialize_default_versions(&mut self) {
        if self.default_versions.is_empty() {
            let defaults = [
                // Data processing
                ("numpy", ">=1.24.0"),
                ("pandas", ">=2.0.0"),
                ("scipy", ">=1.10.0"),
                // Machine Learning
                ("scikit-learn", ">=1.2.0"),
                ("tensorflow", ">=2.12.0"),
                ("torch", ">=2.0.0"),
                // API and Web
                ("fastapi", ">=0.95.0"),
                ("flask", ">=2.3.0"),
                ("requests", ">=2.30.0"),
                ("aiohttp", ">=3.8.0"),
                ("httpx", ">=0.24.0"),
                // AWS Lambda
                ("boto3", ">=1.26.0"),
                ("aws-lambda-powertools", ">=2.18.0"),
                // Database
                ("sqlalchemy", ">=2.0.0"),
                ("pymongo", ">=4.3.0"),
                ("redis", ">=4.5.0"),
                // Utils
                ("pydantic", ">=2.0.0"),
                ("python-dotenv", ">=1.0.0"),
                ("pillow", ">=9.5.0"),
            ];

            self.default_versions = defaults
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        }
    }

    fn get_default_version(&self, package_name: &str) -> String {
        self.default_versions
            .get(package_name)
            .cloned()
            .unwrap_or_else(|| ">=1.0.0".to_string())
    }
}

#[async_trait]
impl DependencyAnalyzer for PythonAnalyzer {
    fn language(&self) -> &'static str {
        "python"
    }

    async fn analyze_dependencies(&self, source_code: &str) -> Result<Vec<Package>> {
        let mut this = self.clone();
        this.initialize_default_versions();
        let mut packages = HashSet::new();
        let mut explicit_versions = HashMap::new();

        // Process each line
        for line in source_code.lines() {
            // Check for explicit pip dependencies
            if let Some(cap) = this.get_pip_re().captures(line) {
                let name = cap.get(1).unwrap().as_str();
                let version = cap.get(2).map(|m| m.as_str().to_string());
                explicit_versions.insert(name.to_string(), version);
            }

            // Check for imports
            if let Some(cap) = this.get_import_re().captures(line) {
                let module = cap
                    .get(1)
                    .or_else(|| cap.get(2))
                    .map(|m| m.as_str())
                    .unwrap_or("");

                let base_module = module.split('.').next().unwrap_or(module);

                if !this.is_stdlib_module(base_module) {
                    // Map certain module names to their package names
                    let package_name = match base_module {
                        "PIL" => "pillow",
                        _ => base_module,
                    };

                    let version = explicit_versions
                        .get(package_name)
                        .cloned()
                        .flatten()
                        .unwrap_or_else(|| this.get_default_version(package_name));

                    packages.insert(Package {
                        name: package_name.to_string(),
                        version: Some(version),
                        source: PackageSource::Custom("pip".to_string()),
                    });
                }
            }
        }

        Ok(packages.into_iter().collect())
    }

    fn can_handle(&self, source_code: &str) -> bool {
        // Python-specific patterns
        let has_python_shebang = source_code.starts_with("#!/usr/bin/env python")
            || source_code.starts_with("#!/usr/bin/python");

        let has_python_imports = source_code.contains("import ") 
            && !source_code.contains("from '")  // Not JS/TS
            && !source_code.contains("package main"); // Not Go

        let has_python_def = source_code.contains("def ")
            && source_code.contains(":")
            && !source_code.contains("package main"); // Not Go

        has_python_shebang || (has_python_imports && has_python_def)
    }

    fn is_dependency_line(&self, line: &str) -> bool {
        let mut this = self.clone();
        this.get_import_re().is_match(line) || this.get_pip_re().is_match(line)
    }

    fn extract_package_info(&self, line: &str) -> Option<Package> {
        let mut this = self.clone();

        // Try pip comment first
        if let Some(cap) = this.get_pip_re().captures(line) {
            let name = cap.get(1).unwrap().as_str();
            let version = cap.get(2).map(|m| m.as_str().to_string());
            return Some(Package {
                name: name.to_string(),
                version,
                source: PackageSource::Custom("pip".to_string()),
            });
        }

        // Try import statement
        if let Some(cap) = this.get_import_re().captures(line) {
            let module = cap
                .get(1)
                .or_else(|| cap.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");

            let base_module = module.split('.').next().unwrap_or(module);

            if !this.is_stdlib_module(base_module) {
                return Some(Package {
                    name: base_module.to_string(),
                    version: None,
                    source: PackageSource::Custom("pip".to_string()),
                });
            }
        }

        None
    }
}
