use std::time::Duration;

use crate::{
    error::Error,
    service::CodeExecutionService,
    types::{ExecutionRequest, Language, ResourceLimits},
};

mod test_cases {
    pub const PYTHON_HELLO: &str = r#"print("Hello from Python!")"#;
    pub const JS_HELLO: &str = r#"console.log('Hello from JavaScript!')"#;
    pub const TS_HELLO: &str = r#"console.log('Hello from TypeScript!')"#;
    pub const JAVA_HELLO: &str = r#"
        public class Main {
            public static void main(String[] args) {
                System.out.println("Hello from Java!");
            }
        }
    "#;
    pub const GO_HELLO: &str = r#"
        package main
        import "fmt"
        func main() {
            fmt.Println("Hello from Go!")
        }
    "#;
    pub const CPP_HELLO: &str = r#"
        #include <iostream>
        int main() {
            std::cout << "Hello from C++!" << std::endl;
            return 0;
        }
    "#;
    pub const PHP_HELLO: &str = r#"<?php echo "Hello from PHP!\n"; ?>"#;
    pub const SWIFT_HELLO: &str = r#"print("Hello from Swift!")"#;
    pub const RUST_HELLO: &str = r#"
        fn main() {
            println!("Hello from Rust!");
        }
    "#;
    pub const PYTHON_WITH_DEPS: &str = r#"
        import numpy as np
        arr = np.array([1, 2, 3])
        print(f"NumPy sum: {arr.sum()}")
    "#;
    pub const JS_WITH_DEPS: &str = r#"
        const _ = require('lodash');
        console.log(_.capitalize('hello world'));
    "#;
    pub const RUST_WITH_DEPS: &str = r#"
        use serde_json::json;
        
        fn main() {
            let data = json!({
                "message": "Hello from Rust with serde!"
            });
            println!("{}", data.to_string());
        }
    "#;
    pub const PYTHON_MULTILINE: &str = r#"
        def factorial(n):
            if n <= 1:
                return 1
            return n * factorial(n - 1)

        result = factorial(5)
        print(f"Factorial of 5 is {result}")
    "#;
    pub const PYTHON_WITH_INPUT: &str = r#"
        name = input()
        print(f"Hello, {name}!")
    "#;
    pub const JS_WITH_TIMEOUT: &str = r#"
        setTimeout(() => {
            console.log('This should not print due to timeout');
        }, 6000);
    "#;
    pub const PYTHON_RESOURCE_HEAVY: &str = r#"
        # Should be limited by memory constraints
        big_list = list(range(10**7))
        print(len(big_list))
    "#;
}

async fn test_language_execution(language: Language, code: &str) -> Result<(), Error> {
    let service = CodeExecutionService::new(1, ResourceLimits::default()).await?;

    let request = ExecutionRequest {
        language,
        code: code.to_string(),
        input: None,
        dependencies: vec![],
        timeout: Duration::from_secs(10),
        env_vars: Default::default(),
    };

    let result = service.execute(request).await?;
    assert!(result.stdout.contains("Hello from"));
    assert!(result.stderr.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_python_execution() -> Result<(), Error> {
    test_language_execution(Language::Python, test_cases::PYTHON_HELLO).await
}

#[tokio::test]
async fn test_javascript_execution() -> Result<(), Error> {
    test_language_execution(Language::JavaScript, test_cases::JS_HELLO).await
}

#[tokio::test]
async fn test_typescript_execution() -> Result<(), Error> {
    test_language_execution(Language::TypeScript, test_cases::TS_HELLO).await
}

#[tokio::test]
async fn test_java_execution() -> Result<(), Error> {
    test_language_execution(Language::Java, test_cases::JAVA_HELLO).await
}

#[tokio::test]
async fn test_go_execution() -> Result<(), Error> {
    test_language_execution(Language::Go, test_cases::GO_HELLO).await
}

#[tokio::test]
async fn test_cpp_execution() -> Result<(), Error> {
    test_language_execution(Language::Cpp, test_cases::CPP_HELLO).await
}

#[tokio::test]
async fn test_php_execution() -> Result<(), Error> {
    test_language_execution(Language::Php, test_cases::PHP_HELLO).await
}

#[tokio::test]
async fn test_swift_execution() -> Result<(), Error> {
    test_language_execution(Language::Swift, test_cases::SWIFT_HELLO).await
}

#[tokio::test]
async fn test_concurrent_mixed_execution() -> Result<(), Error> {
    let service = CodeExecutionService::new(3, ResourceLimits::default()).await?;

    let requests = vec![
        (Language::Python, test_cases::PYTHON_HELLO),
        (Language::JavaScript, test_cases::JS_HELLO),
        (Language::TypeScript, test_cases::TS_HELLO),
    ];

    let mut handles = vec![];
    for (language, code) in requests {
        let request = ExecutionRequest {
            language,
            code: code.to_string(),
            input: None,
            dependencies: vec![],
            timeout: Duration::from_secs(10),
            env_vars: Default::default(),
        };
        let service_clone = service.clone();
        handles.push(tokio::spawn(
            async move { service_clone.execute(request).await },
        ));
    }

    for handle in handles {
        let result = handle.await.unwrap()?;
        assert!(result.stdout.contains("Hello from"));
        assert!(result.stderr.is_empty());
    }

    Ok(())
}

// Add dependency definitions
fn numpy_dependency() -> crate::types::Dependency {
    crate::types::Dependency {
        name: "numpy".to_string(),
        version: "1.24.0".to_string(),
        source: None,
    }
}

fn lodash_dependency() -> crate::types::Dependency {
    crate::types::Dependency {
        name: "lodash".to_string(),
        version: "4.17.21".to_string(),
        source: None,
    }
}

fn serde_dependencies() -> Vec<crate::types::Dependency> {
    vec![
        crate::types::Dependency {
            name: "serde".to_string(),
            version: "1.0".to_string(),
            source: None,
        },
        crate::types::Dependency {
            name: "serde_json".to_string(),
            version: "1.0".to_string(),
            source: None,
        },
    ]
}

// Add new test cases
#[tokio::test]
async fn test_python_with_deps() -> Result<(), Error> {
    let service = CodeExecutionService::new(1, ResourceLimits::default()).await?;

    let request = ExecutionRequest {
        language: Language::Python,
        code: test_cases::PYTHON_WITH_DEPS.to_string(),
        input: None,
        dependencies: vec![numpy_dependency()],
        timeout: Duration::from_secs(30),
        env_vars: Default::default(),
    };

    let result = service.execute(request).await?;
    assert!(result.stdout.contains("NumPy sum: 6"));
    assert!(result.stderr.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_python_with_input() -> Result<(), Error> {
    let service = CodeExecutionService::new(1, ResourceLimits::default()).await?;

    let request = ExecutionRequest {
        language: Language::Python,
        code: test_cases::PYTHON_WITH_INPUT.to_string(),
        input: Some("Test User".to_string()),
        dependencies: vec![],
        timeout: Duration::from_secs(5),
        env_vars: Default::default(),
    };

    let result = service.execute(request).await?;
    assert_eq!(result.stdout.trim(), "Hello, Test User!");
    Ok(())
}

#[tokio::test]
async fn test_timeout_handling() -> Result<(), Error> {
    let service = CodeExecutionService::new(1, ResourceLimits::default()).await?;

    let request = ExecutionRequest {
        language: Language::JavaScript,
        code: test_cases::JS_WITH_TIMEOUT.to_string(),
        input: None,
        dependencies: vec![],
        timeout: Duration::from_secs(2),
        env_vars: Default::default(),
    };

    let result = service.execute(request).await;
    assert!(matches!(result, Err(Error::Timeout(_))));
    Ok(())
}

#[tokio::test]
async fn test_resource_limits() -> Result<(), Error> {
    let limits = ResourceLimits {
        memory: 10 * 1024 * 1024, // 10MB
        ..ResourceLimits::default()
    };

    let service = CodeExecutionService::new(1, limits).await?;

    let request = ExecutionRequest {
        language: Language::Python,
        code: test_cases::PYTHON_RESOURCE_HEAVY.to_string(),
        input: None,
        dependencies: vec![],
        timeout: Duration::from_secs(5),
        env_vars: Default::default(),
    };

    let result = service.execute(request).await;
    assert!(matches!(result, Err(Error::ResourceExceeded(_))));
    Ok(())
}

#[tokio::test]
async fn test_concurrent_with_deps() -> Result<(), Error> {
    let service = CodeExecutionService::new(3, ResourceLimits::default()).await?;

    let requests = vec![
        ExecutionRequest {
            language: Language::Python,
            code: test_cases::PYTHON_WITH_DEPS.to_string(),
            dependencies: vec![numpy_dependency()],
            input: None,
            timeout: Duration::from_secs(30),
            env_vars: Default::default(),
        },
        ExecutionRequest {
            language: Language::JavaScript,
            code: test_cases::JS_WITH_DEPS.to_string(),
            dependencies: vec![lodash_dependency()],
            input: None,
            timeout: Duration::from_secs(30),
            env_vars: Default::default(),
        },
        ExecutionRequest {
            language: Language::Rust,
            code: test_cases::RUST_WITH_DEPS.to_string(),
            dependencies: serde_dependencies(),
            input: None,
            timeout: Duration::from_secs(30),
            env_vars: Default::default(),
        },
        ExecutionRequest {
            language: Language::Python,
            code: test_cases::PYTHON_MULTILINE.to_string(),
            dependencies: vec![],
            input: None,
            timeout: Duration::from_secs(30),
            env_vars: Default::default(),
        },
        ExecutionRequest {
            language: Language::Rust,
            code: test_cases::RUST_HELLO.to_string(),
            dependencies: vec![],
            input: None,
            timeout: Duration::from_secs(30),
            env_vars: Default::default(),
        },
    ];

    let mut handles = vec![];
    for request in requests {
        let service_clone = service.clone();
        handles.push(tokio::spawn(
            async move { service_clone.execute(request).await },
        ));
    }

    for handle in handles {
        let result = handle.await.unwrap()?;
        assert!(!result.stdout.is_empty());
        assert!(result.stderr.is_empty());
        // Verify specific outputs
        match result.stdout.as_str() {
            s if s.contains("NumPy sum: 6") => (),
            s if s.contains("Hello World") => (),
            s if s.contains("Hello from Rust with serde") => (),
            s => panic!("Unexpected output: {}", s),
        }
    }

    Ok(())
}
