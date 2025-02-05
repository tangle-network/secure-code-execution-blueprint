use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Java,
    Go,
    Cpp,
    Php,
    Swift,
}

/// Code execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Programming language
    pub language: Language,
    /// Source code to execute
    pub code: String,
    /// Input data for the program
    #[serde(default)]
    pub input: Option<String>,
    /// Dependencies required by the code
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    /// Execution timeout
    #[serde(with = "duration_serde")]
    pub timeout: Duration,
    /// Environment variables
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

/// Dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Name of the dependency
    pub name: String,
    /// Version requirement
    pub version: String,
    /// Source/registry for the dependency
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStats {
    pub memory_usage: u64,
    pub peak_memory: u64,
    #[serde(with = "duration_serde")]
    pub execution_time: Duration,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Execution status
    pub status: ExecutionStatus,
    /// Program output (stdout)
    pub stdout: String,
    /// Program errors (stderr)
    pub stderr: String,
    /// Process statistics
    pub process_stats: ProcessStats,
}

/// Execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Success,
    Error,
    Timeout,
    CompilationError,
    SystemError,
}

/// Resource limits for code execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum CPU time (seconds)
    pub cpu_time: u32,
    /// Maximum memory (bytes)
    pub memory: u64,
    /// Maximum disk space (bytes)
    pub disk_space: u64,
    /// Maximum number of processes
    pub processes: u32,
    /// Maximum file size (bytes)
    pub file_size: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_time: 30,
            memory: 512 * 1024 * 1024,     // 512MB
            disk_space: 100 * 1024 * 1024, // 100MB
            processes: 10,
            file_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
