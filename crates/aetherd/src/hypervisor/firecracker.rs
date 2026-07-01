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

/// Configuration for the Firecracker jailer sandbox.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JailerConfig {
    /// UID that Firecracker will run as.
    pub uid: u32,
    /// GID that Firecracker will run as.
    pub gid: u32,
    /// Directory where the chroot jail will be built.
    pub chroot_base_dir: String,
    /// NUMA node index where the process will be pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_index: Option<u32>,
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
    /// Optional jailer sandbox configuration.
    pub jailer_config: Option<JailerConfig>,
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
            jailer_config: None,
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
            if let Some(ref jc) = self.jailer_config {
                let mut jailer_args: Vec<String> = vec![
                    "--id".to_string(),
                    self.id.clone(),
                    "--exec-file".to_string(),
                    self.bin_path.clone(),
                    "--uid".to_string(),
                    jc.uid.to_string(),
                    "--gid".to_string(),
                    jc.gid.to_string(),
                    "--chroot-base-dir".to_string(),
                    jc.chroot_base_dir.clone(),
                    "--".to_string(),
                    "--config-file".to_string(),
                    self.config_path.clone(),
                ];
                // node_index was removed from jailer in newer versions;
                // only pass it if set (for forward/backward compat)
                if let Some(node) = jc.node_index {
                    jailer_args.push("--node".to_string());
                    jailer_args.push(node.to_string());
                }
                jailer_args
            } else {
                vec!["--config-file".to_string(), self.config_path.clone()]
            }
        } else {
            self.extra_args.clone()
        };

        let cmd_bin = if self.jailer_config.is_some() {
            "jailer"
        } else {
            &self.bin_path
        };

        let mut cmd = tokio::process::Command::new(cmd_bin);
        cmd.args(&args)
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_stderr));

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn child process '{}': {}", cmd_bin, e))?;

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

        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        let nix_pid = Pid::from_raw(pid as i32);

        // Dispatch SIGTERM
        let _ = kill(nix_pid, Signal::SIGTERM);

        // Wait up to 500ms for process to exit
        for _ in 0..10 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if self.query_status().await? == "STOPPED" {
                let _ = tokio::fs::remove_file(&pid_path).await;
                return Ok(());
            }
        }

        // Force SIGKILL
        let _ = kill(nix_pid, Signal::SIGKILL);

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

        let pid: u32 = pid_trim
            .parse()
            .map_err(|e| format!("Failed to parse PID: {}", e))?;

        use nix::sys::signal::kill;
        use nix::unistd::Pid;
        let nix_pid = Pid::from_raw(pid as i32);

        // Run kill(pid, None) to verify process exists
        match kill(nix_pid, None) {
            Ok(_) => Ok("RUNNING".to_string()),
            Err(nix::errno::Errno::EPERM) => Ok("RUNNING".to_string()),
            _ => Ok("STOPPED".to_string()),
        }
    }

    fn get_qmp_socket_path(&self) -> Option<String> {
        None
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

    #[test]
    fn test_jailer_config_args() {
        let config = get_sample_config();
        let mut hypervisor = FirecrackerHypervisor::new(
            "test-vm".to_string(),
            "/usr/bin/firecracker".to_string(),
            "vm.json".to_string(),
            "vm.log".to_string(),
            config,
        );

        let jc = JailerConfig {
            uid: 1000,
            gid: 1000,
            chroot_base_dir: "/srv/jailer".to_string(),
            node_index: Some(0),
        };
        hypervisor.jailer_config = Some(jc);

        assert_eq!(hypervisor.id, "test-vm");
        assert_eq!(hypervisor.bin_path, "/usr/bin/firecracker");
        assert_eq!(hypervisor.jailer_config.as_ref().unwrap().uid, 1000);
        assert_eq!(
            hypervisor.jailer_config.as_ref().unwrap().chroot_base_dir,
            "/srv/jailer"
        );
    }

    /// Test spawn with jailer_config set — exercises the jailer binary path
    /// and the full jailer CLI args vector (lines 158-174).
    /// Since jailer v1.10.0 requires the exec-file to contain "firecracker",
    /// we test with a minimal config — spawn() succeeds (PID file written)
    /// but firecracker exits before we can query it.
    #[tokio::test]
    async fn test_firecracker_spawn_with_jailer() {
        let dir = tempdir().unwrap();
        let config_path = dir
            .path()
            .join("vm-jailer.json")
            .to_str()
            .unwrap()
            .to_string();
        let log_path = dir
            .path()
            .join("vm-jailer.log")
            .to_str()
            .unwrap()
            .to_string();
        let chroot = dir.path().join("chroot");
        std::fs::create_dir_all(&chroot).unwrap();

        let mut config = get_sample_config();
        config.network_interfaces.clear();

        let jailer_config = JailerConfig {
            uid: 1000,
            gid: 1000,
            chroot_base_dir: chroot.to_str().unwrap().to_string(),
            node_index: None,
        };

        let mut hypervisor = FirecrackerHypervisor::new(
            "jailer-vm".to_string(),
            "/usr/local/bin/firecracker".to_string(),
            config_path.clone(),
            log_path.clone(),
            config,
        );

        hypervisor.jailer_config = Some(jailer_config);

        // spawn() should succeed — the PID file is written even if
        // firecracker exits immediately with a bad config
        let result = hypervisor.spawn().await;
        assert!(result.is_ok(), "spawn failed: {:?}", result.err());

        // Verify PID file was created (proves jailer CLI args path was exercised)
        let pid_path = hypervisor.pid_path();
        assert!(Path::new(&pid_path).exists(), "PID file not created");

        // Give firecracker a moment to exit
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // stop() should handle the exited process gracefully
        hypervisor.stop().await.unwrap();
    }

    /// Test stop when PID file doesn't exist — returns Ok early (line 213).
    #[tokio::test]
    async fn test_stop_no_pid_file() {
        let config = get_sample_config();
        let hypervisor = FirecrackerHypervisor::new(
            "no-pid-vm".to_string(),
            "/usr/bin/firecracker".to_string(),
            "/tmp/nonexistent_aether_test.pid.json".to_string(),
            "/tmp/nonexistent_aether_test.log".to_string(),
            config,
        );

        // stop() should return Ok because PID file doesn't exist
        let result = hypervisor.stop().await;
        assert!(result.is_ok());
    }

    /// Test query_status when PID file exists but PID is empty (line 262).
    #[tokio::test]
    async fn test_query_status_empty_pid_file() {
        let dir = tempdir().unwrap();
        let pid_path = dir.path().join("empty.pid");
        std::fs::write(&pid_path, "   ").unwrap();

        let config = get_sample_config();
        let hypervisor = FirecrackerHypervisor::new(
            "empty-pid-vm".to_string(),
            "/usr/bin/firecracker".to_string(),
            pid_path.to_str().unwrap().to_string(),
            dir.path().join("empty.log").to_str().unwrap().to_string(),
            config,
        );

        assert_eq!(hypervisor.query_status().await.unwrap(), "STOPPED");
    }

    /// Test config write failure — write to an inaccessible directory (line 138).
    #[tokio::test]
    async fn test_spawn_config_write_error() {
        let config = get_sample_config();
        // Use /proc/nonexistent which has no write permission
        let hypervisor = FirecrackerHypervisor::new(
            "error-vm".to_string(),
            "/usr/bin/firecracker".to_string(),
            "/proc/1/aether_config_error_test.json".to_string(),
            "/tmp/aether_error_test.log".to_string(),
            config,
        );

        let result = hypervisor.spawn().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to write config file"));
    }
}
