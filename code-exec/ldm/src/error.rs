use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Package manager error: {0}")]
    PackageManager(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Version error: {0}")]
    Version(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Environment error: {0}")]
    Environment(String),

    #[error("System error: {0}")]
    System(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, Error>;
