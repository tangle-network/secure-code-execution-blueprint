use super::{
    fixtures::{code_samples::TS_HELLO, code_with_deps::JS_WITH_DEPS, test_scenarios::*},
    utils::dependencies::lodash_dependency,
};
use crate::{
    executor::LanguageExecutor,
    languages::{check_requirements, ToolCheck, TypeScriptExecutor},
    CodeExecutionService, Dependency, Error, ExecutionRequest, Language,
};

use super::*;

#[tokio::test]
async fn test_typescript_basic() -> std::result::Result<(), Error> {
    test_language_execution(Language::TypeScript, TS_HELLO).await
}

#[tokio::test]
async fn test_typescript_with_deps() -> std::result::Result<(), Error> {
    test_language_with_deps(
        Language::TypeScript,
        JS_WITH_DEPS, // Reuse JS code since it's compatible
        vec![lodash_dependency()],
    )
    .await
}

#[tokio::test]
async fn test_typescript_timeout() -> std::result::Result<(), Error> {
    test_language_timeout(Language::TypeScript, JS_WITH_TIMEOUT).await
}

// TypeScript executor specific tests
#[tokio::test]
async fn test_typescript_requirements() -> std::result::Result<(), Error> {
    let executor = TypeScriptExecutor::new(None, None);
    check_requirements(&executor).await?;
    Ok(())
}
