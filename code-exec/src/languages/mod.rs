//! Language-specific executor implementations

mod cpp;
mod go;
mod java;
mod javascript;
mod php;
mod python;
mod swift;
mod typescript;

pub use cpp::CppExecutor;
pub use go::GoExecutor;
pub use java::JavaExecutor;
pub use javascript::JavaScriptExecutor;
pub use php::PhpExecutor;
pub use python::PythonExecutor;
pub use swift::SwiftExecutor;
pub use typescript::TypeScriptExecutor;

use crate::error::Error;
use std::process::Command;
use tracing::info;
use which::which;

pub trait ToolCheck {
    fn required_tools(&self) -> Vec<&str>;

    async fn install_missing_tools(&self) -> Result<(), Error> {
        let missing: Vec<_> = self
            .required_tools()
            .iter()
            .filter(|tool| which(tool).is_err())
            .map(|s| (*s).to_string())
            .collect();

        if !missing.is_empty() {
            info!("Installing missing tools: {}", missing.join(", "));

            // Detect package manager
            let (pkg_mgr, install_cmd) = if which("apt-get").is_ok() {
                ("apt-get", vec!["install", "-y"])
            } else if which("dnf").is_ok() {
                ("dnf", vec!["install", "-y"])
            } else if which("yum").is_ok() {
                ("yum", vec!["install", "-y"])
            } else if which("pacman").is_ok() {
                ("pacman", vec!["-S", "--noconfirm"])
            } else if which("brew").is_ok() {
                ("brew", vec!["install"])
            } else {
                return Err(Error::System(
                    "No supported package manager found".to_string(),
                ));
            };

            // Map tool names to package names with OS-specific handling
            let packages = missing
                .iter()
                .map(|tool| {
                    match tool.as_str() {
                        "mvn" => {
                            if pkg_mgr == "apt-get" {
                                "maven"
                            } else if pkg_mgr == "brew" {
                                "maven"
                            } else if pkg_mgr == "pacman" {
                                "maven"
                            } else {
                                "maven" // Default
                            }
                        }
                        "g++" => "g++",
                        "cmake" => "cmake",
                        "make" => "make",
                        "python3" => "python3",
                        "pip3" => "python3-pip",
                        "virtualenv" => "python3-virtualenv",
                        "node" => "nodejs",
                        "npm" => "npm",
                        "tsc" => "typescript",
                        "java" => "openjdk-17-jdk",
                        "javac" => "openjdk-17-jdk",
                        "php" => {
                            if pkg_mgr == "apt-get" {
                                "php-cli"
                            } else {
                                "php"
                            }
                        }
                        "composer" => {
                            if pkg_mgr == "apt-get" {
                                "composer"
                            } else if pkg_mgr == "pacman" {
                                "php-composer"
                            } else {
                                "composer"
                            }
                        }
                        "swift" => "swift",
                        "swiftc" => "swift",
                        "go" => "golang",
                        _ => tool.as_str(),
                    }
                })
                .collect::<Vec<_>>();

            // Add required repositories for certain package managers
            if pkg_mgr == "apt-get" {
                // Add universe repository for some packages
                Command::new("apt-get")
                    .args(["install", "-y", "software-properties-common"])
                    .status()?;

                Command::new("add-apt-repository")
                    .args(["-y", "universe"])
                    .status()?;

                Command::new("apt-get").arg("update").status()?;
            }

            // Install packages
            let mut cmd = Command::new(pkg_mgr);
            cmd.args(install_cmd);
            cmd.args(&packages);

            let install_status = cmd
                .status()
                .map_err(|e| Error::System(format!("Failed to install packages: {}", e)))?;

            if !install_status.success() {
                return Err(Error::System(format!(
                    "Failed to install packages: {}",
                    packages.join(", ")
                )));
            }

            // Verify installation
            let still_missing: Vec<_> = self
                .required_tools()
                .iter()
                .filter(|tool| which(tool).is_err())
                .map(|s| (*s).to_string())
                .collect();

            if !still_missing.is_empty() {
                return Err(Error::System(format!(
                    "Failed to install tools: {}",
                    still_missing.join(", ")
                )));
            }
        }

        Ok(())
    }

    fn check_tools(&self) -> Result<(), Error> {
        let missing: Vec<_> = self
            .required_tools()
            .iter()
            .filter(|tool| which(tool).is_err())
            .map(|s| (*s).to_string())
            .collect();

        if !missing.is_empty() {
            return Err(Error::System(format!(
                "Missing required tools: {}",
                missing.join(", ")
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) async fn check_requirements<T: ToolCheck>(executor: &T) -> Result<(), Error> {
    if let Err(_) = executor.check_tools() {
        // Try to install missing tools
        executor.install_missing_tools().await?;
    }
    Ok(())
}

pub(crate) fn check_command(cmd: &str) -> bool {
    which(cmd).is_ok()
}

#[cfg(test)]
pub(crate) fn skip_if_not_available(tools: &[&str]) -> bool {
    let missing: Vec<_> = tools
        .iter()
        .filter(|tool| which(**tool).is_err())
        .map(|s| (*s).to_string())
        .collect();

    if !missing.is_empty() {
        eprintln!("Skipping test: {} not available", missing.join(", "));
        return true;
    }
    false
}
