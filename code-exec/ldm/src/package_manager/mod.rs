use async_trait::async_trait;
use std::collections::HashMap;

pub mod apt;
pub mod brew;

use crate::{error::Result, types::Package};

#[async_trait]
pub trait PackageManager: Send + Sync {
    /// Returns true if this package manager is available on the current system
    fn is_available(&self) -> bool;

    /// Returns the package name for a given tool
    fn get_package_name(&self, tool: &str) -> String;

    /// Returns a map of tool names to their package names
    fn get_package_map(&self) -> HashMap<&'static str, &'static str>;

    /// Checks if a package is installed
    async fn is_installed(&self, package: &Package) -> Result<bool>;

    /// Installs a package
    async fn install(&self, package: &Package) -> Result<()>;

    /// Uninstalls a package
    async fn uninstall(&self, package: &Package) -> Result<()>;

    /// Updates a package
    async fn update(&self, package: &Package) -> Result<()>;

    /// Updates the package manager's package list
    async fn update_package_list(&self) -> Result<()>;

    /// Cleans up any temporary files or cached data
    async fn cleanup(&self) -> Result<()>;
}
