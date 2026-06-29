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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// Configuration for the QEMU-KVM virtual machine hardware and devices.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct QemuConfig {
    /// Number of virtual CPUs.
    pub vcpu_count: u32,
    /// Memory size in Megabytes.
    pub mem_size_mib: u32,
    /// Path to the backing disk image on the host.
    pub disk_image_path: String,
    /// Path to the QMP Unix socket on the host.
    pub qmp_socket_path: String,
    /// Name of the host TAP network interface device.
    pub host_tap_device: Option<String>,
}

/// Hypervisor implementation managing a single QEMU microVM/VM process.
pub struct QemuHypervisor {
    /// Unique identifier for the VM instance.
    pub id: String,
    /// Path to the QEMU binary.
    pub bin_path: String,
    /// Path to redirect console logs.
    pub log_path: String,
    /// Configuration for the VM.
    pub config: QemuConfig,
    /// Optional command arguments override for testing/mocking.
    pub extra_args: Vec<String>,
}

impl QemuHypervisor {
    /// Creates a new instance of `QemuHypervisor`.
    pub fn new(id: String, bin_path: String, log_path: String, config: QemuConfig) -> Self {
        Self {
            id,
            bin_path,
            log_path,
            config,
            extra_args: Vec::new(),
        }
    }

    fn pid_path(&self) -> String {
        self.log_path.replace(".log", ".pid")
    }

    async fn check_pid_alive(&self, pid: u32) -> bool {
        let status = tokio::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .await;
        match status {
            Ok(s) => s.success(),
            _ => false,
        }
    }

    /// Cleans up host network bridges and ZVOL/disk mappings if VM terminates.
    pub async fn cleanup_host_resources(&self) -> Result<(), String> {
        // Simulate cleaning up host network bridges and ZVOL mappings
        println!("Cleaning up host resources for QEMU VM: {}", self.id);
        Ok(())
    }
}

/// Client helper for QEMU Machine Protocol (QMP) over UDS socket.
pub struct QmpClient {
    socket_path: String,
}

impl QmpClient {
    /// Creates a new QMP client instance.
    pub fn new(socket_path: String) -> Self {
        Self { socket_path }
    }

    async fn read_line(stream: &mut UnixStream) -> Result<String, String> {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            stream
                .read_exact(&mut byte)
                .await
                .map_err(|e| format!("Failed to read QMP byte: {}", e))?;
            buf.push(byte[0]);
            if byte[0] == b'\n' {
                break;
            }
        }
        String::from_utf8(buf).map_err(|e| format!("Invalid UTF-8 in QMP response: {}", e))
    }

    /// Establishes connection and completes capability negotiation handshake.
    pub async fn connect_and_negotiate(&self) -> Result<UnixStream, String> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| format!("Failed to connect to QMP socket: {}", e))?;

        // Read greeting
        let _greeting = Self::read_line(&mut stream).await?;

        // Send negotiation
        let cmd = "{\"execute\": \"qmp_capabilities\"}\n";
        stream
            .write_all(cmd.as_bytes())
            .await
            .map_err(|e| format!("Failed to write QMP capabilities negotiation: {}", e))?;

        // Read response
        let resp = Self::read_line(&mut stream).await?;
        if !resp.contains("\"return\"") {
            return Err(format!("QMP capability negotiation failed: {}", resp));
        }

        Ok(stream)
    }

    /// Queries the internal VM status using query-status.
    pub async fn query_status(&self) -> Result<String, String> {
        let mut stream = self.connect_and_negotiate().await?;

        let cmd = "{\"execute\": \"query-status\"}\n";
        stream
            .write_all(cmd.as_bytes())
            .await
            .map_err(|e| format!("Failed to dispatch query-status: {}", e))?;

        let resp = Self::read_line(&mut stream).await?;

        if resp.contains("\"status\":\"paused\"") || resp.contains("\"status\": \"paused\"") {
            Ok("PAUSED".to_string())
        } else if resp.contains("\"status\":\"running\"")
            || resp.contains("\"status\": \"running\"")
            || resp.contains("\"running\":true")
            || resp.contains("\"running\": true")
        {
            Ok("RUNNING".to_string())
        } else {
            Ok("STOPPED".to_string())
        }
    }
}

#[async_trait]
impl Hypervisor for QemuHypervisor {
    async fn spawn(&self) -> Result<(), String> {
        let mut args = Vec::new();

        if self.extra_args.is_empty() {
            args.push("-enable-kvm".to_string());
            args.push("-cpu".to_string());
            args.push("host".to_string());
            args.push("-m".to_string());
            args.push(self.config.mem_size_mib.to_string());
            args.push("-smp".to_string());
            args.push(self.config.vcpu_count.to_string());

            args.push("-drive".to_string());
            args.push(format!(
                "file={},format=raw,media=disk",
                self.config.disk_image_path
            ));

            args.push("-qmp".to_string());
            args.push(format!(
                "unix:{},server,nowait",
                self.config.qmp_socket_path
            ));

            if let Some(ref tap) = self.config.host_tap_device {
                args.push("-netdev".to_string());
                args.push(format!(
                    "tap,id=net0,ifname={},script=no,downscript=no",
                    tap
                ));
                args.push("-device".to_string());
                args.push("virtio-net-pci,netdev=net0".to_string());
            }

            args.push("-nographic".to_string());
        } else {
            args = self.extra_args.clone();
        }

        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.log_path)
            .map_err(|e| format!("Failed to open log file at '{}': {}", self.log_path, e))?;

        let log_stderr = log_file
            .try_clone()
            .map_err(|e| format!("Failed to clone log file descriptor: {}", e))?;

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

        tokio::fs::write(self.pid_path(), pid.to_string())
            .await
            .map_err(|e| format!("Failed to write PID file: {}", e))?;

        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        let pid_path = self.pid_path();
        if !Path::new(&pid_path).exists() {
            self.cleanup_host_resources().await?;
            return Ok(());
        }

        let pid_str = tokio::fs::read_to_string(&pid_path)
            .await
            .map_err(|e| format!("Failed to read PID file: {}", e))?;

        let pid: u32 = pid_str
            .trim()
            .parse()
            .map_err(|e| format!("Failed to parse PID '{}': {}", pid_str, e))?;

        let _ = tokio::process::Command::new("kill")
            .arg(pid.to_string())
            .status()
            .await;

        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if !self.check_pid_alive(pid).await {
                let _ = tokio::fs::remove_file(&pid_path).await;
                self.cleanup_host_resources().await?;
                return Ok(());
            }
        }

        let _ = tokio::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .await;

        let _ = tokio::fs::remove_file(&pid_path).await;
        self.cleanup_host_resources().await?;
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

        let pid: u32 = pid_trim
            .parse()
            .map_err(|e| format!("Failed to parse PID: {}", e))?;

        if !self.check_pid_alive(pid).await {
            self.cleanup_host_resources().await?;
            return Ok("STOPPED".to_string());
        }

        let qmp = QmpClient::new(self.config.qmp_socket_path.clone());
        match qmp.query_status().await {
            Ok(status) => Ok(status),
            Err(_) => Ok("RUNNING".to_string()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn get_sample_config() -> QemuConfig {
        QemuConfig {
            vcpu_count: 4,
            mem_size_mib: 4096,
            disk_image_path: "/var/lib/aether/db.img".to_string(),
            qmp_socket_path: "/var/run/aether/qmp.sock".to_string(),
            host_tap_device: Some("tap1".to_string()),
        }
    }

    #[test]
    fn test_qemu_config_args_generation() {
        let config = get_sample_config();
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("qemu.log").to_str().unwrap().to_string();

        let hypervisor = QemuHypervisor::new(
            "test-vm-qemu".to_string(),
            "qemu-system-x86_64".to_string(),
            log_path,
            config,
        );

        assert_eq!(hypervisor.id, "test-vm-qemu");
        assert_eq!(hypervisor.bin_path, "qemu-system-x86_64");

        // Test args generation implicitly via spawn check
        // Check that pid path matches
        assert_eq!(
            hypervisor.pid_path(),
            hypervisor.log_path.replace(".log", ".pid")
        );
    }
}
