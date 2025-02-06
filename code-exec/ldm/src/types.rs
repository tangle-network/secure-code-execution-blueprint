use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
    pub source: PackageSource,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PackageSource {
    System,
    Custom(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub version: Option<String>,
    pub binary_name: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationConfig {
    pub cache_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub allow_system_packages: bool,
    pub retry_attempts: u32,
    pub timeout_secs: u64,
    pub quiet_mode: bool,
}

impl Default for InstallationConfig {
    fn default() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("ldm"),
            temp_dir: std::env::temp_dir().join("ldm"),
            allow_system_packages: true,
            retry_attempts: 3,
            timeout_secs: 300,
            quiet_mode: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationStatus {
    NotStarted,
    InProgress,
    Complete,
    Failed,
}

#[derive(Debug, Clone)]
pub struct InstallationProgress {
    pub status: InstallationStatus,
    pub current_step: String,
    pub total_steps: usize,
    pub current_step_index: usize,
}
