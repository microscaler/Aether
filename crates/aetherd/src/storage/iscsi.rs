// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use async_trait::async_trait;
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

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

// ---------------------------------------------------------------------------
// Sysfs helpers shared between RealIscsiManager and unit tests
// ---------------------------------------------------------------------------

/// Walk the SCSI device tree under an iSCSI session directory and return the
/// first block-device name found (e.g. `"sdb"`).
///
/// Expected sysfs layout (all path components are kernel-assigned names):
/// ```text
/// <session_path>/
///   target<H>:<C>:<T>/
///     <H>:<C>:<T>:<L>/
///       block/
///         <sdX>/   ← name returned
/// ```
async fn find_block_device_in_session(session_path: &Path) -> io::Result<String> {
    let mut dir = tokio::fs::read_dir(session_path).await?;
    while let Some(entry) = dir.next_entry().await? {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Target directories are named `target<H>:<C>:<T>` and always contain
        // at least one colon; the `targetname` attribute file does not.
        let is_target_dir = name_str.starts_with("target")
            && name_str.contains(':')
            && entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
        if is_target_dir {
            let mut lun_dir = tokio::fs::read_dir(entry.path()).await?;
            while let Some(lun_entry) = lun_dir.next_entry().await? {
                let block_path = lun_entry.path().join("block");
                if tokio::fs::metadata(&block_path).await.is_ok() {
                    let mut block_dir = tokio::fs::read_dir(&block_path).await?;
                    if let Some(dev_entry) = block_dir.next_entry().await? {
                        return Ok(dev_entry.file_name().to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No SCSI block device found under iSCSI session",
    ))
}

/// Scan `<sysfs_root>/class/iscsi_session/` for a session whose `targetname`
/// attribute matches `target_iqn`, then walk its SCSI device tree to locate
/// the corresponding block device.  Returns the full `/dev/<name>` path.
///
/// # Arguments
/// * `target_iqn`  – The iSCSI Qualified Name to look up (e.g.
///   `iqn.2024-01.com.example:vol1`).
/// * `sysfs_root`  – Root of the sysfs filesystem tree.  Pass `/sys` in
///   production; pass a `tempdir` path in unit tests so the function can
///   be exercised without root privileges or real iSCSI hardware.
///
/// # Returns
/// The absolute `/dev/<name>` path of the block device associated with the
/// session (e.g. `/dev/sdb`).
///
/// # Errors
/// Returns `io::ErrorKind::NotFound` when no session matching `target_iqn`
/// exists, or when the matching session has no visible block device yet.
/// Returns other `io::Error` variants on filesystem read failures.
pub async fn find_iscsi_block_device(target_iqn: &str, sysfs_root: &Path) -> io::Result<String> {
    let sessions_dir = sysfs_root.join("class/iscsi_session");
    let mut dir = tokio::fs::read_dir(&sessions_dir).await?;

    while let Some(entry) = dir.next_entry().await? {
        let session_path = entry.path();
        let targetname_path = session_path.join("targetname");

        match tokio::fs::read_to_string(&targetname_path).await {
            Ok(iqn) if iqn.trim() == target_iqn => {
                if let Ok(dev) = find_block_device_in_session(&session_path).await {
                    return Ok(format!("/dev/{}", dev));
                }
            }
            _ => continue,
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("No block device found for iSCSI target: {}", target_iqn),
    ))
}

// ---------------------------------------------------------------------------
// Real implementation
// ---------------------------------------------------------------------------

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
        // 1. Perform login via iscsiadm.
        //    Exit code 0  → success
        //    Exit code 15 → ISCSI_ERR_SESS_EXISTS (already logged in) – treat as success
        let status = tokio::process::Command::new("iscsiadm")
            .args(["-m", "node", "-T", target_iqn, "-p", portal_ip, "--login"])
            .status()
            .await?;

        let code = status.code().unwrap_or(-1);
        if !status.success() && code != 15 {
            return Err(io::Error::other(format!(
                "iscsiadm login failed with exit code {}",
                code
            )));
        }

        // 2. Poll sysfs until udev has created the block device node.
        //    A short sleep between retries is sufficient for typical systems.
        const MAX_RETRIES: u32 = 10;
        const RETRY_DELAY_MS: u64 = 200;
        let sysfs_root = Path::new("/sys");

        for attempt in 0..MAX_RETRIES {
            if let Ok(dev_path) = find_iscsi_block_device(target_iqn, sysfs_root).await {
                return Ok(dev_path);
            }
            if attempt + 1 < MAX_RETRIES {
                log::debug!(
                    "Waiting for block device for IQN {} (attempt {}/{}) …",
                    target_iqn,
                    attempt + 1,
                    MAX_RETRIES
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }

        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "Block device not found for iSCSI target '{}' after {}ms",
                target_iqn,
                MAX_RETRIES as u64 * RETRY_DELAY_MS
            ),
        ))
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

// ---------------------------------------------------------------------------
// Mock implementation for unit/integration tests
// ---------------------------------------------------------------------------

struct MockIscsiState {
    /// Active sessions: target_iqn → block device path
    sessions: HashMap<String, String>,
    /// Discoverable targets per portal: portal_ip → Vec<IQN>
    targets: HashMap<String, Vec<String>>,
    /// Monotone counter used to produce unique mock device paths
    device_counter: usize,
}

/// Simulated iSCSI manager for unit tests and CI environments that lack real
/// iSCSI hardware.  Follows the same pattern as `MockZvolManager`.
pub struct MockIscsiManager {
    state: Arc<Mutex<MockIscsiState>>,
}

impl MockIscsiManager {
    /// Create a new mock with no pre-configured portals or sessions.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockIscsiState {
                sessions: HashMap::new(),
                targets: HashMap::new(),
                device_counter: 0,
            })),
        }
    }

    /// Pre-configure a list of IQNs that will be returned by `discover_targets`
    /// for `portal_ip`.
    pub async fn add_portal_targets(&self, portal_ip: &str, iqns: Vec<String>) {
        let mut state = self.state.lock().await;
        state.targets.insert(portal_ip.to_string(), iqns);
    }
}

impl Default for MockIscsiManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a monotone counter into a Linux-style sd-device suffix.
///
/// Follows the kernel naming convention (skipping `a` to avoid `sda`):
/// `0` → `"b"`, `1` → `"c"`, …, `24` → `"z"`,
/// `25` → `"aa"`, `26` → `"ab"`, …
/// The mapping is injective for all `u32` values so device names are
/// always unique regardless of logout/login cycling.
fn counter_to_sdname(n: usize) -> String {
    const ALPHABET: &[u8; 26] = b"abcdefghijklmnopqrstuvwxyz";
    // Shift by 1 so index 0 produces 'b' instead of 'a'.
    let mut val = n + 1;
    let mut chars: Vec<char> = Vec::new();
    loop {
        chars.push(char::from(ALPHABET[val % 26]));
        val /= 26;
        if val == 0 {
            break;
        }
        val -= 1; // adjust for the 1-indexed shift
    }
    chars.reverse();
    chars.into_iter().collect()
}

#[async_trait]
impl IscsiManager for MockIscsiManager {
    async fn discover_targets(&self, portal_ip: &str) -> io::Result<Vec<String>> {
        let state = self.state.lock().await;
        Ok(state.targets.get(portal_ip).cloned().unwrap_or_default())
    }

    async fn login_target(&self, _portal_ip: &str, target_iqn: &str) -> io::Result<String> {
        let mut state = self.state.lock().await;
        // Return existing device if already logged in (idempotent)
        if let Some(path) = state.sessions.get(target_iqn) {
            return Ok(path.clone());
        }
        // Generate a unique device name following Linux sd-naming conventions:
        // 0 → sdb, 1 → sdc, …, 24 → sdz, 25 → sdaa, 26 → sdab, …
        // The counter is strictly monotone so names never collide even after
        // repeated logout/login cycles on many volumes.
        let dev_name = format!("/dev/sd{}", counter_to_sdname(state.device_counter));
        state.device_counter += 1;
        state
            .sessions
            .insert(target_iqn.to_string(), dev_name.clone());
        Ok(dev_name)
    }

    async fn logout_target(&self, target_iqn: &str) -> io::Result<()> {
        let mut state = self.state.lock().await;
        state.sessions.remove(target_iqn);
        Ok(())
    }

    async fn rescan_session(&self, _target_iqn: &str) -> io::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use tokio::fs;

    // ------------------------------------------------------------------
    // MockIscsiManager tests
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_mock_discover_returns_configured_targets() {
        let mgr = MockIscsiManager::new();
        mgr.add_portal_targets(
            "10.0.0.1",
            vec![
                "iqn.2024-01.com.example:vol1".to_string(),
                "iqn.2024-01.com.example:vol2".to_string(),
            ],
        )
        .await;

        let targets = mgr
            .discover_targets("10.0.0.1")
            .await
            .expect("discover_targets failed");
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&"iqn.2024-01.com.example:vol1".to_string()));
    }

    #[tokio::test]
    async fn test_mock_discover_unknown_portal_returns_empty() {
        let mgr = MockIscsiManager::new();
        let targets = mgr
            .discover_targets("192.168.99.99")
            .await
            .expect("discover_targets failed");
        assert!(targets.is_empty());
    }

    #[tokio::test]
    async fn test_mock_login_returns_device_path() {
        let mgr = MockIscsiManager::new();
        let path = mgr
            .login_target("10.0.0.1", "iqn.2024-01.com.example:vol1")
            .await
            .expect("login_target failed");

        assert!(
            path.starts_with("/dev/sd"),
            "Expected /dev/sdX, got: {}",
            path
        );
    }

    #[tokio::test]
    async fn test_mock_login_is_idempotent() {
        let mgr = MockIscsiManager::new();
        let iqn = "iqn.2024-01.com.example:vol1";
        let path1 = mgr
            .login_target("10.0.0.1", iqn)
            .await
            .expect("first login failed");
        let path2 = mgr
            .login_target("10.0.0.1", iqn)
            .await
            .expect("second login failed");

        assert_eq!(path1, path2, "Repeated login should return the same device");
    }

    #[tokio::test]
    async fn test_mock_login_assigns_unique_devices_for_distinct_iqns() {
        let mgr = MockIscsiManager::new();
        let path1 = mgr
            .login_target("10.0.0.1", "iqn.2024-01.com.example:vol1")
            .await
            .expect("login vol1 failed");
        let path2 = mgr
            .login_target("10.0.0.1", "iqn.2024-01.com.example:vol2")
            .await
            .expect("login vol2 failed");

        assert_ne!(path1, path2, "Different IQNs must get different devices");
    }

    #[tokio::test]
    async fn test_mock_logout_removes_session() {
        let mgr = MockIscsiManager::new();
        let iqn = "iqn.2024-01.com.example:vol1";
        let path_before = mgr
            .login_target("10.0.0.1", iqn)
            .await
            .expect("login failed");

        mgr.logout_target(iqn).await.expect("logout failed");

        // After logout, a new login allocates the next counter slot so the
        // device path must differ from the one assigned before logout.
        let path_after = mgr
            .login_target("10.0.0.1", iqn)
            .await
            .expect("re-login after logout failed");

        assert!(
            path_after.starts_with("/dev/sd"),
            "Expected /dev/sdX after re-login, got: {}",
            path_after
        );
        assert_ne!(
            path_before, path_after,
            "Re-login after logout should allocate a new device path"
        );
    }

    #[tokio::test]
    async fn test_mock_rescan_is_no_op() {
        let mgr = MockIscsiManager::new();
        mgr.rescan_session("iqn.2024-01.com.example:vol1")
            .await
            .expect("rescan_session should not fail");
    }

    // ------------------------------------------------------------------
    // find_iscsi_block_device sysfs parsing tests
    // ------------------------------------------------------------------

    /// Build a fake sysfs tree under `root`:
    /// ```text
    /// <root>/class/iscsi_session/
    ///   session0/
    ///     targetname          ← "iqn.2024-01.com.example:vol1"
    ///     target0:0:0/
    ///       0:0:0:0/
    ///         block/
    ///           sdb/
    ///   session1/
    ///     targetname          ← "iqn.2024-01.com.example:vol2"
    ///     target1:0:0/
    ///       1:0:0:0/
    ///         block/
    ///           sdc/
    /// ```
    async fn build_fake_sysfs(root: &Path) {
        for (idx, (iqn, dev)) in [
            ("iqn.2024-01.com.example:vol1", "sdb"),
            ("iqn.2024-01.com.example:vol2", "sdc"),
        ]
        .iter()
        .enumerate()
        {
            let session = format!("session{}", idx);
            let base = root.join("class").join("iscsi_session").join(&session);
            fs::create_dir_all(&base).await.expect("mkdir session");
            fs::write(base.join("targetname"), iqn)
                .await
                .expect("write targetname");

            let target_dir = format!("target{}:0:0", idx);
            let lun_dir = format!("{}:0:0:0", idx);
            let block_dev = base
                .join(&target_dir)
                .join(&lun_dir)
                .join("block")
                .join(dev);
            fs::create_dir_all(&block_dev)
                .await
                .expect("mkdir block device");
        }
    }

    #[tokio::test]
    async fn test_find_iscsi_block_device_finds_first_session() {
        let dir = tempfile::tempdir().expect("tempdir");
        build_fake_sysfs(dir.path()).await;

        let result = find_iscsi_block_device("iqn.2024-01.com.example:vol1", dir.path()).await;
        assert_eq!(result.expect("should find vol1"), "/dev/sdb");
    }

    #[tokio::test]
    async fn test_find_iscsi_block_device_finds_second_session() {
        let dir = tempfile::tempdir().expect("tempdir");
        build_fake_sysfs(dir.path()).await;

        let result = find_iscsi_block_device("iqn.2024-01.com.example:vol2", dir.path()).await;
        assert_eq!(result.expect("should find vol2"), "/dev/sdc");
    }

    #[tokio::test]
    async fn test_find_iscsi_block_device_not_found_returns_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        build_fake_sysfs(dir.path()).await;

        let result =
            find_iscsi_block_device("iqn.2024-01.com.example:nonexistent", dir.path()).await;
        assert!(result.is_err(), "Should fail for unknown IQN");
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_find_iscsi_block_device_missing_sysfs_root() {
        let result =
            find_iscsi_block_device("iqn.2024-01.com.example:vol1", Path::new("/nonexistent"))
                .await;
        assert!(result.is_err(), "Should fail when sysfs root doesn't exist");
    }
}
