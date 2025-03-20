use blueprint_sdk::Job;
use blueprint_sdk::tangle::layers::TangleLayer;
use blueprint_sdk::testing::tempfile;
use blueprint_sdk::testing::utils::harness::TestHarness;
use blueprint_sdk::testing::utils::setup_log;
use blueprint_sdk::testing::utils::tangle::TangleTestHarness;
use blueprint_sdk::testing::utils::tangle::blueprint_serde::to_field;
use secure_code_execution_blueprint_blueprint_lib::{ServiceContext, execute_code};

// The number of nodes to spawn in the test
const N: usize = 1;

#[tokio::test]
async fn test_blueprint() -> color_eyre::Result<()> {
    setup_log();

    // Initialize test harness (node, keys, deployment)
    let temp_dir = tempfile::TempDir::new()?;
    let context = ServiceContext::new();
    let harness = TangleTestHarness::setup(temp_dir, context).await?;

    // Setup service with `N` nodes
    let (mut test_env, service_id, _) = harness.setup_services::<N>(false).await?;

    // Setup the node(s)
    test_env.initialize().await?;
    test_env.add_job(execute_code.layer(TangleLayer)).await;

    // Start the test environment. It is now ready to receive job calls.
    test_env.start().await?;

    // Submit the job call
    let job_inputs = vec![
        to_field("python".to_string()).unwrap(),
        to_field("print('Hello World')".to_string()).unwrap(),
        to_field(None::<String>).unwrap(),
    ];
    let job = harness.submit_job(service_id, 0, job_inputs).await?;

    let results = harness.wait_for_job_execution(service_id, job).await?;

    // Verify results match expected output
    let expected_outputs = vec![to_field("Hello, Alice!").unwrap()];
    harness.verify_job(&results, expected_outputs);

    assert_eq!(results.service_id, service_id);
    Ok(())
}
