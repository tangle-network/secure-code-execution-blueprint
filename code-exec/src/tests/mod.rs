pub mod fixtures;
pub mod integration;
pub mod languages;
pub mod sandbox;
pub mod utils;

use crate::Result;

#[tokio::test]
async fn test_sandbox_creation() -> Result<()> {
    let sandbox = utils::defaults::setup_test_sandbox().await?;
    assert!(sandbox.root_dir.exists());
    Ok(())
}
