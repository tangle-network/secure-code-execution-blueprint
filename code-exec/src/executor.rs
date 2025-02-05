use async_trait::async_trait;
use gadget_std::{end_timer, start_timer};
use std::path::PathBuf;
use tokio::fs;

use crate::{
    error::Error,
    languages::{
        CppExecutor, GoExecutor, JavaExecutor, JavaScriptExecutor, PhpExecutor, PythonExecutor,
        SwiftExecutor, TypeScriptExecutor,
    },
    sandbox::Sandbox,
    types::{ExecutionRequest, ExecutionResult, ExecutionStatus, Language, ResourceLimits},
};

/// Trait for language-specific code executors
#[async_trait]
pub trait LanguageExecutor: Send + Sync {
    /// Get the file extension for source files
    fn file_extension(&self) -> &str;

    /// Get the command to run the code
    fn run_command(&self) -> &str;

    /// Get additional arguments for the run command
    fn run_args(&self) -> Vec<String> {
        Vec::new()
    }

    /// Set up the execution environment
    async fn setup_environment(&self, _sandbox_dir: &PathBuf) -> Result<(), Error>;

    /// Install required dependencies
    async fn install_dependencies(
        &self,
        _sandbox_dir: &PathBuf,
        _dependencies: &[crate::types::Dependency],
    ) -> Result<(), Error>;

    /// Compile the code if needed
    async fn compile(&self, _sandbox_dir: &PathBuf, _source_file: &PathBuf) -> Result<(), Error>;

    /// Check if all required tools are available
    async fn check_tools(&self) -> Result<(), Error>;

    /// Install missing tools
    async fn install_missing_tools(&self) -> Result<(), Error>;

    /// Ensure directories are set up
    async fn ensure_directories(&self, _sandbox_dir: &PathBuf) -> Result<(), Error>;
}

/// Generic code executor that uses a sandbox
pub struct CodeExecutor {
    sandbox: Sandbox,
}

impl CodeExecutor {
    /// Create a new code executor
    pub async fn new() -> Result<Self, Error> {
        Ok(Self {
            sandbox: Sandbox::new(ResourceLimits::default()).await?,
        })
    }

    /// Execute code according to the request
    pub async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult, Error> {
        let executor = self.create_executor(request.language)?;

        // Use the same pattern as check_requirements
        if let Err(_) = executor.check_tools().await {
            executor.install_missing_tools().await?;
        }

        let source_file = self
            .write_source_file(&request, executor.file_extension())
            .await?;

        // Set up environment
        let sandbox_dir = self.sandbox.root_dir.clone();
        executor.ensure_directories(&sandbox_dir).await?;
        executor.setup_environment(&sandbox_dir).await?;

        // Install dependencies
        if !request.dependencies.is_empty() {
            executor
                .install_dependencies(&sandbox_dir, &request.dependencies)
                .await?;
        }

        // Compile if needed
        executor.compile(&sandbox_dir, &source_file).await?;

        // Convert env_vars to the correct type
        let env_vars: Vec<(String, String)> = request
            .env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // let exec_time = start_timer!(|| "execute");

        // Execute
        let (stdout, stderr, process_stats) = self
            .sandbox
            .execute(
                executor.run_command(),
                &executor
                    .run_args()
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
                &env_vars,
                request.input.as_deref(),
                request.timeout,
            )
            .await?;

        // end_timer!(exec_time);

        Ok(ExecutionResult {
            status: ExecutionStatus::Success,
            stdout,
            stderr,
            process_stats,
        })
    }

    async fn write_source_file(
        &self,
        request: &ExecutionRequest,
        extension: &str,
    ) -> Result<PathBuf, Error> {
        let filename = format!("source.{}", extension);
        let path = self.sandbox.root_dir.join("tmp").join(filename);
        fs::write(&path, &request.code).await.map_err(Error::Io)?;
        Ok(path)
    }

    fn create_executor(&self, language: Language) -> Result<Box<dyn LanguageExecutor>, Error> {
        match language {
            Language::Python => Ok(Box::new(PythonExecutor::new(None))),
            Language::JavaScript => Ok(Box::new(JavaScriptExecutor::new(None))),
            Language::TypeScript => Ok(Box::new(TypeScriptExecutor::new(None, None))),
            Language::Java => Ok(Box::new(JavaExecutor::new(None))),
            Language::Go => Ok(Box::new(GoExecutor::new(None))),
            Language::Cpp => Ok(Box::new(CppExecutor::new(None, None))),
            Language::Php => Ok(Box::new(PhpExecutor::new(None))),
            Language::Swift => Ok(Box::new(SwiftExecutor::new(None))),
        }
    }
}
