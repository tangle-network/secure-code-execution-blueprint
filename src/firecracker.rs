use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use tokio::net::UnixStream;
use tokio::time::{sleep, Duration};

#[derive(Debug, Error)]
pub enum FirecrackerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("KVM not available: {0}")]
    KvmNotAvailable(String),
    #[error("Network setup failed: {0}")]
    NetworkSetupFailed(String),
    #[error("VM configuration error: {0}")]
    ConfigurationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirecrackerConfig {
    pub kernel_image_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub api_socket_path: PathBuf,
    pub tap_device_name: String,
    pub tap_ip: String,
    pub guest_mac: String,
    pub vcpu_count: i32,
    pub mem_size_mib: i32,
}

impl Default for FirecrackerConfig {
    fn default() -> Self {
        Self {
            kernel_image_path: PathBuf::from("/var/lib/firecracker/vmlinux"),
            rootfs_path: PathBuf::from("/var/lib/firecracker/rootfs.ext4"),
            api_socket_path: PathBuf::from("/tmp/firecracker.socket"),
            tap_device_name: "tap0".to_string(),
            tap_ip: "172.16.0.1".to_string(),
            guest_mac: "06:00:AC:10:00:02".to_string(),
            vcpu_count: 2,
            mem_size_mib: 1024,
        }
    }
}

pub struct FirecrackerManager {
    config: FirecrackerConfig,
    client: Client,
}

impl FirecrackerManager {
    pub fn new(config: FirecrackerConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn check_kvm_available(&self) -> Result<(), FirecrackerError> {
        if !Path::new("/dev/kvm").exists() {
            return Err(FirecrackerError::KvmNotAvailable(
                "KVM device not found".to_string(),
            ));
        }

        // Check if current user has access to KVM
        let output = Command::new("sh")
            .arg("-c")
            .arg("[ -r /dev/kvm ] && [ -w /dev/kvm ] && echo 'OK' || echo 'FAIL'")
            .output()?;

        if !String::from_utf8_lossy(&output.stdout).contains("OK") {
            return Err(FirecrackerError::KvmNotAvailable(
                "Current user does not have KVM access".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn setup_network(&self) -> Result<(), FirecrackerError> {
        // Remove existing tap device if it exists
        Command::new("ip")
            .args(&["link", "del", &self.config.tap_device_name])
            .output()
            .ok();

        // Create new tap device
        Command::new("ip")
            .args(&[
                "tuntap",
                "add",
                "dev",
                &self.config.tap_device_name,
                "mode",
                "tap",
            ])
            .output()?;

        // Configure IP address
        Command::new("ip")
            .args(&[
                "addr",
                "add",
                &format!("{}/30", self.config.tap_ip),
                "dev",
                &self.config.tap_device_name,
            ])
            .output()?;

        // Bring up the interface
        Command::new("ip")
            .args(&["link", "set", "dev", &self.config.tap_device_name, "up"])
            .output()?;

        // Enable IP forwarding
        fs::write("/proc/sys/net/ipv4/ip_forward", "1")?;

        // Setup iptables for NAT
        let host_iface = String::from_utf8(
            Command::new("sh")
                .arg("-c")
                .arg("ip -j route list default | jq -r '.[0].dev'")
                .output()?
                .stdout,
        )?
        .trim()
        .to_string();

        Command::new("iptables")
            .args(&[
                "-t",
                "nat",
                "-A",
                "POSTROUTING",
                "-o",
                &host_iface,
                "-j",
                "MASQUERADE",
            ])
            .output()?;

        Ok(())
    }

    async fn wait_for_socket(&self) -> Result<(), FirecrackerError> {
        for _ in 0..30 {
            if UnixStream::connect(&self.config.api_socket_path)
                .await
                .is_ok()
            {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }
        Err(FirecrackerError::ConfigurationError(
            "Timeout waiting for API socket".to_string(),
        ))
    }

    pub async fn start(&self) -> Result<(), FirecrackerError> {
        // Remove existing socket if it exists
        fs::remove_file(&self.config.api_socket_path).ok();

        // Start Firecracker process
        let _child = Command::new("firecracker")
            .args(&["--api-sock", self.config.api_socket_path.to_str().unwrap()])
            .spawn()?;

        self.wait_for_socket().await?;

        // Configure the VM
        self.configure_vm().await?;

        Ok(())
    }

    async fn configure_vm(&self) -> Result<(), FirecrackerError> {
        // Configure boot source
        let boot_source = serde_json::json!({
            "kernel_image_path": self.config.kernel_image_path,
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
        });

        self.client
            .put(&format!(
                "http://localhost/boot-source{}",
                self.config.api_socket_path.display()
            ))
            .json(&boot_source)
            .send()
            .await?;

        // Configure rootfs
        let rootfs = serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": self.config.rootfs_path,
            "is_root_device": true,
            "is_read_only": false
        });

        self.client
            .put(&format!(
                "http://localhost/drives/rootfs{}",
                self.config.api_socket_path.display()
            ))
            .json(&rootfs)
            .send()
            .await?;

        // Configure network
        let network = serde_json::json!({
            "iface_id": "net1",
            "guest_mac": self.config.guest_mac,
            "host_dev_name": self.config.tap_device_name
        });

        self.client
            .put(&format!(
                "http://localhost/network-interfaces/net1{}",
                self.config.api_socket_path.display()
            ))
            .json(&network)
            .send()
            .await?;

        // Configure machine
        let machine_config = serde_json::json!({
            "vcpu_count": self.config.vcpu_count,
            "mem_size_mib": self.config.mem_size_mib,
            "ht_enabled": false
        });

        self.client
            .put(&format!(
                "http://localhost/machine-config{}",
                self.config.api_socket_path.display()
            ))
            .json(&machine_config)
            .send()
            .await?;

        // Start the VM
        let action = serde_json::json!({
            "action_type": "InstanceStart"
        });

        self.client
            .put(&format!(
                "http://localhost/actions{}",
                self.config.api_socket_path.display()
            ))
            .json(&action)
            .send()
            .await?;

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), FirecrackerError> {
        // Send InstanceHalt action
        let action = serde_json::json!({
            "action_type": "InstanceHalt"
        });

        self.client
            .put(&format!(
                "http://localhost/actions{}",
                self.config.api_socket_path.display()
            ))
            .json(&action)
            .send()
            .await?;

        Ok(())
    }
}
