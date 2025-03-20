use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),

    #[error("Compilation failed: {0}")]
    CompilationError(String),

    #[error("Execution failed: {0}")]
    ExecutionError(String),

    #[error("Timeout after {0} seconds")]
    Timeout(u64),

    #[error("System error: {0}")]
    System(String),

    #[error("Invalid dependency specification: {0}")]
    InvalidDependency(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Sandbox error: {0}")]
    Sandbox(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Resource exceeded: {0}")]
    ResourceExceeded(String),

    #[error("Resource limit error: {0}")]
    ResourceLimitError(String),
}
