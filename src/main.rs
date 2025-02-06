use crate::{CodeExecutionServer, CodeExecutionService, ResourceLimits};
use blueprint::firecracker::{FirecrackerConfig, FirecrackerManager};
use blueprint_sdk::logging;
use blueprint_sdk::runners::core::runner::BlueprintRunner;
use blueprint_sdk::runners::tangle::tangle::TangleConfig;
use blueprint_sdk::tokio::task;
use reqwest::Client;
use secure_code_execution_blueprint as blueprint;
use std::time::Duration;

const CODE_EXEC_PORT: u16 = 8080;
const MAX_CONCURRENT_EXECUTIONS: usize = 10;

#[blueprint_sdk::main(env)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Firecracker
    let fc_config = FirecrackerConfig::default();
    let fc_manager = FirecrackerManager::new(fc_config.clone());

    // Check KVM availability
    fc_manager.check_kvm_available().await?;

    // Setup network
    fc_manager.setup_network().await?;

    // Start Firecracker VM
    fc_manager.start().await?;

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
    let tangle_config = TangleConfig::default().with_exit_after_register(false);

    let runner = BlueprintRunner::new(tangle_config, env).job(execute_handler);

    // Run the blueprint
    runner.run().await?;

    // Wait for the server to finish (which should be never)
    server_handle.await?;

    // Stop Firecracker VM
    fc_manager.stop().await?;

    logging::info!("Exiting...");
    Ok(())
}
