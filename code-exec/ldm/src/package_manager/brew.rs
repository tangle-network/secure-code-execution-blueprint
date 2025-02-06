use super::PackageManager;
use crate::{
    error::{Error, Result},
    types::Package,
};
use async_trait::async_trait;
use std::{collections::HashMap, process::Command};
use tracing::{debug, error, info};
use which::which;

pub struct HomebrewPackageManager {
    package_map: HashMap<&'static str, &'static str>,
}

impl Default for HomebrewPackageManager {
    fn default() -> Self {
        let mut package_map = HashMap::new();
        package_map.insert("python", "python@3");
        package_map.insert("pip", "python@3"); // pip comes with python3
        package_map.insert("node", "node");
        package_map.insert("npm", "node"); // npm comes with node
        package_map.insert("java", "openjdk");
        package_map.insert("javac", "openjdk");
        package_map.insert("mvn", "maven");
        package_map.insert("php", "php");
        package_map.insert("composer", "composer");
        package_map.insert("g++", "gcc");
        package_map.insert("make", "make");
        package_map.insert("cmake", "cmake");
        package_map.insert("go", "go");
        package_map.insert("rustc", "rust");
        package_map.insert("cargo", "rust");

        Self { package_map }
    }
}

#[async_trait]
impl PackageManager for HomebrewPackageManager {
    fn is_available(&self) -> bool {
        which("brew").is_ok()
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
        let output = Command::new("brew")
            .args(["list", &package.name])
            .output()
            .map_err(|e| Error::PackageManager(format!("Failed to check package status: {}", e)))?;

        Ok(output.status.success())
    }

    async fn install(&self, package: &Package) -> Result<()> {
        info!("Installing package: {}", package.name);

        let status = Command::new("brew")
            .args(["install", "--quiet", &package.name])
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

        let status = Command::new("brew")
            .args(["uninstall", &package.name])
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

        let status = Command::new("brew")
            .args(["upgrade", &package.name])
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
        info!("Updating Homebrew package list");

        let status = Command::new("brew")
            .args(["update", "--quiet"])
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
        info!("Cleaning up Homebrew cache");

        let status = Command::new("brew")
            .args(["cleanup"])
            .status()
            .map_err(|e| Error::PackageManager(format!("Failed to clean Homebrew cache: {}", e)))?;

        if !status.success() {
            error!("Failed to clean Homebrew cache");
            return Err(Error::PackageManager(
                "Failed to clean Homebrew cache".into(),
            ));
        }

        debug!("Successfully cleaned Homebrew cache");
        Ok(())
    }
}
