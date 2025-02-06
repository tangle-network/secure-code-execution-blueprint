use super::{
    fixtures::code_samples::*,
    fixtures::code_with_deps::*,
    fixtures::test_scenarios::*,
    utils::defaults::{default_test_limits, default_timeout, extended_timeout},
    utils::dependencies::{lodash_dependency, numpy_dependency, serde_dependencies},
};
use crate::{
    executor::CodeExecutor, sandbox::Sandbox, CodeExecutionService, Error, ExecutionRequest,
    Language, Result,
};
use std::collections::HashMap;
use tokio::time::Duration;

/// Basic language execution tests
pub mod basic_execution {
    use super::*;

    async fn test_language_execution(language: Language, code: &str) -> Result<()> {
        let service = CodeExecutionService::new(1, default_test_limits()).await?;

        let request = ExecutionRequest {
            language,
            code: code.to_string(),
            input: None,
            dependencies: vec![],
            timeout: default_timeout(),
            env_vars: Default::default(),
        };

        let result = service.execute(request).await?;
        assert!(result.stdout.contains("Hello from"));
        assert!(result.stderr.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_python_execution() -> Result<()> {
        test_language_execution(Language::Python, PYTHON_HELLO).await
    }

    #[tokio::test]
    async fn test_javascript_execution() -> Result<()> {
        test_language_execution(Language::JavaScript, JS_HELLO).await
    }

    #[tokio::test]
    async fn test_typescript_execution() -> Result<()> {
        test_language_execution(Language::TypeScript, TS_HELLO).await
    }

    #[tokio::test]
    async fn test_go_execution() -> Result<()> {
        test_language_execution(Language::Go, GO_HELLO).await
    }
}

/// Tests for concurrent execution
pub mod concurrent_execution {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_mixed_execution() -> Result<()> {
        let service = CodeExecutionService::new(3, default_test_limits()).await?;

        let requests = vec![
            (Language::Python, PYTHON_HELLO),
            (Language::JavaScript, JS_HELLO),
            (Language::TypeScript, TS_HELLO),
        ];

        let mut handles = vec![];
        for (language, code) in requests {
            let request = ExecutionRequest {
                language,
                code: code.to_string(),
                input: None,
                dependencies: vec![],
                timeout: default_timeout(),
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

    #[tokio::test]
    async fn test_concurrent_with_deps() -> Result<()> {
        let service = CodeExecutionService::new(3, default_test_limits()).await?;

        let requests = vec![
            ExecutionRequest {
                language: Language::Python,
                code: PYTHON_WITH_DEPS.to_string(),
                dependencies: vec![numpy_dependency()],
                input: None,
                timeout: extended_timeout(),
                env_vars: Default::default(),
            },
            ExecutionRequest {
                language: Language::JavaScript,
                code: JS_WITH_DEPS.to_string(),
                dependencies: vec![lodash_dependency()],
                input: None,
                timeout: extended_timeout(),
                env_vars: Default::default(),
            },
            ExecutionRequest {
                language: Language::Rust,
                code: RUST_WITH_DEPS.to_string(),
                dependencies: serde_dependencies(),
                input: None,
                timeout: extended_timeout(),
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
        }

        Ok(())
    }
}

/// Tests for error conditions and resource limits
pub mod error_handling {
    use super::*;

    #[tokio::test]
    async fn test_timeout_handling() -> Result<()> {
        let service = CodeExecutionService::new(1, default_test_limits()).await?;

        let request = ExecutionRequest {
            language: Language::JavaScript,
            code: JS_WITH_TIMEOUT.to_string(),
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
    async fn test_resource_limits() -> Result<()> {
        let executor = CodeExecutor::new().await?;
        let mut sandbox = Sandbox::new(default_test_limits()).await?;

        let request = ExecutionRequest {
            language: Language::Python,
            code: PYTHON_RESOURCE_HEAVY.to_string(),
            dependencies: vec![],
            env_vars: HashMap::new(),
            input: None,
            timeout: default_timeout(),
        };

        let result = executor.execute_in_sandbox(request, &mut sandbox).await;

        #[cfg(target_os = "linux")]
        {
            assert!(matches!(result, Err(Error::ResourceExceeded(_))));
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, we can't enforce memory limits, so the program should complete
            assert!(result.is_ok());
        }

        Ok(())
    }
}
