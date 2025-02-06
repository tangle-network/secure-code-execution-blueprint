use crate::{error::Error, types::ResourceLimits, ProcessStats};
use nix::sys::resource::{getrusage, setrlimit, Resource, Usage, UsageWho};
use std::{path::PathBuf, process::Stdio, time::Instant};
use tokio::process::Child;
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::Command,
    time::{self, Duration},
};
use tracing::{debug, error, warn};
use uuid::Uuid;

/// Sandbox environment for secure code execution
pub struct Sandbox {
    /// Root directory for the sandbox
    pub root_dir: PathBuf,
    /// Resource limits
    limits: ResourceLimits,
    /// Unique ID for this sandbox instance
    id: String,
    /// Start time of current execution
    start_time: Option<Instant>,
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

        // Create required directories
        for dir in &["bin", "lib", "usr", "tmp", "home"] {
            fs::create_dir_all(root_dir.join(dir)).await.map_err(|e| {
                Error::Sandbox(format!("Failed to create {} directory: {}", dir, e))
            })?;
        }

        let sandbox = Sandbox {
            root_dir,
            limits,
            id: id.to_string(),
            start_time: None,
        };

        Ok(sandbox)
    }

    /// Check if resource limits have been exceeded
    fn check_resource_usage(&self) -> Result<(), Error> {
        let usage = getrusage(UsageWho::RUSAGE_CHILDREN)
            .map_err(|e| Error::Sandbox(format!("Failed to get resource usage: {}", e)))?;

        // Check CPU time
        if let Some(start_time) = self.start_time {
            let elapsed = start_time.elapsed();
            if elapsed.as_secs() as u32 > self.limits.cpu_time {
                return Err(Error::ResourceExceeded(format!(
                    "CPU time limit exceeded: {} > {}",
                    elapsed.as_secs(),
                    self.limits.cpu_time
                )));
            }
        }

        // On macOS, getrusage reports unreliable memory values so we skip the check
        #[cfg(target_os = "linux")]
        {
            // Check memory usage (RSS)
            let memory_kb = usage.max_rss() as u64 * 1024; // Convert KB to bytes
            if memory_kb > self.limits.memory {
                return Err(Error::ResourceExceeded(format!(
                    "Memory limit exceeded: {} > {}",
                    memory_kb, self.limits.memory
                )));
            }
        }

        #[cfg(target_os = "macos")]
        {
            debug!("Skipping memory limit check on macOS due to unreliable rusage values");
        }

        Ok(())
    }

    /// Execute a command in the sandbox with proper resource limits and monitoring
    pub async fn execute(
        &mut self,
        cmd: &str,
        args: &[&str],
        env: &[(String, String)],
        input: Option<&str>,
        timeout: Duration,
    ) -> Result<(String, String, ProcessStats), Error> {
        self.start_time = Some(Instant::now());

        debug!("Sandbox execute - Command: {}", cmd);
        debug!("Sandbox execute - Args: {:?}", args);
        debug!("Sandbox execute - Env: {:?}", env);
        debug!("Sandbox execute - Root dir: {:?}", self.root_dir);

        // For system commands, use their absolute path directly
        let cmd_path = if cmd.starts_with("./") {
            PathBuf::from(cmd)
        } else if let Ok(path) = which::which(cmd) {
            path
        } else {
            return Err(Error::Sandbox(format!("Command not found: {}", cmd)));
        };

        let mut command = Command::new(&cmd_path);
        command
            .args(args)
            .env_clear()
            .envs(env.iter().map(|(k, v)| (k, v)))
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin") // Set minimal PATH for system commands
            .env("HOME", self.root_dir.join("home"))
            .current_dir(&self.root_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            });

        // Store limits in stack-allocated variables to avoid closure lifetime issues
        let file_size = self.limits.file_size;
        let cpu_time = self.limits.cpu_time;

        unsafe {
            command.pre_exec(move || {
                #[cfg(target_os = "linux")]
                {
                    if let Err(e) = setrlimit(Resource::RLIMIT_FSIZE, file_size, file_size) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to set file size limit: {}", e),
                        ));
                    }
                    if let Err(e) = setrlimit(Resource::RLIMIT_CPU, cpu_time as u64, cpu_time as u64) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to set CPU time limit: {}", e),
                        ));
                    }
                }

                #[cfg(target_os = "macos")]
                {
                    warn!("Resource limits are limited on macOS. For full resource limiting, use Linux.");
                    if let Err(e) = setrlimit(Resource::RLIMIT_CPU, cpu_time as u64, cpu_time as u64) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to set CPU time limit: {}", e),
                        ));
                    }
                }

                Ok(())
            });
        }

        let mut child = command
            .spawn()
            .map_err(|e| Error::Sandbox(format!("Failed to spawn process: {}", e)))?;

        // Write input if provided
        if let Some(input_str) = input {
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(input_str.as_bytes())
                    .await
                    .map_err(|e| Error::Sandbox(format!("Failed to write input: {}", e)))?;
                // Explicitly close stdin to signal EOF
                drop(stdin);
            }
        }

        // Wait for completion with timeout
        let child_id = child.id();
        let output = match time::timeout(timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => Ok((output.status, output.stdout, output.stderr)),
            Ok(Err(e)) => Err(Error::Sandbox(format!("Process error: {}", e))),
            Err(_) => {
                if let Some(id) = child_id {
                    // Send SIGTERM and wait briefly for graceful shutdown
                    let _ = Command::new("kill").arg(id.to_string()).status().await;

                    // Give a very short grace period
                    tokio::time::sleep(Duration::from_millis(10)).await;

                    // Force kill with SIGKILL
                    let _ = Command::new("kill")
                        .arg("-9")
                        .arg(id.to_string())
                        .status()
                        .await;
                }
                Err(Error::Timeout(timeout.as_secs()))
            }
        }?;

        // Check resource usage after execution
        if let Err(e) = self.check_resource_usage() {
            return Err(e);
        }

        if !output.0.success() {
            // Check if the process was killed by a signal
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                if let Some(signal) = output.0.signal() {
                    if signal == 9 || signal == 15 {
                        // SIGKILL or SIGTERM
                        return Err(Error::Timeout(timeout.as_secs()));
                    }
                }
            }

            return Err(Error::Sandbox(format!(
                "Process exited with status: {} (stderr: {})",
                output.0,
                String::from_utf8_lossy(&output.2)
            )));
        }

        let execution_time = self.start_time.unwrap().elapsed();
        let usage = getrusage(UsageWho::RUSAGE_CHILDREN)
            .map_err(|e| Error::Sandbox(format!("Failed to get resource usage: {}", e)))?;

        Ok((
            String::from_utf8_lossy(&output.1).to_string(),
            String::from_utf8_lossy(&output.2).to_string(),
            ProcessStats {
                max_rss: usage.max_rss() as u64,
                minor_page_faults: usage.minor_page_faults() as u64,
                major_page_faults: usage.major_page_faults() as u64,
                block_reads: usage.block_reads() as u64,
                block_writes: usage.block_writes() as u64,
                voluntary_context_switches: usage.voluntary_context_switches() as u64,
                involuntary_context_switches: usage.involuntary_context_switches() as u64,
                user_time: Duration::from_micros(
                    (usage.user_time().tv_sec() as i64 * 1_000_000
                        + usage.user_time().tv_usec() as i64) as u64,
                ),
                system_time: Duration::from_micros(
                    (usage.system_time().tv_sec() as i64 * 1_000_000
                        + usage.system_time().tv_usec() as i64) as u64,
                ),
                execution_time,
            },
        ))
    }

    /// Copy a system binary into the sandbox
    async fn copy_binary(&self, cmd: &str) -> Result<PathBuf, Error> {
        if let Ok(system_path) = which::which(cmd) {
            let sandbox_bin = self.root_dir.join("bin").join(cmd);
            fs::copy(&system_path, &sandbox_bin)
                .await
                .map_err(|e| Error::Sandbox(format!("Failed to copy binary {}: {}", cmd, e)))?;

            // Make the binary executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&sandbox_bin)
                    .await
                    .map_err(|e| Error::Sandbox(format!("Failed to get permissions: {}", e)))?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&sandbox_bin, perms)
                    .await
                    .map_err(|e| Error::Sandbox(format!("Failed to set permissions: {}", e)))?;
            }

            Ok(sandbox_bin)
        } else {
            Err(Error::Sandbox(format!("Command not found: {}", cmd)))
        }
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
