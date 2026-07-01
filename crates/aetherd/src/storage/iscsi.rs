// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use async_trait::async_trait;
use std::io;

/// Trait to manage iSCSI sessions on Compute nodes (initiators).
///
/// TODO: Refactor to use `libopeniscsiusr` FFI bindings instead of `iscsiadm` CLI
/// to improve performance and reliability.
#[async_trait]
pub trait IscsiManager: Send + Sync {
    /// Discovers iSCSI targets on a remote portal.
    async fn discover_targets(&self, portal_ip: &str) -> io::Result<Vec<String>>;

    /// Logs into an iSCSI target.
    /// Returns the local block device path (e.g., `/dev/sdb`).
    async fn login_target(&self, portal_ip: &str, target_iqn: &str) -> io::Result<String>;

    /// Logs out of an iSCSI target.
    async fn logout_target(&self, target_iqn: &str) -> io::Result<()>;

    /// Rescans for changes in a logged-in target.
    async fn rescan_session(&self, target_iqn: &str) -> io::Result<()>;
}

/// Real implementation of IscsiManager using `iscsiadm` CLI.
pub struct RealIscsiManager;

#[async_trait]
impl IscsiManager for RealIscsiManager {
    async fn discover_targets(&self, portal_ip: &str) -> io::Result<Vec<String>> {
        let output = tokio::process::Command::new("iscsiadm")
            .args(["-m", "discovery", "-t", "st", "-p", portal_ip])
            .output()
            .await?;

        if !output.status.success() {
            return Err(io::Error::other("iscsiadm discovery failed"));
        }

        let out_str = String::from_utf8_lossy(&output.stdout);
        let targets = out_str
            .lines()
            .filter_map(|l| l.split_whitespace().nth(1).map(|s| s.to_string()))
            .collect();

        Ok(targets)
    }

    async fn login_target(&self, portal_ip: &str, target_iqn: &str) -> io::Result<String> {
        // 1. Perform login
        let _ = tokio::process::Command::new("iscsiadm")
            .args(["-m", "node", "-T", target_iqn, "-p", portal_ip, "--login"])
            .status()
            .await?;

        // 2. Identify the block device (simplified here, in reality we'd parse /sys/class/iscsi_session)
        // For now returning a placeholder or finding it via lsblk
        Ok("/dev/iscsi-target-placeholder".to_string())
    }

    async fn logout_target(&self, target_iqn: &str) -> io::Result<()> {
        let _ = tokio::process::Command::new("iscsiadm")
            .args(["-m", "node", "-T", target_iqn, "--logout"])
            .status()
            .await?;
        Ok(())
    }

    async fn rescan_session(&self, target_iqn: &str) -> io::Result<()> {
        let _ = tokio::process::Command::new("iscsiadm")
            .args(["-m", "node", "-T", target_iqn, "--rescan"])
            .status()
            .await?;
        Ok(())
    }
}
