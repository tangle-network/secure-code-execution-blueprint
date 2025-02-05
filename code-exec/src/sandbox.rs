use crate::{error::Error, types::ResourceLimits};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::{Child, Command},
    time::{self, Duration},
};
use tracing::{debug, error, info};
use uuid::Uuid;
use which;

/// Sandbox environment for secure code execution
pub struct Sandbox {
    /// Root directory for the sandbox
    pub root_dir: PathBuf,
    /// Resource limits
    limits: ResourceLimits,
    /// Unique ID for this sandbox instance
    id: String,
}

impl Sandbox {
    /// Create a new sandbox environment
    pub async fn new(limits: ResourceLimits) -> Result<Self, Error> {
        let id = Uuid::new_v4().to_string();
        let root_dir = PathBuf::from("/tmp").join(format!("sandbox-{}", id));

        // Create sandbox directory structure
        fs::create_dir_all(&root_dir)
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to create sandbox directory: {}", e)))?;

        // Create required directories even in limited mode
        for dir in &["bin", "lib", "usr", "tmp"] {
            fs::create_dir_all(root_dir.join(dir)).await.map_err(|e| {
                Error::Sandbox(format!("Failed to create {} directory: {}", dir, e))
            })?;
        }

        let sandbox = Sandbox {
            root_dir,
            limits,
            id: id.to_string(),
        };

        Ok(sandbox)
    }

    /// Execute a command in the sandbox
    pub async fn execute(
        &self,
        cmd: &str,
        args: &[&str],
        env: &[(String, String)],
        input: Option<&str>,
        timeout: Duration,
    ) -> Result<(String, String, u64), Error> {
        let start = std::time::Instant::now();

        // Check if we can use unshare
        let can_unshare =
            which::which("unshare").is_ok() && nix::unistd::Uid::effective().is_root();

        let mut command = if can_unshare {
            let mut unshare_cmd = Command::new("unshare");
            unshare_cmd.args([
                "--pid",
                "--fork",
                "--mount",
                "--mount-proc",
                "--root",
                &self.root_dir.to_string_lossy(),
                cmd,
            ]);
            unshare_cmd
        } else {
            Command::new(cmd)
        };

        command
            .args(args)
            .env_clear()
            .envs(env.iter().map(|(k, v)| (k, v)))
            .current_dir(&self.root_dir.join("tmp"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add input if provided
        if let Some(input_str) = input {
            command.stdin(Stdio::piped());
        }

        // Start the command
        let mut child = command
            .spawn()
            .map_err(|e| Error::Sandbox(format!("Failed to spawn sandboxed process: {}", e)))?;

        // Write input if provided
        if let Some(input_str) = input {
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(input_str.as_bytes())
                    .await
                    .map_err(|e| Error::Sandbox(format!("Failed to write input: {}", e)))?;
            }
        }

        // Wait for completion with timeout
        let output = match time::timeout(timeout, async {
            let status = child.wait().await?;
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| Error::Sandbox("Failed to capture stdout".to_string()))?;
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| Error::Sandbox("Failed to capture stderr".to_string()))?;
            Ok::<_, Error>((status, stdout, stderr))
        })
        .await
        {
            Ok(Ok((status, stdout, stderr))) => {
                let mut stdout_str = String::new();
                let mut stderr_str = String::new();

                tokio::io::AsyncReadExt::read_to_string(
                    &mut tokio::io::BufReader::new(stdout),
                    &mut stdout_str,
                )
                .await
                .map_err(|e| Error::Sandbox(format!("Failed to read stdout: {}", e)))?;

                tokio::io::AsyncReadExt::read_to_string(
                    &mut tokio::io::BufReader::new(stderr),
                    &mut stderr_str,
                )
                .await
                .map_err(|e| Error::Sandbox(format!("Failed to read stderr: {}", e)))?;

                Ok((status, stdout_str, stderr_str))
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout case
                if let Err(e) = child.kill().await {
                    error!("Failed to kill process: {}", e);
                }
                Err(Error::Timeout(timeout.as_secs()))
            }
        }?;

        if !output.0.success() {
            return Err(Error::Sandbox(format!(
                "Process exited with status: {}",
                output.0
            )));
        }

        let execution_time = start.elapsed().as_millis() as u64;

        Ok((output.1, output.2, execution_time))
    }

    /// Set up cgroup limits for the sandbox
    async fn setup_cgroups(&self) -> Result<(), Error> {
        // Create cgroup
        let cgroup_path = PathBuf::from("/sys/fs/cgroup/sandbox").join(&self.id);

        // Set up memory limit
        fs::write(
            cgroup_path.join("memory.max"),
            self.limits.memory.to_string(),
        )
        .await
        .map_err(|e| Error::Sandbox(format!("Failed to set memory limit: {}", e)))?;

        // Set up CPU limit
        fs::write(
            cgroup_path.join("cpu.max"),
            format!("{} 100000", self.limits.cpu_time * 100000),
        )
        .await
        .map_err(|e| Error::Sandbox(format!("Failed to set CPU limit: {}", e)))?;

        Ok(())
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // Clean up sandbox directory
        if let Err(e) = std::fs::remove_dir_all(&self.root_dir) {
            error!("Failed to clean up sandbox directory: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn setup_sandbox() -> Result<Sandbox, Error> {
        let can_unshare =
            which::which("unshare").is_ok() && nix::unistd::Uid::effective().is_root();

        let sandbox = Sandbox::new(ResourceLimits::default()).await?;

        if !can_unshare {
            eprintln!("Warning: Full sandbox tests require root privileges and unshare");
            eprintln!("Running in limited test mode");
        }

        Ok(sandbox)
    }

    #[tokio::test]
    async fn test_sandbox_basic() -> Result<(), Error> {
        let sandbox = setup_sandbox().await?;

        // If we can't use unshare, just verify directory creation
        if !which::which("unshare").is_ok() || !nix::unistd::Uid::effective().is_root() {
            assert!(sandbox.root_dir.exists());
            return Ok(());
        }

        let (stdout, stderr, time) = sandbox
            .execute(
                "/bin/sh",
                &["-c", "echo 'Hello, World!'"],
                &[],
                None,
                Duration::from_secs(5),
            )
            .await?;

        assert_eq!(stdout.trim(), "Hello, World!");
        assert!(stderr.is_empty());
        assert!(time < 1000);
        Ok(())
    }

    #[tokio::test]
    async fn test_sandbox_timeout() -> Result<(), Error> {
        let sandbox = setup_sandbox().await?;

        let result = sandbox
            .execute(
                "/bin/sh",
                &["-c", "sleep 10"],
                &[],
                None,
                Duration::from_secs(1),
            )
            .await;

        assert!(matches!(result, Err(Error::Timeout(1))));
        Ok(())
    }

    #[tokio::test]
    async fn test_sandbox_input() -> Result<(), Error> {
        let sandbox = setup_sandbox().await?;

        let (stdout, stderr, _) = sandbox
            .execute(
                "/bin/sh",
                &["-c", "cat"],
                &[],
                Some("test input"),
                Duration::from_secs(5),
            )
            .await?;

        assert_eq!(stdout.trim(), "test input");
        assert!(stderr.is_empty());
        Ok(())
    }
}
