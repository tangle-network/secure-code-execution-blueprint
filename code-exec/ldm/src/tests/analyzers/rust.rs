use crate::{
    analyzer::{rust::RustAnalyzer, DependencyAnalyzer},
    PackageSource,
};

#[tokio::test]
async fn test_rust_dependency_analysis() {
    let analyzer = RustAnalyzer::default();

    let test_cases = vec![
        (
            // Basic use statement
            r#"
            use tokio::runtime::Runtime;
            use serde::Deserialize;
            "#,
            vec!["tokio", "serde"],
        ),
        (
            // AWS SDK imports
            r#"
            use aws_config::meta::region::RegionProviderChain;
            use aws_sdk_dynamodb::Client;
            "#,
            vec!["aws-config", "aws-sdk-dynamodb"],
        ),
        (
            // Complex nested imports
            r#"
            use serde_json::{Value, Map};
            use tokio::{
                sync::{mpsc, oneshot},
                time::sleep,
            };
            "#,
            vec!["serde_json", "tokio"],
        ),
        (
            // Explicit version requirements
            r#"
            // cargo-version: tokio = "1.25"
            use tokio::runtime::Runtime;
            "#,
            vec!["tokio"],
        ),
        (
            // Standard library imports (should be ignored)
            r#"
            use std::collections::HashMap;
            use core::fmt::Debug;
            use tokio::sync::Mutex;
            "#,
            vec!["tokio"],
        ),
        (
            // External crate declarations
            r#"
            extern crate serde;
            use serde::Deserialize;
            "#,
            vec!["serde"],
        ),
    ];

    for (source_code, expected_crates) in test_cases {
        let packages = analyzer.analyze_dependencies(source_code).await.unwrap();

        // Verify language detection
        assert!(analyzer.can_handle(source_code));

        // Check that all expected crates are present
        for expected_crate in expected_crates {
            assert!(
                packages.iter().any(|p| p.name == expected_crate),
                "Expected crate '{}' not found in packages: {:?}",
                expected_crate,
                packages
            );
        }

        // Verify versions are present
        for package in packages {
            assert!(
                package.version.is_some(),
                "Package {} is missing version",
                package.name
            );
            assert_eq!(package.source, PackageSource::Custom("cargo".to_string()));
        }
    }
}

#[tokio::test]
async fn test_rust_language_detection() {
    let analyzer = RustAnalyzer::default();

    let rust_code = r#"
    use tokio::runtime::Runtime;
    
    fn main() {
        let rt = Runtime::new().unwrap();
    }
    "#;

    let typescript_code = r#"
    import { useState } from 'react';
    
    function App() {
        const [count, setCount] = useState(0);
        return <div>{count}</div>;
    }
    "#;

    let python_code = r#"
    import pandas as pd
    
    def main():
        df = pd.DataFrame({'a': [1, 2, 3]})
    "#;

    assert!(analyzer.can_handle(rust_code));
    assert!(!analyzer.can_handle(typescript_code));
    assert!(!analyzer.can_handle(python_code));
}

#[tokio::test]
async fn test_rust_version_parsing() {
    let analyzer = RustAnalyzer::default();

    let source_with_version = r#"
    // cargo-version: tokio = "1.25"
    // cargo-version: serde = "2.0"
    use tokio::runtime::Runtime;
    use serde::Deserialize;
    "#;

    let packages = analyzer
        .analyze_dependencies(source_with_version)
        .await
        .unwrap();

    let tokio_pkg = packages.iter().find(|p| p.name == "tokio").unwrap();
    let serde_pkg = packages.iter().find(|p| p.name == "serde").unwrap();

    assert_eq!(tokio_pkg.version.as_ref().unwrap(), "1.25");
    assert_eq!(serde_pkg.version.as_ref().unwrap(), "2.0");
}
