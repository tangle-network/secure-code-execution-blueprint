use super::PackageManager;
use crate::{
    error::{Error, Result},
    types::Package,
};
use async_trait::async_trait;
use std::{collections::HashMap, process::Command};
use tracing::{debug, error, info};
use which::which;

pub struct AptPackageManager {
    package_map: HashMap<&'static str, &'static str>,
}

impl Default for AptPackageManager {
    fn default() -> Self {
        let mut package_map = HashMap::new();
        package_map.insert("python", "python3");
        package_map.insert("pip", "python3-pip");
        package_map.insert("node", "nodejs");
        package_map.insert("npm", "npm");
        package_map.insert("java", "openjdk-17-jdk");
        package_map.insert("javac", "openjdk-17-jdk");
        package_map.insert("mvn", "maven");
        package_map.insert("php", "php-cli");
        package_map.insert("composer", "composer");
        package_map.insert("g++", "g++");
        package_map.insert("make", "make");
        package_map.insert("cmake", "cmake");
        package_map.insert("go", "golang");

        Self { package_map }
    }
}

#[async_trait]
impl PackageManager for AptPackageManager {
    fn is_available(&self) -> bool {
        which("apt-get").is_ok()
    }

    fn get_package_name(&self, tool: &str) -> String {
        self.package_map
            .get(tool)
            .map(|&s| s.to_string())
            .unwrap_or_else(|| tool.to_string())
    }

    fn get_package_map(&self) -> HashMap<&'static str, &'static str> {
        self.package_map.clone()
    }

    async fn is_installed(&self, package: &Package) -> Result<bool> {
        let output = Command::new("dpkg")
            .args(["-s", &package.name])
            .output()
            .map_err(|e| Error::PackageManager(format!("Failed to check package status: {}", e)))?;

        Ok(output.status.success())
    }

    async fn install(&self, package: &Package) -> Result<()> {
        info!("Installing package: {}", package.name);

        let status = Command::new("apt-get")
            .args([
                "install",
                "-y",
                "-qq",
                "--no-install-recommends",
                &package.name,
            ])
            .status()
            .map_err(|e| Error::PackageManager(format!("Failed to install package: {}", e)))?;

        if !status.success() {
            error!("Failed to install package: {}", package.name);
            return Err(Error::InstallationFailed(format!(
                "Package installation failed: {}",
                package.name
            )));
        }

        debug!("Successfully installed package: {}", package.name);
        Ok(())
    }

    async fn uninstall(&self, package: &Package) -> Result<()> {
        info!("Uninstalling package: {}", package.name);

        let status = Command::new("apt-get")
            .args(["remove", "-y", &package.name])
            .status()
            .map_err(|e| Error::PackageManager(format!("Failed to uninstall package: {}", e)))?;

        if !status.success() {
            error!("Failed to uninstall package: {}", package.name);
            return Err(Error::PackageManager(format!(
                "Failed to uninstall package: {}",
                package.name
            )));
        }

        debug!("Successfully uninstalled package: {}", package.name);
        Ok(())
    }

    async fn update(&self, package: &Package) -> Result<()> {
        info!("Updating package: {}", package.name);

        let status = Command::new("apt-get")
            .args(["install", "--only-upgrade", "-y", &package.name])
            .status()
            .map_err(|e| Error::PackageManager(format!("Failed to update package: {}", e)))?;

        if !status.success() {
            error!("Failed to update package: {}", package.name);
            return Err(Error::PackageManager(format!(
                "Failed to update package: {}",
                package.name
            )));
        }

        debug!("Successfully updated package: {}", package.name);
        Ok(())
    }

    async fn update_package_list(&self) -> Result<()> {
        info!("Updating package list");

        let status = Command::new("apt-get")
            .args(["update", "-qq"])
            .status()
            .map_err(|e| Error::PackageManager(format!("Failed to update package list: {}", e)))?;

        if !status.success() {
            error!("Failed to update package list");
            return Err(Error::PackageManager(
                "Failed to update package list".into(),
            ));
        }

        debug!("Successfully updated package list");
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        info!("Cleaning up APT cache");

        let status = Command::new("apt-get")
            .args(["clean"])
            .status()
            .map_err(|e| Error::PackageManager(format!("Failed to clean APT cache: {}", e)))?;

        if !status.success() {
            error!("Failed to clean APT cache");
            return Err(Error::PackageManager("Failed to clean APT cache".into()));
        }

        debug!("Successfully cleaned APT cache");
        Ok(())
    }
}
