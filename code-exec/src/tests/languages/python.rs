use super::{
    fixtures::test_scenarios::{PYTHON_RESOURCE_HEAVY, PYTHON_WITH_INPUT},
    fixtures::{code_samples::PYTHON_HELLO, code_with_deps::PYTHON_WITH_DEPS, test_scenarios::*},
    utils::dependencies::numpy_dependency,
};
use crate::{
    executor::LanguageExecutor,
    languages::{check_requirements, PythonExecutor},
    CodeExecutionService, Dependency, Error, ExecutionRequest, Language,
};
use tempfile::tempdir;
use tokio::process::Command;

use super::*;

#[tokio::test]
async fn test_python_basic() -> std::result::Result<(), Error> {
    test_language_execution(Language::Python, PYTHON_HELLO).await
}

#[tokio::test]
async fn test_python_with_deps() -> std::result::Result<(), Error> {
    test_language_with_deps(Language::Python, PYTHON_WITH_DEPS, vec![numpy_dependency()]).await
}

#[tokio::test]
async fn test_python_timeout() -> std::result::Result<(), Error> {
    test_language_timeout(Language::Python, PYTHON_RESOURCE_HEAVY).await
}

#[tokio::test]
async fn test_python_input() -> std::result::Result<(), Error> {
    let service = CodeExecutionService::new(1, default_test_limits()).await?;

    let request = ExecutionRequest {
        language: Language::Python,
        code: PYTHON_WITH_INPUT.to_string(),
        input: Some("test user\n".to_string()),
        dependencies: vec![],
        timeout: default_timeout(),
        env_vars: Default::default(),
    };

    let result = service.execute(request).await?;
    assert!(result.stdout.contains("Hello, test user"));
    assert!(result.stderr.is_empty());
    Ok(())
}

// Python executor specific tests
#[tokio::test]
async fn test_python_requirements() -> std::result::Result<(), Error> {
    let executor = PythonExecutor::new(None);
    check_requirements(&executor).await?;
    Ok(())
}
