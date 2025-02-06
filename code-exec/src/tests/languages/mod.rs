use super::utils::defaults::*;
pub use super::*;
use crate::{CodeExecutionService, Dependency, Error, ExecutionRequest, Language, Result};
use tokio::time::Duration;

pub mod golang;
pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

// Common test utilities for language tests
pub(crate) async fn test_language_execution(language: Language, code: &str) -> Result<()> {
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

pub(crate) async fn test_language_with_deps(
    language: Language,
    code: &str,
    dependencies: Vec<Dependency>,
) -> Result<()> {
    let service = CodeExecutionService::new(1, default_test_limits()).await?;

    let request = ExecutionRequest {
        language,
        code: code.to_string(),
        input: None,
        dependencies,
        timeout: extended_timeout(),
        env_vars: Default::default(),
    };

    let result = service.execute(request).await?;
    assert!(!result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    Ok(())
}

pub(crate) async fn test_language_timeout(language: Language, code: &str) -> Result<()> {
    let service = CodeExecutionService::new(1, default_test_limits()).await?;

    let request = ExecutionRequest {
        language,
        code: code.to_string(),
        input: None,
        dependencies: vec![],
        timeout: Duration::from_millis(100),
        env_vars: Default::default(),
    };

    let result = service.execute(request).await;
    assert!(matches!(result, Err(Error::Timeout(_))));
    Ok(())
}
