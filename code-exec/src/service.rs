use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info};

use crate::{
    error::Error,
    executor::CodeExecutor,
    sandbox::Sandbox,
    types::{ExecutionRequest, ExecutionResult, ResourceLimits},
};

#[derive(Clone)]
pub struct CodeExecutionService {
    executor: Arc<CodeExecutor>,
    semaphore: Arc<Semaphore>,
    resource_limits: ResourceLimits,
}

impl CodeExecutionService {
    pub async fn new(
        max_concurrent_executions: usize,
        resource_limits: ResourceLimits,
    ) -> Result<Self, Error> {
        let executor = CodeExecutor::new().await?;

        Ok(Self {
            executor: Arc::new(executor),
            semaphore: Arc::new(Semaphore::new(max_concurrent_executions)),
            resource_limits: resource_limits,
        })
    }

    pub async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult, Error> {
        // Acquire execution permit
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| Error::System(format!("Failed to acquire execution permit: {}", e)))?;

        debug!(
            "Starting code execution for language: {:?}",
            request.language
        );

        // Create new sandbox for this execution
        let sandbox = Sandbox::new(self.resource_limits.clone()).await?;

        // Execute using shared executor but with isolated sandbox
        let result = self.executor.execute_in_sandbox(request, sandbox).await;

        match &result {
            Ok(_) => info!("Code execution completed successfully"),
            Err(e) => error!("Code execution failed: {}", e),
        }

        result
    }

    pub fn get_available_slots(&self) -> usize {
        self.semaphore.available_permits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Language;
    use std::time::Duration;

    #[tokio::test]
    async fn test_concurrent_executions() -> Result<(), Error> {
        let service = CodeExecutionService::new(3, ResourceLimits::default()).await?;
        let service = Arc::new(service);

        let requests = vec![
            ExecutionRequest {
                language: Language::Python,
                code: r#"print("Hello from Python!")"#.to_string(),
                input: None,
                dependencies: vec![],
                timeout: Duration::from_secs(5),
                env_vars: Default::default(),
            },
            ExecutionRequest {
                language: Language::JavaScript,
                code: r#"console.log('Hello from JavaScript!')"#.to_string(),
                input: None,
                dependencies: vec![],
                timeout: Duration::from_secs(5),
                env_vars: Default::default(),
            },
            ExecutionRequest {
                language: Language::Python,
                code: r#"print("Another Python execution!")"#.to_string(),
                input: None,
                dependencies: vec![],
                timeout: Duration::from_secs(5),
                env_vars: Default::default(),
            },
        ];

        // Execute all requests concurrently
        let mut handles = vec![];
        for request in requests {
            let service = service.clone();
            handles.push(tokio::spawn(async move { service.execute(request).await }));
        }

        // Verify all executions completed successfully
        for handle in handles {
            let result = handle.await.unwrap()?;
            assert!(result.stdout.contains("Hello") || result.stdout.contains("Another"));
            assert!(result.stderr.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_execution_limit() -> Result<(), Error> {
        let max_executions = 2;
        let service = CodeExecutionService::new(max_executions, ResourceLimits::default()).await?;

        assert_eq!(service.get_available_slots(), max_executions);

        // Should match max_executions
        assert_eq!(service.semaphore.available_permits(), max_executions);

        Ok(())
    }
}
