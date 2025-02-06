use super::{
    fixtures::{
        code_samples::RUST_HELLO,
        code_with_deps::RUST_WITH_DEPS,
        test_scenarios::{RUST_RESOURCE_HEAVY, RUST_WITH_TIMEOUT},
    },
    utils::dependencies::serde_dependencies,
};
use crate::{
    languages::{check_requirements, RustExecutor},
    Error,
};

use super::*;

#[tokio::test]
async fn test_rust_basic() -> std::result::Result<(), Error> {
    test_language_execution(Language::Rust, RUST_HELLO).await
}

#[tokio::test]
async fn test_rust_with_deps() -> std::result::Result<(), Error> {
    test_language_with_deps(Language::Rust, RUST_WITH_DEPS, serde_dependencies()).await
}

#[tokio::test]
async fn test_rust_timeout() -> std::result::Result<(), Error> {
    test_language_timeout(Language::Rust, RUST_WITH_TIMEOUT).await
}

#[tokio::test]
async fn test_rust_resource_limits() -> std::result::Result<(), Error> {
    test_language_timeout(Language::Rust, RUST_RESOURCE_HEAVY).await
}

// Rust executor specific tests
#[tokio::test]
async fn test_rust_requirements() -> std::result::Result<(), Error> {
    let executor = RustExecutor::new(None);
    check_requirements(&executor).await?;
    Ok(())
}
