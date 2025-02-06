use crate::{CodeExecutionServer, CodeExecutionService, ResourceLimits};
use blueprint_sdk::logging;
use blueprint_sdk::testing::tempfile;
use blueprint_sdk::testing::utils::harness::TestHarness;
use blueprint_sdk::testing::utils::runner::TestEnv;
use blueprint_sdk::testing::utils::tangle::blueprint_serde::to_field;
use blueprint_sdk::testing::utils::tangle::TangleTestHarness;
use blueprint_sdk::tokio;
use reqwest::Client;
use secure_code_execution_blueprint::{ExecuteCodeEventHandler, ServiceContext};
use std::time::Duration;

const CODE_EXEC_PORT: u16 = 8080;
const MAX_CONCURRENT_EXECUTIONS: usize = 10;

#[tokio::test]
async fn test_blueprint() -> color_eyre::Result<()> {
    logging::setup_log();

    // Initialize test harness (node, keys, deployment)
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let env = harness.env().clone();

    // Initialize code execution service
    let service =
        CodeExecutionService::new(MAX_CONCURRENT_EXECUTIONS, ResourceLimits::default()).await?;
    let server = CodeExecutionServer::new(service, CODE_EXEC_PORT);

    // Start server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            logging::error!("Server error: {}", e);
        }
    });

    // Create HTTP client
    let http_client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create blueprint-specific context
    let blueprint_ctx = ServiceContext {
        config: env.clone(),
        call_id: None,
        code_exec_url: format!("http://localhost:{}", CODE_EXEC_PORT),
        http_client,
    };

    // Initialize event handler
    let handler = ExecuteCodeEventHandler::new(&env.clone(), blueprint_ctx)
        .await
        .unwrap();

    // Setup service
    let (mut test_env, service_id) = harness.setup_services().await?;
    test_env.add_job(handler);

    tokio::spawn(async move {
        test_env.run_runner().await.unwrap();
    });

    // Execute job and verify result
    let job_inputs = vec![
        to_field("python").unwrap(),
        to_field("print('Hello, World!')").unwrap(),
        to_field("").unwrap(),
    ];
    let expected_outputs = vec![to_field("Hello, World!\n").unwrap()];

    let results = harness
        .execute_job(service_id, 0, job_inputs, expected_outputs)
        .await?;

    assert_eq!(results.service_id, service_id);

    // Cleanup
    server_handle.abort();
    Ok(())
}
