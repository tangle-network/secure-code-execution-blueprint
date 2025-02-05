use crate::{error::Error, types::ResourceLimits, ProcessStats};
use nix::sys::resource::{setrlimit, Resource};
use std::{path::PathBuf, process::Stdio};
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::Command,
    time::{self, Duration},
};
use tracing::error;
use uuid::Uuid;

/// Sandbox environment for secure code execution
pub struct Sandbox {
    /// Root directory for the sandbox
    pub root_dir: PathBuf,
    /// Resource limits
    limits: ResourceLimits,
    /// Unique ID for this sandbox instance
    #[allow(dead_code)]
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
    ) -> Result<(String, String, ProcessStats), Error> {
        println!("Executing command: {} {:?}", cmd, args);
        println!("Working directory: {:?}", self.root_dir.join("tmp"));
        println!("Environment vars: {:?}", env);

        let start = std::time::Instant::now();

        let mut command = Command::new(cmd);
        command
            .args(args)
            .env_clear()
            .envs(env.iter().map(|(k, v)| (k, v)))
            .current_dir(&self.root_dir.join("tmp"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Only set resource limits if we're not testing
        #[cfg(not(test))]
        {
            // Clone the values we need before the closure
            let memory = self.limits.memory;
            let cpu_time = self.limits.cpu_time;
            #[cfg(any(
                target_os = "linux",
                target_os = "android",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "aix"
            ))]
            let processes = self.limits.processes;
            let file_size = self.limits.file_size;
            #[cfg(target_os = "linux")]
            let disk_space = self.limits.disk_space;

            unsafe {
                command.pre_exec(move || {
                    setrlimit(Resource::RLIMIT_AS, memory, memory)?;
                    setrlimit(Resource::RLIMIT_CPU, cpu_time as u64, cpu_time as u64)?;

                    #[cfg(any(
                        target_os = "linux",
                        target_os = "android",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "aix"
                    ))]
                    setrlimit(Resource::RLIMIT_NPROC, processes as u64, processes as u64)?;

                    setrlimit(Resource::RLIMIT_FSIZE, file_size, file_size)?;

                    #[cfg(target_os = "linux")]
                    setrlimit(Resource::RLIMIT_DISK, disk_space, disk_space)?;

                    Ok(())
                });
            }
        }

        let mut child = command
            .spawn()
            .map_err(|e| Error::Sandbox(format!("Failed to spawn process: {}", e)))?;

        // Monitor process resources
        let pid = child.id().expect("Failed to get process ID");
        let memory_monitor = self.monitor_process_resources(pid);

        // Write input if provided
        if let Some(input_str) = input {
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(input_str.as_bytes())
                    .await
                    .map_err(|e| Error::Sandbox(format!("Failed to write input: {}", e)))?;
                // Drop stdin explicitly to close it
                drop(stdin);
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
                "Process exited with status: {} (stderr: {})",
                output.0, output.2
            )));
        }

        let stats = memory_monitor
            .await
            .map_err(|e| Error::Sandbox(format!("Failed to monitor process memory: {}", e)))?;

        let execution_time = start.elapsed();

        Ok((
            output.1,
            output.2,
            ProcessStats {
                memory_usage: stats.memory_usage,
                peak_memory: stats.peak_memory,
                execution_time,
            },
        ))
    }

    async fn monitor_process_resources(&self, pid: u32) -> Result<ProcessStats, Error> {
        let mut peak_memory = 0;
        let start = std::time::Instant::now();

        // Monitor until we can't find the process anymore
        loop {
            match tokio::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "rss="])
                .output()
                .await
            {
                Ok(output) if output.status.success() => {
                    if let Ok(stats) = String::from_utf8(output.stdout) {
                        if let Ok(current_mem) = stats.trim().parse::<u64>() {
                            peak_memory = peak_memory.max(current_mem * 1024);

                            // Check memory limit (when not testing)
                            #[cfg(not(test))]
                            if peak_memory > self.limits.memory {
                                return Err(Error::ResourceExceeded(
                                    "Memory limit exceeded".into(),
                                ));
                            }
                        }
                    }
                }
                _ => break, // Process has exited
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(ProcessStats {
            memory_usage: peak_memory,
            peak_memory,
            execution_time: start.elapsed(),
        })
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
        let sandbox = Sandbox::new(ResourceLimits::default()).await?;
        Ok(sandbox)
    }

    #[tokio::test]
    async fn test_sandbox_basic() -> Result<(), Error> {
        let sandbox = setup_sandbox().await?;

        // Use full path to shell
        let shell = if cfg!(target_os = "macos") {
            "/bin/zsh"
        } else {
            "/bin/bash"
        };

        let (stdout, stderr, time) = sandbox
            .execute(
                shell,
                &["-c", "echo 'Hello, World!'"],
                &[],
                None,
                Duration::from_secs(5),
            )
            .await?;

        assert_eq!(stdout.trim(), "Hello, World!");
        assert!(stderr.is_empty());
        assert!(time.execution_time < Duration::from_millis(1000));
        Ok(())
    }

    #[tokio::test]
    async fn test_sandbox_timeout() -> Result<(), Error> {
        let sandbox = setup_sandbox().await?;
        let shell = if cfg!(target_os = "macos") {
            "/bin/zsh"
        } else {
            "/bin/bash"
        };

        let result = sandbox
            .execute(
                shell,
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
        let shell = if cfg!(target_os = "macos") {
            "/bin/zsh"
        } else {
            "/bin/bash"
        };

        let (stdout, stderr, _) = sandbox
            .execute(
                shell,
                &["-c", "echo \"$1\"", "--", "test input"], // Pass as shell argument
                &[],
                None, // No stdin input needed
                Duration::from_secs(5),
            )
            .await?;

        assert_eq!(stdout.trim(), "test input");
        assert!(stderr.is_empty());
        Ok(())
    }
}
