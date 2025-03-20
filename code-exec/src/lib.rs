//! # Code Execution Service
//!
//! A secure code execution service that runs arbitrary code snippets in TEE environments.
//! Supports multiple programming languages with secure isolation and resource limits.

pub mod error;
pub mod executor;
pub mod languages;
pub mod sandbox;
pub mod service;
pub mod types;

pub use crate::languages::*;
pub use error::Error;
pub use executor::{CodeExecutor, LanguageExecutor};
pub use service::CodeExecutionService;
pub use types::{
    Dependency, ExecutionRequest, ExecutionResult, ExecutionStatus, Language, ProcessStats,
    ResourceLimits,
};

/// Result type for code execution operations
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests;
