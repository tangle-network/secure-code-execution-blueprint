use crate::{sandbox::Sandbox, tests::utils::defaults::default_test_limits, Error, Result};
use tokio::time::Duration;

#[tokio::test]
async fn test_sandbox_basic() -> Result<()> {
    let mut sandbox = Sandbox::new(default_test_limits()).await?;
    let (stdout, stderr, _) = sandbox
        .execute("echo", &["Hello"], &[], None, Duration::from_secs(5))
        .await?;
    assert_eq!(stdout.trim(), "Hello");
    assert!(stderr.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_sandbox_input() -> Result<()> {
    let mut sandbox = Sandbox::new(default_test_limits()).await?;
    let (stdout, stderr, _) = sandbox
        .execute("cat", &[], &[], Some("test input"), Duration::from_secs(5))
        .await?;
    assert_eq!(stdout, "test input");
    assert!(stderr.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_sandbox_timeout() -> Result<()> {
    let mut sandbox = Sandbox::new(default_test_limits()).await?;
    let result = sandbox
        .execute("sleep", &["10"], &[], None, Duration::from_secs(1))
        .await;
    assert!(matches!(result, Err(Error::Timeout(_))));
    Ok(())
}

#[tokio::test]
async fn test_sandbox_resource_limits() -> Result<()> {
    let mut sandbox = Sandbox::new(default_test_limits()).await?;

    // Create a program that allocates a large amount of memory
    let result = sandbox
        .execute(
            "python3",
            &["-c", "x = list(range(10**7)); print(len(x))"],
            &[],
            None,
            Duration::from_secs(5),
        )
        .await;

    #[cfg(target_os = "linux")]
    {
        assert!(matches!(result, Err(Error::ResourceExceeded(_))));
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, we can't enforce memory limits, so the program should complete
        assert!(result.is_ok());
    }

    Ok(())
}
