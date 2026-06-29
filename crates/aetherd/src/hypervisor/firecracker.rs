// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::hypervisor::Hypervisor;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

/// Configuration for the boot source (kernel image and parameters).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BootSource {
    /// Host filesystem path to the kernel image.
    pub kernel_image_path: String,
    /// Kernel command line arguments.
    pub boot_args: String,
}

/// Configuration for a block device (drive) attached to the microVM.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Drive {
    /// Unique identifier for the drive.
    pub drive_id: String,
    /// Host filesystem path to the backing file or block device.
    pub path_on_host: String,
    /// Whether this drive represents the root filesystem.
    pub is_root_device: bool,
    /// Whether this drive should be mounted read-only.
    pub is_read_only: bool,
}

/// Configuration for the microVM's virtual hardware resources.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MachineConfig {
    /// Number of virtual CPUs to allocate.
    pub vcpu_count: u32,
    /// Memory allocation in megabytes.
    pub mem_size_mib: u32,
    /// SMT (hyperthreading) option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smt: Option<bool>,
}

/// Configuration for a virtual network interface.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NetworkInterface {
    /// Unique identifier for the network interface.
    pub iface_id: String,
    /// Name of the TAP device on the host system.
    pub host_dev_name: String,
}

/// The overall configuration layout matching the Firecracker API schema.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FirecrackerConfig {
    /// Boot source configuration.
    #[serde(rename = "boot-source")]
    pub boot_source: BootSource,
    /// Drives to attach.
    pub drives: Vec<Drive>,
    /// CPU and Memory limits.
    #[serde(rename = "machine-config")]
    pub machine_config: MachineConfig,
    /// Network interface mappings.
    #[serde(rename = "network-interfaces", skip_serializing_if = "Vec::is_empty")]
    pub network_interfaces: Vec<NetworkInterface>,
}

/// Hypervisor implementation managing a single Firecracker microVM process.
pub struct FirecrackerHypervisor {
    /// Unique identifier for the microVM instance.
    pub id: String,
    /// Path to the Firecracker binary.
    pub bin_path: String,
    /// Destination path where the JSON configuration will be written.
    pub config_path: String,
    /// Log file path where console output will be redirected.
    pub log_path: String,
    /// Firecracker configuration spec.
    pub config: FirecrackerConfig,
    /// Optional command arguments override for testing/mocking.
    pub extra_args: Vec<String>,
}

impl FirecrackerHypervisor {
    /// Creates a new instance of `FirecrackerHypervisor`.
    pub fn new(
        id: String,
        bin_path: String,
        config_path: String,
        log_path: String,
        config: FirecrackerConfig,
    ) -> Self {
        Self {
            id,
            bin_path,
            config_path,
            log_path,
            config,
            extra_args: Vec::new(),
        }
    }

    fn pid_path(&self) -> String {
        self.config_path.replace(".json", ".pid")
    }
}

#[async_trait]
impl Hypervisor for FirecrackerHypervisor {
    async fn spawn(&self) -> Result<(), String> {
        // Serialize config to JSON
        let config_json = serde_json::to_string_pretty(&self.config)
            .map_err(|e| format!("Failed to serialize Firecracker config: {}", e))?;

        tokio::fs::write(&self.config_path, config_json)
            .await
            .map_err(|e| {
                format!(
                    "Failed to write config file to '{}': {}",
                    self.config_path, e
                )
            })?;

        // Create console log file, redirecting standard output and stderr
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.log_path)
            .map_err(|e| format!("Failed to open log file at '{}': {}", self.log_path, e))?;

        let log_stderr = log_file
            .try_clone()
            .map_err(|e| format!("Failed to clone log file descriptor: {}", e))?;

        // Setup process execution arguments
        let args = if self.extra_args.is_empty() {
            vec!["--config-file".to_string(), self.config_path.clone()]
        } else {
            self.extra_args.clone()
        };

        let mut cmd = tokio::process::Command::new(&self.bin_path);
        cmd.args(&args)
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_stderr));

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn child process '{}': {}", self.bin_path, e))?;

        let pid = child
            .id()
            .ok_or_else(|| "Failed to retrieve process ID from spawned child".to_string())?;

        // Write PID file
        tokio::fs::write(self.pid_path(), pid.to_string())
            .await
            .map_err(|e| format!("Failed to write PID file: {}", e))?;

        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        let pid_path = self.pid_path();
        if !Path::new(&pid_path).exists() {
            return Ok(());
        }

        let pid_str = tokio::fs::read_to_string(&pid_path)
            .await
            .map_err(|e| format!("Failed to read PID file: {}", e))?;

        let pid: u32 = pid_str
            .trim()
            .parse()
            .map_err(|e| format!("Failed to parse PID '{}': {}", pid_str, e))?;

        // Dispatch SIGTERM (kill <pid>)
        let _ = tokio::process::Command::new("kill")
            .arg(pid.to_string())
            .status()
            .await;

        // Wait up to 500ms for process to exit
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if self.query_status().await? == "STOPPED" {
                let _ = tokio::fs::remove_file(&pid_path).await;
                return Ok(());
            }
        }

        // Force SIGKILL (kill -9 <pid>) if process refuses to exit
        let _ = tokio::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .await;

        let _ = tokio::fs::remove_file(&pid_path).await;
        Ok(())
    }

    async fn query_status(&self) -> Result<String, String> {
        let pid_path = self.pid_path();
        if !Path::new(&pid_path).exists() {
            return Ok("STOPPED".to_string());
        }

        let pid_str = tokio::fs::read_to_string(&pid_path)
            .await
            .map_err(|e| format!("Failed to read PID file: {}", e))?;

        let pid_trim = pid_str.trim();
        if pid_trim.is_empty() {
            return Ok("STOPPED".to_string());
        }

        // Run kill -0 <pid> to verify process exists
        let status = tokio::process::Command::new("kill")
            .args(["-0", pid_trim])
            .status()
            .await;

        match status {
            Ok(s) if s.success() => Ok("RUNNING".to_string()),
            _ => Ok("STOPPED".to_string()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn get_sample_config() -> FirecrackerConfig {
        FirecrackerConfig {
            boot_source: BootSource {
                kernel_image_path: "/path/to/kernel".to_string(),
                boot_args: "console=ttyS0 reboot=k panic=1 pci=off".to_string(),
            },
            drives: vec![Drive {
                drive_id: "rootfs".to_string(),
                path_on_host: "/path/to/rootfs".to_string(),
                is_root_device: true,
                is_read_only: false,
            }],
            machine_config: MachineConfig {
                vcpu_count: 2,
                mem_size_mib: 1024,
                smt: Some(false),
            },
            network_interfaces: vec![NetworkInterface {
                iface_id: "eth0".to_string(),
                host_dev_name: "tap0".to_string(),
            }],
        }
    }

    #[test]
    fn test_serialization() {
        let config = get_sample_config();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains("\"boot-source\""));
        assert!(serialized.contains("\"machine-config\""));
        assert!(serialized.contains("\"network-interfaces\""));
        assert!(serialized.contains("\"smt\":false"));
    }

    #[tokio::test]
    async fn test_mock_process_lifecycle() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("vm.json").to_str().unwrap().to_string();
        let log_path = dir.path().join("vm.log").to_str().unwrap().to_string();

        let config = get_sample_config();
        let mut hypervisor = FirecrackerHypervisor::new(
            "test-vm".to_string(),
            "sleep".to_string(),
            config_path,
            log_path,
            config,
        );
        hypervisor.extra_args = vec!["10".to_string()];

        assert_eq!(hypervisor.query_status().await.unwrap(), "STOPPED");

        hypervisor.spawn().await.unwrap();
        assert_eq!(hypervisor.query_status().await.unwrap(), "RUNNING");

        hypervisor.stop().await.unwrap();
        assert_eq!(hypervisor.query_status().await.unwrap(), "STOPPED");
    }
}
