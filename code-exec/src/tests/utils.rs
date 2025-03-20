pub mod defaults {
    use crate::{sandbox::Sandbox, Error, ResourceLimits, Result};
    use tokio::time::Duration;

    pub fn default_test_limits() -> ResourceLimits {
        #[cfg(target_os = "linux")]
        {
            ResourceLimits {
                memory: 100 * 1024 * 1024, // 100MB
                cpu_time: 5,               // 5 seconds
                processes: 10,
                file_size: 10 * 1024 * 1024,   // 10MB
                disk_space: 100 * 1024 * 1024, // 100MB
            }
        }

        #[cfg(target_os = "macos")]
        {
            ResourceLimits {
                memory: u64::MAX, // Memory limits not reliable on macOS
                cpu_time: 5,      // 5 seconds
                processes: 10,
                file_size: 10 * 1024 * 1024,   // 10MB
                disk_space: 100 * 1024 * 1024, // 100MB
            }
        }
    }

    pub async fn setup_test_sandbox() -> Result<Sandbox> {
        Sandbox::new(default_test_limits()).await
    }

    pub fn default_timeout() -> Duration {
        Duration::from_secs(5)
    }

    pub fn extended_timeout() -> Duration {
        Duration::from_secs(30)
    }
}

pub mod dependencies {
    use crate::Dependency;

    pub fn numpy_dependency() -> Dependency {
        Dependency {
            name: "numpy".to_string(),
            version: "1.24.0".to_string(),
            source: None,
        }
    }

    pub fn lodash_dependency() -> Dependency {
        Dependency {
            name: "lodash".to_string(),
            version: "4.17.21".to_string(),
            source: None,
        }
    }

    pub fn serde_dependencies() -> Vec<Dependency> {
        vec![
            Dependency {
                name: "serde".to_string(),
                version: "1.0".to_string(),
                source: None,
            },
            Dependency {
                name: "serde_json".to_string(),
                version: "1.0".to_string(),
                source: None,
            },
        ]
    }
}
