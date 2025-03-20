use crate::{
    error::{Error, Result},
    language::LanguageProvider,
    package_manager::PackageManager,
    types::{InstallationConfig, InstallationProgress, InstallationStatus},
};
use std::{env, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, info, span, Level};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsType {
    Linux,
    MacOS,
}

impl OsType {
    pub fn current() -> Self {
        match env::consts::OS {
            "linux" => OsType::Linux,
            "macos" => OsType::MacOS,
            _ => panic!("Unsupported OS"),
        }
    }

    pub fn get_supported_package_managers(&self) -> Vec<Arc<dyn PackageManager>> {
        match self {
            OsType::Linux => vec![Arc::new(
                crate::package_manager::apt::AptPackageManager::default(),
            )],
            OsType::MacOS => vec![Arc::new(
                crate::package_manager::brew::HomebrewPackageManager::default(),
            )],
        }
    }
}

pub struct InstallationManager {
    package_managers: Vec<Arc<dyn PackageManager>>,
    progress: Arc<Mutex<InstallationProgress>>,
    quiet_mode: bool,
}

impl InstallationManager {
    pub fn new(config: InstallationConfig, package_managers: Vec<Arc<dyn PackageManager>>) -> Self {
        Self {
            package_managers,
            progress: Arc::new(Mutex::new(InstallationProgress {
                status: InstallationStatus::NotStarted,
                current_step: String::new(),
                total_steps: 0,
                current_step_index: 0,
            })),
            quiet_mode: config.quiet_mode,
        }
    }

    pub fn new_for_current_os(config: InstallationConfig) -> Self {
        let os = OsType::current();
        let package_managers = os.get_supported_package_managers();
        Self::new(config, package_managers)
    }

    pub async fn install_dependencies(
        &self,
        language_provider: &mut dyn LanguageProvider,
    ) -> Result<()> {
        let span = span!(
            Level::INFO,
            "install_dependencies",
            language = language_provider.name()
        );
        let _enter = span.enter();

        if !self.quiet_mode {
            info!("Installing dependencies for {}", language_provider.name());
        }

        let mut progress = self.progress.lock().await;
        progress.status = InstallationStatus::InProgress;
        progress.total_steps = language_provider.required_packages().len() + 2; // +2 for validation and setup
        progress.current_step_index = 0;
        drop(progress);

        // Find an available package manager
        let package_manager = self
            .find_available_package_manager()
            .ok_or_else(|| Error::System("No package manager available".into()))?;

        // Update package list
        self.update_progress("Updating package list".into()).await;
        if !self.quiet_mode {
            debug!("Updating package list");
        }
        package_manager.update_package_list().await?;

        // Install required packages
        for package in language_provider.required_packages() {
            if !self.quiet_mode {
                debug!("Checking package: {}", package.name);
            }

            if !package_manager.is_installed(&package).await? {
                self.update_progress(format!("Installing package: {}", package.name))
                    .await;
                if !self.quiet_mode {
                    info!("Installing package: {}", package.name);
                }
                package_manager.install(&package).await?;
            }
        }

        // Validate installation
        self.update_progress("Validating installation".into()).await;
        if !self.quiet_mode {
            debug!("Validating installation");
        }
        language_provider.validate_installation().await?;

        // Setup environment
        self.update_progress("Setting up environment".into()).await;
        if !self.quiet_mode {
            debug!("Setting up environment");
        }
        language_provider.setup_environment().await?;

        let mut progress = self.progress.lock().await;
        progress.status = InstallationStatus::Complete;
        progress.current_step = "Installation complete".into();

        if !self.quiet_mode {
            info!(
                "Dependencies installed successfully for {}",
                language_provider.name()
            );
        }
        Ok(())
    }

    pub async fn cleanup(&self, language_provider: &dyn LanguageProvider) -> Result<()> {
        info!("Cleaning up installation for {}", language_provider.name());

        // Clean up language-specific resources
        language_provider.cleanup().await?;

        // Clean up package manager resources
        if let Some(package_manager) = self.find_available_package_manager() {
            package_manager.cleanup().await?;
        }

        debug!("Cleanup completed successfully");
        Ok(())
    }

    pub async fn get_progress(&self) -> InstallationProgress {
        self.progress.lock().await.clone()
    }

    async fn update_progress(&self, step: String) {
        let mut progress = self.progress.lock().await;
        progress.current_step = step;
        progress.current_step_index += 1;
    }

    pub fn find_available_package_manager(&self) -> Option<&Arc<dyn PackageManager>> {
        self.package_managers.iter().find(|pm| pm.is_available())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{language::python::PythonProvider, package_manager::apt::AptPackageManager};

    #[tokio::test]
    async fn test_installation_manager() {
        let config = InstallationConfig::default();
        let package_managers: Vec<Arc<dyn PackageManager>> =
            vec![Arc::new(AptPackageManager::default())];
        let manager = InstallationManager::new(config, package_managers);
        let mut python_provider = PythonProvider::default();

        // Only run the test if apt-get is available
        if manager.find_available_package_manager().is_some() {
            let result = manager.install_dependencies(&mut python_provider).await;
            assert!(result.is_ok());

            let progress = manager.get_progress().await;
            assert_eq!(progress.status, InstallationStatus::Complete);

            let cleanup_result = manager.cleanup(&python_provider).await;
            assert!(cleanup_result.is_ok());
        }
    }
}
