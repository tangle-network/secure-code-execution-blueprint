use super::fixtures::{
    code_samples::GO_HELLO,
    code_with_deps::GO_WITH_DEPS,
    test_scenarios::{GO_RESOURCE_HEAVY, GO_WITH_TIMEOUT},
};
use crate::{
    executor::LanguageExecutor,
    languages::{check_requirements, GoExecutor, ToolCheck},
    Dependency, Error,
};

use super::*;

#[tokio::test]
async fn test_go_basic() -> std::result::Result<(), Error> {
    test_language_execution(Language::Go, GO_HELLO).await
}

#[tokio::test]
async fn test_go_with_deps() -> std::result::Result<(), Error> {
    let uuid_dep = Dependency {
        name: "github.com/google/uuid".to_string(),
        version: "1.4.0".to_string(),
        source: None,
    };
    test_language_with_deps(Language::Go, GO_WITH_DEPS, vec![uuid_dep]).await
}

#[tokio::test]
async fn test_go_timeout() -> std::result::Result<(), Error> {
    test_language_timeout(Language::Go, GO_WITH_TIMEOUT).await
}

#[tokio::test]
async fn test_go_resource_limits() -> std::result::Result<(), Error> {
    test_language_timeout(Language::Go, GO_RESOURCE_HEAVY).await
}

// Go executor specific tests
#[tokio::test]
async fn test_go_requirements() -> std::result::Result<(), Error> {
    let executor = GoExecutor::new(None);
    check_requirements(&executor).await?;
    Ok(())
}
