//! # Code Execution Service
//!
//! A secure code execution service that runs arbitrary code snippets in TEE environments.
//! Supports multiple programming languages with secure isolation and resource limits.

mod error;
mod executor;
mod languages;
mod sandbox;
mod types;

pub use error::Error;
pub use executor::CodeExecutor;
pub use types::*;

/// Result type for code execution operations
pub type Result<T> = std::result::Result<T, Error>;
