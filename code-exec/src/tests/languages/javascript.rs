use super::{
    fixtures::{code_samples::JS_HELLO, code_with_deps::JS_WITH_DEPS, test_scenarios::*},
    utils::dependencies::lodash_dependency,
};
use crate::{
    executor::LanguageExecutor,
    languages::{check_requirements, JavaScriptExecutor, ToolCheck},
    CodeExecutionService, Dependency, Error, ExecutionRequest, Language,
};

use super::*;

#[tokio::test]
async fn test_javascript_basic() -> std::result::Result<(), Error> {
    test_language_execution(Language::JavaScript, JS_HELLO).await
}

#[tokio::test]
async fn test_javascript_with_deps() -> std::result::Result<(), Error> {
    test_language_with_deps(
        Language::JavaScript,
        JS_WITH_DEPS,
        vec![lodash_dependency()],
    )
    .await
}

#[tokio::test]
async fn test_javascript_timeout() -> std::result::Result<(), Error> {
    test_language_timeout(Language::JavaScript, JS_WITH_TIMEOUT).await
}

// JavaScript executor specific tests
#[tokio::test]
async fn test_javascript_requirements() -> std::result::Result<(), Error> {
    let executor = JavaScriptExecutor::new(None);
    check_requirements(&executor).await?;
    Ok(())
}
