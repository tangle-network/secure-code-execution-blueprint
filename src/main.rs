use blueprint_sdk::logging;
use blueprint_sdk::runners::core::runner::BlueprintRunner;
use blueprint_sdk::runners::tangle::tangle::TangleConfig;
use blueprint_sdk::tokio::task;
use code_exec::{CodeExecutionServer, CodeExecutionService, ResourceLimits};
use phala_tee_blueprint_template as blueprint;
use reqwest::Client;
use std::time::Duration;

const CODE_EXEC_PORT: u16 = 8080;
const MAX_CONCURRENT_EXECUTIONS: usize = 10;

#[blueprint_sdk::main(env)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the code execution service
    let service =
        CodeExecutionService::new(MAX_CONCURRENT_EXECUTIONS, ResourceLimits::default()).await?;
    let server = CodeExecutionServer::new(service, CODE_EXEC_PORT);

    // Start the server in a separate task
    let server_handle = task::spawn(async move {
        if let Err(e) = server.run().await {
            logging::error!("Server error: {}", e);
        }
    });

    // Create HTTP client for the service context
    let http_client = Client::builder().timeout(Duration::from_secs(30)).build()?;

    // Create service context
    let context = blueprint::ServiceContext {
        config: env.clone(),
        call_id: None,
        code_exec_url: format!("http://localhost:{}", CODE_EXEC_PORT),
        http_client,
    };

    // Create the event handlers
    let execute_handler = blueprint::ExecuteCodeEventHandler::new(&env, context).await?;

    logging::info!("Starting the event watcher ...");
    let tangle_config = TangleConfig::default();
    BlueprintRunner::new(tangle_config, env)
        .job(execute_handler)
        .run()
        .await?;

    // Wait for the server to finish (which should be never)
    server_handle.await?;

    logging::info!("Exiting...");
    Ok(())
}
